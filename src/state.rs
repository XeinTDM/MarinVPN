use dioxus::prelude::*;
use std::collections::HashSet;
use std::time::Duration;
use crate::models::{ConnectionStatus, SettingsState, Protocol, DnsBlockingState, IpVersion};
use crate::storage::{load_config, save_config, AppConfig};

#[derive(Clone, Copy)]
pub struct ConnectionState {
    pub status: Signal<ConnectionStatus>,
    pub current_location: Signal<String>,
    pub account_number: Signal<Option<String>>,
    pub settings: Signal<SettingsState>,
    pub connected_since: Signal<Option<f64>>, 
    pub favorites: Signal<HashSet<String>>,
    pub scroll_to: Signal<Option<String>>,
    pub download_speed: Signal<f64>,
    pub upload_speed: Signal<f64>,
    pub device_name: Signal<String>,
}

#[component]
pub fn AppStateProvider(children: Element) -> Element {
    let config = load_config();

    let status = use_signal(|| ConnectionStatus::Disconnected);
    let current_location = use_signal(|| "Sweden, Stockholm".to_string());
    let account_number = use_signal(|| config.account_number);
    let device_name = use_signal(|| config.device_name.unwrap_or_else(|| "Unknown Device".to_string()));
    let settings = use_signal(|| config.settings.unwrap_or(SettingsState {
        dark_mode: true,
        launch_on_startup: false,
        auto_connect: false,
        local_sharing: true,
        
        protocol: Protocol::WireGuard,
        ipv6_support: true,
        quantum_resistant: false,
        split_tunneling: false,
        multi_hop: false,
        
        kill_switch: true,
        lockdown_mode: false,
        obfuscation: false,
        daita_enabled: false,
        dns_blocking: DnsBlockingState {
            ads: false,
            trackers: false,
            malware: true,
            gambling: false,
            adult_content: false,
            social_media: false,
        },
        custom_dns: false,
        ip_version: IpVersion::Automatic,
        mtu: 1420,
    }));
    let connected_since = use_signal(|| None);
    let favorites = use_signal(|| config.favorites.unwrap_or_else(|| HashSet::new()));
    let scroll_to = use_signal(|| None);
    let mut download_speed = use_signal(|| 0.0);
    let mut upload_speed = use_signal(|| 0.0);

    // Simulation task
    use_future(move || async move {
        use rand::Rng;
        let mut interval = tokio::time::interval(Duration::from_millis(1000));
        loop {
            interval.tick().await;
            if status() == ConnectionStatus::Connected {
                let mut rng = rand::thread_rng();
                let base_down: f64 = 85.0;
                let base_up: f64 = 45.0;
                download_speed.set((base_down + rng.gen_range(-15.0..15.0)).max(0.0));
                upload_speed.set((base_up + rng.gen_range(-5.0..15.0)).max(0.0));
            } else {
                download_speed.set(0.0);
                upload_speed.set(0.0);
            }
        }
    });

    // Watch for changes and save to disk with debouncing
    use_future(move || {
        let account_number = account_number();
        let device_name = device_name();
        let settings = settings();
        let favorites = favorites.cloned();

        async move {
            // Wait for 1 second of stability
            tokio::time::sleep(Duration::from_secs(1)).await;
            
            let _ = save_config(&AppConfig {
                account_number,
                device_name: Some(device_name),
                settings: Some(settings),
                favorites: Some(favorites),
            });
        }
    });

    use_context_provider(|| ConnectionState {
        status,
        current_location,
        account_number,
        settings,
        connected_since,
        favorites,
        scroll_to,
        download_speed,
        upload_speed,
        device_name,
    });
    
    rsx! {
        {children}
    }
}
