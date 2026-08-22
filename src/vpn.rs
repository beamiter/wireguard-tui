use crate::commands::CommandExecutor;
use crate::config::validate_interface_name;
use anyhow::{bail, Context, Result};
use std::net::IpAddr;
use std::time::Duration;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VpnStatus {
    pub interface: String,
    pub public_key: String,
    pub listening_port: String,
    pub endpoint: String,
    pub allowed_ips: String,
    pub latest_handshake: String,
    pub transfer_received: String,
    pub transfer_sent: String,
}

pub struct VpnManager;

const MAX_STATUS_BYTES: usize = 64 * 1024;
const MAX_STATUS_LINES: usize = 4096;
const MAX_STATUS_LINE_BYTES: usize = 4096;
const MAX_STATUS_PEERS: usize = 256;

impl VpnManager {
    pub async fn connect(config_name: &str) -> Result<String> {
        validate_interface_name(config_name)?;
        let config_name = config_name.to_string();
        tokio::task::spawn_blocking(move || {
            if CommandExecutor::check_interface_exists(&config_name)? {
                CommandExecutor::disconnect_vpn(&config_name)?;
            }

            CommandExecutor::connect_vpn(&config_name)
        })
        .await?
    }

    pub async fn disconnect(config_name: &str) -> Result<String> {
        validate_interface_name(config_name)?;
        let config_name = config_name.to_string();
        tokio::task::spawn_blocking(move || CommandExecutor::disconnect_vpn(&config_name)).await?
    }

    pub async fn get_status(config_name: &str) -> Result<VpnStatus> {
        validate_interface_name(config_name)?;
        let config_name = config_name.to_string();
        tokio::task::spawn_blocking(move || -> Result<VpnStatus> {
            let output = CommandExecutor::get_vpn_status(&config_name)?;
            Self::parse_status(&output, &config_name)
        })
        .await?
    }

    pub async fn get_active_connections() -> Result<Vec<String>> {
        tokio::task::spawn_blocking(CommandExecutor::get_active_vpns).await?
    }

    pub async fn get_current_ip() -> Result<String> {
        const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
        const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("failed to build the public IP HTTP client")?;
        let mut response = client
            .get("https://api.ipify.org")
            .header(reqwest::header::ACCEPT, "text/plain")
            .send()
            .await
            .context("failed to query the public IP service")?
            .error_for_status()
            .context("the public IP service returned an error status")?;
        const MAX_IP_RESPONSE_BYTES: usize = 128;
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .context("failed to read the public IP response")?
        {
            if body.len().saturating_add(chunk.len()) > MAX_IP_RESPONSE_BYTES {
                bail!("public IP service returned an oversized response");
            }
            body.extend_from_slice(&chunk);
        }
        let body = std::str::from_utf8(&body)
            .context("public IP service returned a non-UTF-8 response")?;

        Self::parse_ip_response(body)
    }

    fn parse_ip_response(body: &str) -> Result<String> {
        let value = body.trim();
        let ip = value
            .parse::<IpAddr>()
            .with_context(|| format!("public IP service returned an invalid address: {value:?}"))?;
        Ok(ip.to_string())
    }

    fn parse_status(output: &str, interface: &str) -> Result<VpnStatus> {
        if output.len() > MAX_STATUS_BYTES {
            bail!("wg status output exceeds the {MAX_STATUS_BYTES} byte safety limit");
        }

        let mut status = VpnStatus {
            interface: interface.to_string(),
            ..Default::default()
        };
        let mut peer_count = 0usize;
        let mut interface_seen = false;

        for (index, line) in output.lines().enumerate() {
            if index >= MAX_STATUS_LINES {
                bail!("wg status output exceeds the {MAX_STATUS_LINES} line safety limit");
            }
            if line.len() > MAX_STATUS_LINE_BYTES {
                bail!("wg status output contains an oversized line");
            }
            let Some((label, value)) = line.trim().split_once(':') else {
                continue;
            };
            let value = value.trim();
            if value.chars().any(char::is_control) {
                bail!("wg status output contains control characters");
            }
            let label = label.trim().to_ascii_lowercase();

            if label == "peer" {
                if !interface_seen {
                    bail!("wg status output contains a peer before its interface record");
                }
                peer_count += 1;
                if peer_count > MAX_STATUS_PEERS {
                    bail!("wg status output exceeds the {MAX_STATUS_PEERS} peer safety limit");
                }
                continue;
            }
            let in_first_peer = peer_count == 1;

            match label.as_str() {
                "interface" => {
                    if interface_seen {
                        bail!("wg status output contains duplicate interface records");
                    }
                    if peer_count != 0 {
                        bail!("wg status output contains an interface after peer records");
                    }
                    validate_interface_name(value)
                        .context("wg status output contains an invalid interface name")?;
                    if value != interface {
                        bail!("wg status output does not match the requested interface");
                    }
                    interface_seen = true;
                    status.interface = value.to_string();
                }
                "public key" if peer_count == 0 => {
                    if !interface_seen {
                        bail!("wg status output contains an interface field before its interface record");
                    }
                    if !status.public_key.is_empty() {
                        bail!("wg status output contains duplicate interface public keys");
                    }
                    status.public_key = value.to_string();
                }
                "listening port" if peer_count == 0 => {
                    if !interface_seen {
                        bail!("wg status output contains an interface field before its interface record");
                    }
                    if !status.listening_port.is_empty() {
                        bail!("wg status output contains duplicate listening ports");
                    }
                    status.listening_port = value.to_string();
                }
                "endpoint" if in_first_peer && status.endpoint.is_empty() => {
                    status.endpoint = value.to_string();
                }
                "allowed ips" if in_first_peer && status.allowed_ips.is_empty() => {
                    status.allowed_ips = value.to_string();
                }
                "latest handshake" if in_first_peer && status.latest_handshake.is_empty() => {
                    status.latest_handshake = value.to_string();
                }
                "transfer" if in_first_peer && status.transfer_received.is_empty() => {
                    if let Some((received, sent)) = value.split_once(',') {
                        status.transfer_received =
                            Self::trim_transfer_direction(received, "received");
                        status.transfer_sent = Self::trim_transfer_direction(sent, "sent");
                    }
                }
                _ => {}
            }
        }

        if !interface_seen {
            bail!("wg status output is missing its interface record");
        }
        Ok(status)
    }

