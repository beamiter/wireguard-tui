use crate::commands::CommandExecutor;
use crate::config::{ConfigDetails, ConfigManager};
use crate::download::{ConfigDownloader, ImportReport};
use crate::vpn::{VpnManager, VpnStatus};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

const NOTICE_TTL: Duration = Duration::from_secs(5);
const STATUS_REFRESH_INTERVAL: Duration = Duration::from_secs(3);
const STATUS_ERROR_RETRY_INTERVAL: Duration = Duration::from_secs(30);
const DETAILS_DEBOUNCE: Duration = Duration::from_millis(120);
const SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Main,
    Download,
    Import,
    Status,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    None,
    Info(String),
    Error(String),
    Success(String),
}

impl Message {
    pub fn text(&self) -> &str {
        match self {
            Self::None => "Ready",
            Self::Info(message) | Self::Error(message) | Self::Success(message) => message,
        }
    }
}

/// The single source of truth for connection state shown by the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Unknown,
    Disconnected,
    Connecting(String),
    Connected(String),
    Disconnecting(String),
    Degraded(String),
    /// The kernel reports multiple interfaces, or one that is not managed by
    /// the current configuration list. No single server may be implied.
    Ambiguous(Vec<String>),
}

impl ConnectionState {
    pub fn server(&self) -> Option<&str> {
        match self {
            Self::Unknown | Self::Disconnected => None,
            Self::Connecting(server)
            | Self::Connected(server)
            | Self::Disconnecting(server)
            | Self::Degraded(server) => Some(server),
            Self::Ambiguous(_) => None,
        }
    }

    pub fn is_up(&self) -> bool {
        matches!(
            self,
            Self::Connected(_) | Self::Disconnecting(_) | Self::Degraded(_) | Self::Ambiguous(_)
        )
    }

    pub fn contains(&self, server: &str) -> bool {
        self.server() == Some(server)
            || matches!(self, Self::Ambiguous(servers) if servers.iter().any(|active| active == server))
    }
}

enum OperationOutput {
    Connected {
        server: String,
        status: VpnStatus,
        status_error: Option<String>,
        active_interfaces: Vec<String>,
    },
    Disconnected {
        server: String,
        active_interfaces: Vec<String>,
    },
    Deleted {
        server: String,
        servers: std::result::Result<Vec<String>, String>,
        active_interfaces: Option<Vec<String>>,
    },
    Imported {
        report: ImportReport,
        servers: std::result::Result<Vec<String>, String>,
    },
    Refreshed(Vec<String>),
    Status {
        server: String,
        status: VpnStatus,
        ip: Option<String>,
    },
}

struct PendingOperation {
    label: String,
    connection_may_change: bool,
    fallback: ConnectionState,
    task: JoinHandle<Result<OperationOutput>>,
}

struct StatusSnapshot {
    active_interfaces: Vec<String>,
    managed_server: Option<String>,
    status: Option<VpnStatus>,
    status_error: Option<String>,
}

struct DetailsTask {
    server: String,
    task: JoinHandle<Result<ConfigDetails>>,
}

pub struct App {
    pub current_screen: Screen,
    /// Index within the filtered server list, not directly within `servers`.
    pub selected_index: usize,
    pub servers: Vec<String>,
    pub connection: ConnectionState,
    pub status: VpnStatus,
    pub status_stale: bool,
    pub status_error: Option<String>,
    pub current_ip: Option<String>,
    pub message: Message,
    pub import_configs: Vec<PathBuf>,
    pub import_display: Vec<String>,
    pub import_selected: usize,
    pub import_checked: Vec<bool>,
    pub selected_details: Option<ConfigDetails>,
    pub selected_details_error: Option<String>,
    pub search_query: String,
    pub search_active: bool,
    pub download_scroll: u16,
    pub status_scroll: u16,
    pub pending_delete: Option<String>,
    pub spinner_frame: usize,
    pub config_manager: ConfigManager,
    pub downloader: ConfigDownloader,
    pending_operation: Option<PendingOperation>,
    status_task: Option<JoinHandle<Result<StatusSnapshot>>>,
    details_task: Option<DetailsTask>,
    details_request: Option<(String, Instant)>,
    last_status_poll: Option<Instant>,
    status_poll_failed: bool,
    message_time: Option<Instant>,
}

