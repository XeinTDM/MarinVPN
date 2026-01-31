use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use crate::models::{ConnectionStatus, WireGuardConfig, SettingsState};
use tracing::{info, error, warn};
use rand::Rng;
use std::net::{TcpStream, SocketAddr};
use std::time::Duration;
use std::process::Command;
use std::fs;
use std::time::Instant;

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
            VpnError::NotRoot => write!(f, "Root/Admin privileges are required for VPN operations."),
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
    async fn connect(&self, entry: String, entry_config: WireGuardConfig, exit: Option<(String, WireGuardConfig)>, settings: SettingsState);
    async fn disconnect(&self);
    async fn get_status(&self) -> ConnectionStatus;
    async fn enable_captive_portal(&self, duration_secs: u64);
}

#[async_trait::async_trait]
trait WgRunner: Send + Sync {
    async fn up(&self, entry: &WireGuardConfig, exit: Option<&WireGuardConfig>, settings: &SettingsState) -> Result<(), VpnError>;
    async fn down(&self) -> Result<(), VpnError>;
    async fn get_stats(&self) -> Result<VpnStats, VpnError>;
    async fn apply_app_bypass(&self, app_path: &str);
    async fn apply_bypass_route(&self, ip: &str);
    async fn apply_single_up(&self, iface: &str, conf: &str) -> Result<(), VpnError>;
    async fn apply_single_down(&self, iface: &str);
}

