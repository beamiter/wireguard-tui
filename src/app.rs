use crate::config::ConfigManager;
use crate::download::ConfigDownloader;
use crate::vpn::{VpnManager, VpnStatus};
use anyhow::Result;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Main,
    Download,
    Import,
    Status,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    Info(String),
    Error(String),
    Success(String),
}

pub struct App {
    pub current_screen: Screen,
    pub selected_index: usize,
    pub servers: Vec<String>,
    pub active_server: Option<String>,
    pub status: VpnStatus,
    pub message: Message,
    pub message_time: Option<Instant>,
    pub config_manager: ConfigManager,
    pub downloader: ConfigDownloader,
    pub loading: bool,
    pub username: String,
    pub password: String,
    pub last_update: Instant,
    pub import_configs: Vec<std::path::PathBuf>,
    pub import_selected: usize,
    pub import_checked: Vec<bool>,
}

impl App {
    pub async fn new() -> Result<Self> {
        let config_manager = ConfigManager::new()?;
        let config = config_manager.load_config()?;

        let mut servers = config_manager.list_configs()?;
        servers.sort();

        let active_server = VpnManager::get_active_connection().await.ok().flatten();

        // Check if credentials are still default template values
        let credentials_configured = !config.username.is_empty()
            && config.username != "a314393"
            && !config.password.is_empty()
            && config.password != "L7W8cXG3MH";

        let initial_message = if !credentials_configured {
            let config_path = config_manager.get_config_path_str();
            Message::Info(format!("⚠️  Please configure credentials in: {}", config_path))
        } else {
            Message::None
        };

        let app = Self {
            current_screen: Screen::Main,
            selected_index: 0,
            servers,
            active_server,
            status: VpnStatus::default(),
            message: initial_message,
            message_time: Some(Instant::now()),
            config_manager,
            downloader: ConfigDownloader::new(),
            loading: false,
            username: config.username,
            password: config.password,
            last_update: Instant::now(),
            import_configs: Vec::new(),
            import_selected: 0,
            import_checked: Vec::new(),
        };

        VpnManager::install_if_needed().await.ok();

        Ok(app)
    }

