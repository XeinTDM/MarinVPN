use crate::components::toast::{ToastManager, ToastType};
use crate::hooks::use_account::AccountState;
use crate::models::{ConnectionStatus, SettingsState, VpnAction};
use crate::services::vpn::{VpnEvent, VpnService, WireGuardService};
use crate::services::{AppService, ProductionAppService};
use chrono::Utc;
use dioxus::prelude::*;
use futures_util::StreamExt;
use std::time::Duration;

#[derive(Clone, Copy)]
pub struct VpnState {
    pub status: Signal<ConnectionStatus>,
    pub current_location: Signal<String>,
    pub connected_since: Signal<Option<f64>>,
    pub download_speed: Signal<f64>,
    pub upload_speed: Signal<f64>,
    pub vpn_action: Coroutine<VpnAction>,
}

pub fn use_connection(
    account_state: AccountState,
    settings: Signal<SettingsState>,
) -> VpnState {
    let vpn_service = use_hook(WireGuardService::new);
    let app_service = use_hook(|| ProductionAppService);
    
    use_connection_internal(account_state, settings, vpn_service, app_service)
}

pub fn use_connection_with_service<S: AppService, V: VpnService + Clone + 'static>(
    account_state: AccountState,
    settings: Signal<SettingsState>,
    vpn_service: V,
    app_service: S,
) -> VpnState {
    use_connection_internal(account_state, settings, vpn_service, app_service)
}

fn use_connection_internal<S: AppService, V: VpnService + Clone + 'static>(
    account_state: AccountState,
    settings: Signal<SettingsState>,
    vpn_service: V,
    app_service: S,
) -> VpnState {
    let mut status = use_signal(|| ConnectionStatus::Disconnected);
    let mut current_location = use_signal(|| "Sweden, Stockholm".to_string());
    let mut connected_since = use_signal(|| None);
    let mut download_speed = use_signal(|| 0.0);
    let mut upload_speed = use_signal(|| 0.0);
    let mut auto_connect_started = use_signal(|| false);

    let toast_manager = use_context::<ToastManager>();

    // Action Coroutine
    let vpn_service_action = vpn_service.clone();
    let app_service_action = app_service.clone();
    let account_number = account_state.account_number;
    let auth_token = account_state.auth_token;

    let vpn_action = use_coroutine(move |mut rx: UnboundedReceiver<VpnAction>| {
        let vpn_service = vpn_service_action.clone();
        let app_service = app_service_action.clone();
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

                            match app_service.find_best_server(country).await {
                                Ok(best) => {
                                    location = format!("{}, {}", best.country, best.city);
                                    current_location.set(location.clone());
                                }
                                Err(e) => {
                                    toasts.show(
                                        &e.user_friendly_message(),
                                        ToastType::Error,
                                    );
                                    continue;
                                }
                            }
                        }

                        let s = settings.peek().clone();
                        let auth = Some((acc_num.clone(), token.clone()));
                        match app_service.get_anonymous_config(
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
                                toasts.show(&e.user_friendly_message(), ToastType::Error)
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
                            match app_service.find_best_server(None).await {
                                Ok(best) => {
                                    entry_loc = format!("{}, {}", best.country, best.city);
                                }
                                Err(e) => {
                                    toasts.show(
                                        &e.user_friendly_message(),
                                        ToastType::Error,
                                    );
                                    continue;
                                }
                            }
                        }
                        if exit_loc == "Automatic" || exit_loc.contains("Auto") {
                            let exclude_entry = vec![entry_loc.clone()];
                            match app_service.find_best_server_excluding(None, &exclude_entry)
                                .await
                            {
                                Ok(best) => {
                                    exit_loc = format!("{}, {}", best.country, best.city);
                                }
                                Err(e) => {
                                    toasts.show(
                                        &e.user_friendly_message(),
                                        ToastType::Error,
                                    );
                                    continue;
                                }
                            }
                        }
                        if entry_loc == exit_loc {
                            let exclude_entry = vec![entry_loc.clone()];
                            if let Ok(best) =
                                app_service.find_best_server_excluding(None, &exclude_entry)
                                    .await
                            {
                                let candidate = format!("{}, {}", best.country, best.city);
                                if candidate != entry_loc {
                                    exit_loc = candidate;
                                }
                            }
                        }

                        let entry_fut = app_service.get_anonymous_config(
                            &entry_loc,
                            &token,
                            Some(s.dns_blocking.clone()),
                            s.quantum_resistant,
                        );
                        let exit_fut = app_service.get_anonymous_config(
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
                            (Err(e), _) => toasts.show(&e.user_friendly_message(), ToastType::Error),
                            (_, Err(e)) => toasts.show(&e.user_friendly_message(), ToastType::Error),
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

    // VPN Event Listener - use_hook to run only once
    let vpn_service_listener = vpn_service.clone();
    use_hook(move || {
        let mut rx = vpn_service_listener.subscribe();
        let mut toasts = toast_manager;
        let mut prev_status = ConnectionStatus::Disconnected;
        spawn(async move {
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
        });
    });

    // Lockdown Mode
    let vpn_service_lockdown = vpn_service.clone();
    use_effect(move || {
        let s = settings();
        let svc = vpn_service_lockdown.clone();
        spawn(async move {
            let _ = svc.apply_lockdown(&s).await;
        });
    });

    // Auto Connect
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

    VpnState {
        status,
        current_location,
        connected_since,
        download_speed,
        upload_speed,
        vpn_action,
    }
}
