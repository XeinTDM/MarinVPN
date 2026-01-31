use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerSettings {
    pub port: u16,
    pub host: String,
    pub log_level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSettings {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthSettings {
    pub jwt_secret: String,
    pub attestation_secret: String,
    pub account_salt: String,
    pub panic_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub server: ServerSettings,
    pub database: DatabaseSettings,
    pub auth: AuthSettings,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            // Start with default settings
            .set_default("server.port", 3000)?
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.log_level", "info")?
            .set_default("database.url", "sqlite:marinvpn.db")?
            .set_default(
                "auth.jwt_secret",
                "replace-with-a-real-secret-in-production",
            )?
            .set_default(
                "auth.attestation_secret",
                "marinvpn_secure_attestation_2026_top_tier",
            )?
            .set_default("auth.account_salt", "marinvpn_default_salt_2026")?
            .set_default("auth.panic_key", "emergency_default_2026")?
            // Load from file
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            // Load from environment (e.g. APP_SERVER__PORT=8080)
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?;

        s.try_deserialize()
    }
}