impl App {
    pub async fn new() -> Result<Self> {
        let config_manager = ConfigManager::new()?;
        config_manager.load_config()?;

        let (mut servers, list_error) = match config_manager.list_configs() {
            Ok(servers) => (servers, None),
            Err(error) => (Vec::new(), Some(error.to_string())),
        };
        servers.sort_by_key(|name| name.to_ascii_lowercase());

        let (connection, status, status_error) = Self::initial_connection(&servers).await;
        let status_stale = status_error.is_some();

        let initial_message = if !CommandExecutor::check_wireguard_installed().unwrap_or(false) {
            Message::Error(
                "WireGuard tools are not installed; install wg and wg-quick before connecting"
                    .to_string(),
            )
        } else if let Some(error) = list_error {
            Message::Error(format!("Unable to read WireGuard configs: {error}"))
        } else {
            Message::None
        };

        let mut app = Self {
            current_screen: Screen::Main,
            selected_index: 0,
            servers,
            connection,
            status,
            status_stale,
            status_error,
            current_ip: None,
            message: initial_message,
            import_configs: Vec::new(),
            import_display: Vec::new(),
            import_selected: 0,
            import_checked: Vec::new(),
            selected_details: None,
            selected_details_error: None,
            search_query: String::new(),
            search_active: false,
            download_scroll: 0,
            status_scroll: 0,
            pending_delete: None,
            spinner_frame: 0,
            config_manager,
            downloader: ConfigDownloader::new(),
            pending_operation: None,
            status_task: None,
            details_task: None,
            details_request: None,
            last_status_poll: None,
            status_poll_failed: false,
            message_time: Some(Instant::now()),
        };

        app.select_connected_server();
        app.refresh_selected_details();
        Ok(app)
    }

    async fn initial_connection(
        servers: &[String],
    ) -> (ConnectionState, VpnStatus, Option<String>) {
        match VpnManager::get_active_connections().await {
            Ok(active) => {
                if active.is_empty() {
                    return (ConnectionState::Disconnected, VpnStatus::default(), None);
                }
                let Some(server) = Self::single_managed_active(&active, servers) else {
                    let diagnostic = Self::ambiguous_active_diagnostic(&active);
                    return (
                        ConnectionState::Ambiguous(active),
                        VpnStatus::default(),
                        Some(diagnostic),
                    );
                };

                match VpnManager::get_status(&server).await {
                    Ok(status) => (ConnectionState::Connected(server), status, None),
                    Err(error) => (
                        ConnectionState::Degraded(server),
                        VpnStatus::default(),
                        Some(error.to_string()),
                    ),
                }
            }
            Err(error) => (
                ConnectionState::Unknown,
                VpnStatus::default(),
                Some(error.to_string()),
            ),
        }
    }

    pub fn set_startup_warning(&mut self, warning: impl Into<String>) {
        self.set_error(warning);
    }

    pub fn is_busy(&self) -> bool {
        self.pending_operation.is_some()
    }

    pub fn operation_label(&self) -> Option<&str> {
        self.pending_operation
            .as_ref()
            .map(|operation| operation.label.as_str())
    }

    pub fn visible_server_indices(&self) -> Vec<usize> {
        let query = self.search_query.trim().to_ascii_lowercase();
        self.servers
            .iter()
            .enumerate()
            .filter_map(|(index, server)| {
                (query.is_empty() || server.to_ascii_lowercase().contains(&query)).then_some(index)
            })
            .collect()
    }

    pub fn selected_server(&self) -> Option<&str> {
        let indices = self.visible_server_indices();
        indices
            .get(self.selected_index)
            .and_then(|index| self.servers.get(*index))
            .map(String::as_str)
    }

