use anyhow::Result;
use crate::commands::CommandExecutor;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct VpnStatus {
    pub interface: String,
    pub public_key: String,
    pub listening_port: String,
    pub endpoint: String,
    pub allowed_ips: String,
    pub latest_handshake: String,
    pub transfer_received: String,
    pub transfer_sent: String,
    pub is_connected: bool,
}

impl Default for VpnStatus {
    fn default() -> Self {
        Self {
            interface: String::new(),
            public_key: String::new(),
            listening_port: String::new(),
            endpoint: String::new(),
            allowed_ips: String::new(),
            latest_handshake: String::new(),
            transfer_received: String::new(),
            transfer_sent: String::new(),
            is_connected: false,
        }
    }
}

pub struct VpnManager;

impl VpnManager {
    pub async fn check_wireguard() -> Result<bool> {
        CommandExecutor::check_wireguard_installed()
    }

    pub async fn install_if_needed() -> Result<()> {
        if !CommandExecutor::check_wireguard_installed()? {
            CommandExecutor::install_wireguard()?;
        }
        Ok(())
    }

    pub async fn connect(config_name: &str) -> Result<String> {
        CommandExecutor::connect_vpn(config_name)
    }

    pub async fn disconnect(config_name: &str) -> Result<String> {
        CommandExecutor::disconnect_vpn(config_name)
    }

    pub async fn get_status(config_name: &str) -> Result<VpnStatus> {
        match CommandExecutor::get_vpn_status(config_name) {
            Ok(output) => {
                let status = Self::parse_status(&output, config_name);
                Ok(status)
            }
            Err(_) => Ok(VpnStatus {
                interface: config_name.to_string(),
                is_connected: false,
                ..Default::default()
            }),
        }
    }

    pub async fn get_active_connection() -> Result<Option<String>> {
        CommandExecutor::get_active_vpn()
    }

    pub async fn get_current_ip() -> Result<String> {
        CommandExecutor::get_current_ip()
    }

    fn parse_status(output: &str, interface: &str) -> VpnStatus {
        let mut status = VpnStatus {
            interface: interface.to_string(),
            is_connected: true,
            ..Default::default()
        };

        let re_port = Regex::new(r"Listening port:\s*(\d+)").unwrap();
        let re_pubkey = Regex::new(r"public key:\s*([^\n]+)").unwrap();
        let re_endpoint = Regex::new(r"endpoint:\s*([^\n]+)").unwrap();
        let re_allowed = Regex::new(r"Allowed IPs:\s*([^\n]+)").unwrap();
        let re_handshake = Regex::new(r"Latest handshake:\s*([^\n]+)").unwrap();
        let re_transfer = Regex::new(r"transfer:\s*([^\n]+)").unwrap();

        if let Some(cap) = re_port.captures(output) {
            status.listening_port = cap[1].to_string();
        }

        if let Some(cap) = re_pubkey.captures(output) {
            status.public_key = cap[1].trim().to_string();
        }

        if let Some(cap) = re_endpoint.captures(output) {
            status.endpoint = cap[1].trim().to_string();
        }

        if let Some(cap) = re_allowed.captures(output) {
            status.allowed_ips = cap[1].trim().to_string();
        }

        if let Some(cap) = re_handshake.captures(output) {
            status.latest_handshake = cap[1].trim().to_string();
        }

        if let Some(cap) = re_transfer.captures(output) {
            let transfer_str = cap[1].trim();
            if let Some((received, sent)) = transfer_str.split_once(',') {
                status.transfer_received = received.trim().to_string();
                status.transfer_sent = sent.trim().to_string();
            }
        }

        status
    }
}
