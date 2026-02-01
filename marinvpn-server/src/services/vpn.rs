use crate::error::AppResult;
use tokio::process::Command;
use tracing::{error, info, warn};

pub struct VpnOrchestrator {
    interface: String,
    mock_mode: bool,
}

impl VpnOrchestrator {
    pub fn new(interface: String) -> Self {
        let mock_mode = std::process::Command::new("wg")
            .arg("--version")
            .output()
            .is_err();

        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".to_string());
        let is_prod = matches!(run_mode.to_lowercase().as_str(), "production" | "prod");

        if mock_mode {
            if is_prod {
                panic!("CRITICAL: 'wg' command not found in PRODUCTION mode. Server cannot start.");
            }
            warn!("'wg' command not found. VpnOrchestrator running in MOCK mode.");
        }

        Self {
            interface,
            mock_mode,
        }
    }

    pub async fn register_peer(&self, pub_key: &str, allowed_ip: &str) -> AppResult<()> {
        let masked_key = if pub_key.len() >= 8 {
            format!("{}...", &pub_key[0..8])
        } else {
            "***".to_string()
        };

        if self.mock_mode {
            info!(
                "[MOCK] Registering peer {} on {}",
                masked_key, self.interface
            );
            return Ok(());
        }

        let ip_only = allowed_ip.split('/').next().unwrap_or(allowed_ip);

        info!("Registering peer {} on {}", masked_key, self.interface);

        let output = Command::new("wg")
            .arg("set")
            .arg(&self.interface)
            .arg("peer")
            .arg(pub_key)
            .arg("allowed-ips")
            .arg(format!("{}/32", ip_only))
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => {
                info!("Successfully registered peer on WireGuard interface");
                Ok(())
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr);
                error!("Failed to register peer: {}", err);
                Err(anyhow::anyhow!("WireGuard command failed: {}", err).into())
            }
            Err(e) => {
                error!("Exec error calling wg: {}", e);
                Err(anyhow::anyhow!("Failed to execute wg command: {}", e).into())
            }
        }
    }

    pub async fn remove_peer(&self, pub_key: &str) -> AppResult<()> {
        let masked_key = if pub_key.len() >= 8 {
            format!("{}...", &pub_key[0..8])
        } else {
            "***".to_string()
        };

        if self.mock_mode {
            info!(
                "[MOCK] Removing peer {} from {}",
                masked_key, self.interface
            );
            return Ok(());
        }

        info!("Removing peer {} from {}", masked_key, self.interface);

        let output = Command::new("wg")
            .arg("set")
            .arg(&self.interface)
            .arg("peer")
            .arg(pub_key)
            .arg("remove")
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr);
                error!("Failed to remove peer: {}", err);
                Err(anyhow::anyhow!("WireGuard command failed: {}", err).into())
            }
            Err(e) => Err(anyhow::anyhow!("Failed to execute wg command: {}", e).into()),
        }
    }

    pub async fn remove_all_peers(&self) -> AppResult<()> {
        if self.mock_mode {
            info!("[MOCK] Removing all peers from {}", self.interface);
            return Ok(());
        }

        info!(
            "CRITICAL: Removing all peers from WireGuard interface {}",
            self.interface
        );

        let _ = Command::new("ip")
            .args(["link", "delete", &self.interface])
            .status()
            .await;
        let _ = Command::new("ip")
            .args(["link", "add", &self.interface, "type", "wireguard"])
            .status()
            .await;

        Ok(())
    }
}