    pub fn handle_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.refresh_selected_details();
        }
    }

    pub fn handle_down(&mut self) {
        let last = self.visible_server_indices().len().saturating_sub(1);
        if self.selected_index < last {
            self.selected_index += 1;
            self.refresh_selected_details();
        }
    }

    pub fn handle_home(&mut self) {
        self.selected_index = 0;
        self.refresh_selected_details();
    }

    pub fn handle_end(&mut self) {
        self.selected_index = self.visible_server_indices().len().saturating_sub(1);
        self.refresh_selected_details();
    }

    pub fn handle_page_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(10);
        self.refresh_selected_details();
    }

    pub fn handle_page_down(&mut self) {
        let last = self.visible_server_indices().len().saturating_sub(1);
        self.selected_index = self.selected_index.saturating_add(10).min(last);
        self.refresh_selected_details();
    }

    pub fn begin_search(&mut self) {
        self.search_active = true;
    }

    pub fn push_search(&mut self, character: char) {
        if !character.is_control() {
            self.search_query.push(character);
            self.selected_index = 0;
            self.refresh_selected_details();
        }
    }

    pub fn pop_search(&mut self) {
        self.search_query.pop();
        self.selected_index = 0;
        self.refresh_selected_details();
    }

    pub fn finish_search(&mut self) {
        self.search_active = false;
    }

    pub fn clear_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.selected_index = 0;
        self.refresh_selected_details();
    }

    pub fn handle_enter(&mut self) {
        if self.is_busy() {
            self.set_info("Another operation is already in progress");
            return;
        }

        let Some(server) = self.selected_server().map(str::to_owned) else {
            self.set_error("No matching server configuration is available");
            return;
        };

        if matches!(self.connection, ConnectionState::Unknown) {
            self.set_error("Connection state is unknown; refresh before changing interfaces");
            self.last_status_poll = None;
            return;
        }

        if let ConnectionState::Ambiguous(active) = &self.connection {
            if active.iter().any(|name| name == &server) {
                self.start_disconnect(server);
            } else {
                self.set_error(format!(
                    "Active interfaces must be resolved first: {}",
                    active.join(", ")
                ));
            }
            return;
        }

        if self.connection.contains(&server) && self.connection.is_up() {
            self.start_disconnect(server);
        } else {
            self.start_connect(server);
        }
    }

    fn start_connect(&mut self, server: String) {
        self.cancel_status_refresh();
        let fallback = self.connection.clone();
        let previous = self
            .connection
            .server()
            .filter(|_| self.connection.is_up())
            .map(str::to_owned);
        let expected_active: Vec<String> = previous.iter().cloned().collect();
        self.connection = ConnectionState::Connecting(server.clone());
        self.current_ip = None;
        self.status = VpnStatus::default();
        self.status_stale = true;
        self.status_error = None;

        let task_server = server.clone();
        let config_manager = self.config_manager.clone();
        let validation_server = task_server.clone();
        let validation_previous = previous.clone();
        let task = tokio::spawn(async move {
            tokio::task::spawn_blocking(move || -> Result<()> {
                config_manager
                    .validate_config(&validation_server)
                    .with_context(|| {
                        format!("refusing unsafe configuration {validation_server}")
                    })?;
                if let Some(previous_server) = validation_previous.as_deref() {
                    if previous_server != validation_server {
                        config_manager
                            .validate_config(previous_server)
                            .with_context(|| {
                                format!(
                                    "refusing to switch because active config {previous_server} is unsafe"
                                )
                            })?;
                    }
                }
                Ok(())
            })
            .await
            .context("configuration validator stopped unexpectedly")??;

            let observed = VpnManager::get_active_connections()
                .await
                .context("unable to verify active interfaces before connecting")?;
            if !Self::same_active_set(&observed, &expected_active) {
                anyhow::bail!(
                    "active interfaces changed before connecting (expected: {}; observed: {})",
                    Self::format_active_set(&expected_active),
                    Self::format_active_set(&observed)
                );
            }

            if let Some(previous_server) = previous.as_deref() {
                if previous_server != task_server {
                    VpnManager::disconnect(previous_server)
                        .await
                        .with_context(|| format!("failed to disconnect {previous_server}"))?;
                }
            }

            if let Err(connect_error) = VpnManager::connect(&task_server).await {
                // A killed or failed `wg-quick up` can leave a partially configured interface.
                // Never restore the previous tunnel until the target is confirmed absent.
                let cleanup_result = async {
                    let active = VpnManager::get_active_connections()
                        .await
                        .context("unable to determine whether the failed target is still active")?;
                    if active.iter().any(|name| name == &task_server) {
                        VpnManager::disconnect(&task_server)
                            .await
                            .context("failed to clean the partially connected target")?;
                        let remaining = VpnManager::get_active_connections()
                            .await
                            .context("unable to verify target cleanup")?;
                        if remaining.iter().any(|name| name == &task_server) {
                            anyhow::bail!("target remained active after cleanup");
                        }
                    }
                    Ok::<(), anyhow::Error>(())
                }
                .await;

                if let Err(cleanup_error) = cleanup_result {
                    return Err(connect_error).context(format!(
                        "failed to connect {task_server}; target state is uncertain and the previous interface was not restored: {cleanup_error:#}"
                    ));
                }

                if let Some(previous_server) = previous.as_deref() {
                    if previous_server != task_server {
                        let rollback = VpnManager::connect(previous_server).await;
                        return match rollback {
                            Ok(_) => {
                                let active = VpnManager::get_active_connections().await;
                                match active {
                                    Ok(active)
                                        if active.iter().any(|name| name == previous_server)
                                            && !active.iter().any(|name| name == &task_server) =>
                                    {
                                        Err(connect_error).context(format!(
                                            "failed to connect {task_server}; restored {previous_server}"
                                        ))
                                    }
                                    Ok(active) => Err(connect_error).context(format!(
                                        "failed to connect {task_server}; rollback returned success but active interfaces are: {}",
                                        active.join(", ")
                                    )),
                                    Err(error) => Err(connect_error).context(format!(
                                        "failed to connect {task_server}; rollback returned success but verification failed: {error}"
                                    )),
                                }
                            }
                            Err(rollback_error) => Err(connect_error).context(format!(
                                "failed to connect {task_server}; rollback to {previous_server} also failed: {rollback_error}"
                            )),
                        };
                    }
                }
                return Err(connect_error);
            }

            let active_interfaces = VpnManager::get_active_connections()
                .await
                .context("unable to verify active interfaces after connecting")?;

            let (status, status_error) = match VpnManager::get_status(&task_server).await {
                Ok(status) => (status, None),
                Err(error) => (
                    VpnStatus {
                        interface: task_server.clone(),
                        ..VpnStatus::default()
                    },
                    Some(error.to_string()),
                ),
            };
            Ok(OperationOutput::Connected {
                server: task_server,
                status,
                status_error,
                active_interfaces,
            })
        });

        self.pending_operation = Some(PendingOperation {
            label: format!("Connecting to {server}"),
            connection_may_change: true,
            fallback,
            task,
        });
    }

    fn start_disconnect(&mut self, server: String) {
        self.cancel_status_refresh();
        let fallback = self.connection.clone();
        self.connection = ConnectionState::Disconnecting(server.clone());
        let task_server = server.clone();
        let config_manager = self.config_manager.clone();
        let validation_server = task_server.clone();
        let task = tokio::spawn(async move {
            tokio::task::spawn_blocking(move || config_manager.validate_config(&validation_server))
                .await
                .context("configuration validator stopped unexpectedly")?
                .with_context(|| format!("refusing unsafe configuration {task_server}"))?;
            VpnManager::disconnect(&task_server).await?;
            let active_interfaces = VpnManager::get_active_connections()
                .await
                .context("unable to verify active interfaces after disconnecting")?;
            Ok(OperationOutput::Disconnected {
                server: task_server,
                active_interfaces,
            })
        });

        self.pending_operation = Some(PendingOperation {
            label: format!("Disconnecting {server}"),
            connection_may_change: true,
            fallback,
            task,
        });
    }

    pub fn show_download(&mut self) {
        self.current_screen = Screen::Download;
        self.download_scroll = 0;
    }

    pub fn open_download_page(&mut self) {
        match self.downloader.open_in_browser() {
            Ok(()) => self.set_success("Opened the download page in your browser"),
            Err(error) => self.set_error(format!("Unable to open browser: {error}")),
        }
    }

    pub fn handle_import(&mut self) {
        if self.is_busy() {
            self.set_info("Wait for the current operation to finish");
            return;
        }

        self.current_screen = Screen::Import;
        match self.downloader.scan_downloads() {
            Ok(configs) => {
                let count = configs.len();
                self.import_display = configs
                    .iter()
                    .map(|path| ConfigDownloader::format_config_info(path))
                    .collect();
                self.import_configs = configs;
                self.import_checked = vec![true; count];
                self.import_selected = 0;
                if count == 0 {
                    self.set_info(format!(
                        "No safe .conf files found in {}",
                        self.downloader.downloads_dir().display()
                    ));
                } else {
                    self.set_info(format!("Found {count} configuration file(s)"));
                }
            }
            Err(error) => {
                self.import_configs.clear();
                self.import_display.clear();
                self.import_checked.clear();
                self.set_error(format!("Unable to scan downloads: {error}"));
            }
        }
    }

    pub fn handle_import_up(&mut self) {
        self.import_selected = self.import_selected.saturating_sub(1);
    }

    pub fn handle_import_down(&mut self) {
        self.import_selected = self
            .import_selected
            .saturating_add(1)
            .min(self.import_configs.len().saturating_sub(1));
    }

    pub fn handle_toggle_check(&mut self) {
        if let Some(checked) = self.import_checked.get_mut(self.import_selected) {
            *checked = !*checked;
        }
    }

    pub fn set_all_imports(&mut self, checked: bool) {
        self.import_checked.fill(checked);
    }

    pub fn handle_import_selected(&mut self) {
        if self.is_busy() {
            self.set_info("Another operation is already in progress");
            return;
        }

        let selected_paths: Vec<_> = self
            .import_configs
            .iter()
            .zip(&self.import_checked)
            .filter_map(|(path, checked)| checked.then_some(path.clone()))
            .collect();

        if selected_paths.is_empty() {
            self.set_error("Select at least one configuration to import");
            return;
        }

        self.cancel_status_refresh();

        let downloader = self.downloader.clone();
        let config_manager = self.config_manager.clone();
        let target_dir = self.config_manager.get_wg_config_dir().to_path_buf();
        let fallback = self.connection.clone();
        let count = selected_paths.len();
        let task = tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                let report = downloader.import_configs(&selected_paths, &target_dir)?;
                let servers = config_manager
                    .list_configs()
                    .map_err(|error| format!("{error:#}"));
                Ok(OperationOutput::Imported { report, servers })
            })
            .await
            .context("import worker stopped unexpectedly")?
        });

        self.pending_operation = Some(PendingOperation {
            label: format!("Importing {count} configuration(s)"),
            connection_may_change: false,
            fallback,
            task,
        });
    }

    pub fn request_delete(&mut self) {
        if self.is_busy() {
            self.set_info("Wait for the current operation to finish");
            return;
        }

        let Some(server) = self.selected_server().map(str::to_owned) else {
            self.set_error("No server is selected");
            return;
        };
        self.pending_delete = Some(server);
    }

    pub fn cancel_delete(&mut self) {
        self.pending_delete = None;
        self.set_info("Delete cancelled");
    }

    pub fn confirm_delete(&mut self) {
        let Some(server) = self.pending_delete.take() else {
            return;
        };
        if self.is_busy() {
            self.set_info("Wait for the current operation to finish");
            return;
        }

        self.cancel_status_refresh();

        let target = match self.config_manager.checked_config_path(&server) {
            Ok(target) => target,
            Err(error) => {
                self.set_error(format!("Invalid configuration name: {error}"));
                return;
            }
        };
        let was_active = self.connection.contains(&server) && self.connection.is_up();
        let fallback = self.connection.clone();
        if was_active {
            self.connection = ConnectionState::Disconnecting(server.clone());
        }

        let task_server = server.clone();
        let config_manager = self.config_manager.clone();
        let validation_manager = config_manager.clone();
        let validation_server = task_server.clone();
        let task = tokio::spawn(async move {
            let active_interfaces = if was_active {
                tokio::task::spawn_blocking(move || {
                    validation_manager.validate_config(&validation_server)
                })
                .await
                .context("configuration validator stopped unexpectedly")?
                .with_context(|| {
                    format!("refusing to run hooks while disconnecting unsafe config {task_server}")
                })?;
                VpnManager::disconnect(&task_server).await.context(
                    "refusing to delete an active config that could not be disconnected",
                )?;
                Some(
                    VpnManager::get_active_connections()
                        .await
                        .context("unable to verify active interfaces after disconnecting")?,
                )
            } else {
                None
            };
            tokio::task::spawn_blocking(move || {
                CommandExecutor::remove_config(&target)?;
                let servers = config_manager
                    .list_configs()
                    .map_err(|error| format!("{error:#}"));
                Ok(OperationOutput::Deleted {
                    server: task_server,
                    servers,
                    active_interfaces,
                })
            })
            .await
            .context("delete worker stopped unexpectedly")?
        });

        self.pending_operation = Some(PendingOperation {
            label: format!("Deleting {server}"),
            connection_may_change: was_active,
            fallback,
            task,
        });
    }

    pub fn show_status(&mut self) {
        self.current_screen = Screen::Status;
        self.status_scroll = 0;
        self.refresh_status_now();
    }

    pub fn refresh_status_now(&mut self) {
        if self.is_busy() {
            self.set_info("Wait for the current operation to finish");
            return;
        }
        let Some(server) = self.connection.server().map(str::to_owned) else {
            self.set_error("No managed WireGuard connection is active");
            return;
        };

        self.cancel_status_refresh();
        let fallback = self.connection.clone();
        let task = tokio::spawn(async move {
            let (status_result, ip_result) = tokio::join!(
                VpnManager::get_status(&server),
                VpnManager::get_current_ip()
            );
            let status = status_result?;
            Ok(OperationOutput::Status {
                server,
                status,
                ip: ip_result.ok(),
            })
        });
        self.pending_operation = Some(PendingOperation {
            label: "Refreshing connection status".to_string(),
            connection_may_change: false,
            fallback,
            task,
        });
    }

    pub fn refresh_all(&mut self) {
        if self.is_busy() {
            self.set_info("Wait for the active operation before refreshing");
            return;
        }
        self.cancel_status_refresh();
        let fallback = self.connection.clone();
        let config_manager = self.config_manager.clone();
        let task = tokio::spawn(async move {
            let servers = tokio::task::spawn_blocking(move || config_manager.list_configs())
                .await
                .context("configuration refresh worker stopped unexpectedly")??;
            Ok(OperationOutput::Refreshed(servers))
        });
        self.pending_operation = Some(PendingOperation {
            label: "Refreshing configuration list".to_string(),
            connection_may_change: false,
            fallback,
            task,
        });
        self.last_status_poll = None;
    }

    pub fn scroll_download_up(&mut self) {
        self.download_scroll = self.download_scroll.saturating_sub(1);
    }

    pub fn scroll_download_down(&mut self) {
        self.download_scroll = self.download_scroll.saturating_add(1);
    }

    pub fn scroll_download_home(&mut self) {
        self.download_scroll = 0;
    }

    pub fn scroll_download_end(&mut self) {
        self.download_scroll = u16::MAX;
    }

    pub fn scroll_status_up(&mut self) {
        self.status_scroll = self.status_scroll.saturating_sub(1);
    }

    pub fn scroll_status_down(&mut self) {
        self.status_scroll = self.status_scroll.saturating_add(1);
    }

    pub fn scroll_status_home(&mut self) {
        self.status_scroll = 0;
    }

    pub fn scroll_status_end(&mut self) {
        self.status_scroll = u16::MAX;
    }

    pub fn dismiss_message(&mut self) {
        self.message = Message::None;
        self.message_time = None;
    }

    pub fn request_quit(&mut self) -> bool {
        if self.is_busy() {
            self.set_info("Wait for the active operation to finish before quitting");
            false
        } else {
            true
        }
    }

    pub async fn tick(&mut self) {
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
        self.expire_notice();
        self.finish_operation().await;
        self.finish_status_refresh().await;
        self.finish_details_refresh().await;
        self.start_details_refresh();

        let refresh_interval = if self.status_poll_failed {
            STATUS_ERROR_RETRY_INTERVAL
        } else {
            STATUS_REFRESH_INTERVAL
        };
        let refresh_due = match self.last_status_poll {
            Some(last) => last.elapsed() >= refresh_interval,
            None => true,
        };
        if refresh_due && self.status_task.is_none() && !self.is_busy() {
            self.start_status_refresh();
        }
    }

    fn expire_notice(&mut self) {
        if matches!(self.message, Message::Error(_)) {
            return;
        }
        if !matches!(self.message, Message::None)
            && self
                .message_time
                .is_some_and(|time| time.elapsed() >= NOTICE_TTL)
        {
            self.dismiss_message();
        }
    }

    async fn finish_operation(&mut self) {
        let finished = self
            .pending_operation
            .as_ref()
            .is_some_and(|operation| operation.task.is_finished());
        if !finished {
            return;
        }

        let operation = self.pending_operation.take().expect("operation exists");
        let connection_may_change = operation.connection_may_change;
        let fallback = operation.fallback;
        match operation.task.await {
            Ok(Ok(output)) => self.apply_operation_output(output),
            Ok(Err(error)) => {
                self.connection = Self::uncertain_fallback(connection_may_change, fallback);
                self.status_stale = true;
                self.status_error = Some(error.to_string());
                self.last_status_poll = None;
                self.status_poll_failed = false;
                self.set_error(format!("{} failed: {error:#}", operation.label));
            }
            Err(error) => {
                self.connection = Self::uncertain_fallback(connection_may_change, fallback);
                self.status_stale = true;
                self.status_error = Some(error.to_string());
                self.last_status_poll = None;
                self.status_poll_failed = false;
                self.set_error(format!("{} stopped unexpectedly: {error}", operation.label));
            }
        }

        if connection_may_change {
            self.last_status_poll = None;
        }
    }

    fn uncertain_fallback(
        connection_may_change: bool,
        fallback: ConnectionState,
    ) -> ConnectionState {
        if connection_may_change {
            ConnectionState::Unknown
        } else {
            fallback
        }
    }

    fn apply_operation_output(&mut self, output: OperationOutput) {
        match output {
            OperationOutput::Connected {
                server,
                status,
                status_error,
                active_interfaces,
            } => {
                if !Self::same_active_set(&active_interfaces, std::slice::from_ref(&server)) {
                    let observed = Self::format_active_set(&active_interfaces);
                    self.reconcile_active_interfaces(active_interfaces);
                    self.set_error(format!(
                        "Connection command completed, but active interfaces are: {observed}"
                    ));
                    return;
                }
                self.connection = if status_error.is_some() {
                    ConnectionState::Degraded(server.clone())
                } else {
                    ConnectionState::Connected(server.clone())
                };
                self.status = status;
                self.status_stale = status_error.is_some();
                self.status_error = status_error;
                self.status_poll_failed = false;
                if self.status_stale {
                    self.set_info(format!(
                        "Connected to {server}; live status is temporarily unavailable"
                    ));
                } else {
                    self.set_success(format!("Connected to {server}"));
                }
            }
            OperationOutput::Disconnected {
                server,
                active_interfaces,
            } => {
                let remaining = Self::format_active_set(&active_interfaces);
                let all_disconnected = active_interfaces.is_empty();
                self.reconcile_active_interfaces(active_interfaces);
                if all_disconnected {
                    self.set_success(format!("Disconnected from {server}"));
                } else {
                    self.set_info(format!(
                        "Disconnected from {server}; active interfaces remain: {remaining}"
                    ));
                }
            }
            OperationOutput::Deleted {
                server,
                servers,
                active_interfaces,
            } => {
                let refresh_error = match servers {
                    Ok(servers) => {
                        self.replace_servers(servers);
                        None
                    }
                    Err(error) => Some(error),
                };
                let remaining = active_interfaces
                    .as_ref()
                    .filter(|active| !active.is_empty())
                    .map(|active| Self::format_active_set(active));
                if let Some(active) = active_interfaces {
                    self.reconcile_active_interfaces(active);
                }
                if let Some(error) = refresh_error {
                    self.set_error(format!(
                        "Deleted {server}, but failed to refresh the list: {error}"
                    ));
                } else if let Some(remaining) = remaining {
                    self.set_info(format!(
                        "Deleted {server}; active interfaces remain: {remaining}"
                    ));
                } else {
                    self.set_success(format!("Deleted {server}"));
                }
            }
            OperationOutput::Imported { report, servers } => {
                let imported_count = report.imported.len();
                let failure_count = report.failures.len();
                let refresh_failed = match servers {
                    Ok(servers) => {
                        self.replace_servers(servers);
                        false
                    }
                    Err(error) => {
                        self.set_error(format!("Import completed but refresh failed: {error}"));
                        true
                    }
                };
                if !refresh_failed && failure_count == 0 {
                    self.current_screen = Screen::Main;
                    self.set_success(format!("Imported {imported_count} configuration(s)"));
                } else if !refresh_failed {
                    let sample = report
                        .failures
                        .first()
                        .map(|failure| failure.error.as_str())
                        .unwrap_or("unknown error");
                    self.set_error(format!(
                        "Imported {imported_count}; {failure_count} failed ({sample})"
                    ));
                }
            }
            OperationOutput::Refreshed(servers) => {
                self.replace_servers(servers);
                self.set_success("Configuration list refreshed");
            }
            OperationOutput::Status { server, status, ip } => {
                self.connection = ConnectionState::Connected(server);
                self.status = status;
                self.current_ip = ip;
                self.status_stale = false;
                self.status_error = None;
                self.status_poll_failed = false;
                self.set_success("Connection status refreshed");
            }
        }
    }

    fn start_status_refresh(&mut self) {
        let servers = self.servers.clone();
        self.last_status_poll = Some(Instant::now());
        self.status_task = Some(tokio::spawn(async move {
            let active = VpnManager::get_active_connections().await?;
            let managed_server = Self::single_managed_active(&active, &servers);

            let (status, status_error) = match managed_server.as_deref() {
                Some(server) => match VpnManager::get_status(server).await {
                    Ok(status) => (Some(status), None),
                    Err(error) => (None, Some(error.to_string())),
                },
                None => (None, None),
            };
            Ok(StatusSnapshot {
                active_interfaces: active,
                managed_server,
                status,
                status_error,
            })
        }));
    }

    fn cancel_status_refresh(&mut self) {
        if let Some(task) = self.status_task.take() {
            task.abort();
        }
    }

    async fn finish_status_refresh(&mut self) {
        let finished = self
            .status_task
            .as_ref()
            .is_some_and(JoinHandle::is_finished);
        if !finished {
            return;
        }

        let task = self.status_task.take().expect("status task exists");
        match task.await {
            Ok(Ok(snapshot)) => {
                if let Some(server) = snapshot.managed_server {
                    if self.connection.server() != Some(server.as_str()) {
                        self.current_ip = None;
                    }
                    self.connection = if snapshot.status_error.is_some() {
                        ConnectionState::Degraded(server)
                    } else {
                        ConnectionState::Connected(server)
                    };
                    if let Some(status) = snapshot.status {
                        self.status = status;
                    }
                    self.status_stale = snapshot.status_error.is_some();
                    self.status_error = snapshot.status_error;
                } else if snapshot.active_interfaces.is_empty() {
                    self.connection = ConnectionState::Disconnected;
                    self.status = VpnStatus::default();
                    self.current_ip = None;
                    self.status_stale = false;
                    self.status_error = None;
                } else {
                    self.status_error = Some(Self::ambiguous_active_diagnostic(
                        &snapshot.active_interfaces,
                    ));
                    self.connection = ConnectionState::Ambiguous(snapshot.active_interfaces);
                    self.status = VpnStatus::default();
                    self.current_ip = None;
                    self.status_stale = true;
                }
                self.status_poll_failed = false;
            }
            Ok(Err(error)) => {
                self.status_stale = true;
                self.status_error = Some(error.to_string());
                self.status_poll_failed = true;
                if let Some(server) = self.connection.server().map(str::to_owned) {
                    self.connection = ConnectionState::Degraded(server);
                } else {
                    self.connection = ConnectionState::Unknown;
                }
            }
            Err(error) => {
                self.status_stale = true;
                self.status_error = Some(error.to_string());
                self.status_poll_failed = true;
                if self.connection.server().is_none() {
                    self.connection = ConnectionState::Unknown;
                }
            }
        }
    }

    pub async fn shutdown(&mut self) {
        self.cancel_status_refresh();
        if let Some(details) = self.details_task.take() {
            details.task.abort();
        }
        if let Some(operation) = self.pending_operation.take() {
            // Preserve a normal switch/rollback when possible, but never make terminal
            // shutdown wait through every command in a failed multi-step transaction.
            let mut task = operation.task;
            if tokio::time::timeout(SHUTDOWN_GRACE_PERIOD, &mut task)
                .await
                .is_err()
            {
                task.abort();
                let _ = task.await;
            }
        }
    }

    fn replace_servers(&mut self, mut servers: Vec<String>) {
        let selected = self.selected_server().map(str::to_owned);
        servers.sort_by_key(|name| name.to_ascii_lowercase());
        servers.dedup();
        self.servers = servers;

        let indices = self.visible_server_indices();
        self.selected_index = selected
            .as_ref()
            .and_then(|selected| {
                indices.iter().position(|index| {
                    self.servers
                        .get(*index)
                        .is_some_and(|name| name == selected)
                })
            })
            .unwrap_or(0)
            .min(indices.len().saturating_sub(1));
        self.refresh_selected_details();
    }

    fn select_connected_server(&mut self) {
        let active = self.connection.server().or_else(|| match &self.connection {
            ConnectionState::Ambiguous(active) => active
                .iter()
                .find(|name| self.servers.iter().any(|server| server == *name))
                .map(String::as_str),
            _ => None,
        });
        let Some(active) = active else {
            return;
        };
        let indices = self.visible_server_indices();
        if let Some(position) = indices.iter().position(|index| {
            self.servers
                .get(*index)
                .is_some_and(|server| server == active)
        }) {
            self.selected_index = position;
        }
    }

    fn ambiguous_active_diagnostic(active: &[String]) -> String {
        if active.len() == 1 {
            format!(
                "Active WireGuard interface '{}' is not managed by this configuration list",
                active[0]
            )
        } else {
            format!(
                "Multiple WireGuard interfaces are active: {}",
                active.join(", ")
            )
        }
    }

    fn single_managed_active(active: &[String], servers: &[String]) -> Option<String> {
        (active.len() == 1 && servers.contains(&active[0])).then(|| active[0].clone())
    }

    fn same_active_set(left: &[String], right: &[String]) -> bool {
        left.len() == right.len()
            && left.iter().all(|name| right.contains(name))
            && right.iter().all(|name| left.contains(name))
    }

    fn format_active_set(active: &[String]) -> String {
        if active.is_empty() {
            "none".to_string()
        } else {
            active.join(", ")
        }
    }

    fn reconcile_active_interfaces(&mut self, active: Vec<String>) {
        self.status = VpnStatus::default();
        self.current_ip = None;
        self.status_poll_failed = false;
        self.last_status_poll = None;

        if active.is_empty() {
            self.connection = ConnectionState::Disconnected;
            self.status_stale = false;
            self.status_error = None;
        } else if let Some(server) = Self::single_managed_active(&active, &self.servers) {
            self.connection = ConnectionState::Degraded(server);
            self.status_stale = true;
            self.status_error = Some("Waiting for a fresh connection status".to_string());
        } else {
            let diagnostic = Self::ambiguous_active_diagnostic(&active);
            self.connection = ConnectionState::Ambiguous(active);
            self.status_stale = true;
            self.status_error = Some(diagnostic);
        }
    }

    fn set_error(&mut self, message: impl Into<String>) {
        self.message = Message::Error(message.into());
        self.message_time = Some(Instant::now());
    }

    fn set_success(&mut self, message: impl Into<String>) {
        self.message = Message::Success(message.into());
        self.message_time = Some(Instant::now());
    }

    fn set_info(&mut self, message: impl Into<String>) {
        self.message = Message::Info(message.into());
        self.message_time = Some(Instant::now());
    }

    fn refresh_selected_details(&mut self) {
        let Some(server) = self.selected_server().map(str::to_owned) else {
            self.selected_details = None;
            self.selected_details_error = None;
            self.details_request = None;
            return;
        };

        self.selected_details = None;
        self.selected_details_error = None;
        self.details_request = Some((server, Instant::now()));
    }

    fn start_details_refresh(&mut self) {
        if self.details_task.is_some()
            || self
                .details_request
                .as_ref()
                .is_none_or(|(_, requested)| requested.elapsed() < DETAILS_DEBOUNCE)
        {
            return;
        }

        let (server, _) = self.details_request.take().expect("details request exists");
        let manager = self.config_manager.clone();
        let task_server = server.clone();
        let task = tokio::task::spawn_blocking(move || manager.load_config_details(&task_server));
        self.details_task = Some(DetailsTask { server, task });
    }

    async fn finish_details_refresh(&mut self) {
        let finished = self
            .details_task
            .as_ref()
            .is_some_and(|details| details.task.is_finished());
        if !finished {
            return;
        }

        let details = self.details_task.take().expect("details task exists");
        if self.selected_server() != Some(details.server.as_str()) {
            let _ = details.task.await;
            return;
        }

        if self
            .details_request
            .as_ref()
            .is_some_and(|(server, _)| server == &details.server)
        {
            self.details_request = None;
        }

        match details.task.await {
            Ok(Ok(config)) => {
                self.selected_details = Some(config);
                self.selected_details_error = None;
            }
            Ok(Err(error)) => {
                self.selected_details = None;
                self.selected_details_error = Some(error.to_string());
            }
            Err(error) => {
                self.selected_details = None;
                self.selected_details_error = Some(format!("Config reader stopped: {error}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{App, ConnectionState};

    #[test]
    fn connection_state_exposes_one_consistent_server() {
        assert_eq!(ConnectionState::Unknown.server(), None);
        assert_eq!(ConnectionState::Disconnected.server(), None);
        assert_eq!(
            ConnectionState::Connected("wg0".to_string()).server(),
            Some("wg0")
        );
        assert!(ConnectionState::Degraded("wg0".to_string()).is_up());
        assert!(!ConnectionState::Connecting("wg0".to_string()).is_up());
        let ambiguous = ConnectionState::Ambiguous(vec!["wg0".into(), "wg1".into()]);
        assert_eq!(ambiguous.server(), None);
        assert!(ambiguous.is_up());
        assert!(ambiguous.contains("wg1"));
        assert!(!ambiguous.contains("wg2"));
    }

    #[test]
    fn only_one_managed_active_interface_is_unambiguous() {
        let servers = vec!["wg0".to_string(), "wg1".to_string()];
        assert_eq!(
            App::single_managed_active(&["wg1".to_string()], &servers),
            Some("wg1".to_string())
        );
        assert_eq!(
            App::single_managed_active(&["external".to_string()], &servers),
            None
        );
        assert_eq!(
            App::single_managed_active(&["wg0".to_string(), "wg1".to_string()], &servers),
            None
        );
    }

    #[test]
    fn mutable_failure_never_claims_the_tunnel_is_down() {
        assert_eq!(
            App::uncertain_fallback(true, ConnectionState::Disconnected),
            ConnectionState::Unknown
        );
        assert_eq!(
            App::uncertain_fallback(false, ConnectionState::Disconnected),
            ConnectionState::Disconnected
        );
    }

    #[test]
    fn active_interface_sets_are_compared_without_order() {
        let first = vec!["wg0".to_string(), "wg1".to_string()];
        let reversed = vec!["wg1".to_string(), "wg0".to_string()];
        assert!(App::same_active_set(&first, &reversed));
        assert!(!App::same_active_set(&first, &["wg0".to_string()]));
    }
}
