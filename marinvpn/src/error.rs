use reqwest::StatusCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("API error {status}: {message}")]
    Api {
        status: StatusCode,
        message: String,
    },

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Session expired")]
    SessionExpired,

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("VPN error: {0}")]
    Vpn(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl AppError {
    pub fn user_friendly_message(&self) -> String {
        match self {
            AppError::Network(_) => "Check your internet connection.".to_string(),
            AppError::Api { status, .. } => match *status {
                StatusCode::TOO_MANY_REQUESTS => "Too many requests. Please try again later.".to_string(),
                StatusCode::SERVICE_UNAVAILABLE => "Server is currently unavailable.".to_string(),
                _ => format!("Server error ({})", status),
            },
            AppError::Auth(msg) => format!("Login failed: {}", msg),
            AppError::SessionExpired => "Your session has expired. Please log in again.".to_string(),
            AppError::Vpn(msg) => format!("VPN Connection Error: {}", msg),
            _ => self.to_string(),
        }
    }
}