    fn trim_transfer_direction(value: &str, direction: &str) -> String {
        value
            .trim()
            .strip_suffix(direction)
            .unwrap_or(value.trim())
            .trim()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        VpnManager, MAX_STATUS_BYTES, MAX_STATUS_LINES, MAX_STATUS_LINE_BYTES, MAX_STATUS_PEERS,
    };

    const WG_SHOW_FIXTURE: &str = r#"interface: wg-home
  public key: interface-public-key=
  private key: (hidden)
  listening port: 51820

peer: peer-public-key=
  endpoint: 198.51.100.24:51820
  allowed ips: 0.0.0.0/0, ::/0
  latest handshake: 42 seconds ago
  transfer: 12.34 MiB received, 5.67 MiB sent
  persistent keepalive: every 25 seconds
"#;

    #[test]
    fn parses_real_lowercase_wg_show_output() {
        let status = VpnManager::parse_status(WG_SHOW_FIXTURE, "wg-home").unwrap();

        assert_eq!(status.interface, "wg-home");
        assert_eq!(status.public_key, "interface-public-key=");
        assert_eq!(status.listening_port, "51820");
        assert_eq!(status.endpoint, "198.51.100.24:51820");
        assert_eq!(status.allowed_ips, "0.0.0.0/0, ::/0");
        assert_eq!(status.latest_handshake, "42 seconds ago");
        assert_eq!(status.transfer_received, "12.34 MiB");
        assert_eq!(status.transfer_sent, "5.67 MiB");
    }

    #[test]
    fn status_parser_requires_interface_before_peer_records() {
        assert!(VpnManager::parse_status("", "wg0").is_err());
        assert!(VpnManager::parse_status("peer: key\n  endpoint: host:51820\n", "wg0").is_err());
        assert!(VpnManager::parse_status("public key: peer-key\n", "wg0").is_err());
    }

    #[test]
    fn status_fields_never_mix_multiple_peers() {
        let status = VpnManager::parse_status(
            "interface: wg0\npeer: first\n  allowed ips: 10.0.0.0/8\npeer: second\n  endpoint: second.example:51820\n  latest handshake: now\n  transfer: 9 MiB received, 8 MiB sent\n",
            "wg0",
        )
        .unwrap();

        assert_eq!(status.allowed_ips, "10.0.0.0/8");
        assert!(status.endpoint.is_empty());
        assert!(status.latest_handshake.is_empty());
        assert!(status.transfer_received.is_empty());
        assert!(status.transfer_sent.is_empty());
    }

    #[test]
    fn parses_and_canonicalizes_public_ip_responses() {
        assert_eq!(
            VpnManager::parse_ip_response(" 203.0.113.7\n").unwrap(),
            "203.0.113.7"
        );
        assert_eq!(
            VpnManager::parse_ip_response("2001:0db8::1").unwrap(),
            "2001:db8::1"
        );
    }

    #[test]
    fn rejects_non_ip_public_ip_responses() {
        assert!(VpnManager::parse_ip_response("").is_err());
        assert!(VpnManager::parse_ip_response("203.0.113.7 extra").is_err());
        assert!(VpnManager::parse_ip_response("<html>error</html>").is_err());
    }

    #[test]
    fn status_parser_rejects_mismatched_duplicate_and_invalid_interfaces() {
        assert!(VpnManager::parse_status("interface: wg1\n", "wg0").is_err());
        assert!(VpnManager::parse_status("interface: ../wg0\n", "wg0").is_err());
        assert!(VpnManager::parse_status("interface: wg0\ninterface: wg0\n", "wg0").is_err());
        assert!(VpnManager::parse_status(
            "interface: wg0\n  listening port: 1\n  listening port: 2\n",
            "wg0",
        )
        .is_err());
        assert!(VpnManager::parse_status(
            "interface: wg0\n  public key: first\n  public key: second\n",
            "wg0",
        )
        .is_err());
    }

    #[test]
    fn status_parser_enforces_output_resource_limits() {
        assert!(VpnManager::parse_status(&"x".repeat(MAX_STATUS_BYTES + 1), "wg0").is_err());
        assert!(VpnManager::parse_status(
            &format!("endpoint: {}\n", "x".repeat(MAX_STATUS_LINE_BYTES)),
            "wg0",
        )
        .is_err());

        let too_many_peers = format!(
            "interface: wg0\n{}",
            "peer: key\n".repeat(MAX_STATUS_PEERS + 1)
        );
        assert!(VpnManager::parse_status(&too_many_peers, "wg0").is_err());

        let too_many_lines = "\n".repeat(MAX_STATUS_LINES + 1);
        assert!(VpnManager::parse_status(&too_many_lines, "wg0").is_err());
    }

    #[test]
    fn status_parser_rejects_control_characters_in_values() {
        assert!(VpnManager::parse_status("interface: wg0\0suffix\n", "wg0").is_err());
    }
}
