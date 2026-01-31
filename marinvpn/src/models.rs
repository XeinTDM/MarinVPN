use serde::{Serialize, Deserialize};

pub use marinvpn_common::{
    Account, Device, WireGuardConfig, ConnectionStatus, Protocol, 
    DnsBlockingState, IpVersion, LoginRequest, LoginResponse, 
    GenerateResponse, ConfigRequest, RemoveDeviceRequest, 
    ReportRequest, ErrorResponse, VpnServer as CommonVpnServer
};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    English,
    Swedish,
    German,
    French,
    Spanish,
    Italian,
    Dutch,
    Portuguese,
    Polish,
    Norwegian,
    Danish,
    Finnish,
}

impl Language {
    pub fn name(&self) -> &'static str {
        match self {
            Language::English => "English (US)",
            Language::Swedish => "Svenska",
            Language::German => "Deutsch",
            Language::French => "Français",
            Language::Spanish => "Español",
            Language::Italian => "Italiano",
            Language::Dutch => "Nederlands",
            Language::Portuguese => "Português",
            Language::Polish => "Polski",
            Language::Norwegian => "Norsk",
            Language::Danish => "Dansk",
            Language::Finnish => "Suomi",
        }
    }

    pub fn all() -> &'static [Language] {
        &[
            Language::English, 
            Language::Swedish, 
            Language::German, 
            Language::French,
            Language::Spanish,
            Language::Italian,
            Language::Dutch,
            Language::Portuguese,
            Language::Polish,
            Language::Norwegian,
            Language::Danish,
            Language::Finnish,
        ]
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Default)]
pub enum StealthMode {
    #[default]
    Automatic,
    WireGuardPort,
    Lwo,
    Quic,
    Shadowsocks,
    Tcp,
    None,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SettingsState {
    pub dark_mode: bool,
    pub launch_on_startup: bool,
    pub auto_connect: bool,
    pub local_sharing: bool,
    pub language: Language,
    pub protocol: Protocol,
    pub stealth_mode: StealthMode,
    pub ipv6_support: bool,
    pub quantum_resistant: bool,
    pub split_tunneling: bool,
    pub multi_hop: bool,
    pub entry_location: String,
    pub exit_location: String,
    pub lockdown_mode: bool,
    pub obfuscation: bool,
    pub daita_enabled: bool,
    pub dns_blocking: DnsBlockingState,
    pub custom_dns: bool,
    pub custom_dns_server: String,
    pub ip_version: IpVersion,
    pub mtu: u32,
    pub excluded_ips: Vec<String>,
    pub excluded_apps: Vec<AppInfo>,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            dark_mode: true,
            launch_on_startup: false,
            auto_connect: false,
            local_sharing: false,
            language: Language::English,
            protocol: Protocol::WireGuard,
            stealth_mode: StealthMode::None,
            ipv6_support: true,
            quantum_resistant: false,
            split_tunneling: false,
            multi_hop: false,
            entry_location: "Automatic".to_string(),
            exit_location: "Automatic".to_string(),
            lockdown_mode: false,
            obfuscation: false,
            daita_enabled: false,
            dns_blocking: DnsBlockingState::default(),
            custom_dns: false,
            custom_dns_server: "1.1.1.1".to_string(),
            ip_version: IpVersion::Automatic,
            mtu: 1420,
            excluded_ips: vec![],
            excluded_apps: vec![],
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct City {
    pub name: String,
    pub load: u8,
    pub ping: u8,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Region {
    pub name: String,
    pub flag: String,
    pub map_x: f64,
    pub map_y: f64,
    pub cities: Vec<City>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LocationInfo {
    pub country: String,
    pub city: String,
}

impl LocationInfo {
    pub fn from_string(s: &str) -> Self {
        let parts: Vec<&str> = s.split(',').collect();
        let country = parts.get(0).unwrap_or(&"Unknown").trim().to_string();
        let city = parts.get(1).unwrap_or(&"Unknown").trim().to_string();
        Self { country, city }
    }
}
