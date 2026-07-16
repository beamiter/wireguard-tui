use crate::commands::CommandExecutor;
use crate::config::{validate_interface_name, validate_wireguard_client_config};
use anyhow::{anyhow, bail, Context, Result};
use directories::UserDirs;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

pub const MAX_WIREGUARD_CONFIG_SIZE: u64 = 64 * 1024;
const DOWNLOAD_URL: &str = "https://tools.strongvpn.asia/share/strong-wg/strong-wg.html";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFailure {
    pub path: PathBuf,
    pub error: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: Vec<String>,
    pub failures: Vec<ImportFailure>,
}

#[derive(Debug, Clone)]
pub struct ConfigDownloader {
    download_url: String,
    downloads_dir: PathBuf,
}

impl ConfigDownloader {
    pub fn new() -> Self {
        let downloads_dir = UserDirs::new()
            .map(|directories| {
                directories
                    .download_dir()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| directories.home_dir().join("Downloads"))
            })
            .unwrap_or_else(|| PathBuf::from("Downloads"));
        Self::with_downloads_dir(downloads_dir)
    }

    /// Construct a downloader with an explicit directory. This is also the
    /// preferred entry point for deterministic tests and embedded callers.
    pub fn with_downloads_dir(downloads_dir: impl Into<PathBuf>) -> Self {
        Self {
            download_url: DOWNLOAD_URL.to_string(),
            downloads_dir: downloads_dir.into(),
        }
    }

    pub fn get_download_url(&self) -> &str {
        &self.download_url
    }

    pub fn downloads_dir(&self) -> &Path {
        &self.downloads_dir
    }

    pub fn open_in_browser(&self) -> Result<()> {
        CommandExecutor::open_url(&self.download_url).context("Failed to open the download URL")
    }

    /// Scan the download directory for safe regular-file candidates. Content
    /// is validated again immediately before import, where failures can be
    /// reported to the user rather than silently hidden.
    pub fn scan_downloads(&self) -> Result<Vec<PathBuf>> {
        let mut configs = Vec::new();

        if !self.downloads_dir.exists() {
            return Ok(configs);
        }

        for entry in fs::read_dir(&self.downloads_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }

            let path = entry.path();
            if validated_filename(&path).is_err() {
                continue;
            }

            let metadata = entry.metadata()?;
            if metadata.len() > MAX_WIREGUARD_CONFIG_SIZE {
                continue;
            }

            configs.push(path);
        }

        configs.sort_by(|a, b| {
            let a_time = fs::symlink_metadata(a)
                .and_then(|metadata| metadata.modified())
                .ok();
            let b_time = fs::symlink_metadata(b)
                .and_then(|metadata| metadata.modified())
                .ok();
            b_time.cmp(&a_time)
        });

        Ok(configs)
    }

    /// Validate and import one configuration. Only the already-read immutable
    /// bytes cross the privilege boundary; the privileged installer never
    /// opens a path controlled by the Downloads directory.
    pub fn import_config(&self, source_path: &Path, target_dir: &Path) -> Result<String> {
        let (interface_name, filename) = validated_filename(source_path)?;
        validate_interface_name(&interface_name)?;

        let metadata = fs::symlink_metadata(source_path)
            .with_context(|| format!("Failed to inspect {}", source_path.display()))?;
        if !metadata.file_type().is_file() {
            bail!("Source is not a regular file: {}", source_path.display());
        }
        if metadata.len() > MAX_WIREGUARD_CONFIG_SIZE {
            bail!(
                "WireGuard configuration exceeds the {} byte limit",
                MAX_WIREGUARD_CONFIG_SIZE
            );
        }

        let contents = read_limited(source_path, &metadata)?;
        let config_text = std::str::from_utf8(&contents)
            .context("WireGuard configuration must be valid UTF-8")?;
        validate_wireguard_client_config(config_text)?;

        // The filename and interface were validated before constructing the
        // target, so this join cannot escape target_dir or inject an option.
        let target_path = target_dir.join(&filename);
        CommandExecutor::install_config(&contents, &target_path)
            .with_context(|| format!("Failed to install {filename}"))?;

        Ok(filename)
    }

    /// Import every selected file and retain both successes and per-file
    /// failures. A bad file no longer hides the outcome of the rest of a batch.
    pub fn import_configs(
        &self,
        source_paths: &[PathBuf],
        target_dir: &Path,
    ) -> Result<ImportReport> {
        let mut report = ImportReport::default();

        for source_path in source_paths {
            match self.import_config(source_path, target_dir) {
                Ok(filename) => report.imported.push(filename),
                Err(error) => report.failures.push(ImportFailure {
                    path: source_path.clone(),
                    error: format!("{error:#}"),
                }),
            }
        }

        Ok(report)
    }

    pub fn format_config_info(path: &Path) -> String {
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown");

        let metadata = fs::symlink_metadata(path).ok();
        let size = metadata.as_ref().map(|value| value.len()).unwrap_or(0);
        let modified = metadata
            .and_then(|value| value.modified().ok())
            .and_then(|time| SystemTime::now().duration_since(time).ok())
            .map(|elapsed| elapsed.as_secs())
            .map(|seconds| {
                if seconds < 60 {
                    format!("{seconds} seconds ago")
                } else if seconds < 3600 {
                    format!("{} minutes ago", seconds / 60)
                } else if seconds < 86400 {
                    format!("{} hours ago", seconds / 3600)
                } else {
                    format!("{} days ago", seconds / 86400)
                }
            })
            .unwrap_or_else(|| "unknown time".to_string());

        format!("{filename} ({size} bytes, {modified})")
    }
}

