use serde::{Deserialize, Serialize};

pub use marinvpn_common::{
    Account, ConfigRequest, ConnectionStatus, Device, DnsBlockingState, ErrorResponse,
    GenerateResponse, IpVersion, LoginRequest, LoginResponse, Protocol, RefreshRequest,
    RefreshResponse, RemoveDeviceRequest, ReportRequest, VpnServer as CommonVpnServer,
    WireGuardConfig,
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
    PortugueseBrazilian,
    Polish,
    Norwegian,
    Danish,
    Finnish,
    Russian,
    ChineseSimplified,
    ChineseTraditional,
    Arabic,
    Turkish,
    Persian,
    Thai,
    Japanese,
    Korean,
    Indonesian,
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
            Language::PortugueseBrazilian => "Português (Brasil)",
            Language::Polish => "Polski",
            Language::Norwegian => "Norsk",
            Language::Danish => "Dansk",
            Language::Finnish => "Suomi",
            Language::Russian => "Русский",
            Language::ChineseSimplified => "简体中文",
            Language::ChineseTraditional => "繁體中文",
            Language::Arabic => "العربية",
            Language::Turkish => "Türkçe",
            Language::Persian => "فارسی",
            Language::Thai => "ไทย",
            Language::Japanese => "日本語",
            Language::Korean => "한국어",
            Language::Indonesian => "Bahasa Indonesia",
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
            Language::PortugueseBrazilian,
            Language::Polish,
            Language::Norwegian,
            Language::Danish,
            Language::Finnish,
            Language::Russian,
            Language::ChineseSimplified,
            Language::ChineseTraditional,
            Language::Arabic,
            Language::Turkish,
            Language::Persian,
            Language::Thai,
            Language::Japanese,
            Language::Korean,
            Language::Indonesian,
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
    pub branding_preset: String,
    pub branding_name: String,
    pub branding_accent_color: String,
    pub branding_logo_path: String,
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
            branding_preset: "custom".to_string(),
            branding_name: "MarinVPN".to_string(),
            branding_accent_color: "#6D28D9".to_string(),
            branding_logo_path: "".to_string(),
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
        let country = parts.first().unwrap_or(&"Unknown").trim().to_string();
        let city = parts.get(1).unwrap_or(&"Unknown").trim().to_string();
        Self { country, city }
    }
}
