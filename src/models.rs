use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Protocol {
    WireGuard,
    OpenVPN,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct DnsBlockingState {
    pub ads: bool,
    pub trackers: bool,
    pub malware: bool,
    pub gambling: bool,
    pub adult_content: bool,
    pub social_media: bool,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum IpVersion {
    Automatic,
    Ipv4,
    Ipv6,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct SettingsState {
    pub dark_mode: bool,
    pub launch_on_startup: bool,
    pub auto_connect: bool,
    pub local_sharing: bool,
    pub protocol: Protocol,
    pub ipv6_support: bool,
    pub quantum_resistant: bool,
    pub split_tunneling: bool,
    pub multi_hop: bool,
    pub kill_switch: bool,
    pub lockdown_mode: bool,
    pub obfuscation: bool,
    pub daita_enabled: bool,
    pub dns_blocking: DnsBlockingState,
    pub custom_dns: bool,
    pub ip_version: IpVersion,
    pub mtu: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct City {
    pub name: &'static str,
    pub load: u8,
    pub ping: u8,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Region {
    pub name: &'static str,
    pub flag: &'static str,
    pub map_x: f64,
    pub map_y: f64,
    pub cities: &'static [City],
}
