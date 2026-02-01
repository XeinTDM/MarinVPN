use crate::storage::{load_config, AppConfig};
use dioxus::prelude::*;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq)]
pub struct AccountState {
    pub account_number: Signal<Option<String>>,
    pub auth_token: Signal<Option<String>>,
    pub refresh_token: Signal<Option<String>>,
    pub account_expiry: Signal<Option<i64>>,
    pub device_name: Signal<String>,
}

pub fn use_account(initial_config: &AppConfig) -> AccountState {
    let account_number = use_signal(|| initial_config.account_number.clone());
    let mut auth_token = use_signal(|| initial_config.auth_token.clone());
    let mut refresh_token = use_signal(|| initial_config.refresh_token.clone());
    let account_expiry = use_signal(|| initial_config.account_expiry);
    let device_name = use_signal(|| {
        initial_config
            .device_name
            .clone()
            .unwrap_or_else(|| "Unknown Device".to_string())
    });

    // Auto-save auth info
    use_effect(move || {
        let acc = account_number();
        let auth = auth_token();
        let refresh = refresh_token();
        let exp = account_expiry();
        let dev = device_name();
        spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = tokio::task::spawn_blocking(move || {
                crate::storage::save_auth_info(acc, auth, refresh, exp, Some(dev))
            })
            .await;
        });
    });

    // Sync tokens from disk (in case another window/process updates them)
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_secs(15)).await;
            let cfg = tokio::task::spawn_blocking(load_config)
                .await
                .unwrap_or_default();

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

    AccountState {
        account_number,
        auth_token,
        refresh_token,
        account_expiry,
        device_name,
    }
}