    pub fn handle_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn handle_down(&mut self) {
        if self.selected_index < self.servers.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub async fn handle_enter(&mut self) -> Result<()> {
        if self.servers.is_empty() {
            self.set_error("No servers available. Download configurations first.");
            return Ok(());
        }

        let server = &self.servers[self.selected_index].clone();

        if Some(server) == self.active_server.as_ref() {
            self.disconnect_vpn(server).await?;
        } else {
            self.connect_vpn(server).await?;
        }

        Ok(())
    }

    /// 显示下载信息
    pub async fn handle_show_download_info(&mut self) -> Result<()> {
        self.current_screen = Screen::Download;
        Ok(())
    }

    /// 导入下载的配置文件
    pub async fn handle_import(&mut self) -> Result<()> {
        self.current_screen = Screen::Import;
        self.loading = true;

        // 扫描 Downloads 目录
        match self.downloader.scan_downloads() {
            Ok(configs) => {
                if configs.is_empty() {
                    self.set_error("No .conf files found in ~/Downloads/. Download them first by pressing 'o'.");
                    self.loading = false;
                    self.current_screen = Screen::Main;
                } else {
                    let count = configs.len();
                    self.import_configs = configs;
                    self.import_selected = 0;
                    // 初始化选中状态，默认全部选中
                    self.import_checked = vec![true; count];
                    self.loading = false;
                    self.set_info(format!("Found {} config(s). Use Space to check/uncheck, Enter to import selected.", count));
                }
            }
            Err(e) => {
                self.set_error(format!("Failed to scan downloads: {}", e));
                self.loading = false;
                self.current_screen = Screen::Main;
            }
        }

        Ok(())
    }

    /// 切换当前项的选中状态
    pub fn handle_toggle_check(&mut self) {
        if self.import_selected < self.import_checked.len() {
            self.import_checked[self.import_selected] = !self.import_checked[self.import_selected];
        }
    }

    /// 导入选中的配置文件
    pub async fn handle_import_selected(&mut self) -> Result<()> {
        if self.import_configs.is_empty() {
            return Ok(());
        }

        self.loading = true;

        // 收集所有选中的文件
        let selected_paths: Vec<_> = self.import_configs
            .iter()
            .enumerate()
            .filter(|(idx, _)| self.import_checked.get(*idx).copied().unwrap_or(false))
            .map(|(_, path)| path.clone())
            .collect();

        if selected_paths.is_empty() {
            self.set_error("No files selected. Use Space to check files.");
            self.loading = false;
            return Ok(());
        }

        match self.downloader.import_configs(&selected_paths, self.config_manager.get_wg_config_dir()) {
            Ok(imported) => {
                self.servers = self.config_manager.list_configs()?;
                self.servers.sort();
                self.set_success(format!("Imported {} config(s)", imported.len()));
                self.current_screen = Screen::Main;
            }
            Err(e) => {
                self.set_error(format!("Failed to import: {}", e));
            }
        }

        self.loading = false;
        Ok(())
    }

    pub async fn handle_delete(&mut self) -> Result<()> {
        if self.servers.is_empty() {
            return Ok(());
        }

        let server = &self.servers[self.selected_index].clone();

        if Some(server) == self.active_server.as_ref() {
            self.disconnect_vpn(server).await?;
        }

        let path = self.config_manager.get_config_path(server);
        std::fs::remove_file(path)?;

        self.servers.remove(self.selected_index);
        if self.selected_index >= self.servers.len() && self.selected_index > 0 {
            self.selected_index -= 1;
        }

        self.set_success(format!("Deleted {}", server));
        Ok(())
    }

    pub async fn handle_status(&mut self) -> Result<()> {
        self.current_screen = Screen::Status;

        if let Some(server) = &self.active_server {
            self.status = VpnManager::get_status(server).await?;

            if let Ok(ip) = VpnManager::get_current_ip().await {
                self.message = Message::Info(format!("Current IP: {}", ip));
            }
        } else {
            self.set_error("No active VPN connection");
        }

        Ok(())
    }

    async fn connect_vpn(&mut self, server: &str) -> Result<()> {
        self.loading = true;

        match VpnManager::connect(server).await {
            Ok(_) => {
                self.active_server = Some(server.to_string());
                self.status = VpnManager::get_status(server).await?;
                self.set_success(format!("Connected to {}", server));
            }
            Err(e) => {
                self.set_error(format!("Connection failed: {}", e));
            }
        }

        self.loading = false;
        Ok(())
    }

    async fn disconnect_vpn(&mut self, server: &str) -> Result<()> {
        self.loading = true;

        match VpnManager::disconnect(server).await {
            Ok(_) => {
                self.active_server = None;
                self.set_success(format!("Disconnected from {}", server));
            }
            Err(e) => {
                self.set_error(format!("Disconnection failed: {}", e));
            }
        }

        self.loading = false;
        Ok(())
    }

    pub async fn tick(&mut self) -> Result<()> {
        if let Message::None = self.message {
        } else {
            if let Some(time) = self.message_time {
                if time.elapsed().as_secs() > 3 {
                    self.message = Message::None;
                    self.message_time = None;
                }
            }
        }

        if self.last_update.elapsed().as_secs() >= 2 && self.active_server.is_some() {
            if let Some(server) = &self.active_server {
                self.status = VpnManager::get_status(server).await.unwrap_or_default();
            }
            self.last_update = Instant::now();
        }

        Ok(())
    }

    fn set_error(&mut self, msg: impl Into<String>) {
        self.message = Message::Error(msg.into());
        self.message_time = Some(Instant::now());
    }

    fn set_success(&mut self, msg: impl Into<String>) {
        self.message = Message::Success(msg.into());
        self.message_time = Some(Instant::now());
    }

    fn set_info(&mut self, msg: impl Into<String>) {
        self.message = Message::Info(msg.into());
        self.message_time = Some(Instant::now());
    }
}
