use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[cfg(feature = "validation")]
use validator::Validate;

#[cfg(feature = "db")]
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Zeroize, ZeroizeOnDrop)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct DnsBlockingState {
    pub ads: bool,
    pub trackers: bool,
    pub malware: bool,
    pub gambling: bool,
    pub adult_content: bool,
    pub social_media: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "db", derive(FromRow))]
pub struct Device {
    pub name: String,
    pub created_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Zeroize, ZeroizeOnDrop)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "db", derive(FromRow))]
pub struct Account {
    pub account_number: String,
    pub expiry_date: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Zeroize, ZeroizeOnDrop)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct AnonymousConfigRequest {
    pub message: String,
    pub signature: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 100)))]
    pub location: String,
    #[cfg_attr(feature = "validation", validate(length(min = 40, max = 50)))]
    pub pub_key: String,
    pub dns_blocking: Option<DnsBlockingState>,
    pub quantum_resistant: bool,
    pub pqc_public_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Zeroize, ZeroizeOnDrop)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct ConfigRequest {
    #[cfg_attr(
        feature = "validation",
        validate(custom(function = "validate_account_number"))
    )]
    pub account_number: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 100)))]
    pub location: String,
    #[cfg_attr(feature = "validation", validate(length(min = 40, max = 50)))]
    pub pub_key: String,
    pub dns_blocking: Option<DnsBlockingState>,
    pub quantum_resistant: bool,
    pub pqc_public_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Zeroize, ZeroizeOnDrop)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct WireGuardConfig {
    pub private_key: String,
    pub public_key: String,
    pub preshared_key: Option<String>,
    pub endpoint: String,
    pub allowed_ips: String,
    pub address: String,
    pub dns: Option<String>,
    pub pqc_handshake: Option<String>,
    pub pqc_provider: Option<String>,
    pub pqc_ciphertext: Option<String>,
    pub obfuscation_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "db", derive(FromRow))]
pub struct VpnServer {
    pub country: String,
    pub city: String,
    pub endpoint: String,
    pub public_key: String,
    pub current_load: u8,
    pub avg_latency: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Zeroize, ZeroizeOnDrop)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct LoginRequest {
    #[cfg_attr(
        feature = "validation",
        validate(custom(function = "validate_account_number"))
    )]
    pub account_number: String,
    #[cfg_attr(feature = "validation", validate(length(min = 40, max = 80)))]
    pub device_pubkey: Option<String>,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 50)))]
    pub kick_device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct LoginResponse {
    pub success: bool,
    pub auth_token: Option<String>,
    pub refresh_token: Option<String>,
    pub account_info: Option<Account>,
    pub current_device: Option<String>,
    pub devices: Option<Vec<Device>>,
    pub error_code: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct GenerateResponse {
    pub account_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Zeroize, ZeroizeOnDrop)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct RefreshRequest {
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 4096)))]
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct RefreshResponse {
    pub auth_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct RemoveDeviceRequest {
    #[cfg_attr(
        feature = "validation",
        validate(custom(function = "validate_account_number"))
    )]
    pub account_number: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 50)))]
    pub device_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Zeroize, ZeroizeOnDrop)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "validation", derive(Validate))]
pub struct ReportRequest {
    #[cfg_attr(
        feature = "validation",
        validate(custom(function = "validate_account_number"))
    )]
    pub account_number: String,
    #[cfg_attr(feature = "validation", validate(length(min = 1, max = 2000)))]
    pub message: String,
    pub is_encrypted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ErrorResponse {
    pub error: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Zeroize, ZeroizeOnDrop)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct BlindTokenRequest {
    pub blinded_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Zeroize, ZeroizeOnDrop)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct BlindTokenResponse {
    pub signed_blinded_message: String,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum Protocol {
    #[default]
    WireGuard,
    Shadowsocks,
    Quic,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum IpVersion {
    #[default]
    Automatic,
    Ipv4,
    Ipv6,
}

#[cfg(test)]
mod tests;

#[cfg(feature = "validation")]
fn validate_account_number(value: &str) -> Result<(), validator::ValidationError> {
    const ALLOWED: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let cleaned: String = value
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase();
    if cleaned.len() != 16 {
        return Err(validator::ValidationError::new("account_length"));
    }
    if !cleaned.chars().all(|c| ALLOWED.contains(c)) {
        return Err(validator::ValidationError::new("account_charset"));
    }

    if value.chars().any(|c| c.is_whitespace() && c != ' ') {
        return Err(validator::ValidationError::new("account_grouping"));
    }

    if value.contains(' ') {
        let parts: Vec<&str> = value.split(' ').collect();
        if parts.len() != 4 || parts.iter().any(|p| p.len() != 4) {
            return Err(validator::ValidationError::new("account_grouping"));
        }
    } else if value.len() != 16 {
        return Err(validator::ValidationError::new("account_grouping"));
    }

    Ok(())
}
