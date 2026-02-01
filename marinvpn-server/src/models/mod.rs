use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

pub use marinvpn_common::{
    Account, AnonymousConfigRequest, BlindTokenRequest, BlindTokenResponse, ConfigRequest,
    Device as CommonDevice, ErrorResponse, GenerateResponse, LoginRequest as CommonLoginRequest,
    LoginResponse as CommonLoginResponse, RefreshRequest as CommonRefreshRequest,
    RefreshResponse as CommonRefreshResponse, RemoveDeviceRequest, ReportRequest,
    VpnServer as CommonVpnServer, WireGuardConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Device {
    pub id: Option<i64>,
    pub account_id: String,
    pub name: String,
    pub added_at: i64,
    pub attestation_pubkey: Option<String>,
}

impl Device {
    pub fn into_common(self) -> CommonDevice {
        CommonDevice {
            name: self.name,
            created_date: chrono::DateTime::from_timestamp(self.added_at, 0)
                .unwrap_or_else(|| chrono::Utc::now())
                .format("%Y-%m-%d")
                .to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct VpnServer {
    pub id: i64,
    pub country: String,
    pub city: String,
    pub endpoint: String,
    pub public_key: String,
    pub is_active: bool,
    pub current_load: i64,
    pub avg_latency: i64,
}

impl VpnServer {
    pub fn health_score(&self) -> f64 {
        (self.current_load as f64 * 0.7) + (self.avg_latency as f64 * 0.3)
    }

    pub fn into_common(self) -> CommonVpnServer {
        CommonVpnServer {
            country: self.country,
            city: self.city,
            endpoint: self.endpoint,
            public_key: self.public_key,
            current_load: self.current_load as u8,
            avg_latency: self.avg_latency as u32,
        }
    }
}

pub mod requests {
    pub use marinvpn_common::{
        AnonymousConfigRequest, BlindTokenRequest, ConfigRequest, LoginRequest, RefreshRequest,
        RemoveDeviceRequest, ReportRequest,
    };
}

pub mod responses {
    pub use marinvpn_common::{
        BlindTokenResponse, ErrorResponse, GenerateResponse, LoginResponse, RefreshResponse,
    };
}
