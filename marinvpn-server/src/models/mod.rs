use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

// Re-export common types for easier access
pub use marinvpn_common::{
    Account, Device as CommonDevice, WireGuardConfig, 
    LoginRequest as CommonLoginRequest, 
    LoginResponse as CommonLoginResponse,
    GenerateResponse, ErrorResponse,
    ConfigRequest, RemoveDeviceRequest, ReportRequest,
    VpnServer as CommonVpnServer
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
}

impl VpnServer {
    pub fn into_common(self) -> CommonVpnServer {
        CommonVpnServer {
            country: self.country,
            city: self.city,
            endpoint: self.endpoint,
            public_key: self.public_key,
        }
    }
}

pub mod requests {
    pub use marinvpn_common::{LoginRequest, ConfigRequest, RemoveDeviceRequest, ReportRequest};
}

pub mod responses {
    pub use marinvpn_common::{LoginResponse, GenerateResponse, ErrorResponse};
}