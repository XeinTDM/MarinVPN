use crate::models::CommonVpnServer;
use std::time::{Instant, Duration};
use tokio::net::TcpStream;
use once_cell::sync::Lazy;

pub struct ServersService;

static API_BASE: Lazy<String> = Lazy::new(|| {
    std::env::var("MARIN_API_URL").unwrap_or_else(|_| "http://127.0.0.1:3000/api/v1".to_string())
});

use tokio::sync::Mutex;

static SERVER_CACHE: Lazy<Mutex<(Vec<CommonVpnServer>, Instant)>> = Lazy::new(|| {
    Mutex::new((Vec::new(), Instant::now() - Duration::from_secs(3600)))
});

impl ServersService {
    pub async fn get_servers() -> Result<Vec<CommonVpnServer>, String> {
        let mut cache = SERVER_CACHE.lock().await;
        if !cache.0.is_empty() && cache.1.elapsed() < Duration::from_secs(300) {
            return Ok(cache.0.clone());
        }

        let client = reqwest::Client::new();
        let res = client.get(format!("{}/vpn/servers", *API_BASE))
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Server error: {}", res.status()));
        }

        let servers: Vec<CommonVpnServer> = res.json()
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

        if candidates.is_empty() {
            return Err("No servers found".to_string());
        }

        // PERFORMANCE: Probe all candidates in parallel using tokio tasks
        let mut futures = Vec::new();
        for server in candidates {
            futures.push(tokio::spawn(async move {
                let latency = Self::measure_latency(&server.endpoint).await.unwrap_or(9999);
                (server, latency)
            }));
        }

        let results = futures_util::future::join_all(futures).await;
        
        let mut best_option: Option<(CommonVpnServer, u32)> = None;

        for res in results {
            if let Ok((server, latency)) = res {
                if best_option.is_none() || latency < best_option.as_ref().unwrap().1 {
                    best_option = Some((server, latency));
                }
            }
        }

        best_option.map(|(s, _)| s).ok_or_else(|| "Failed to measure any server".to_string())
    }

    pub async fn measure_latency(endpoint: &str) -> Option<u32> {
        let start = Instant::now();
        let timeout = Duration::from_millis(1500);
        
        match tokio::time::timeout(timeout, TcpStream::connect(endpoint)).await {
            Ok(Ok(_)) => {
                let duration = start.elapsed().as_millis() as u32;
                Some(duration)
            }
            _ => None,
        }
    }
}