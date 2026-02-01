use crate::hooks::use_account::use_account;
use crate::hooks::use_connection::use_connection;
use crate::hooks::use_servers::use_servers;
use crate::models::{ConnectionStatus, Region, SettingsState, VpnAction};
use crate::storage::load_config;
use dioxus::prelude::*;
use std::collections::HashSet;
use std::time::Duration;

#[derive(Clone, Copy)]
pub struct ConnectionState {
    pub status: Signal<ConnectionStatus>,
    pub current_location: Signal<String>,
    pub regions: Signal<Vec<Region>>,
    pub account_number: Signal<Option<String>>,
    pub auth_token: Signal<Option<String>>,
    pub refresh_token: Signal<Option<String>>,
    pub account_expiry: Signal<Option<i64>>,
    pub settings: Signal<SettingsState>,
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

    // Account Hook
    let account_state = use_account(&config);

    // Servers Hook
    let regions = use_servers();

    // Settings (still here for now)
    let settings = use_signal(|| config.get_settings());
    let favorites = use_signal(|| config.favorites.clone().unwrap_or_default());
    let scroll_to = use_signal(|| None);

    // Connection Hook (depends on Account and Settings)
    let vpn_state = use_connection(account_state, settings);

    // Persistence Effects
    use_effect(move || {
        let s = settings();
        spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = tokio::task::spawn_blocking(move || crate::storage::save_settings(s)).await;
        });
    });

    use_effect(move || {
        let f = favorites.read().clone();
        spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = tokio::task::spawn_blocking(move || crate::storage::save_favorites(f)).await;
        });
    });

    use_context_provider(|| ConnectionState {
        status: vpn_state.status,
        current_location: vpn_state.current_location,
        regions,
        account_number: account_state.account_number,
        auth_token: account_state.auth_token,
        refresh_token: account_state.refresh_token,
        account_expiry: account_state.account_expiry,
        settings,
        connected_since: vpn_state.connected_since,
        favorites,
        scroll_to,
        download_speed: vpn_state.download_speed,
        upload_speed: vpn_state.upload_speed,
        device_name: account_state.device_name,
        vpn_action: vpn_state.vpn_action,
    });

    rsx! {
        {children}
    }
}