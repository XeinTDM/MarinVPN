use crate::models::CommonVpnServer;
use futures_util::stream::{FuturesUnordered, StreamExt};
use once_cell::sync::Lazy;
use std::time::{Duration, Instant};

pub struct ServersService;

static API_BASE: Lazy<String> = Lazy::new(|| {
    std::env::var("MARIN_API_URL").unwrap_or_else(|_| "http://127.0.0.1:3000/api/v1".to_string())
});

use tokio::sync::Mutex;

static SERVER_CACHE: Lazy<Mutex<(Vec<CommonVpnServer>, Instant)>> =
    Lazy::new(|| Mutex::new((Vec::new(), Instant::now() - Duration::from_secs(3600))));

impl ServersService {
    pub async fn get_servers() -> Result<Vec<CommonVpnServer>, String> {
        let mut cache = SERVER_CACHE.lock().await;
        if !cache.0.is_empty() && cache.1.elapsed() < Duration::from_secs(300) {
            return Ok(cache.0.clone());
        }

        let client = reqwest::Client::new();
        let res = client
            .get(format!("{}/vpn/servers", *API_BASE))
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Server error: {}", res.status()));
        }

        let servers: Vec<CommonVpnServer> = res
            .json()
            .await
            .map_err(|e| format!("Server error: {}", e))?;

        *cache = (servers.clone(), Instant::now());
        Ok(servers)
    }

    pub async fn find_best_server(country: Option<&str>) -> Result<CommonVpnServer, String> {
        let servers = Self::get_servers().await?;
        let candidates: Vec<CommonVpnServer> = if let Some(c) = country {
            servers.into_iter().filter(|s| s.country == c).collect()
        } else {
            servers
        };

        Self::select_best_server_from_candidates(candidates).await
    }

    pub async fn find_best_server_excluding(
        country: Option<&str>,
        exclude_locations: &[String],
    ) -> Result<CommonVpnServer, String> {
        if exclude_locations.is_empty() {
            return Self::find_best_server(country).await;
        }

        let servers = Self::get_servers().await?;
        let mut candidates: Vec<CommonVpnServer> = Vec::new();
        for server in servers.into_iter() {
            if country.map_or(true, |c| server.country == c) {
                let location = format!("{}, {}", server.country, server.city);
                if exclude_locations.iter().any(|ex| ex == &location) {
                    continue;
                }
                candidates.push(server);
            }
        }

        if candidates.is_empty() {
            return Self::find_best_server(country).await;
        }

        Self::select_best_server_from_candidates(candidates).await
    }

    async fn select_best_server_from_candidates(
        candidates: Vec<CommonVpnServer>,
    ) -> Result<CommonVpnServer, String> {
        if candidates.is_empty() {
            return Err("No servers found".to_string());
        }

        let mut futures = FuturesUnordered::new();
        for server in candidates {
            futures.push(async move {
                let latency = Self::measure_latency(&server.endpoint)
                    .await
                    .unwrap_or(9999);
                (server, latency)
            });
        }

        let mut best_option: Option<(CommonVpnServer, f64)> = None;
        while let Some((server, latency)) = futures.next().await {
            let local_score = (server.current_load as f64 * 0.7) + (latency as f64 * 0.3);

            if best_option.is_none() || local_score < best_option.as_ref().unwrap().1 {
                best_option = Some((server, local_score));
            }
        }

        best_option
            .map(|(s, _)| s)
            .ok_or_else(|| "Failed to measure any server".to_string())
    }

    pub async fn measure_latency(endpoint: &str) -> Option<u32> {
        let start = Instant::now();
        let timeout = Duration::from_millis(800);

        if tokio::time::timeout(timeout, async {
            if let Ok(socket) = tokio::net::UdpSocket::bind("0.0.0.0:0").await {
                let _ = socket.connect(endpoint).await;
            }
        })
        .await
        .is_ok()
        {
            let elapsed_ms = start.elapsed().as_millis();
            let capped = std::cmp::min(elapsed_ms, timeout.as_millis()) as u32;
            return Some(capped.max(1));
        }

        if let Ok(cache) = SERVER_CACHE.try_lock() {
            if let Some(s) = cache.0.iter().find(|s| s.endpoint == endpoint) {
                return Some(s.avg_latency);
            }
        }

        None
    }
}
