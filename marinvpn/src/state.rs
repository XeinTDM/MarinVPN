use dioxus::prelude::*;
use std::collections::HashSet;
use std::time::Duration;
use crate::models::{ConnectionStatus, Region, City};
use crate::storage::{load_config, save_config, AppConfig};
use crate::services::vpn::{WireGuardService, VpnEvent, VpnService};
use crate::services::auth::AuthService;
use crate::services::servers::ServersService;
use crate::components::toast::{ToastManager, ToastType};
use chrono::Utc;
use futures_util::StreamExt;

pub enum VpnAction {
    Connect(String),
    MultiHopConnect(String, String),
    Disconnect,
    Reconnect,
}

#[derive(Clone, Copy)]
pub struct ConnectionState {
    pub status: Signal<ConnectionStatus>,
    pub current_location: Signal<String>,
    pub regions: Signal<Vec<Region>>,
    pub account_number: Signal<Option<String>>,
    pub auth_token: Signal<Option<String>>,
    pub account_expiry: Signal<Option<i64>>,
    pub settings: Signal<crate::models::SettingsState>,
    pub connected_since: Signal<Option<f64>>, 
    pub favorites: Signal<HashSet<String>>,
    pub scroll_to: Signal<Option<String>>,
    pub download_speed: Signal<f64>,
    pub upload_speed: Signal<f64>,
    pub device_name: Signal<String>,
    pub vpn_action: Coroutine<VpnAction>,
}

