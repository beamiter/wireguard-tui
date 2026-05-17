use anyhow::{anyhow, Result};
use std::process::Command;

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn check_wireguard_installed() -> Result<bool> {
        Ok(Command::new("which")
            .arg("wg-quick")
            .output()
            .map_or(false, |o| o.status.success()))
    }

    pub fn check_resolvconf_installed() -> Result<bool> {
        Ok(Command::new("which")
            .arg("resolvconf")
            .output()
            .map_or(false, |o| o.status.success()))
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

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let full_output = format!("{}\n{}", stdout, stderr).trim().to_string();
            return Err(anyhow!("Failed to connect:\n{}", full_output));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
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
        // 首先通过 wg show interfaces 获取所有活动的 WireGuard 接口
        let output = Command::new("sudo")
            .arg("wg")
            .arg("show")
            .arg("interfaces")
            .output()
            .ok()
            .filter(|o| o.status.success());

        if let Some(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let interfaces: Vec<&str> = stdout.split_whitespace().collect();

            // 返回第一个接口（如果有多个，通常只会有一个活动）
            if !interfaces.is_empty() {
                return Ok(Some(interfaces[0].to_string()));
            }
        }

        Ok(None)
    }

    pub fn check_interface_exists(config_name: &str) -> Result<bool> {
        let output = Command::new("sudo")
            .arg("wg")
            .arg("show")
            .arg(config_name)
            .output()?;

        Ok(output.status.success())
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
            .arg("--connect-timeout")
            .arg("3")  // 连接超时 3 秒
            .arg("--max-time")
            .arg("5")  // 总超时 5 秒
            .arg("https://api.ipify.org")
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get IP"));
        }

        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if ip.is_empty() {
            return Err(anyhow!("Empty IP response"));
        }

        Ok(ip)
    }
}
