use crate::models::{ConnectionStatus, SettingsState, StealthMode, WireGuardConfig};
use base64::Engine;
use rand::Rng;
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::{broadcast, Mutex};
use tracing::{error, info, warn};

#[derive(Clone, Debug)]
pub struct VpnStats {
    pub download_speed: f64,
    pub upload_speed: f64,
    pub total_download: u64,
    pub total_upload: u64,
    pub latest_handshake: u64,
}

#[derive(Clone, Debug)]
pub enum VpnError {
    ConfigMissing,
    NetworkUnreachable,
    ConnectionFailed(String),
    InterfaceError(String),
    PermissionDenied,
    DriverMissing,
    NotRoot,
    FirewallError(String),
}

impl std::fmt::Display for VpnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VpnError::ConfigMissing => write!(f, "Missing WireGuard configuration."),
            VpnError::NetworkUnreachable => write!(f, "No internet connection detected."),
            VpnError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            VpnError::InterfaceError(msg) => write!(f, "Interface error: {}", msg),
            VpnError::PermissionDenied => write!(f, "Administrator privileges required."),
            VpnError::DriverMissing => write!(f, "WireGuard driver/tools not found."),
            VpnError::NotRoot => {
                write!(f, "Root/Admin privileges are required for VPN operations.")
            }
            VpnError::FirewallError(msg) => write!(f, "Firewall/Kill-switch error: {}", msg),
        }
    }
}

#[derive(Clone, Debug)]
pub enum VpnEvent {
    StatusChanged(ConnectionStatus),
    LocationChanged(String),
    StatsUpdated(VpnStats),
    Error(VpnError),
    CaptivePortalActive(bool),
}

#[async_trait::async_trait]
pub trait VpnService: Send + Sync {
    fn subscribe(&self) -> broadcast::Receiver<VpnEvent>;
    async fn connect(
        &self,
        entry: String,
        entry_config: WireGuardConfig,
        exit: Option<(String, WireGuardConfig)>,
        settings: SettingsState,
        auth: Option<(String, String)>,
    );
    async fn disconnect(&self);
    async fn get_status(&self) -> ConnectionStatus;
    async fn enable_captive_portal(&self, duration_secs: u64);
    async fn apply_lockdown(&self, settings: &SettingsState) -> Result<(), VpnError>;
    async fn disable_kill_switch(&self);
}

#[async_trait::async_trait]
trait WgRunner: Send + Sync {
    async fn up(
        &self,
        entry: &WireGuardConfig,
        exit: Option<&WireGuardConfig>,
        settings: &SettingsState,
    ) -> Result<(), VpnError>;
    async fn down(&self) -> Result<(), VpnError>;
    async fn get_stats(&self) -> Result<VpnStats, VpnError>;
    async fn apply_app_bypass(&self, app_path: &str);
    async fn apply_bypass_route(&self, ip: &str);
    async fn apply_single_up(&self, iface: &str, conf: &str) -> Result<(), VpnError>;
    async fn apply_single_down(&self, iface: &str);
    async fn enable_kill_switch(
        &self,
        endpoint: &str,
        settings: &SettingsState,
    ) -> Result<(), VpnError>;
    async fn disable_kill_switch(&self);
}

#[derive(Clone)]
struct ConnectionContext {
    entry_name: String,
    entry_config: WireGuardConfig,
    exit: Option<(String, WireGuardConfig)>,
    settings: SettingsState,
    account_number: Option<String>,
    auth_token: Option<String>,
}

#[derive(Clone)]
pub struct WireGuardService {
    event_tx: broadcast::Sender<VpnEvent>,
    current_status: Arc<Mutex<ConnectionStatus>>,
    runner: Arc<Box<dyn WgRunner>>,
    active_context: Arc<Mutex<Option<ConnectionContext>>>,
}