impl Default for ConfigDownloader {
    fn default() -> Self {
        Self::new()
    }
}

fn validated_filename(path: &Path) -> Result<(String, String)> {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Configuration filename must be valid UTF-8"))?;

    if filename.chars().any(|character| character.is_control()) {
        bail!("Configuration filename contains control characters");
    }

    if path.extension().and_then(|extension| extension.to_str()) != Some("conf") {
        bail!("WireGuard configuration filename must end in .conf");
    }

    let interface_name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Configuration filename has no valid interface name"))?;
    validate_interface_name(interface_name)?;

    Ok((interface_name.to_string(), filename.to_string()))
}

fn read_limited(path: &Path, expected_metadata: &fs::Metadata) -> Result<Vec<u8>> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK | libc::O_NOCTTY);

    let file = options
        .open(path)
        .with_context(|| format!("Failed to open configuration {}", path.display()))?;
    let opened_metadata = file
        .metadata()
        .with_context(|| format!("Failed to inspect opened configuration {}", path.display()))?;
    if !opened_metadata.file_type().is_file() {
        bail!("Source is not a regular file: {}", path.display());
    }
    if opened_metadata.len() > MAX_WIREGUARD_CONFIG_SIZE {
        bail!(
            "WireGuard configuration exceeds the {} byte limit",
            MAX_WIREGUARD_CONFIG_SIZE
        );
    }

    #[cfg(unix)]
    if expected_metadata.dev() != opened_metadata.dev()
        || expected_metadata.ino() != opened_metadata.ino()
    {
        bail!(
            "Source changed while it was being opened: {}",
            path.display()
        );
    }

    #[cfg(unix)]
    if opened_metadata.mode() & 0o077 != 0 {
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .with_context(|| {
                format!(
                    "Refusing to import a group/world-readable private-key file that could not be secured: {}",
                    path.display()
                )
            })?;
    }

    let mut contents = Vec::new();
    file.take(MAX_WIREGUARD_CONFIG_SIZE + 1)
        .read_to_end(&mut contents)
        .with_context(|| format!("Failed to read configuration {}", path.display()))?;

    if contents.len() as u64 > MAX_WIREGUARD_CONFIG_SIZE {
        bail!(
            "WireGuard configuration exceeds the {} byte limit",
            MAX_WIREGUARD_CONFIG_SIZE
        );
    }

    Ok(contents)
}
