use std::fs;
use std::path::PathBuf;
use std::collections::HashSet;
use serde::{Serialize, Deserialize};
use crate::models::SettingsState;
use directories::ProjectDirs;
use keyring::Entry;
use tracing::{info, error, warn};

const CONFIG_FILENAME: &str = "config.json";
const KEYRING_SERVICE: &str = "marinvpn";
const KEYRING_USER: &str = "active_user";

#[derive(Serialize, Deserialize, Default, PartialEq, Clone)]
pub struct AppConfig {
    pub settings: Option<SettingsState>,
    pub favorites: Option<HashSet<String>>,
    #[serde(skip)]
    pub account_number: Option<String>,
    #[serde(skip)]
    pub auth_token: Option<String>,
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
    
    // Fallback to current dir if something goes wrong
    std::env::current_dir().unwrap_or_default().join(CONFIG_FILENAME)
}

fn get_account_entry() -> Result<Entry, keyring::Error> {
    Entry::new(KEYRING_SERVICE, "account_number")
}

fn get_token_entry() -> Result<Entry, keyring::Error> {
    Entry::new(KEYRING_SERVICE, "auth_token")
}

pub fn load_config() -> AppConfig {
    let path = get_config_path();
    let mut config = match fs::read_to_string(&path) {
        Ok(contents) => {
            // Check for legacy account number in JSON
            let legacy_account = match serde_json::from_str::<serde_json::Value>(&contents) {
                Ok(v) => v.get("account_number").and_then(|s| s.as_str()).map(|s| s.to_string()),
                Err(_) => None,
            };

            match serde_json::from_str::<AppConfig>(&contents) {
                Ok(mut cfg) => {
                    // If we found a legacy account, use it
                    if let Some(acc) = legacy_account {
                        info!("Found legacy plain-text account number. Migrating to secure storage...");
                        cfg.account_number = Some(acc);
                        // Force a save to secure it and remove from file
                        if let Err(e) = save_config(&cfg) {
                            error!("Failed to migrate account number to secure storage: {}", e);
                        }
                    }
                    cfg
                },
                Err(e) => {
                    error!("Failed to parse config at {:?}: {}", path, e);
                    AppConfig::default()
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            AppConfig::default()
        }
        Err(e) => {
            error!("Failed to read config at {:?}: {}", path, e);
            AppConfig::default()
        }
    };

    // Try keyring for account and token
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

    config
}

pub fn save_config(config: &AppConfig) -> std::io::Result<()> {
    // 1. Save Account to Keyring
    if let Ok(entry) = get_account_entry() {
        if let Some(ref acc) = config.account_number {
            let _ = entry.set_password(acc);
        } else {
            let _ = entry.delete_password();
        }
    }

    // 2. Save Token to Keyring
    if let Ok(entry) = get_token_entry() {
        if let Some(ref token) = config.auth_token {
            let _ = entry.set_password(token);
        } else {
            let _ = entry.delete_password();
        }
    }

    // 3. Save to JSON
    let path = get_config_path();
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    fs::write(&path, json)?;
    Ok(())
}

pub fn save_settings(settings: SettingsState) -> std::io::Result<()> {
    let mut config = load_config();
    config.settings = Some(settings);
    save_config(&config)
}