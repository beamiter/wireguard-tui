use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

pub struct ConfigDownloader {
    download_url: String,
    downloads_dir: PathBuf,
}

impl ConfigDownloader {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let downloads_dir = PathBuf::from(home).join("Downloads");

        Self {
            download_url: "https://tools.strongvpn.asia/share/strong-wg/strong-wg.html".to_string(),
            downloads_dir,
        }
    }

    /// 获取下载页面 URL
    pub fn get_download_url(&self) -> &str {
        &self.download_url
    }

    /// 扫描 Downloads 目录，查找所有 WireGuard 配置文件
    pub fn scan_downloads(&self) -> Result<Vec<PathBuf>> {
        let mut configs = Vec::new();

        if !self.downloads_dir.exists() {
            return Ok(configs);
        }

        for entry in fs::read_dir(&self.downloads_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "conf" {
                        // 包含所有 .conf 文件
                        configs.push(path);
                    }
                }
            }
        }

        // 按修改时间排序，最新的在前面
        configs.sort_by(|a, b| {
            let a_time = fs::metadata(a).and_then(|m| m.modified()).ok();
            let b_time = fs::metadata(b).and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        Ok(configs)
    }

    /// 导入配置文件到 WireGuard 目录
    pub fn import_config(&self, source_path: &Path, target_dir: &Path) -> Result<String> {
        if !source_path.exists() {
            return Err(anyhow!("Source file does not exist: {:?}", source_path));
        }

        let filename = source_path
            .file_name()
            .ok_or_else(|| anyhow!("Invalid filename"))?
            .to_str()
            .ok_or_else(|| anyhow!("Invalid UTF-8 in filename"))?;

        let target_path = target_dir.join(filename);

        // 使用 sudo cp 复制文件到 /etc/wireguard
        let status = Command::new("sudo")
            .arg("cp")
            .arg(source_path)
            .arg(&target_path)
            .status()
            .map_err(|e| anyhow!("Failed to copy file: {}", e))?;

        if !status.success() {
            return Err(anyhow!("Failed to import config file"));
        }

        Ok(filename.to_string())
    }

    /// 批量导入多个配置文件
    pub fn import_configs(&self, source_paths: &[PathBuf], target_dir: &Path) -> Result<Vec<String>> {
        let mut imported = Vec::new();

        for source_path in source_paths {
            match self.import_config(source_path, target_dir) {
                Ok(filename) => {
                    imported.push(filename);
                }
                Err(e) => {
                    eprintln!("Failed to import {:?}: {}", source_path, e);
                }
            }
        }

        Ok(imported)
    }

    /// 获取配置文件名（不含路径）
    pub fn get_config_name(path: &Path) -> Option<String> {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }

    /// 格式化显示配置文件信息
    pub fn format_config_info(path: &Path) -> String {
        let filename = path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let size = fs::metadata(path)
            .map(|m| m.len())
            .unwrap_or(0);

        let modified = fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| {
                t.duration_since(SystemTime::UNIX_EPOCH).ok()
            })
            .map(|d| {
                let secs = d.as_secs();
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let diff = now - secs;

                if diff < 60 {
                    format!("{} seconds ago", diff)
                } else if diff < 3600 {
                    format!("{} minutes ago", diff / 60)
                } else if diff < 86400 {
                    format!("{} hours ago", diff / 3600)
                } else {
                    format!("{} days ago", diff / 86400)
                }
            })
            .unwrap_or_else(|| "unknown time".to_string());

        format!("{} ({} bytes, {})", filename, size, modified)
    }
}
