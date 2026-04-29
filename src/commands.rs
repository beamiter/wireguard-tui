use anyhow::{anyhow, Result};
use std::process::Command;

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn check_wireguard_installed() -> Result<bool> {
        let output = Command::new("which")
            .arg("wg-quick")
            .output();

        Ok(output.map_or(false, |o| o.status.success()))
    }

    pub fn check_resolvconf_installed() -> Result<bool> {
        let output = Command::new("which")
            .arg("resolvconf")
            .output();

        Ok(output.map_or(false, |o| o.status.success()))
    }

    pub fn install_resolvconf() -> Result<()> {
        let distro = Self::detect_distro()?;

        let cmd = match distro.as_str() {
            "ubuntu" | "debian" => "sudo apt-get update && sudo apt-get install -y resolvconf",
            "fedora" => "sudo dnf install -y systemd-resolved && sudo ln -sf /usr/bin/resolvectl /usr/local/bin/resolvconf",
            "arch" => "sudo pacman -S --noconfirm openresolv",
            "opensuse" => "sudo zypper install -y openresolv",
            _ => return Err(anyhow!("Unsupported Linux distribution: {}", distro)),
        };

        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to install resolvconf"));
        }

        Ok(())
    }

    pub fn install_wireguard() -> Result<()> {
        let distro = Self::detect_distro()?;

        let cmd = match distro.as_str() {
            "ubuntu" | "debian" => "sudo apt-get update && sudo apt-get install -y wireguard",
            "fedora" => "sudo dnf install -y wireguard-tools",
            "arch" => "sudo pacman -S --noconfirm wireguard-tools",
            "opensuse" => "sudo zypper install -y wireguard-tools",
            _ => return Err(anyhow!("Unsupported Linux distribution: {}", distro)),
        };

        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to install WireGuard"));
        }

        Ok(())
    }

    pub fn connect_vpn(config_name: &str) -> Result<String> {
        let output = Command::new("sudo")
            .arg("wg-quick")
            .arg("up")
            .arg(config_name)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            // 合并 stdout 和 stderr 来获取完整错误信息
            let full_output = format!("{}\n{}", stdout, stderr).trim().to_string();
            return Err(anyhow!("Failed to connect:\n{}", full_output));
        }

        Ok(stdout.to_string())
    }

    pub fn disconnect_vpn(config_name: &str) -> Result<String> {
        let output = Command::new("sudo")
            .arg("wg-quick")
            .arg("down")
            .arg(config_name)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to disconnect: {}", stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn get_vpn_status(config_name: &str) -> Result<String> {
        let output = Command::new("sudo")
            .arg("wg")
            .arg("show")
            .arg(config_name)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("VPN not connected"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn get_active_vpn() -> Result<Option<String>> {
        // 尝试获取活动 VPN 连接，如果失败返回 None
        let output = match Command::new("ip")
            .arg("link")
            .arg("show")
            .output() {
                Ok(o) => o,
                Err(_) => return Ok(None),  // 命令执行失败，返回 None
            };

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("wireguard") {
                if let Some(name) = line.split(':').next() {
                    let name = name.trim_start_matches(|c: char| c.is_numeric() || c == ':');
                    return Ok(Some(name.to_string()));
                }
            }
        }

        Ok(None)
    }

    fn detect_distro() -> Result<String> {
        let output = Command::new("sh")
            .arg("-c")
            .arg("cat /etc/os-release | grep ^ID= | cut -d= -f2 | tr -d '\"'")
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to detect Linux distribution"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn get_current_ip() -> Result<String> {
        let output = Command::new("curl")
            .arg("-s")
            .arg("https://api.ipify.org")
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get IP"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}
