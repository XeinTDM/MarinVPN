pub mod apps;
pub mod auth;
pub mod servers;
pub mod vpn;

use crate::error::AppError;
use crate::models::{CommonVpnServer, WireGuardConfig};
use async_trait::async_trait;
use marinvpn_common::DnsBlockingState;

#[async_trait]
pub trait AppService: Clone + Send + Sync + 'static {
    async fn find_best_server(&self, country: Option<&str>) -> Result<CommonVpnServer, AppError>;
    async fn find_best_server_excluding(
        &self,
        country: Option<&str>,
        exclude: &[String],
    ) -> Result<CommonVpnServer, AppError>;
    async fn get_anonymous_config(
        &self,
        location: &str,
        token: &str,
        dns_blocking: Option<DnsBlockingState>,
        quantum_resistant: bool,
    ) -> Result<WireGuardConfig, AppError>;
    async fn get_servers(&self) -> Result<Vec<CommonVpnServer>, AppError>;
    async fn measure_latency(&self, endpoint: &str) -> Option<u32>;
}

#[derive(Clone, Copy)]
pub struct ProductionAppService;

#[async_trait]
impl AppService for ProductionAppService {
    async fn find_best_server(&self, country: Option<&str>) -> Result<CommonVpnServer, AppError> {
        servers::ServersService::find_best_server(country).await
    }

    async fn find_best_server_excluding(
        &self,
        country: Option<&str>,
        exclude: &[String],
    ) -> Result<CommonVpnServer, AppError> {
        servers::ServersService::find_best_server_excluding(country, exclude).await
    }

    async fn get_anonymous_config(
        &self,
        location: &str,
        token: &str,
        dns_blocking: Option<DnsBlockingState>,
        quantum_resistant: bool,
    ) -> Result<WireGuardConfig, AppError> {
        auth::AuthService::get_anonymous_config(location, token, dns_blocking, quantum_resistant)
            .await
    }

    async fn get_servers(&self) -> Result<Vec<CommonVpnServer>, AppError> {
        servers::ServersService::get_servers().await
    }

    async fn measure_latency(&self, endpoint: &str) -> Option<u32> {
        servers::ServersService::measure_latency(endpoint).await
    }
}