#[component]
pub fn AppStateProvider(children: Element) -> Element {
    let config = use_hook(|| load_config());

    let mut status = use_signal(|| ConnectionStatus::Disconnected);
    let mut current_location = use_signal(|| "Sweden, Stockholm".to_string());
    let mut regions = use_signal(|| crate::data::get_default_regions());
    let account_number = use_signal(|| config.account_number.clone());
    let auth_token = use_signal(|| config.auth_token.clone());
    let account_expiry = use_signal(|| config.account_expiry);
    let device_name = use_signal(|| config.device_name.clone().unwrap_or_else(|| "Unknown Device".to_string()));
    let settings = use_signal(|| config.get_settings());
    let mut connected_since = use_signal(|| None);
    let favorites = use_signal(|| config.favorites.clone().unwrap_or_default());
    let scroll_to = use_signal(|| None);
    let mut download_speed = use_signal(|| 0.0);
    let mut upload_speed = use_signal(|| 0.0);

    let vpn_service = use_hook(|| WireGuardService::new());
    let toast_manager = use_context::<ToastManager>();

    use_future(move || async move {
        loop {
            match ServersService::get_servers().await {
                Ok(api_servers) => {
                    let mut new_regions: Vec<Region> = Vec::new();
                    for s in api_servers {
                        let ping = ServersService::measure_latency(&s.endpoint).await.unwrap_or(999);
                        
                        if let Some(reg) = new_regions.iter_mut().find(|r| r.name == s.country) {
                            if !reg.cities.iter().any(|c| c.name == s.city) {
                                reg.cities.push(City { name: s.city, load: 0, ping: ping as u8 });
                            }
                        } else {
                            let defaults = crate::data::get_default_regions();
                            let (flag, x, y) = defaults.iter().find(|r| r.name == s.country)
                                .map(|r| (r.flag.clone(), r.map_x, r.map_y))
                                .unwrap_or(("🌐".to_string(), 0.0, 0.0));
                            
                            new_regions.push(Region {
                                name: s.country, flag, map_x: x, map_y: y,
                                cities: vec![City { name: s.city, load: 0, ping: ping as u8 }],
                            });
                        }
                    }
                    if !new_regions.is_empty() {
                        regions.set(new_regions);
                    }
                }
                Err(e) => tracing::error!("Failed to sync server list: {}", e),
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });

    let vpn_service_action = vpn_service.clone();
    let vpn_action = use_coroutine(move |mut rx: UnboundedReceiver<VpnAction>| {
        let vpn_service = vpn_service_action.clone();
        let mut toasts = toast_manager;
        async move {
            while let Some(msg) = rx.next().await {
                match msg {
                    VpnAction::Connect(location) => {
                        let acc_num = account_number.peek().clone().unwrap_or_default();
                        let token = auth_token.peek().clone().unwrap_or_default();
                        if acc_num.is_empty() {
                            toasts.show("Please log in first", ToastType::Error);
                            continue;
                        }
                        let s = settings.peek().clone();
                        match AuthService::get_config(&acc_num, &location, &token, Some(s.dns_blocking), s.quantum_resistant).await {
                            Ok(config) => vpn_service.connect(location, config, None, s).await,
                            Err(e) => toasts.show(&format!("Config error: {}", e), ToastType::Error),
                        }
                    }
                    VpnAction::MultiHopConnect(entry, exit) => {
                        let acc_num = account_number.peek().clone().unwrap_or_default();
                        let token = auth_token.peek().clone().unwrap_or_default();
                        if acc_num.is_empty() {
                            toasts.show("Please log in first", ToastType::Error);
                            continue;
                        }
                        let s = settings.peek().clone();
                        let entry_fut = AuthService::get_config(&acc_num, &entry, &token, Some(s.dns_blocking), s.quantum_resistant);
                        let exit_fut = AuthService::get_config(&acc_num, &exit, &token, Some(s.dns_blocking), s.quantum_resistant);
                        match tokio::join!(entry_fut, exit_fut) {
                            (Ok(e_cfg), Ok(x_cfg)) => vpn_service.connect(entry, e_cfg, Some((exit, x_cfg)), s).await,
                            _ => toasts.show("Failed to fetch Multi-hop configs", ToastType::Error),
                        }
                    }
                    VpnAction::Disconnect => vpn_service.disconnect().await,
                    VpnAction::Reconnect => {
                        let _ = vpn_service.disconnect().await;
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    });

    use_future(move || {
        let cfg = AppConfig {
            account_number: account_number(),
            auth_token: auth_token(),
            account_expiry: account_expiry(),
            device_name: Some(device_name()),
            settings: Some(settings()),
            favorites: Some(favorites.cloned()),
        };
        async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = tokio::task::spawn_blocking(move || save_config(&cfg)).await;
        }
    });

    // Listen to VPN Service Events
    let vpn_service_listener = vpn_service.clone();
    use_coroutine(move |_: UnboundedReceiver<()>| {
        let mut rx = vpn_service_listener.subscribe();
        let mut toasts = toast_manager;
        let mut prev_status = ConnectionStatus::Disconnected; 
        async move {
            while let Ok(event) = rx.recv().await {
                match event {
                    VpnEvent::StatusChanged(new_status) => {
                        status.set(new_status);
                        if new_status == ConnectionStatus::Connected {
                            connected_since.set(Some(Utc::now().timestamp() as f64));
                            toasts.show("Connected securely", ToastType::Success);
                        } else if new_status == ConnectionStatus::Disconnected {
                            connected_since.set(None);
                            if prev_status == ConnectionStatus::Connected || prev_status == ConnectionStatus::Disconnecting {
                                toasts.show("Disconnected", ToastType::Info);
                            }
                        }
                        prev_status = new_status;
                    }
                    VpnEvent::LocationChanged(loc) => current_location.set(loc),
                    VpnEvent::StatsUpdated(stats) => {
                        download_speed.set(stats.download_speed);
                        upload_speed.set(stats.upload_speed);
                    }
                    VpnEvent::Error(err) => toasts.show(&err.to_string(), ToastType::Error),
                }
            }
        }
    });

    use_context_provider(|| ConnectionState {
        status, current_location, regions, account_number, auth_token, account_expiry, settings,
        connected_since, favorites, scroll_to, download_speed, upload_speed, device_name, vpn_action,
    });
    
    rsx! { {children} }
}