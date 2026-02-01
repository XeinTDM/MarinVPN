use crate::components::toast::{ToastManager, ToastType};
use crate::models::{City, ConnectionStatus, Region};
use crate::services::auth::AuthService;
use crate::services::servers::ServersService;
use crate::services::vpn::{VpnEvent, VpnService, WireGuardService};
use crate::storage::{load_config, save_config, AppConfig};
use chrono::Utc;
use dioxus::prelude::*;
use futures_util::{future, StreamExt};
use std::collections::HashSet;
use std::time::Duration;

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
    pub refresh_token: Signal<Option<String>>,
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
    let config = use_hook(load_config);

    let mut status = use_signal(|| ConnectionStatus::Disconnected);
    let mut current_location = use_signal(|| "Sweden, Stockholm".to_string());
    let mut regions = use_signal(crate::data::get_default_regions);
    let account_number = use_signal(|| config.account_number.clone());
    let mut auth_token = use_signal(|| config.auth_token.clone());
    let mut refresh_token = use_signal(|| config.refresh_token.clone());
    let account_expiry = use_signal(|| config.account_expiry);
    let device_name = use_signal(|| {
        config
            .device_name
            .clone()
            .unwrap_or_else(|| "Unknown Device".to_string())
    });
    let settings = use_signal(|| config.get_settings());
    let mut connected_since = use_signal(|| None);
    let favorites = use_signal(|| config.favorites.clone().unwrap_or_default());
    let scroll_to = use_signal(|| None);
    let mut download_speed = use_signal(|| 0.0);
    let mut upload_speed = use_signal(|| 0.0);
    let mut auto_connect_started = use_signal(|| false);

    let vpn_service = use_hook(WireGuardService::new);
    let toast_manager = use_context::<ToastManager>();

    use_future(move || async move {
        loop {
            match ServersService::get_servers().await {
                Ok(api_servers) => {
                    let mut ping_tasks = Vec::new();
                    for s in &api_servers {
                        let endpoint = s.endpoint.clone();
                        ping_tasks.push(async move {
                            ServersService::measure_latency(&endpoint)
                                .await
                                .unwrap_or(999)
                        });
                    }

                    let pings = future::join_all(ping_tasks).await;

                    let mut new_regions: Vec<Region> = Vec::new();
                    for (i, s) in api_servers.into_iter().enumerate() {
                        let ping = pings[i];

                        if let Some(reg) = new_regions.iter_mut().find(|r| r.name == s.country) {
                            if !reg.cities.iter().any(|c| c.name == s.city) {
                                reg.cities.push(City {
                                    name: s.city,
                                    load: 0,
                                    ping: ping as u8,
                                });
                            }
                        } else {
                            let defaults = crate::data::get_default_regions();
                            let (flag, x, y) = defaults
                                .iter()
                                .find(|r| r.name == s.country)
                                .map(|r| (r.flag.clone(), r.map_x, r.map_y))
                                .unwrap_or(("🌐".to_string(), 0.0, 0.0));

                            new_regions.push(Region {
                                name: s.country,
                                flag,
                                map_x: x,
                                map_y: y,
                                cities: vec![City {
                                    name: s.city,
                                    load: 0,
                                    ping: ping as u8,
                                }],
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
                    VpnAction::Connect(mut location) => {
                        let acc_num = account_number.peek().clone().unwrap_or_default();
                        let token = auth_token.peek().clone().unwrap_or_default();
                        if acc_num.is_empty() {
                            toasts.show("Please log in first", ToastType::Error);
                            continue;
                        }

                        if location == "Automatic" || location.contains("Auto") {
                            toasts.show("Finding best server...", ToastType::Info);
                            let country = if location.contains(",") {
                                location.split(',').next().map(|s| s.trim())
                            } else {
                                None
                            };

                            match ServersService::find_best_server(country).await {
                                Ok(best) => {
                                    location = format!("{}, {}", best.country, best.city);
                                    current_location.set(location.clone());
                                }
                                Err(e) => {
                                    toasts.show(
                                        &format!("Auto-select failed: {}", e),
                                        ToastType::Error,
                                    );
                                    continue;
                                }
                            }
                        }

                        let s = settings.peek().clone();
                        let auth = Some((acc_num.clone(), token.clone()));
                        match AuthService::get_anonymous_config(
                            &location,
                            &token,
                            Some(s.dns_blocking.clone()),
                            s.quantum_resistant,
                        )
                        .await
                        {
                            Ok(config) => {
                                vpn_service.connect(location, config, None, s, auth).await
                            }
                            Err(e) => {
                                toasts.show(&format!("Config error: {}", e), ToastType::Error)
                            }
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
                        let auth = Some((acc_num.clone(), token.clone()));
                        let mut entry_loc = entry;
                        let mut exit_loc = exit;
                        if entry_loc == "Automatic" || entry_loc.contains("Auto") {
                            match ServersService::find_best_server(None).await {
                                Ok(best) => {
                                    entry_loc = format!("{}, {}", best.country, best.city);
                                }
                                Err(e) => {
                                    toasts.show(
                                        &format!("Auto-select entry failed: {}", e),
                                        ToastType::Error,
                                    );
                                    continue;
                                }
                            }
                        }
                        if exit_loc == "Automatic" || exit_loc.contains("Auto") {
                            let exclude_entry = vec![entry_loc.clone()];
                            match ServersService::find_best_server_excluding(None, &exclude_entry)
                                .await
                            {
                                Ok(best) => {
                                    exit_loc = format!("{}, {}", best.country, best.city);
                                }
                                Err(e) => {
                                    toasts.show(
                                        &format!("Auto-select exit failed: {}", e),
                                        ToastType::Error,
                                    );
                                    continue;
                                }
                            }
                        }
                        if entry_loc == exit_loc {
                            let exclude_entry = vec![entry_loc.clone()];
                            if let Ok(best) =
                                ServersService::find_best_server_excluding(None, &exclude_entry)
                                    .await
                            {
                                let candidate = format!("{}, {}", best.country, best.city);
                                if candidate != entry_loc {
                                    exit_loc = candidate;
                                }
                            }
                        }

                        let entry_fut = AuthService::get_anonymous_config(
                            &entry_loc,
                            &token,
                            Some(s.dns_blocking.clone()),
                            s.quantum_resistant,
                        );
                        let exit_fut = AuthService::get_anonymous_config(
                            &exit_loc,
                            &token,
                            Some(s.dns_blocking.clone()),
                            s.quantum_resistant,
                        );
                        match tokio::join!(entry_fut, exit_fut) {
                            (Ok(e_cfg), Ok(x_cfg)) => {
                                vpn_service
                                    .connect(entry_loc, e_cfg, Some((exit_loc, x_cfg)), s, auth)
                                    .await
                            }
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
            refresh_token: refresh_token(),
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

    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_secs(15)).await;
            let cfg = load_config();
            let current_auth = auth_token.peek().clone();
            if current_auth != cfg.auth_token {
                auth_token.set(cfg.auth_token.clone());
            }
            let current_refresh = refresh_token.peek().clone();
            if current_refresh != cfg.refresh_token {
                refresh_token.set(cfg.refresh_token.clone());
            }
        }
    });

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
                            if prev_status == ConnectionStatus::Connected
                                || prev_status == ConnectionStatus::Disconnecting
                            {
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
                    VpnEvent::CaptivePortalActive(active) => {
                        if active {
                            toasts.show(
                                "Captive Portal Detected. Firewall temporarily disabled.",
                                ToastType::Info,
                            );
                        }
                    }
                }
            }
        }
    });

    let vpn_service_lockdown = vpn_service.clone();
    use_effect(move || {
        let s = settings();
        let svc = vpn_service_lockdown.clone();
        spawn(async move {
            let _ = svc.apply_lockdown(&s).await;
        });
    });

    let vpn_action_auto = vpn_action;
    use_effect(move || {
        let s = settings();
        let account = account_number();
        let has_account = account
            .as_ref()
            .map(|value| !value.is_empty())
            .unwrap_or(false);
        if has_account && !auto_connect_started() && s.auto_connect {
            auto_connect_started.set(true);
            if s.multi_hop {
                let entry = if s.entry_location.is_empty() {
                    "Automatic".to_string()
                } else {
                    s.entry_location.clone()
                };
                let exit = if s.exit_location.is_empty() {
                    "Automatic".to_string()
                } else {
                    s.exit_location.clone()
                };
                vpn_action_auto.send(VpnAction::MultiHopConnect(entry, exit));
            } else {
                let loc = if s.entry_location.is_empty() {
                    "Automatic".to_string()
                } else {
                    s.entry_location.clone()
                };
                vpn_action_auto.send(VpnAction::Connect(loc));
            }
        }
    });

    use_context_provider(|| ConnectionState {
        status,
        current_location,
        regions,
        account_number,
        auth_token,
        refresh_token,
        account_expiry,
        settings,
        connected_since,
        favorites,
        scroll_to,
        download_speed,
        upload_speed,
        device_name,
        vpn_action,
    });

    rsx! {
        {children}

    }
}