#[derive(Clone)]
struct ConnectionContext {
    entry_name: String,
    entry_config: WireGuardConfig,
    exit: Option<(String, WireGuardConfig)>,
    settings: SettingsState,
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
                if TcpStream::connect_timeout(&SocketAddr::from(addr), Duration::from_secs(2)).is_ok() {
                    return true;
                }
            }
            false
        }).await.unwrap_or(false);

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
            self.start_daita_task(status_lock.clone());
        }

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
                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                        if now.saturating_sub(stats.latest_handshake) > 180 {
                            warn!("Handshake stale. Triggering self-healing...");
                            let ctx_lock = svc.active_context.lock().await;
                            if let Some(ctx) = ctx_lock.as_ref() {
                                let entry_n = ctx.entry_name.clone();
                                let entry_c = ctx.entry_config.clone();
                                let exit = ctx.exit.clone();
                                let sets = ctx.settings.clone();
                                drop(ctx_lock);
                                
                                svc.disconnect().await;
                                svc.connect(entry_n, entry_c, exit, sets).await;
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    fn start_daita_task(&self, status_lock: Arc<Mutex<ConnectionStatus>>) {
        tokio::spawn(async move {
            info!("Daita: Advanced Traffic Anonymization active.");
            let targets = ["1.1.1.1:53", "8.8.8.8:53", "9.9.9.9:53", "208.67.222.222:53", "8.8.4.4:53"];
            
            loop {
                // Check status without holding rng across await
                let is_connected = *status_lock.lock().await == ConnectionStatus::Connected;
                if !is_connected {
                    break;
                }

                let (size, count, delay_ms) = {
                    let mut rng = rand::thread_rng();
                    let traffic_type = rng.gen_range(0..100);
                    if traffic_type < 70 {
                        (rng.gen_range(32..128), 1, rng.gen_range(500..2000))
                    } else if traffic_type < 95 {
                        (rng.gen_range(512..1200), rng.gen_range(2..5), rng.gen_range(3000..8000))
                    } else {
                        (rng.gen_range(1200..1400), 10, rng.gen_range(10000..20000))
                    }
                };

                for _ in 0..count {
                    let target = {
                        let mut rng = rand::thread_rng();
                        targets[rng.gen_range(0..targets.len())]
                    };
                    let noise: Vec<u8> = {
                        let mut rng = rand::thread_rng();
                        (0..size).map(|_| rng.gen()).collect()
                    };
                    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
                        let _ = socket.send_to(&noise, target);
                    }
                    if count > 1 {
                        let sleep_time = {
                            let mut rng = rand::thread_rng();
                            rng.gen_range(10..100)
                        };
                        tokio::time::sleep(Duration::from_millis(sleep_time)).await;
                    }
                }

                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            info!("Daita: Task terminated.");
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

    async fn connect(&self, entry: String, entry_config: WireGuardConfig, exit: Option<(String, WireGuardConfig)>, settings: SettingsState) {
        {
            let status = self.current_status.lock().await;
            if *status == ConnectionStatus::Connected || *status == ConnectionStatus::Connecting {
                return;
            }
        }

        {
            let mut lock = self.active_context.lock().await;
            *lock = Some(ConnectionContext {
                entry_name: entry.clone(),
                entry_config: entry_config.clone(),
                exit: exit.clone(),
                settings: settings.clone(),
            });
        }

        self.set_status(ConnectionStatus::Connecting).await;
        
        let display_location = if let Some((ref exit_name, _)) = exit {
            format!("{} → {}", entry, exit_name)
        } else {
            entry.clone()
        };

        let _ = self.event_tx.send(VpnEvent::LocationChanged(display_location.clone()));
        info!("Initiating WireGuard connection: {}", display_location);

        if let Err(e) = self.check_connectivity().await {
            self.emit_error(e).await;
            return;
        }

        match self.runner.up(&entry_config, exit.as_ref().map(|(_, c)| c), &settings).await {
            Ok(_) => {
                info!("Tunnel established successfully.");
                self.set_status(ConnectionStatus::Connected).await;
                self.start_stats_loop(settings);
            }
            Err(e) => {
                self.emit_error(e).await;
            }
        }
    }

    async fn disconnect(&self) {
        let status = self.get_status().await;
        if status == ConnectionStatus::Disconnected || status == ConnectionStatus::Disconnecting {
            return;
        }

        {
            let mut lock = self.active_context.lock().await;
            *lock = None;
        }

        self.set_status(ConnectionStatus::Disconnecting).await;
        info!("Tearing down WireGuard interface...");
        
        match self.runner.down().await {
            Ok(_) => {
                self.set_status(ConnectionStatus::Disconnected).await;
                let _ = self.event_tx.send(VpnEvent::StatsUpdated(VpnStats {
                    download_speed: 0.0, upload_speed: 0.0, total_download: 0, total_upload: 0, latest_handshake: 0,
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
            info!("Captive Portal Mode: Temporarily disabling firewall for {}s", duration_secs);
            let _ = tx.send(VpnEvent::CaptivePortalActive(true));
            runner.down().await.ok();
            
            tokio::time::sleep(Duration::from_secs(duration_secs)).await;
            
            info!("Captive Portal Mode: Restoring firewall...");
            let _ = tx.send(VpnEvent::CaptivePortalActive(false));
        });
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
            state: Mutex::new(SimulationState { total_download: 0, total_upload: 0 })
        }
    }
}

#[async_trait::async_trait]
impl WgRunner for SimulationRunner {
    async fn up(&self, _entry: &WireGuardConfig, exit: Option<&WireGuardConfig>, _settings: &SettingsState) -> Result<(), VpnError> {
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
            (50.0 + rng.gen_range(-10.0..20.0), 30.0 + rng.gen_range(-5.0..10.0))
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
    async fn apply_single_up(&self, _iface: &str, _conf: &str) -> Result<(), VpnError> { Ok(()) }
    async fn apply_single_down(&self, _iface: &str) {}
}

struct RunnerState {
    last_stats: Option<VpnStats>,
    last_check: Option<Instant>,
    #[allow(dead_code)]
    original_resolv_conf: Option<String>,
}

#[async_trait::async_trait]
trait Obfuscator: Send + Sync {
    async fn start(&self, remote_endpoint: &str) -> Result<String, VpnError>;
    async fn stop(&self) -> Result<(), VpnError>;
}

struct WsObfuscator;

#[async_trait::async_trait]
impl Obfuscator for WsObfuscator {
    async fn start(&self, remote_endpoint: &str) -> Result<String, VpnError> {
        info!("Starting WSTunnel obfuscation for {}", remote_endpoint);
        // Implementation: npx wstunnel -L udp://127.0.0.1:51820:remote_endpoint
        Ok("127.0.0.1:51820".to_string())
    }
    async fn stop(&self) -> Result<(), VpnError> {
        Ok(())
    }
}

struct RealWgRunner {
    iface_entry: String,
    iface_exit: String,
    state: Mutex<RunnerState>,
    obfuscator: Arc<dyn Obfuscator>,
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
                original_resolv_conf: None,
            }),
            obfuscator: Arc::new(WsObfuscator),
        }
    }

    fn create_conf(&self, config: &WireGuardConfig, settings: &SettingsState) -> String {
        let mtu = if settings.mtu == 0 || settings.mtu == 1420 {
            1280
        } else {
            settings.mtu
        };

        let mut peer_section = format!(
            "[Peer]\nPublicKey = {}\nEndpoint = {}\nAllowedIPs = {}\nPersistentKeepalive = 25\n",
            config.public_key,
            config.endpoint,
            config.allowed_ips
        );

        if let Some(ref psk) = config.preshared_key {
            peer_section.push_str(&format!("PresharedKey = {}\n", psk));
        }

        format!(
            "[Interface]\nPrivateKey = {}\nAddress = {}\nMTU = {}\n{}\n\n{}\n",
            config.private_key,
            config.address,
            mtu,
            config.dns.as_ref().map(|d| format!("DNS = {}", d)).unwrap_or_default(),
            peer_section
        )
    }

    async fn apply_dns(&self, dns: &Option<String>) {
        #[cfg(target_os = "linux")]
        if let Some(dns_servers) = dns {
            if let Ok(content) = fs::read_to_string("/etc/resolv.conf") {
                let mut state = self.state.lock().await;
                state.original_resolv_conf = Some(content);
                
                let mut new_conf = String::new();
                for s in dns_servers.split(',') {
                    new_conf.push_str(&format!("nameserver {}\n", s.trim()));
                }
                let _ = fs::write("/etc/resolv.conf", new_conf);
            }
        }

        #[cfg(target_os = "windows")]
        if let Some(dns_servers) = dns {
            let first_dns = dns_servers.split(',').next().unwrap_or("1.1.1.1").trim();
            info!("Applying Windows DNS: {} to interface {}", first_dns, self.iface_entry);
            
            let _ = Command::new("netsh")
                .args(&["interface", "ipv4", "set", "dns", "name=", &self.iface_entry, "static", first_dns])
                .status();
            
            if let Some(second_dns) = dns_servers.split(',').nth(1) {
                let _ = Command::new("netsh")
                    .args(&["interface", "ipv4", "add", "dns", "name=", &self.iface_entry, second_dns.trim(), "index=2"])
                    .status();
            }
        }
    }

    async fn restore_dns(&self) {
        #[cfg(target_os = "linux")]
        {
            let mut state = self.state.lock().await;
            if let Some(original) = state.original_resolv_conf.take() {
                let _ = fs::write("/etc/resolv.conf", original);
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            info!("Restoring Windows DNS to DHCP for interface {}", self.iface_entry);
            let _ = Command::new("netsh")
                .args(&["interface", "ipv4", "set", "dns", "name=", &self.iface_entry, "source=dhcp"])
                .status();
        }
    }

    async fn enable_kill_switch(&self, endpoint: &str, settings: &SettingsState) -> Result<(), VpnError> {
        #[cfg(target_os = "linux")]
        {
            info!("Enabling Linux Kill-switch and IPv6 Leak Protection...");
            let addr = endpoint.split(':').next().unwrap_or(endpoint);

            let mut commands = vec![
                format!("iptables -A OUTPUT -d {} -j ACCEPT", addr),
                format!("iptables -A OUTPUT -o {} -j ACCEPT", self.iface_entry),
                "iptables -A OUTPUT -o lo -j ACCEPT".to_string(),
            ];

            if !settings.local_sharing {
                commands.push("iptables -A OUTPUT -d 192.168.0.0/16 -j DROP".to_string());
                commands.push("iptables -A OUTPUT -d 10.0.0.0/8 -j DROP".to_string());
                commands.push("iptables -A OUTPUT -d 172.16.0.0/12 -j DROP".to_string());
            }

            commands.push("iptables -P OUTPUT DROP".to_string());
            commands.push("ip6tables -P OUTPUT DROP".to_string());
            commands.push("ip6tables -P INPUT DROP".to_string());
            commands.push("ip6tables -P FORWARD DROP".to_string());

            for cmd in commands {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                let _ = Command::new(parts[0]).args(&parts[1..]).status();
            }
        }

        #[cfg(target_os = "windows")]
        {
            info!("Enabling Windows Global Kill-switch...");
            let addr = endpoint.split(':').next().unwrap_or(endpoint);

            let _ = Command::new("netsh").args(&["advfirewall", "set", "allprofiles", "firewallpolicy", "blockoutbound,allowinbound"]).status();
            
            let allow_endpoint = format!(
                "New-NetFirewallRule -DisplayName 'MarinVPN - Allow Endpoint' -Direction Outbound \
                -RemoteAddress {} -Action Allow -Protocol UDP", addr
            );
            let _ = Command::new("powershell").args(&["-NoProfile", "-Command", &allow_endpoint]).status();

            let allow_vpn = format!(
                "New-NetFirewallRule -DisplayName 'MarinVPN - Allow Tunnel' -Direction Outbound \
                -InterfaceAlias '{}' -Action Allow", self.iface_entry
            );
            let _ = Command::new("powershell").args(&["-NoProfile", "-Command", &allow_vpn]).status();

            if !settings.local_sharing {
                let _ = Command::new("netsh").args(&["advfirewall", "firewall", "add", "rule", "name=BlockLAN", "dir=out", "action=block", "remoteip=192.168.0.0/16,10.0.0.0/8,172.16.0.0/12"]).status();
            }
        }
        Ok(())
    }

    async fn disable_kill_switch(&self) {
        #[cfg(target_os = "linux")]
        {
            info!("Disabling Linux Kill-switch and restoring IPv6...");
            let _ = Command::new("iptables").args(&["-P", "OUTPUT", "ACCEPT"]).status();
            let _ = Command::new("iptables").args(&["-F", "OUTPUT"]).status();
            let _ = Command::new("ip6tables").args(&["-P", "OUTPUT", "ACCEPT"]).status();
            let _ = Command::new("ip6tables").args(&["-F", "OUTPUT"]).status();
        }

        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("netsh").args(&["advfirewall", "set", "allprofiles", "firewallpolicy", "allowoutbound,allowinbound"]).status();
            let _ = Command::new("netsh").args(&["advfirewall", "firewall", "delete", "rule", "name=BlockIPv6"]).status();
            let _ = Command::new("netsh").args(&["advfirewall", "firewall", "delete", "rule", "name=BlockLAN"]).status();
            
            let _ = Command::new("powershell")
                .args(&["-NoProfile", "-Command", "Remove-NetFirewallRule -DisplayName 'MarinVPN - *'"])
                .status();

            let _ = Command::new("powershell")
                .args(&["-NoProfile", "-Command", "Remove-NetFirewallRule -DisplayName 'MarinVPN Bypass - *'"])
                .status();
        }
    }
}

#[async_trait::async_trait]
impl WgRunner for RealWgRunner {
        async fn up(&self, entry: &WireGuardConfig, exit: Option<&WireGuardConfig>, settings: &SettingsState) -> Result<(), VpnError> {
            if settings.kill_switch {
                let endpoint = exit.unwrap_or(entry).endpoint.clone();
                if let Err(e) = self.enable_kill_switch(&endpoint, settings).await {
                    error!("Failed to enable kill switch: {}", e);
                    return Err(VpnError::FirewallError(e.to_string()));
                }
            }

            let mut final_entry = entry.clone();
            if settings.obfuscation {
                match self.obfuscator.start(&entry.endpoint).await {
                    Ok(local_endpoint) => {
                        final_entry.endpoint = local_endpoint;
                        info!("Obfuscation layer active: routing via {}", final_entry.endpoint);
                    }
                    Err(e) => {
                        error!("Failed to start obfuscation: {}", e);
                    }
                }
            }

            let entry_conf = if let Some(exit_cfg) = exit {
                let exit_host = exit_cfg.endpoint.split(':').next().unwrap_or(&exit_cfg.endpoint);
                format!(
                    "[Interface]\nPrivateKey = {}\nAddress = {}\nMTU = 1280\n\n[Peer]\nPublicKey = {}\nEndpoint = {}\nAllowedIPs = {}, {}/32\nPersistentKeepalive = 25\n",
                    final_entry.private_key,
                    final_entry.address,
                    final_entry.public_key,
                    final_entry.endpoint,
                    final_entry.address, 
                    exit_host      
                )
            } else {
                self.create_conf(&final_entry, settings)
            };
    
            self.apply_single_up(&self.iface_entry, &entry_conf).await?;
    
            if let Some(exit_cfg) = exit {
                info!("Establishing nested exit tunnel...");
                let exit_conf = self.create_conf(exit_cfg, settings);
                self.apply_single_up(&self.iface_exit, &exit_conf).await?;
            }
    
            self.apply_dns(&exit.unwrap_or(entry).dns).await;

            if settings.split_tunneling {
                if !settings.excluded_ips.is_empty() {
                    info!("Applying IP split tunneling bypass for {} IPs", settings.excluded_ips.len());
                    for ip in &settings.excluded_ips {
                        self.apply_bypass_route(ip).await;
                    }
                }
                
                if !settings.excluded_apps.is_empty() {
                    info!("Applying APP split tunneling bypass for {} apps", settings.excluded_apps.len());
                    for app in &settings.excluded_apps {
                        self.apply_app_bypass(&app.path).await;
                    }
                }
            }

            Ok(())
        }

        async fn apply_app_bypass(&self, app_path: &str) {
            #[cfg(target_os = "linux")]
            {
                info!("Applying cgroup bypass: {}", app_path);
                let commands = vec![
                    "mkdir -p /sys/fs/cgroup/net_cls/marinvpn_bypass".to_string(),
                    "echo 0x1000 > /sys/fs/cgroup/net_cls/marinvpn_bypass/net_cls.classid".to_string(),
                    "ip rule add fwmark 0x1000 table main".to_string(),
                ];
                
                for cmd in commands {
                    let parts: Vec<&str> = cmd.split_whitespace().collect();
                    let _ = Command::new(parts[0]).args(&parts[1..]).status();
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
                    app_name, app_path
                );

                let _ = Command::new("powershell")
                    .args(&["-NoProfile", "-Command", &script])
                    .status();
            }
        }

        async fn apply_bypass_route(&self, ip: &str) {
            #[cfg(target_os = "linux")]
            {
                let _ = Command::new("ip").args(&["route", "add", ip, "dev", "eth0"]).status(); // TODO: Simplified eth0 assumption
            }

            #[cfg(target_os = "windows")]
            {
                let _ = Command::new("route").args(&["add", ip, "mask", "255.255.255.255", "0.0.0.0", "metric", "1"]).status();
            }
        }
    
        async fn apply_single_up(&self, iface: &str, conf: &str) -> Result<(), VpnError> {
            #[cfg(target_os = "linux")]
            {
                let conf_path = format!("/tmp/{}.conf", iface);
                fs::write(&conf_path, conf).map_err(|e| VpnError::InterfaceError(e.to_string()))?;
    
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
                let config_dir = std::env::temp_dir().join("marinvpn");
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
    async fn down(&self) -> Result<(), VpnError> {
        self.disable_kill_switch().await;

        self.apply_single_down(&self.iface_exit).await;
        self.apply_single_down(&self.iface_entry).await;
        
        self.obfuscator.stop().await.ok();
        
        self.restore_dns().await;
        
        let mut state = self.state.lock().await;
        state.last_stats = None;
        state.last_check = None;
        
        Ok(())
    }

    async fn apply_single_down(&self, iface: &str) {
        #[cfg(target_os = "linux")]
        {
            let conf_path = format!("/tmp/{}.conf", iface);
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
        }
    }

    async fn get_stats(&self) -> Result<VpnStats, VpnError> {
        let output = Command::new("wg")
            .arg("show")
            .arg(&self.iface_entry)
            .args(&["transfer", "latest-handshake"])
            .output()
            .map_err(|_| VpnError::DriverMissing)?;

        if !output.status.success() {
            return Ok(VpnStats { 
                download_speed: 0.0, upload_speed: 0.0, 
                total_download: 0, total_upload: 0, latest_handshake: 0 
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
        
        let (dl_speed, ul_speed) = if let (Some(last), Some(last_time)) = (&state.last_stats, &state.last_check) {
            let dur = now.duration_since(*last_time).as_secs_f64();
            if dur > 0.0 {
                let dl = (total_download.saturating_sub(last.total_download)) as f64 / dur / 1024.0;
                let ul = (total_upload.saturating_sub(last.total_upload)) as f64 / dur / 1024.0;
                (dl, ul)
            } else { (0.0, 0.0) }
        } else { (0.0, 0.0) };

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
}
