use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

pub use marinvpn_common::{
    Account, Device as CommonDevice, WireGuardConfig, 
    LoginRequest as CommonLoginRequest, 
    LoginResponse as CommonLoginResponse,
    GenerateResponse, ErrorResponse,
    ConfigRequest, RemoveDeviceRequest, ReportRequest,
    VpnServer as CommonVpnServer, AnonymousConfigRequest, BlindTokenRequest, BlindTokenResponse
};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Device {
    pub id: Option<i64>,
    pub account_id: String,
    pub name: String,
    pub added_at: i64,
}

impl Device {
    pub fn into_common(self) -> CommonDevice {
        CommonDevice {
            name: self.name,
            added_at: self.added_at,
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
    pub use marinvpn_common::{LoginRequest, ConfigRequest, RemoveDeviceRequest, ReportRequest, AnonymousConfigRequest, BlindTokenRequest};
}

pub mod responses {
    pub use marinvpn_common::{LoginResponse, GenerateResponse, ErrorResponse, BlindTokenResponse};
}
