use std::fs;
use std::path::PathBuf;
use std::collections::HashSet;
use serde::{Serialize, Deserialize};
use crate::models::SettingsState;
use directories::ProjectDirs;

const CONFIG_FILENAME: &str = "config.json";

#[derive(Serialize, Deserialize, Default, PartialEq, Clone)]
pub struct AppConfig {
    pub settings: Option<SettingsState>,
    pub favorites: Option<HashSet<String>>,
    pub account_number: Option<String>,
    pub device_name: Option<String>,
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

pub fn load_config() -> AppConfig {
    let path = get_config_path();
    match fs::read_to_string(&path) {
        Ok(contents) => {
            match serde_json::from_str::<AppConfig>(&contents) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Failed to parse config at {:?}: {}", path, e);
                    AppConfig::default()
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            AppConfig::default()
        }
        Err(e) => {
            eprintln!("Failed to read config at {:?}: {}", path, e);
            AppConfig::default()
        }
    }
}

pub fn save_config(config: &AppConfig) -> std::io::Result<()> {
    let path = get_config_path();
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    fs::write(&path, json)?;
    Ok(())
}
