use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerSettings {
    pub port: u16,
    pub host: String,
    pub log_level: String,
    pub max_body_bytes: usize,
    pub admin_token: String,
    pub metrics_allowlist: Vec<String>,
    pub trusted_proxy_hops: u8,
    pub trusted_proxy_cidrs: Vec<String>,
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
            .set_default("server.port", 3000)?
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.log_level", "info")?
            .set_default("server.max_body_bytes", 262_144)?
            .set_default("server.admin_token", "")?
            .set_default("server.metrics_allowlist", Vec::<String>::new())?
            .set_default("server.trusted_proxy_hops", 0)?
            .set_default("server.trusted_proxy_cidrs", Vec::<String>::new())?
            .set_default(
                "database.url",
                "postgres://marinvpn:marinvpn@127.0.0.1:5432/marinvpn",
            )?
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
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?;

        let settings: Settings = s.try_deserialize()?;
        validate_settings(&settings, &run_mode)?;
        Ok(settings)
    }
}

fn validate_settings(settings: &Settings, run_mode: &str) -> Result<(), ConfigError> {
    if !is_production(run_mode) {
        return Ok(());
    }

    if settings.server.host == "127.0.0.1" || settings.server.host == "localhost" {
        return Err(ConfigError::Message(
            "server.host must not be localhost in production".to_string(),
        ));
    }

    let mut bad = Vec::new();
    if is_default_or_weak(&settings.auth.jwt_secret) {
        bad.push("auth.jwt_secret");
    }
    if is_default_or_weak(&settings.auth.attestation_secret) {
        bad.push("auth.attestation_secret");
    }
    if is_default_or_weak(&settings.auth.account_salt) {
        bad.push("auth.account_salt");
    }
    if is_default_or_weak(&settings.auth.panic_key) {
        bad.push("auth.panic_key");
    }
    if settings.server.admin_token.trim().is_empty() {
        bad.push("server.admin_token");
    }
    if settings.server.trusted_proxy_hops > 0 && settings.server.trusted_proxy_cidrs.is_empty() {
        bad.push("server.trusted_proxy_cidrs");
    }

    if !bad.is_empty() {
        return Err(ConfigError::Message(format!(
            "production config invalid (missing/weak secrets): {}",
            bad.join(", ")
        )));
    }

    Ok(())
}

fn is_production(run_mode: &str) -> bool {
    matches!(run_mode.to_lowercase().as_str(), "production" | "prod")
}

fn is_default_or_weak(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() < 32 {
        return true;
    }
    matches!(
        trimmed,
        "replace-with-a-real-secret-in-production"
            | "marinvpn_secure_attestation_2026_top_tier"
            | "marinvpn_default_salt_2026"
            | "emergency_default_2026"
    )
}
