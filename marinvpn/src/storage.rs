use crate::models::SettingsState;
use directories::ProjectDirs;
use keyring::Entry;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::{error, info};

const KEYRING_SERVICE: &str = "marinvpn";
const CONFIG_FILENAME: &str = "marinvpn_config.json";
const DEVICE_KEYRING_KEY: &str = "device_attestation_key";
const REFRESH_TOKEN_KEY: &str = "refresh_token";

static CONFIG_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[derive(Serialize, Deserialize, Default, PartialEq, Clone)]
pub struct AppConfig {
    pub settings: Option<SettingsState>,
    pub favorites: Option<HashSet<String>>,
    #[serde(skip)]
    pub account_number: Option<String>,
    #[serde(skip)]
    pub auth_token: Option<String>,
    #[serde(skip)]
    pub refresh_token: Option<String>,
    pub account_expiry: Option<i64>,
    pub device_name: Option<String>,
}

impl AppConfig {
    pub fn get_settings(&self) -> SettingsState {
        self.settings.clone().unwrap_or_default()
    }
}

pub fn get_config_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "marinvpn", "MarinVPN") {
        let config_dir = proj_dirs.config_dir();
        if !config_dir.exists() {
            let _ = fs::create_dir_all(config_dir);
        }
        return config_dir.join(CONFIG_FILENAME);
    }

    std::env::current_dir()
        .unwrap_or_default()
        .join(CONFIG_FILENAME)
}

fn get_account_entry() -> Result<Entry, keyring::Error> {
    Entry::new(KEYRING_SERVICE, "account_number")
}

fn get_token_entry() -> Result<Entry, keyring::Error> {
    Entry::new(KEYRING_SERVICE, "auth_token")
}

fn get_refresh_entry() -> Result<Entry, keyring::Error> {
    Entry::new(KEYRING_SERVICE, REFRESH_TOKEN_KEY)
}

fn get_device_key_entry() -> Result<Entry, keyring::Error> {
    Entry::new(KEYRING_SERVICE, DEVICE_KEYRING_KEY)
}

fn load_config_inner() -> AppConfig {
    let path = get_config_path();
    let mut config = match fs::read_to_string(&path) {
        Ok(contents) => {
            let legacy_account = match serde_json::from_str::<serde_json::Value>(&contents) {
                Ok(v) => v
                    .get("account_number")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string()),
                Err(_) => None,
            };

            match serde_json::from_str::<AppConfig>(&contents) {
                Ok(mut cfg) => {
                    if let Some(acc) = legacy_account {
                        info!("Found legacy plain-text account number. Migrating to secure storage...");
                        cfg.account_number = Some(acc);
                        // We are inside inner, so calling save_config_inner is safe if we were called from a locked context.
                        // But load_config_inner might be called from load_config (locked).
                        // So calling save_config_inner here is correct.
                        if let Err(e) = save_config_inner(&cfg) {
                            error!("Failed to migrate account number to secure storage: {}", e);
                        }
                    }
                    cfg
                }
                Err(e) => {
                    error!("Failed to parse config at {:?}: {}", path, e);
                    AppConfig::default()
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => AppConfig::default(),
        Err(e) => {
            error!("Failed to read config at {:?}: {}", path, e);
            AppConfig::default()
        }
    };

    if let Ok(entry) = get_account_entry() {
        if let Ok(pwd) = entry.get_password() {
            config.account_number = Some(pwd);
        }
    }

    if let Ok(entry) = get_token_entry() {
        if let Ok(pwd) = entry.get_password() {
            config.auth_token = Some(pwd);
        }
    }

    if let Ok(entry) = get_refresh_entry() {
        if let Ok(pwd) = entry.get_password() {
            config.refresh_token = Some(pwd);
        }
    }

    config
}

pub fn load_config() -> AppConfig {
    let _guard = CONFIG_LOCK.lock().unwrap();
    load_config_inner()
}

fn save_config_inner(config: &AppConfig) -> std::io::Result<()> {
    if let Ok(entry) = get_account_entry() {
        if let Some(ref acc) = config.account_number {
            let _ = entry.set_password(acc);
        } else {
            let _ = entry.delete_password();
        }
    }

    if let Ok(entry) = get_token_entry() {
        if let Some(ref token) = config.auth_token {
            let _ = entry.set_password(token);
        } else {
            let _ = entry.delete_password();
        }
    }

    if let Ok(entry) = get_refresh_entry() {
        if let Some(ref token) = config.refresh_token {
            let _ = entry.set_password(token);
        } else {
            let _ = entry.delete_password();
        }
    }

    let path = get_config_path();
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    fs::write(&path, json)?;
    Ok(())
}

pub fn save_config(config: &AppConfig) -> std::io::Result<()> {
    let _guard = CONFIG_LOCK.lock().unwrap();
    save_config_inner(config)
}

pub fn save_settings(settings: SettingsState) -> std::io::Result<()> {
    let _guard = CONFIG_LOCK.lock().unwrap();
    let mut config = load_config_inner();
    config.settings = Some(settings);
    save_config_inner(&config)
}

pub fn save_favorites(favorites: HashSet<String>) -> std::io::Result<()> {
    let _guard = CONFIG_LOCK.lock().unwrap();
    let mut config = load_config_inner();
    config.favorites = Some(favorites);
    save_config_inner(&config)
}

pub fn save_auth_info(
    account_number: Option<String>,
    auth_token: Option<String>,
    refresh_token: Option<String>,
    account_expiry: Option<i64>,
    device_name: Option<String>,
) -> std::io::Result<()> {
    let _guard = CONFIG_LOCK.lock().unwrap();
    let mut config = load_config_inner();
    config.account_number = account_number;
    config.auth_token = auth_token;
    config.refresh_token = refresh_token;
    config.account_expiry = account_expiry;
    config.device_name = device_name;
    save_config_inner(&config)
}

pub fn update_auth_tokens(
    auth_token: Option<String>,
    refresh_token: Option<String>,
) -> std::io::Result<()> {
    let _guard = CONFIG_LOCK.lock().unwrap();
    let mut config = load_config_inner();
    config.auth_token = auth_token;
    config.refresh_token = refresh_token;
    save_config_inner(&config)
}

pub fn load_device_attestation_key() -> Option<String> {
    get_device_key_entry()
        .ok()
        .and_then(|entry| entry.get_password().ok())
}

pub fn save_device_attestation_key(key: &str) {
    if let Ok(entry) = get_device_key_entry() {
        let _ = entry.set_password(key);
    }
}
