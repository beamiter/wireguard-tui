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
            Ok(Self::parse_status(&output, &config_name))
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

    fn parse_status(output: &str, interface: &str) -> VpnStatus {
        let mut status = VpnStatus {
            interface: interface.to_string(),
            ..Default::default()
        };
        let mut peer_count = 0usize;

        for line in output.lines() {
            let Some((label, value)) = line.trim().split_once(':') else {
                continue;
            };
            let value = value.trim();
            let label = label.trim().to_ascii_lowercase();

            if label == "peer" {
                peer_count += 1;
                continue;
            }
            let in_first_peer = peer_count == 1;

            match label.as_str() {
                "interface" => {
                    if !value.is_empty() {
                        status.interface = value.to_string();
                    }
                }
                "public key" if status.public_key.is_empty() => {
                    status.public_key = value.to_string();
                }
                "listening port" => status.listening_port = value.to_string(),
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

        status
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
    use super::VpnManager;

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
        let status = VpnManager::parse_status(WG_SHOW_FIXTURE, "fallback-name");

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
    fn status_parser_keeps_requested_name_when_interface_line_is_missing() {
        let status = VpnManager::parse_status("peer: key\n  endpoint: host:51820\n", "wg0");
        assert_eq!(status.interface, "wg0");
        assert_eq!(status.endpoint, "host:51820");
    }

    #[test]
    fn status_fields_never_mix_multiple_peers() {
        let status = VpnManager::parse_status(
            "interface: wg0\npeer: first\n  allowed ips: 10.0.0.0/8\npeer: second\n  endpoint: second.example:51820\n  latest handshake: now\n  transfer: 9 MiB received, 8 MiB sent\n",
            "wg0",
        );

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
}