impl WireGuardService {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);

        let runner: Box<dyn WgRunner> = if std::env::var("MARIN_MOCK").is_ok() {
            info!("Initializing VPN Service in MOCK/SIMULATION mode.");
            Box::new(SimulationRunner::new())
        } else {
            Box::new(RealWgRunner::new())
        };

        Self {
            event_tx: tx,
            current_status: Arc::new(Mutex::new(ConnectionStatus::Disconnected)),
            runner: Arc::new(runner),
            active_context: Arc::new(Mutex::new(None)),
        }
    }

    async fn set_status(&self, status: ConnectionStatus) {
        let mut lock = self.current_status.lock().await;
        *lock = status;
        let _ = self.event_tx.send(VpnEvent::StatusChanged(status));
    }

    async fn emit_error(&self, error: VpnError) {
        let msg = error.to_string();
        error!("{}", msg);
        let _ = self.event_tx.send(VpnEvent::Error(error));
        self.set_status(ConnectionStatus::Disconnected).await;
    }

    async fn check_connectivity(&self) -> Result<(), VpnError> {
        let internet_check = tokio::task::spawn_blocking(|| {
            let targets = [([1, 1, 1, 1], 53), ([8, 8, 8, 8], 53)];
            for addr in targets {
                if TcpStream::connect_timeout(&SocketAddr::from(addr), Duration::from_secs(2))
                    .is_ok()
                {
                    return true;
                }
            }
            false
        })
        .await
        .unwrap_or(false);

        if !internet_check {
            return Err(VpnError::NetworkUnreachable);
        }
        Ok(())
    }

    fn start_stats_loop(&self, settings: SettingsState) {
        let tx = self.event_tx.clone();
        let status_lock = self.current_status.clone();
        let runner = self.runner.clone();
        let svc = self.clone();

        if settings.daita_enabled {
            self.start_daita_task(status_lock.clone(), self.active_context.clone());
        }

        self.start_health_monitor(status_lock.clone());

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
            loop {
                interval.tick().await;
                if *status_lock.lock().await != ConnectionStatus::Connected {
                    break;
                }
                if let Ok(stats) = runner.get_stats().await {
                    let _ = tx.send(VpnEvent::StatsUpdated(stats.clone()));

                    if stats.latest_handshake > 0 {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        if now.saturating_sub(stats.latest_handshake) > 180 {
                            warn!("Handshake stale. Triggering self-healing...");
                            let ctx_lock = svc.active_context.lock().await;
                            if let Some(ctx) = ctx_lock.as_ref() {
                                let entry_n = ctx.entry_name.clone();
                                let entry_c = ctx.entry_config.clone();
                                let exit = ctx.exit.clone();
                                let sets = ctx.settings.clone();
                                let auth = if let (Some(a), Some(t)) =
                                    (&ctx.account_number, &ctx.auth_token)
                                {
                                    Some((a.clone(), t.clone()))
                                } else {
                                    None
                                };
                                drop(ctx_lock);

                                svc.disconnect().await;
                                svc.connect(entry_n, entry_c, exit, sets, auth).await;
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    fn start_daita_task(
        &self,
        status_lock: Arc<Mutex<ConnectionStatus>>,
        context_lock: Arc<Mutex<Option<ConnectionContext>>>,
    ) {
        tokio::spawn(async move {
            info!("DAITA: Defense Against AI-guided Traffic Analysis ACTIVE.");
            info!("DAITA: Using multi-modal traffic masking (Browsing, Streaming, VOIP mimics).");

            let fallback_targets = ["1.1.1.1:53", "8.8.8.8:53", "9.9.9.9:53"];

            loop {
                let (is_connected, endpoint) = {
                    let status = *status_lock.lock().await;
                    let ctx = context_lock.lock().await;
                    let ep = ctx.as_ref().map(|c| c.entry_config.endpoint.clone());
                    (status == ConnectionStatus::Connected, ep)
                };

                if !is_connected {
                    break;
                }

                let target = endpoint.unwrap_or_else(|| {
                    let mut rng = rand::thread_rng();
                    fallback_targets[rng.gen_range(0..fallback_targets.len())].to_string()
                });

                let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok();

                let (burst_count, base_delay, packet_size_range, jitter_range, mode) = {
                    let mut rng = rand::thread_rng();
                    let mode = rng.gen_range(0..100);

                    let (burst_count, base_delay, packet_size_range, jitter_range) = if mode < 40 {
                        // Browsing
                        (rng.gen_range(3..10), 1000..3000, 64..1200, 50..500)
                    } else if mode < 70 {
                        // Streaming
                        (rng.gen_range(20..50), 100..500, 800..1450, 5..30)
                    } else if mode < 90 {
                        // VOIP
                        (rng.gen_range(50..100), 20..60, 64..256, 1..5)
                    } else {
                        // Large File Transfer
                        (rng.gen_range(100..250), 5000..15000, 1200..1420, 1..10)
                    };
                    (
                        burst_count,
                        base_delay,
                        packet_size_range,
                        jitter_range,
                        mode,
                    )
                };

                for _ in 0..burst_count {
                    let size;
                    let mut noise;
                    let jitter;

                    {
                        let mut rng = rand::thread_rng();
                        size = rng.gen_range(packet_size_range.clone());
                        noise = vec![0u8; size];
                        rng.fill(&mut noise[..]);
                        jitter = rng.gen_range(jitter_range.clone());
                    }

                    if size > 4 {
                        if mode < 40 {
                            noise[0] = 0x16;
                            noise[1] = 0x03;
                            noise[2] = 0x01;
                        } else if (70..90).contains(&mode) {
                            noise[0] = 0x80;
                            noise[1] = 0x08;
                        }
                    }

                    if let Some(ref s) = socket {
                        let _ = s.send_to(&noise, &target);
                    }

                    tokio::time::sleep(Duration::from_millis(jitter)).await;
                }

                let next_burst_delay = {
                    let mut rng = rand::thread_rng();
                    rng.gen_range(base_delay)
                };
                tokio::time::sleep(Duration::from_millis(next_burst_delay)).await;
            }
        });
    }
    fn start_health_monitor(&self, status_lock: Arc<Mutex<ConnectionStatus>>) {
        let svc = self.clone();
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            let mut failure_count = 0;

            loop {
                interval.tick().await;

                let is_connected = *status_lock.lock().await == ConnectionStatus::Connected;
                if !is_connected {
                    break;
                }

                let health_check = tokio::task::spawn_blocking(|| {
                    let targets = [([1, 1, 1, 1], 53), ([8, 8, 8, 8], 53)];
                    for addr in targets {
                        if TcpStream::connect_timeout(
                            &SocketAddr::from(addr),
                            Duration::from_secs(3),
                        )
                        .is_ok()
                        {
                            return true;
                        }
                    }
                    false
                })
                .await
                .unwrap_or(false);

                if !health_check {
                    failure_count += 1;
                    warn!("Tunnel health check failed ({}/3)", failure_count);

                    if failure_count >= 3 {
                        error!(
                            "Tunnel detected as 'Silent Dead'. Triggering emergency failover..."
                        );
                        let _ = tx.send(VpnEvent::Error(VpnError::ConnectionFailed(
                            "Silent network failure detected. Switching servers...".to_string(),
                        )));

                        let ctx_lock = svc.active_context.lock().await;
                        if let Some(ctx) = ctx_lock.as_ref() {
                            let (en, ec, ex, st) = (
                                ctx.entry_name.clone(),
                                ctx.entry_config.clone(),
                                ctx.exit.clone(),
                                ctx.settings.clone(),
                            );
                            let auth = if let (Some(a), Some(t)) =
                                (&ctx.account_number, &ctx.auth_token)
                            {
                                Some((a.clone(), t.clone()))
                            } else {
                                None
                            };
                            drop(ctx_lock);

                            svc.disconnect().await;
                            tokio::time::sleep(Duration::from_secs(3)).await;

                            if st.entry_location == "Automatic" {
                                info!("Failover: Re-scanning for best available server...");
                                if let Ok(new_server) =
                                    crate::services::servers::ServersService::find_best_server(None)
                                        .await
                                {
                                    info!("Failover: Found new candidate {}. Fetching fresh configuration...", new_server.city);

                                    let mut final_config = ec;
                                    if let Some((ref a, ref t)) = auth {
                                        let location =
                                            format!("{}, {}", new_server.country, new_server.city);
                                        if let Ok(cfg) =
                                            crate::services::auth::AuthService::get_config(
                                                a,
                                                &location,
                                                t,
                                                Some(st.dns_blocking.clone()),
                                                st.quantum_resistant,
                                            )
                                            .await
                                        {
                                            final_config = cfg;
                                        }
                                    }

                                    svc.connect(new_server.city, final_config, ex, st, auth)
                                        .await;
                                } else {
                                    svc.connect(en, ec, ex, st, auth).await;
                                }
                            } else {
                                svc.connect(en, ec, ex, st, auth).await;
                            }
                        }
                        break;
                    }
                } else {
                    failure_count = 0;
                }
            }
        });
    }
}

#[async_trait::async_trait]
impl VpnService for WireGuardService {
    fn subscribe(&self) -> broadcast::Receiver<VpnEvent> {
        self.event_tx.subscribe()
    }

    async fn get_status(&self) -> ConnectionStatus {
        *self.current_status.lock().await
    }

    async fn connect(
        &self,
        entry: String,
        entry_config: WireGuardConfig,
        exit: Option<(String, WireGuardConfig)>,
        settings: SettingsState,
        auth: Option<(String, String)>,
    ) {
        {
            let status = self.current_status.lock().await;
            if *status == ConnectionStatus::Connected || *status == ConnectionStatus::Connecting {
                return;
            }
        }

        {
            let mut lock = self.active_context.lock().await;
            let (account_number, auth_token) = if let Some((a, t)) = auth {
                (Some(a), Some(t))
            } else {
                (None, None)
            };

            *lock = Some(ConnectionContext {
                entry_name: entry.clone(),
                entry_config: entry_config.clone(),
                exit: exit.clone(),
                settings: settings.clone(),
                account_number,
                auth_token,
            });
        }

        self.set_status(ConnectionStatus::Connecting).await;

        let display_location = if let Some((ref exit_name, _)) = exit {
            format!("{} → {}", entry, exit_name)
        } else {
            entry.clone()
        };

        let _ = self
            .event_tx
            .send(VpnEvent::LocationChanged(display_location.clone()));
        info!("Initiating WireGuard connection: {}", display_location);

        if let Err(e) = self.check_connectivity().await {
            self.emit_error(e).await;
            return;
        }

        let endpoint = exit
            .as_ref()
            .map(|(_, c)| &c.endpoint)
            .unwrap_or(&entry_config.endpoint);
        if let Err(e) = self.runner.enable_kill_switch(endpoint, &settings).await {
            self.emit_error(e).await;
            return;
        }

        match self
            .runner
            .up(&entry_config, exit.as_ref().map(|(_, c)| c), &settings)
            .await
        {
            Ok(_) => {
                info!("Tunnel established successfully.");
                self.set_status(ConnectionStatus::Connected).await;
                self.start_stats_loop(settings);
            }
            Err(e) => {
                error!("Failed to establish tunnel: {}", e);
                if !settings.lockdown_mode {
                    warn!("Cleaning up kill-switch after failed connection...");
                    self.runner.disable_kill_switch().await;
                }
                self.emit_error(e).await;
            }
        }
    }

    async fn disconnect(&self) {
        let status = self.get_status().await;
        if status == ConnectionStatus::Disconnected || status == ConnectionStatus::Disconnecting {
            return;
        }

        let settings = {
            let mut lock = self.active_context.lock().await;
            let s = lock.as_ref().map(|ctx| ctx.settings.clone());
            *lock = None;
            s
        };

        self.set_status(ConnectionStatus::Disconnecting).await;
        info!("Tearing down WireGuard interface...");

        match self.runner.down().await {
            Ok(_) => {
                self.set_status(ConnectionStatus::Disconnected).await;

                if let Some(s) = settings {
                    if s.lockdown_mode {
                        warn!("Lockdown Mode active: internet remains blocked after manual disconnect.");
                        let _ = self.runner.enable_kill_switch("0.0.0.0", &s).await;
                    } else {
                        self.runner.disable_kill_switch().await;
                    }
                }

                let _ = self.event_tx.send(VpnEvent::StatsUpdated(VpnStats {
                    download_speed: 0.0,
                    upload_speed: 0.0,
                    total_download: 0,
                    total_upload: 0,
                    latest_handshake: 0,
                }));
            }
            Err(e) => {
                error!("Failed to disconnect: {}", e);
                self.set_status(ConnectionStatus::Disconnected).await;
            }
        }
    }

    async fn enable_captive_portal(&self, duration_secs: u64) {
        let runner = self.runner.clone();
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            info!(
                "Captive Portal Mode: Temporarily disabling firewall for {}s",
                duration_secs
            );
            let _ = tx.send(VpnEvent::CaptivePortalActive(true));
            runner.down().await.ok();
            runner.disable_kill_switch().await;

            tokio::time::sleep(Duration::from_secs(duration_secs)).await;

            info!("Captive Portal Mode: Restoring firewall...");
            let _ = tx.send(VpnEvent::CaptivePortalActive(false));
        });
    }

    async fn apply_lockdown(&self, settings: &SettingsState) -> Result<(), VpnError> {
        if settings.lockdown_mode {
            info!("Lockdown Mode enabled: enforcing persistent fail-closed firewall.");
            self.runner.enable_kill_switch("0.0.0.0", settings).await?;
        } else {
            let status = self.get_status().await;
            if status == ConnectionStatus::Disconnected {
                info!("Lockdown Mode disabled: restoring regular internet access.");
                self.runner.disable_kill_switch().await;
            } else {
                info!(
                    "Lockdown Mode disabled: built-in Kill Switch remains active for this session."
                );
            }
        }
        Ok(())
    }

    async fn disable_kill_switch(&self) {
        self.runner.disable_kill_switch().await;
    }
}

impl Default for WireGuardService {
    fn default() -> Self {
        Self::new()
    }
}

struct SimulationRunner {
    state: Mutex<SimulationState>,
}

struct SimulationState {
    total_download: u64,
    total_upload: u64,
}

impl SimulationRunner {
    fn new() -> Self {
        Self {
            state: Mutex::new(SimulationState {
                total_download: 0,
                total_upload: 0,
            }),
        }
    }
}

#[async_trait::async_trait]
impl WgRunner for SimulationRunner {
    async fn up(
        &self,
        _entry: &WireGuardConfig,
        exit: Option<&WireGuardConfig>,
        _settings: &SettingsState,
    ) -> Result<(), VpnError> {
        tokio::time::sleep(Duration::from_millis(800)).await;
        if exit.is_some() {
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
        Ok(())
    }

    async fn down(&self) -> Result<(), VpnError> {
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    async fn get_stats(&self) -> Result<VpnStats, VpnError> {
        let (dl_speed, ul_speed) = {
            let mut rng = rand::thread_rng();
            (
                50.0 + rng.gen_range(-10.0..20.0),
                30.0 + rng.gen_range(-5.0..10.0),
            )
        };
        let mut state = self.state.lock().await;
        state.total_download += (dl_speed * 1024.0) as u64;
        state.total_upload += (ul_speed * 1024.0) as u64;

        Ok(VpnStats {
            download_speed: dl_speed,
            upload_speed: ul_speed,
            total_download: state.total_download,
            total_upload: state.total_upload,
            latest_handshake: 0,
        })
    }

    async fn apply_app_bypass(&self, _app_path: &str) {}
    async fn apply_bypass_route(&self, _ip: &str) {}
    async fn apply_single_up(&self, _iface: &str, _conf: &str) -> Result<(), VpnError> {
        Ok(())
    }
    async fn apply_single_down(&self, _iface: &str) {}
    async fn enable_kill_switch(
        &self,
        _endpoint: &str,
        _settings: &SettingsState,
    ) -> Result<(), VpnError> {
        Ok(())
    }
    async fn disable_kill_switch(&self) {}
}

struct RunnerState {
    last_stats: Option<VpnStats>,
    last_check: Option<Instant>,
    bypass_routes: Vec<String>,
    #[cfg(target_os = "linux")]
    original_resolv_conf: Option<String>,
    #[cfg(target_os = "linux")]
    systemd_dns_applied: bool,
    #[cfg(target_os = "windows")]
    original_firewall_policy: Option<String>,
    #[cfg(target_os = "windows")]
    original_dns_snapshot: Option<Vec<DnsSnapshot>>,
}

#[cfg(target_os = "windows")]
struct DnsSnapshot {
    interface_alias: String,
    address_family: String,
    server_addresses: Vec<String>,
}

#[async_trait::async_trait]
trait Obfuscator: Send + Sync {
    async fn start(&self, remote_endpoint: &str, key: Option<&str>) -> Result<String, VpnError>;
    async fn stop(&self) -> Result<(), VpnError>;
}

struct WsObfuscator {
    child: Arc<Mutex<Option<std::process::Child>>>,
}

impl WsObfuscator {
    fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl Obfuscator for WsObfuscator {
    async fn start(&self, remote_endpoint: &str, _key: Option<&str>) -> Result<String, VpnError> {
        info!(
            "Starting WSTunnel (WebSocket) obfuscation for {}",
            remote_endpoint
        );

        let local_port = 51820;
        let remote_host = remote_endpoint.split(':').next().unwrap_or(remote_endpoint);

        let child = Command::new("wstunnel")
            .args([
                "client",
                "-l",
                &format!("udp://127.0.0.1:{}", local_port),
                "-r",
                &format!("wss://{}:443", remote_host),
                "--udp",
                "--udp-timeout",
                "60",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                error!(
                    "Failed to spawn wstunnel: {}. Ensure wstunnel is in PATH.",
                    e
                );
                VpnError::DriverMissing
            })?;

        let mut lock = self.child.lock().await;
        *lock = Some(child);
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(format!("127.0.0.1:{}", local_port))
    }

    async fn stop(&self) -> Result<(), VpnError> {
        let mut lock = self.child.lock().await;
        if let Some(mut child) = lock.take() {
            info!("Stopping WSTunnel...");
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

struct SsObfuscator {
    child: Arc<Mutex<Option<std::process::Child>>>,
}

impl SsObfuscator {
    fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl Obfuscator for SsObfuscator {
    async fn start(&self, remote_endpoint: &str, key: Option<&str>) -> Result<String, VpnError> {
        info!(
            "Starting Shadowsocks (AEAD) obfuscation for {}",
            remote_endpoint
        );

        let local_port = 51821;
        let remote_host = remote_endpoint.split(':').next().unwrap_or(remote_endpoint);
        let remote_port = remote_endpoint.split(':').nth(1).unwrap_or("8388");
        let password = key.ok_or_else(|| {
            error!("Shadowsocks requires an obfuscation key but none was provided.");
            VpnError::ConfigMissing
        })?;

        let child = Command::new("ss-local")
            .args([
                "-s",
                remote_host,
                "-p",
                remote_port,
                "-l",
                &local_port.to_string(),
                "-k",
                password,
                "-m",
                "aes-256-gcm",
                "-U",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                error!(
                    "Failed to spawn ss-local: {}. Ensure shadowsocks-libev is installed.",
                    e
                );
                VpnError::DriverMissing
            })?;

        let mut lock = self.child.lock().await;
        *lock = Some(child);
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(format!("127.0.0.1:{}", local_port))
    }

    async fn stop(&self) -> Result<(), VpnError> {
        let mut lock = self.child.lock().await;
        if let Some(mut child) = lock.take() {
            info!("Stopping Shadowsocks...");
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

struct QuicObfuscator {
    child: Arc<Mutex<Option<std::process::Child>>>,
}

impl QuicObfuscator {
    fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl Obfuscator for QuicObfuscator {
    async fn start(&self, remote_endpoint: &str, _key: Option<&str>) -> Result<String, VpnError> {
        info!("Starting QUIC (HTTP/3) obfuscation for {}", remote_endpoint);

        let local_port = 51822;
        let remote_host = remote_endpoint.split(':').next().unwrap_or(remote_endpoint);

        let child = Command::new("quic-tun")
            .args([
                "client",
                "-l",
                &format!("127.0.0.1:{}", local_port),
                "-r",
                &format!("{}:443", remote_host),
                "--cert-verify=false",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                error!(
                    "Failed to spawn quic-tun: {}. Ensure quic-tun is in PATH.",
                    e
                );
                VpnError::DriverMissing
            })?;

        let mut lock = self.child.lock().await;
        *lock = Some(child);
        tokio::time::sleep(Duration::from_millis(600)).await;
        Ok(format!("127.0.0.1:{}", local_port))
    }

    async fn stop(&self) -> Result<(), VpnError> {
        let mut lock = self.child.lock().await;
        if let Some(mut child) = lock.take() {
            info!("Stopping QUIC tunnel...");
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

struct TcpObfuscator {
    child: Arc<Mutex<Option<std::process::Child>>>,
}

impl TcpObfuscator {
    fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl Obfuscator for TcpObfuscator {
    async fn start(&self, remote_endpoint: &str, _key: Option<&str>) -> Result<String, VpnError> {
        info!(
            "Starting raw UDP-over-TCP obfuscation for {}",
            remote_endpoint
        );

        let local_port = 51823;
        let remote_host = remote_endpoint.split(':').next().unwrap_or(remote_endpoint);

        let child = Command::new("wstunnel")
            .args([
                "client",
                "-l",
                &format!("udp://127.0.0.1:{}", local_port),
                "-r",
                &format!("tcp://{}:443", remote_host),
                "--udp",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                error!(
                    "Failed to spawn wstunnel (tcp): {}. Ensure wstunnel is in PATH.",
                    e
                );
                VpnError::DriverMissing
            })?;

        let mut lock = self.child.lock().await;
        *lock = Some(child);
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(format!("127.0.0.1:{}", local_port))
    }

    async fn stop(&self) -> Result<(), VpnError> {
        let mut lock = self.child.lock().await;
        if let Some(mut child) = lock.take() {
            info!("Stopping TCP tunnel...");
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

struct LwoObfuscator {
    stop_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl LwoObfuscator {
    fn new() -> Self {
        Self {
            stop_tx: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl Obfuscator for LwoObfuscator {
    async fn start(&self, remote_endpoint: &str, key: Option<&str>) -> Result<String, VpnError> {
        info!(
            "Starting LWO (Lightweight WireGuard Obfuscation) for {}",
            remote_endpoint
        );

        let local_port = 51824;
        let local_addr = format!("127.0.0.1:{}", local_port);
        let remote_addr = remote_endpoint.to_string();

        let (tx, mut rx) = tokio::sync::oneshot::channel();
        {
            let mut lock = self.stop_tx.lock().await;
            *lock = Some(tx);
        }

        let key_bytes = if let Some(k) = key {
            base64::engine::general_purpose::STANDARD
                .decode(k)
                .unwrap_or_else(|_| vec![0u8; 16])
        } else {
            vec![0u8; 16]
        };

        let local_addr_for_task = local_addr.clone();
        tokio::spawn(async move {
            let socket = match tokio::net::UdpSocket::bind(&local_addr_for_task).await {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    error!(
                        "LWO: Failed to bind local socket {}: {}",
                        local_addr_for_task, e
                    );
                    return;
                }
            };

            let remote_socket = match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    error!("LWO: Failed to bind remote socket: {}", e);
                    return;
                }
            };

            if let Err(e) = remote_socket.connect(&remote_addr).await {
                error!("LWO: Failed to connect to remote {}: {}", remote_addr, e);
                return;
            }

            let mut wg_addr: Option<std::net::SocketAddr> = None;
            let mut xor_key = [0u8; 16];
            let len = key_bytes.len().min(16);
            xor_key[..len].copy_from_slice(&key_bytes[..len]);

            info!("LWO: Proxy active. Header scrambling (16-byte session XOR) engaged.");

            loop {
                let mut b_out = [0u8; 2048];
                let mut b_in = [0u8; 2048];

                tokio::select! {
                    _ = &mut rx => {
                        info!("LWO: Stopping proxy task...");
                        break;
                    }
                    result = socket.recv_from(&mut b_out) => {
                        if let Ok((len, addr)) = result {
                            wg_addr = Some(addr);
                            for i in 0..16.min(len) {
                                b_out[i] ^= xor_key[i];
                            }
                            let _ = remote_socket.send(&b_out[..len]).await;
                        }
                    }
                    result = remote_socket.recv(&mut b_in) => {
                        if let Ok(len) = result {
                            if let Some(target) = wg_addr {
                                for i in 0..16.min(len) {
                                    b_in[i] ^= xor_key[i];
                                }
                                let _ = socket.send_to(&b_in[..len], target).await;
                            }
                        }
                    }
                }
            }
        });

        Ok(local_addr)
    }

    async fn stop(&self) -> Result<(), VpnError> {
        let mut lock = self.stop_tx.lock().await;
        if let Some(tx) = lock.take() {
            let _ = tx.send(());
        }
        Ok(())
    }
}

struct RealWgRunner {
    iface_entry: String,
    iface_exit: String,
    state: Mutex<RunnerState>,
    ws_obfuscator: Arc<WsObfuscator>,
    ss_obfuscator: Arc<SsObfuscator>,
    quic_obfuscator: Arc<QuicObfuscator>,
    tcp_obfuscator: Arc<TcpObfuscator>,
    lwo_obfuscator: Arc<LwoObfuscator>,
}

impl RealWgRunner {
    fn new() -> Self {
        let wg_present = Command::new("wg").arg("--version").output().is_ok();
        if !wg_present {
            warn!("'wg' tool not detected. VPN operations will likely fail.");
        }

        Self {
            iface_entry: "marinvpn0".to_string(),
            iface_exit: "marinvpn1".to_string(),
            state: Mutex::new(RunnerState {
                last_stats: None,
                last_check: None,
                bypass_routes: Vec::new(),
                #[cfg(target_os = "linux")]
                original_resolv_conf: None,
                #[cfg(target_os = "linux")]
                systemd_dns_applied: false,
                #[cfg(target_os = "windows")]
                original_firewall_policy: None,
                #[cfg(target_os = "windows")]
                original_dns_snapshot: None,
            }),
            ws_obfuscator: Arc::new(WsObfuscator::new()),
            ss_obfuscator: Arc::new(SsObfuscator::new()),
            quic_obfuscator: Arc::new(QuicObfuscator::new()),
            tcp_obfuscator: Arc::new(TcpObfuscator::new()),
            lwo_obfuscator: Arc::new(LwoObfuscator::new()),
        }
    }

    fn create_conf(
        &self,
        config: &WireGuardConfig,
        settings: &SettingsState,
        mtu_override: Option<u32>,
    ) -> String {
        let mtu = if let Some(m) = mtu_override {
            m
        } else if settings.mtu == 0 || settings.mtu == 1420 {
            1280
        } else {
            settings.mtu
        };

        let mut peer_section = format!(
            "[Peer]\nPublicKey = {}\nEndpoint = {}\nAllowedIPs = {}\nPersistentKeepalive = 25\n",
            config.public_key, config.endpoint, config.allowed_ips
        );

        if let Some(ref psk) = config.preshared_key {
            peer_section.push_str(&format!("PresharedKey = {}\n", psk));
        }

        format!(
            "[Interface]\nPrivateKey = {}\nAddress = {}\nMTU = {}\n{}\n\n{}\n",
            config.private_key,
            config.address,
            mtu,
            config
                .dns
                .as_ref()
                .map(|d| format!("DNS = {}", d))
                .unwrap_or_default(),
            peer_section
        )
    }

    async fn apply_dns(&self, dns: &Option<String>, settings: &SettingsState) {
        let dns_servers = if settings.custom_dns && !settings.custom_dns_server.is_empty() {
            settings.custom_dns_server.clone()
        } else {
            dns.clone()
                .unwrap_or_else(|| "1.1.1.1, 8.8.8.8".to_string())
        };

        #[cfg(target_os = "linux")]
        {
            let servers: Vec<&str> = dns_servers
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            let resolvectl_check = Command::new("resolvectl").arg("--version").output();
            let mut applied_with_systemd = false;
            if resolvectl_check.is_ok() {
                info!("Applying DNS via resolvectl (systemd-resolved)");
                let dns_status = Command::new("resolvectl")
                    .arg("dns")
                    .arg(&self.iface_entry)
                    .args(&servers)
                    .status();

                if dns_status.map(|s| s.success()).unwrap_or(false) {
                    applied_with_systemd = true;
                } else if let Err(err) = dns_status {
                    warn!("resolvectl dns command failed: {}", err);
                }

                if let Ok(status) = Command::new("resolvectl")
                    .arg("domain")
                    .arg(&self.iface_entry)
                    .arg("~.")
                    .status()
                {
                    if !status.success() {
                        warn!(
                            "resolvectl domain adjustment failed: status={:?}",
                            status.code()
                        );
                    }
                }
            }

            if applied_with_systemd {
                let mut state = self.state.lock().await;
                state.systemd_dns_applied = true;
                state.original_resolv_conf = None;
            } else if let Ok(content) = fs::read_to_string("/etc/resolv.conf") {
                {
                    let mut state = self.state.lock().await;
                    state.original_resolv_conf = Some(content);
                    state.systemd_dns_applied = false;
                }

                let mut new_conf = String::new();
                new_conf.push_str("# Generated by MarinVPN\n");
                for s in servers {
                    new_conf.push_str(&format!("nameserver {}\n", s));
                }
                let _ = fs::write("/etc/resolv.conf", new_conf);
            } else {
                let mut state = self.state.lock().await;
                state.systemd_dns_applied = false;
            }
        }

        #[cfg(target_os = "windows")]
        {
            {
                let mut state = self.state.lock().await;
                if state.original_dns_snapshot.is_none() {
                    state.original_dns_snapshot = Self::capture_dns_snapshot();
                }
            }

            let first_dns = dns_servers.split(',').next().unwrap_or("1.1.1.1").trim();
            info!(
                "Applying Windows DNS: {} to interface {}",
                first_dns, self.iface_entry
            );

            let name_arg = format!("name={}", self.iface_entry);
            let _ = Command::new("netsh")
                .args([
                    "interface",
                    "ipv4",
                    "set",
                    "dns",
                    &name_arg,
                    "static",
                    first_dns,
                ])
                .status();

            if let Some(second_dns) = dns_servers.split(',').nth(1) {
                let _ = Command::new("netsh")
                    .args([
                        "interface",
                        "ipv4",
                        "add",
                        "dns",
                        &name_arg,
                        second_dns.trim(),
                        "index=2",
                    ])
                    .status();
            }

            let block_leaks = format!(
                "$iface = '{}'; \
                Get-NetAdapter | Where-Object {{ $_.InterfaceAlias -ne $iface -and $_.InterfaceAlias -ne 'marinvpn1' }} | ForEach-Object {{ \
                    $alias = $_.InterfaceAlias; \
                    netsh interface ipv4 set dnsservers name=$alias source=static address=127.0.0.1 validate=no; \
                    $doh_ips = @('1.1.1.1', '1.0.0.1', '8.8.8.8', '8.8.4.4', '9.9.9.9', '149.112.112.112'); \
                    foreach ($ip in $doh_ips) {{ \
                        New-NetFirewallRule -DisplayName \"MarinVPN - Block DoH $alias $ip\" -Direction Outbound -InterfaceAlias $alias -RemoteAddress $ip -RemotePort 443 -Protocol TCP -Action Block -Profile Any -Force; \
                    }} \
                }}", self.iface_entry.replace("'", "''"));
            let _ = Command::new("powershell")
                .args(["-NoProfile", "-Command", &block_leaks])
                .status();
        }
    }

    async fn restore_dns(&self) {
        #[cfg(target_os = "linux")]
        {
            let mut state = self.state.lock().await;
            if state.systemd_dns_applied {
                state.systemd_dns_applied = false;
                drop(state);
                let _ = Command::new("resolvectl")
                    .arg("revert")
                    .arg(&self.iface_entry)
                    .status();
            } else if let Some(original) = state.original_resolv_conf.take() {
                drop(state);
                let _ = fs::write("/etc/resolv.conf", original);
            }
        }

        #[cfg(target_os = "windows")]
        {
            let snapshot = {
                let mut state = self.state.lock().await;
                state.original_dns_snapshot.take()
            };
            if let Some(snapshot) = snapshot {
                info!("Restoring Windows DNS from snapshot...");
                Self::restore_dns_snapshot(&snapshot);
            } else {
                info!("No DNS snapshot available; leaving Windows DNS unchanged.");
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn read_firewall_policy() -> Option<String> {
        let output = Command::new("netsh")
            .args(["advfirewall", "show", "allprofiles"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if let Some(idx) = line.find("Firewall Policy") {
                let value = line[idx + "Firewall Policy".len()..].trim();
                if !value.is_empty() {
                    return Some(value.replace(' ', "").to_lowercase());
                }
            }
        }
        None
    }

    async fn clear_bypass_routes(&self) {
        let routes = {
            let mut state = self.state.lock().await;
            std::mem::take(&mut state.bypass_routes)
        };

        for ip in routes {
            #[cfg(target_os = "linux")]
            {
                let _ = Command::new("ip").args(["route", "del", &ip]).status();
            }
            #[cfg(target_os = "windows")]
            {
                let _ = Command::new("route").args(["delete", &ip]).status();
            }
        }
    }

    async fn resolve_endpoint_ips(host: &str) -> (Vec<String>, Vec<String>) {
        let host = host.trim();
        if host.parse::<std::net::IpAddr>().is_ok() {
            if host.contains(':') {
                return (Vec::new(), vec![host.to_string()]);
            }
            return (vec![host.to_string()], Vec::new());
        }
        let mut v4 = Vec::new();
        let mut v6 = Vec::new();
        if let Ok(lookup) = tokio::net::lookup_host(format!("{}:0", host)).await {
            for addr in lookup {
                let ip = addr.ip();
                let ip_str = ip.to_string();
                if ip.is_ipv4() {
                    if !v4.contains(&ip_str) {
                        v4.push(ip_str);
                    }
                } else if !v6.contains(&ip_str) {
                    v6.push(ip_str);
                }
            }
        }
        (v4, v6)
    }

    #[cfg(target_os = "windows")]
    fn capture_dns_snapshot() -> Option<Vec<DnsSnapshot>> {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-DnsClientServerAddress -AddressFamily IPv4,IPv6 | \
                 Select-Object -Property InterfaceAlias,AddressFamily,ServerAddresses | ConvertTo-Json -Compress",
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        if text.trim().is_empty() {
            return None;
        }

        #[derive(serde::Deserialize)]
        struct DnsRow {
            #[serde(rename = "InterfaceAlias")]
            interface_alias: String,
            #[serde(rename = "AddressFamily")]
            address_family: Option<String>,
            #[serde(rename = "ServerAddresses")]
            server_addresses: Option<Vec<String>>,
        }

        let parsed: Result<Vec<DnsRow>, _> = serde_json::from_str(&text);
        let rows = match parsed {
            Ok(rows) => rows,
            Err(_) => {
                let single: DnsRow = serde_json::from_str(&text).ok()?;
                vec![single]
            }
        };

        let snapshot = rows
            .into_iter()
            .filter_map(|row| {
                row.server_addresses.map(|servers| DnsSnapshot {
                    interface_alias: row.interface_alias,
                    address_family: row.address_family.unwrap_or_else(|| "IPv4".to_string()),
                    server_addresses: servers,
                })
            })
            .collect::<Vec<_>>();

        if snapshot.is_empty() {
            None
        } else {
            Some(snapshot)
        }
    }

    #[cfg(target_os = "windows")]
    fn restore_dns_snapshot(snapshot: &[DnsSnapshot]) {
        for entry in snapshot {
            let family = match entry.address_family.as_str() {
                "IPv4" | "IPv6" => entry.address_family.as_str(),
                _ => continue,
            };
            let alias = entry.interface_alias.as_str();
            let servers = entry.server_addresses.as_slice();
            if servers.is_empty() {
                continue;
            }
            let servers_arg = servers
                .iter()
                .map(|s| s.replace("'", "''"))
                .collect::<Vec<_>>()
                .join(",");
            let script = format!(
                "Set-DnsClientServerAddress -InterfaceAlias '{}' -AddressFamily {} -ServerAddresses {}",
                alias.replace("'", "''"),
                family,
                servers_arg
            );
            let _ = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .status();
        }
    }
}

#[async_trait::async_trait]
impl WgRunner for RealWgRunner {
    const DEFAULT_WIREGUARD_PORT: u16 = 51820;

    fn parse_endpoint_host_port(endpoint: &str) -> (String, u16) {
        let trimmed = endpoint.trim();
        if trimmed.starts_with('[') {
            if let Some(end_bracket) = trimmed.find(']') {
                let host = trimmed[1..end_bracket].to_string();
                let port = trimmed[end_bracket + 1..]
                    .strip_prefix(':')
                    .and_then(|p| p.parse::<u16>().ok())
                    .unwrap_or(Self::DEFAULT_WIREGUARD_PORT);
                return (host, port);
            }
        }

        if let Some(colon_idx) = trimmed.rfind(':') {
            let host_part = &trimmed[..colon_idx];
            let port_part = &trimmed[colon_idx + 1..];
            if !port_part.is_empty() && port_part.chars().all(|c| c.is_ascii_digit()) {
                if !host_part.is_empty() {
                    if host_part.contains(':') {
                        if host_part.parse::<std::net::Ipv6Addr>().is_ok() {
                            let port = port_part
                                .parse::<u16>()
                                .unwrap_or(Self::DEFAULT_WIREGUARD_PORT);
                            return (host_part.to_string(), port);
                        }
                    } else if let Ok(port) = port_part.parse::<u16>() {
                        return (host_part.to_string(), port);
                    }
                }
            }
        }

        (trimmed.to_string(), Self::DEFAULT_WIREGUARD_PORT)
    }

    async fn up(
        &self,
        entry: &WireGuardConfig,
        exit: Option<&WireGuardConfig>,
        settings: &SettingsState,
    ) -> Result<(), VpnError> {
        let mut final_entry = entry.clone();
        let obfs_key = entry.obfuscation_key.as_deref();

        match settings.stealth_mode {
            StealthMode::Automatic => {
                info!("Stealth Mode: AUTOMATIC discovery initiated...");
                if let Ok(ep) = self.lwo_obfuscator.start(&entry.endpoint, obfs_key).await {
                    final_entry.endpoint = ep;
                    info!("Auto-Stealth: Selected LWO");
                } else if let Ok(ep) = self.quic_obfuscator.start(&entry.endpoint, obfs_key).await {
                    final_entry.endpoint = ep;
                    info!("Auto-Stealth: Selected QUIC");
                } else {
                    match self.ws_obfuscator.start(&entry.endpoint, obfs_key).await {
                        Ok(ep) => final_entry.endpoint = ep,
                        Err(_) => warn!("Auto-Stealth: All methods failed, using standard UDP"),
                    }
                }
            }
            StealthMode::WireGuardPort => {
                info!("Stealth Mode: WireGuard on Port 53 (DNS) simulation");
                let host = entry.endpoint.split(':').next().unwrap_or(&entry.endpoint);
                final_entry.endpoint = format!("{}:53", host);
            }
            StealthMode::None => {
                // Standard WireGuard
            }
            _ => {
                // Fallback for other methods
            }
        }

        let entry_conf = if let Some(exit_cfg) = exit {
            let exit_host = exit_cfg
                .endpoint
                .split(':')
                .next()
                .unwrap_or(&exit_cfg.endpoint);

            let exit_ip = match tokio::net::lookup_host(format!("{}:51820", exit_host)).await {
                Ok(mut addrs) => addrs
                    .next()
                    .map(|a| a.ip().to_string())
                    .unwrap_or_else(|| exit_host.to_string()),
                Err(_) => exit_host.to_string(),
            };

            format!(
                    "[Interface]\nPrivateKey = {}\nAddress = {}\nMTU = 1320\n\n[Peer]\nPublicKey = {}\nEndpoint = {}\nAllowedIPs = {}, {}/32\nPersistentKeepalive = 25\n",
                    final_entry.private_key,
                    final_entry.address,
                    final_entry.public_key,
                    final_entry.endpoint,
                    final_entry.address,
                    exit_ip
                )
        } else {
            self.create_conf(&final_entry, settings, None)
        };

        self.apply_single_up(&self.iface_entry, &entry_conf).await?;

        if let Some(exit_cfg) = exit {
            info!("Establishing nested exit tunnel with adjusted MTU...");
            let exit_conf = self.create_conf(exit_cfg, settings, Some(1200));
            self.apply_single_up(&self.iface_exit, &exit_conf).await?;
        }

        self.apply_dns(&exit.unwrap_or(entry).dns, settings).await;

        Ok(())
    }

    async fn down(&self) -> Result<(), VpnError> {
        self.apply_single_down(&self.iface_exit).await;
        self.apply_single_down(&self.iface_entry).await;

        self.ws_obfuscator.stop().await.ok();
        self.ss_obfuscator.stop().await.ok();
        self.quic_obfuscator.stop().await.ok();
        self.tcp_obfuscator.stop().await.ok();
        self.lwo_obfuscator.stop().await.ok();

        self.restore_dns().await;
        self.clear_bypass_routes().await;

        let mut state = self.state.lock().await;
        state.last_stats = None;
        state.last_check = None;

        Ok(())
    }

    async fn get_stats(&self) -> Result<VpnStats, VpnError> {
        let output = Command::new("wg")
            .arg("show")
            .arg(&self.iface_entry)
            .args(["transfer", "latest-handshake"])
            .output()
            .map_err(|_| VpnError::DriverMissing)?;

        if !output.status.success() {
            return Ok(VpnStats {
                download_speed: 0.0,
                upload_speed: 0.0,
                total_download: 0,
                total_upload: 0,
                latest_handshake: 0,
            });
        }

        let out_str = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = out_str.lines().collect();

        let mut total_download = 0;
        let mut total_upload = 0;
        let mut latest_handshake = 0;

        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && !line.contains("handshake") {
                total_download = parts[1].parse::<u64>().unwrap_or(0);
                total_upload = parts[2].parse::<u64>().unwrap_or(0);
            } else if line.contains("handshake") && parts.len() >= 2 {
                latest_handshake = parts[1].parse::<u64>().unwrap_or(0);
            }
        }

        let now = Instant::now();
        let mut state = self.state.lock().await;

        let (dl_speed, ul_speed) = if let (Some(last), Some(last_time)) =
            (&state.last_stats, &state.last_check)
        {
            let dur = now.duration_since(*last_time).as_secs_f64();
            if dur > 0.0 {
                let dl = (total_download.saturating_sub(last.total_download)) as f64 / dur / 1024.0;
                let ul = (total_upload.saturating_sub(last.total_upload)) as f64 / dur / 1024.0;
                (dl, ul)
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };

        let stats = VpnStats {
            download_speed: dl_speed,
            upload_speed: ul_speed,
            total_download,
            total_upload,
            latest_handshake,
        };

        state.last_stats = Some(stats.clone());
        state.last_check = Some(now);

        Ok(stats)
    }

    async fn apply_app_bypass(&self, app_path: &str) {
        #[cfg(target_os = "linux")]
        {
            info!("Applying cgroup bypass: {}", app_path);
            let cgroup_dir = "/sys/fs/cgroup/net_cls/marinvpn_bypass";
            if !std::path::Path::new("/sys/fs/cgroup/net_cls").exists() {
                warn!("net_cls cgroup not available; app bypass disabled.");
                return;
            }

            let commands = vec![
                format!("mkdir -p {}", cgroup_dir),
                format!("echo 0x1000 > {}/net_cls.classid", cgroup_dir),
                "ip rule add fwmark 0x1000 table main".to_string(),
            ];

            for cmd in commands {
                let _ = Command::new("sh").args(["-c", &cmd]).status();
            }

            let pid_output = Command::new("sh")
                .args([
                    "-c",
                    &format!(
                        "for p in /proc/[0-9]*/exe; do if [ \"$(readlink -f \"$p\")\" = \"{}\" ]; then echo ${p%/exe} | awk -F/ '{print $3}'; fi; done",
                        app_path.replace('"', "\\\"")
                    ),
                ])
                .output()
                .ok();
            let Some(output) = pid_output else {
                warn!("Failed to locate process for bypass: {}", app_path);
                return;
            };
            let pids = String::from_utf8_lossy(&output.stdout);
            if pids.trim().is_empty() {
                warn!("No running process found for bypass: {}", app_path);
                return;
            }

            for pid in pids.split_whitespace() {
                let cmd = format!("echo {} > {}/cgroup.procs", pid, cgroup_dir);
                let _ = Command::new("sh").args(["-c", &cmd]).status();
            }
        }

        #[cfg(target_os = "windows")]
        {
            info!("Injecting WFP bypass: {}", app_path);

            let app_name = std::path::Path::new(app_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("MarinVPNBypass");

            let script = format!(
                "New-NetFirewallRule -DisplayName 'MarinVPN Bypass - {}' \
                    -Direction Outbound -Program '{}' -Action Allow -Profile Any \
                    -InterfaceAlias '*' -EdgeTraversalPolicy Allow",
                app_name.replace("'", "''"),
                app_path.replace("'", "''")
            );

            let _ = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .status();
        }
    }

    async fn apply_bypass_route(&self, ip: &str) {
        {
            let mut state = self.state.lock().await;
            if !state.bypass_routes.iter().any(|r| r == ip) {
                state.bypass_routes.push(ip.to_string());
            }
        }

        #[cfg(target_os = "linux")]
        {
            let get_iface = "ip route | grep default | awk '{print $5}' | head -n1";
            let iface = Command::new("sh")
                .args(["-c", get_iface])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_else(|_| "eth0".to_string());
            let _ = Command::new("ip")
                .args(["route", "add", ip, "dev", &iface])
                .status();
        }

        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("route")
                .args([
                    "add",
                    ip,
                    "mask",
                    "255.255.255.255",
                    "0.0.0.0",
                    "metric",
                    "1",
                ])
                .status();
        }
    }

    async fn apply_single_up(&self, iface: &str, conf: &str) -> Result<(), VpnError> {
        #[cfg(target_os = "linux")]
        {
            let conf_path = format!("/tmp/marinvpn_{}.conf", iface);

            use std::os::unix::fs::OpenOptionsExt;
            let mut options = fs::OpenOptions::new();
            options.create(true).write(true).truncate(true).mode(0o600);

            use std::io::Write;
            let mut file = options
                .open(&conf_path)
                .map_err(|e| VpnError::InterfaceError(e.to_string()))?;
            file.write_all(conf.as_bytes())
                .map_err(|e| VpnError::InterfaceError(e.to_string()))?;

            let output = Command::new("wg-quick")
                .arg("up")
                .arg(&conf_path)
                .output()
                .map_err(|_| VpnError::DriverMissing)?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                return Err(VpnError::ConnectionFailed(err.to_string()));
            }
        }

        #[cfg(target_os = "windows")]
        {
            let proj_dirs = directories::ProjectDirs::from("com", "marinvpn", "MarinVPN")
                .ok_or_else(|| {
                    VpnError::InterfaceError("Failed to get project directory".to_string())
                })?;
            let config_dir = proj_dirs.cache_dir().join("tunnels");
            let _ = fs::create_dir_all(&config_dir);

            let conf_path = config_dir.join(format!("{}.conf", iface));
            fs::write(&conf_path, conf).map_err(|e| VpnError::InterfaceError(e.to_string()))?;

            let _ = Command::new("wireguard.exe")
                .arg("/installmanagerservice")
                .arg(&conf_path)
                .status()
                .map_err(|_| VpnError::DriverMissing)?;
        }
        Ok(())
    }

    async fn apply_single_down(&self, iface: &str) {
        #[cfg(target_os = "linux")]
        {
            let conf_path = format!("/tmp/marinvpn_{}.conf", iface);
            let _ = Command::new("wg-quick")
                .arg("down")
                .arg(&conf_path)
                .output();
            let _ = fs::remove_file(&conf_path);
        }

        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("wireguard.exe")
                .arg("/uninstallmanagerservice")
                .arg(iface)
                .output();

            if let Some(proj_dirs) = directories::ProjectDirs::from("com", "marinvpn", "MarinVPN") {
                let config_dir = proj_dirs.cache_dir().join("tunnels");
                let conf_path = config_dir.join(format!("{}.conf", iface));
                let _ = fs::remove_file(&conf_path);
            }
        }
    }

    async fn enable_kill_switch(
        &self,
        endpoint: &str,
        settings: &SettingsState,
    ) -> Result<(), VpnError> {
        let (host, port) = Self::parse_endpoint_host_port(endpoint);
        let (resolved_v4, resolved_v6) = if host == "0.0.0.0" {
            (Vec::new(), Vec::new())
        } else {
            Self::resolve_endpoint_ips(&host).await
        };
        if host != "0.0.0.0" && resolved_v4.is_empty() && resolved_v6.is_empty() {
            return Err(VpnError::FirewallError(
                "Failed to resolve endpoint for kill-switch".to_string(),
            ));
        }
        let v4_addrs: Vec<&str> = if host == "0.0.0.0" {
            vec!["0.0.0.0"]
        } else {
            resolved_v4.iter().map(|s| s.as_str()).collect()
        };
        let v6_addrs: Vec<&str> = resolved_v6.iter().map(|s| s.as_str()).collect();

        let mut allow_rules: Vec<(&'static str, u16)> = Vec::new();
        match settings.stealth_mode {
            StealthMode::Automatic => {
                allow_rules.push(("udp", port));
                allow_rules.push(("tcp", 443));
                allow_rules.push(("udp", 443));
            }
            StealthMode::WireGuardPort => {
                allow_rules.push(("udp", 53));
            }
            StealthMode::Lwo => {
                allow_rules.push(("udp", port));
            }
            StealthMode::Quic => {
                allow_rules.push(("udp", 443));
            }
            StealthMode::Tcp => {
                allow_rules.push(("tcp", 443));
            }
            StealthMode::Shadowsocks => {
                let ss_port = if endpoint.contains(':') { port } else { 8388 };
                allow_rules.push(("tcp", ss_port));
                allow_rules.push(("udp", ss_port));
            }
            StealthMode::None => {
                allow_rules.push(("udp", port));
            }
        }

        #[cfg(target_os = "linux")]
        {
            info!("Enabling Linux Kill-switch using nftables...");

            let run_nft = |args: &[&str]| Command::new("nft").args(args).status();

            let _ = run_nft(&["add", "table", "inet", "marinvpn_killswitch"]);
            let _ = run_nft(&[
                "add",
                "chain",
                "inet",
                "marinvpn_killswitch",
                "output",
                "{",
                "type",
                "filter",
                "hook",
                "output",
                "priority",
                "0;",
                "policy",
                "drop;",
                "}",
            ]);
            let _ = run_nft(&[
                "add",
                "chain",
                "inet",
                "marinvpn_killswitch",
                "input",
                "{",
                "type",
                "filter",
                "hook",
                "input",
                "priority",
                "0;",
                "policy",
                "accept;",
                "}",
            ]);
            let _ = run_nft(&[
                "add",
                "rule",
                "inet",
                "marinvpn_killswitch",
                "output",
                "oifname",
                "lo",
                "accept",
            ]);
            if v4_addrs.len() == 1 && v4_addrs[0] == "0.0.0.0" {
                // no specific endpoint
            } else {
                for addr in &v4_addrs {
                    for (proto, port) in &allow_rules {
                        let port_str = port.to_string();
                        let _ = run_nft(&[
                            "add",
                            "rule",
                            "inet",
                            "marinvpn_killswitch",
                            "output",
                            "ip",
                            "daddr",
                            addr,
                            proto,
                            "dport",
                            &port_str,
                            "accept",
                        ]);
                    }
                }
            }
            if !v6_addrs.is_empty() {
                for addr in &v6_addrs {
                    for (proto, port) in &allow_rules {
                        let port_str = port.to_string();
                        let _ = run_nft(&[
                            "add",
                            "rule",
                            "inet",
                            "marinvpn_killswitch",
                            "output",
                            "ip6",
                            "daddr",
                            addr,
                            proto,
                            "dport",
                            &port_str,
                            "accept",
                        ]);
                    }
                }
            }

            let _ = run_nft(&[
                "add",
                "rule",
                "inet",
                "marinvpn_killswitch",
                "output",
                "udp",
                "sport",
                "68",
                "dport",
                "67",
                "accept",
            ]);
            if settings.ipv6_support {
                let _ = run_nft(&[
                    "add",
                    "rule",
                    "inet",
                    "marinvpn_killswitch",
                    "output",
                    "udp",
                    "sport",
                    "546",
                    "dport",
                    "547",
                    "accept",
                ]);
                let _ = run_nft(&[
                    "add",
                    "rule",
                    "inet",
                    "marinvpn_killswitch",
                    "output",
                    "icmpv6",
                    "type",
                    "{",
                    "router-solicitation,",
                    "router-advertisement,",
                    "neighbor-solicitation,",
                    "neighbor-advertisement",
                    "}",
                    "accept",
                ]);
            }

            let _ = run_nft(&[
                "add",
                "rule",
                "inet",
                "marinvpn_killswitch",
                "output",
                "oifname",
                &self.iface_entry,
                "accept",
            ]);

            if settings.split_tunneling {
                let _ = run_nft(&[
                    "add",
                    "rule",
                    "inet",
                    "marinvpn_killswitch",
                    "output",
                    "mark",
                    "0x1000",
                    "accept",
                ]);
            }

            if settings.local_sharing {
                let _ = run_nft(&[
                    "add",
                    "rule",
                    "inet",
                    "marinvpn_killswitch",
                    "output",
                    "ip",
                    "daddr",
                    "{",
                    "192.168.0.0/16,",
                    "10.0.0.0/8,",
                    "172.16.0.0/12",
                    "}",
                    "accept",
                ]);
            }

            let _ = run_nft(&[
                "add",
                "rule",
                "inet",
                "marinvpn_killswitch",
                "output",
                "ip6",
                "daddr",
                "::/0",
                "drop",
            ]);
        }

        #[cfg(target_os = "windows")]
        {
            info!("Enabling Windows Global Kill-switch (Fail-Closed Policy)...");

            {
                let mut state = self.state.lock().await;
                if state.original_firewall_policy.is_none() {
                    state.original_firewall_policy = Self::read_firewall_policy();
                }
                if state.original_dns_snapshot.is_none() {
                    state.original_dns_snapshot = Self::capture_dns_snapshot();
                }
            }

            let _ = Command::new("netsh")
                .args([
                    "advfirewall",
                    "set",
                    "allprofiles",
                    "firewallpolicy",
                    "blockoutbound,allowinbound",
                ])
                .status();

            let allow_loopback =
                "New-NetFirewallRule -DisplayName 'MarinVPN - Allow Loopback' -Direction Outbound \
                    -RemoteAddress 127.0.0.1,::1 -Action Allow -Profile Any -Force";
            let _ = Command::new("powershell")
                .args(["-NoProfile", "-Command", allow_loopback])
                .status();

            if !(v4_addrs.len() == 1 && v4_addrs[0] == "0.0.0.0") {
                for addr in &v4_addrs {
                    for (proto, port) in &allow_rules {
                        let allow_endpoint = format!(
                            "New-NetFirewallRule -DisplayName 'MarinVPN - Allow Endpoint {proto}:{port}' -Direction Outbound \
                            -RemoteAddress '{}' -RemotePort {port} -Action Allow -Protocol {proto} -Profile Any -Force",
                            addr.replace("'", "''")
                        );
                        let _ = Command::new("powershell")
                            .args(["-NoProfile", "-Command", &allow_endpoint])
                            .status();
                    }
                }
            }
            if !v6_addrs.is_empty() {
                for addr in &v6_addrs {
                    for (proto, port) in &allow_rules {
                        let allow_endpoint = format!(
                            "New-NetFirewallRule -DisplayName 'MarinVPN - Allow Endpoint {proto}:{port}' -Direction Outbound \
                            -RemoteAddress '{}' -RemotePort {port} -Action Allow -Protocol {proto} -Profile Any -Force",
                            addr.replace("'", "''")
                        );
                        let _ = Command::new("powershell")
                            .args(["-NoProfile", "-Command", &allow_endpoint])
                            .status();
                    }
                }
            }

            if settings.ipv6_support {
                let allow_ra = "New-NetFirewallRule -DisplayName 'MarinVPN - Allow ICMPv6 ND' -Direction Outbound \
                        -Protocol ICMPv6 -IcmpType 133,134,135,136 -Action Allow -Profile Any -Force";
                let _ = Command::new("powershell")
                    .args(["-NoProfile", "-Command", allow_ra])
                    .status();
                let allow_dhcpv6 = "New-NetFirewallRule -DisplayName 'MarinVPN - Allow DHCPv6' -Direction Outbound \
                        -Protocol UDP -LocalPort 546 -RemotePort 547 -Action Allow -Profile Any -Force";
                let _ = Command::new("powershell")
                    .args(["-NoProfile", "-Command", allow_dhcpv6])
                    .status();
            }

            let allow_vpn = "Get-NetAdapter | Where-Object { $_.InterfaceDescription -like '*Wintun*' -or $_.InterfaceAlias -like 'marinvpn*' } | ForEach-Object { \
                    $alias = $_.InterfaceAlias; \
                    New-NetFirewallRule -DisplayName \"MarinVPN - Allow Tunnel $alias\" -Direction Outbound -InterfaceAlias $alias -Action Allow -Profile Any -Force \
                }";
            let _ = Command::new("powershell")
                .args(["-NoProfile", "-Command", allow_vpn])
                .status();

            // Split Tunneling
            if settings.split_tunneling {
                for ip in &settings.excluded_ips {
                    let allow_ip = format!("New-NetFirewallRule -DisplayName 'MarinVPN - Bypass IP {}' -Direction Outbound -RemoteAddress {} -Action Allow -Profile Any -Force", ip, ip);
                    let _ = Command::new("powershell")
                        .args(["-NoProfile", "-Command", &allow_ip])
                        .status();
                    self.apply_bypass_route(ip).await;
                }
                for app in &settings.excluded_apps {
                    self.apply_app_bypass(&app.path).await;
                }
            }

            let block_v6 = "Get-NetAdapter | Where-Object { $_.InterfaceDescription -notlike '*Wintun*' -and $_.InterfaceAlias -notlike 'marinvpn*' } | ForEach-Object { \
                    $alias = $_.InterfaceAlias; \
                    New-NetFirewallRule -DisplayName \"MarinVPN - Block IPv6 $alias\" -Direction Outbound -InterfaceAlias $alias -RemoteAddress ::/0 -Action Block -Profile Any -Force \
                }";
            let _ = Command::new("powershell")
                .args(["-NoProfile", "-Command", block_v6])
                .status();

            if !settings.local_sharing {
                let block_lan = "New-NetFirewallRule -DisplayName 'MarinVPN - Block LAN' -Direction Outbound \
                        -RemoteAddress 192.168.0.0/16,10.0.0.0/8,172.16.0.0/12 -Action Block -Profile Any -Force";
                let _ = Command::new("powershell")
                    .args(["-NoProfile", "-Command", block_lan])
                    .status();
            }

            // DNS Leak Protection
            let block_dns = "Get-NetAdapter | Where-Object { $_.InterfaceDescription -notlike '*Wintun*' -and $_.InterfaceAlias -notlike 'marinvpn*' } | ForEach-Object { \
                    $alias = $_.InterfaceAlias; \
                    netsh interface ipv4 set dnsservers name=$alias source=static address=127.0.0.1 validate=no; \
                    netsh interface ipv6 set dnsservers name=$alias source=static address=::1 validate=no; \
                    New-NetFirewallRule -DisplayName \"MarinVPN - Leak Protect DNS UDP $alias\" -Direction Outbound -InterfaceAlias $alias -RemotePort 53 -Protocol UDP -Action Block -Profile Any -Force; \
                    New-NetFirewallRule -DisplayName \"MarinVPN - Leak Protect DNS TCP $alias\" -Direction Outbound -InterfaceAlias $alias -RemotePort 53 -Protocol TCP -Action Block -Profile Any -Force; \
                }";
            let _ = Command::new("powershell")
                .args(["-NoProfile", "-Command", block_dns])
                .status();
        }
        Ok(())
    }

    async fn disable_kill_switch(&self) {
        #[cfg(target_os = "linux")]
        {
            info!("Disabling Linux Kill-switch (nftables)...");
            let _ = Command::new("nft")
                .args(["delete", "table", "inet", "marinvpn_killswitch"])
                .status();
        }

        #[cfg(target_os = "windows")]
        {
            let policy = {
                let mut state = self.state.lock().await;
                state.original_firewall_policy.take()
            };
            if let Some(policy) = policy {
                let _ = Command::new("netsh")
                    .args([
                        "advfirewall",
                        "set",
                        "allprofiles",
                        "firewallpolicy",
                        &policy,
                    ])
                    .status();
            }
            let _ = Command::new("netsh")
                .args([
                    "advfirewall",
                    "firewall",
                    "delete",
                    "rule",
                    "name=BlockIPv6",
                ])
                .status();
            let _ = Command::new("netsh")
                .args(["advfirewall", "firewall", "delete", "rule", "name=BlockLAN"])
                .status();

            let _ = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "Remove-NetFirewallRule -DisplayName 'MarinVPN - *'",
                ])
                .status();

            let _ = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "Remove-NetFirewallRule -DisplayName 'MarinVPN Bypass - *'",
                ])
                .status();

            self.restore_dns().await;
        }

        self.clear_bypass_routes().await;
    }
}
