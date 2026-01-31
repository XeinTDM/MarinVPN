use std::process::Command;
use crate::error::AppResult;
use tracing::{info, warn, error};

pub struct VpnOrchestrator {
    interface: String,
    mock_mode: bool,
}

impl VpnOrchestrator {
    pub fn new(interface: String) -> Self {
        // Check if wg command exists
        let mock_mode = Command::new("wg").arg("--version").output().is_err();
        if mock_mode {
            warn!("'wg' command not found. VpnOrchestrator running in MOCK mode.");
        }
        
        Self { interface, mock_mode }
    }

    pub async fn register_peer(&self, pub_key: &str, allowed_ip: &str) -> AppResult<()> {
        if self.mock_mode {
            info!("[MOCK] Registering peer {} with IP {} on {}", pub_key, allowed_ip, self.interface);
            return Ok(());
        }

        // Strip /32 from IP if present for wg set command
        let ip_only = allowed_ip.split('/').next().unwrap_or(allowed_ip);

        info!("Registering peer {} with IP {} on {}", pub_key, ip_only, self.interface);
        
        let output = Command::new("wg")
            .arg("set")
            .arg(&self.interface)
            .arg("peer")
            .arg(pub_key)
            .arg("allowed-ips")
            .arg(format!("{}/32", ip_only))
            .output();

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
}
