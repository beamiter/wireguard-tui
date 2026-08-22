use crate::commands::CommandExecutor;
use crate::config::{validate_interface_name, validate_wireguard_client_config_for_import};
use anyhow::{anyhow, bail, Context, Result};
use directories::UserDirs;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

pub const MAX_WIREGUARD_CONFIG_SIZE: u64 = 64 * 1024;
pub const MAX_DISCOVERED_CONFIGS: usize = 256;
const MAX_DOWNLOAD_DIRECTORY_ENTRIES: usize = 4096;
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
        let mut configs: Vec<(Option<SystemTime>, PathBuf)> = Vec::new();

        if !self.downloads_dir.exists() {
            return Ok(Vec::new());
        }

        for (index, entry) in fs::read_dir(&self.downloads_dir)?.enumerate() {
            if index >= MAX_DOWNLOAD_DIRECTORY_ENTRIES {
                bail!(
                    "Downloads directory exceeds the {MAX_DOWNLOAD_DIRECTORY_ENTRIES} entry scan limit"
                );
            }
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

            if configs.len() >= MAX_DISCOVERED_CONFIGS {
                bail!(
                    "Downloads directory contains more than {MAX_DISCOVERED_CONFIGS} WireGuard configuration candidates"
                );
            }
            configs.push((metadata.modified().ok(), path));
        }

        sort_config_candidates(&mut configs);

        Ok(configs.into_iter().map(|(_, path)| path).collect())
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
            bail!("WireGuard configuration exceeds the {MAX_WIREGUARD_CONFIG_SIZE} byte limit");
        }

        let contents = read_limited(source_path, &metadata)?;
        let config_text = std::str::from_utf8(&contents)
            .context("WireGuard configuration must be valid UTF-8")?;
        validate_wireguard_client_config_for_import(config_text)?;

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
        let mut selected_targets = HashSet::new();

        for source_path in source_paths {
            if let Ok((_, filename)) = validated_filename(source_path) {
                if !selected_targets.insert(filename) {
                    report.failures.push(ImportFailure {
                        path: source_path.clone(),
                        error: "Duplicate target interface was ignored".to_string(),
                    });
                    continue;
                }
            }
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

fn sort_config_candidates(candidates: &mut [(Option<SystemTime>, PathBuf)]) {
    candidates.sort_by(|(a_time, a_path), (b_time, b_path)| {
        b_time.cmp(a_time).then_with(|| a_path.cmp(b_path))
    });
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
        bail!("WireGuard configuration exceeds the {MAX_WIREGUARD_CONFIG_SIZE} byte limit");
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
    {
        if opened_metadata.uid() != unsafe { libc::geteuid() } {
            bail!(
                "Source is not owned by the current user: {}",
                path.display()
            );
        }
        if opened_metadata.nlink() != 1 {
            bail!("Source has multiple hard links: {}", path.display());
        }
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

    let before_read = file
        .metadata()
        .with_context(|| format!("Failed to inspect configuration {}", path.display()))?;

    let mut contents = Vec::new();
    (&file)
        .take(MAX_WIREGUARD_CONFIG_SIZE + 1)
        .read_to_end(&mut contents)
        .with_context(|| format!("Failed to read configuration {}", path.display()))?;

    if contents.len() as u64 > MAX_WIREGUARD_CONFIG_SIZE {
        bail!("WireGuard configuration exceeds the {MAX_WIREGUARD_CONFIG_SIZE} byte limit");
    }

    let after_read = file
        .metadata()
        .with_context(|| format!("Failed to re-inspect configuration {}", path.display()))?;
    if after_read.len() != contents.len() as u64 {
        bail!("Source changed while it was being read: {}", path.display());
    }

    #[cfg(unix)]
    {
        let stable_metadata = before_read.dev() == after_read.dev()
            && before_read.ino() == after_read.ino()
            && before_read.len() == after_read.len()
            && before_read.mtime() == after_read.mtime()
            && before_read.mtime_nsec() == after_read.mtime_nsec()
            && before_read.ctime() == after_read.ctime()
            && before_read.ctime_nsec() == after_read.ctime_nsec();
        if !stable_metadata {
            bail!("Source changed while it was being read: {}", path.display());
        }

        let linked_metadata = fs::symlink_metadata(path)
            .with_context(|| format!("Failed to re-inspect configuration {}", path.display()))?;
        if !linked_metadata.file_type().is_file()
            || linked_metadata.dev() != after_read.dev()
            || linked_metadata.ino() != after_read.ino()
        {
            bail!(
                "Source path changed while it was being read: {}",
                path.display()
            );
        }
    }

    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::sort_config_candidates;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    #[test]
    fn candidate_order_is_newest_first_then_path() {
        let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        let new = SystemTime::UNIX_EPOCH + Duration::from_secs(2);
        let mut candidates = vec![
            (Some(old), PathBuf::from("z.conf")),
            (Some(new), PathBuf::from("b.conf")),
            (None, PathBuf::from("unknown.conf")),
            (Some(new), PathBuf::from("a.conf")),
        ];
        sort_config_candidates(&mut candidates);
        assert_eq!(
            candidates
                .into_iter()
                .map(|(_, path)| path)
                .collect::<Vec<_>>(),
            vec![
                PathBuf::from("a.conf"),
                PathBuf::from("b.conf"),
                PathBuf::from("z.conf"),
                PathBuf::from("unknown.conf"),
            ]
        );
    }
}
