use anyhow;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Account not found")]
    AccountNotFound,

    #[error("Account expired")]
    AccountExpired,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Device already exists")]
    DeviceConflict,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::Internal(e) => {
                tracing::error!("Internal error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database operation failed".to_string(),
                )
            }
            AppError::Migration(e) => {
                tracing::error!("Migration error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Migration failed".to_string(),
                )
            }
            AppError::AccountNotFound => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::AccountExpired => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::DeviceConflict => (StatusCode::CONFLICT, self.to_string()),
        };

        let body = Json(json!({
            "error": error_message,
            "success": false,
        }));

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
