use crate::error::{AppError, AppResult};
use crate::models::Device;
use crate::AppState;
use axum::{
    async_trait, extract::FromRef, extract::FromRequestParts, http::request::Parts, http::HeaderMap,
};
use axum::{extract::State, Json};
use base64::Engine;
use chrono::Utc;
use marinvpn_common::{
    BlindTokenRequest, BlindTokenResponse, ErrorResponse, GenerateResponse, LoginRequest,
    LoginResponse, RefreshRequest, RefreshResponse, RemoveDeviceRequest,
};
use rand::Rng;
use std::sync::Arc;
use validator::Validate;

pub struct AuthUser {
    pub account_number: String,
    pub device_name: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/blind-key",
    responses(
        (status = 200, description = "Public key for blind signatures", body = String)
    )
)]
pub async fn get_blind_public_key(State(state): State<Arc<AppState>>) -> String {
    state.signer.get_public_key_pem()
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/support-key",
    responses(
        (status = 200, description = "Public key for encrypting support messages", body = String)
    )
)]
pub async fn get_support_public_key(State(state): State<Arc<AppState>>) -> String {
    state.support_key.get_public_key_pem()
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/issue-token",
    request_body = BlindTokenRequest,
    responses(
        (status = 200, description = "Blinded token signed successfully", body = BlindTokenResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn issue_blind_token(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<BlindTokenRequest>,
) -> AppResult<Json<BlindTokenResponse>> {
    let account = state
        .db
        .get_account(&auth.account_number)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if account.expiry_date < Utc::now().timestamp() {
        return Err(AppError::AccountExpired);
    }

    let signed = state.signer.sign_blinded(&payload.blinded_message)?;

    let masked = if auth.account_number.len() >= 4 {
        format!("{}****", &auth.account_number[0..4])
    } else {
        "****".to_string()
    };
    tracing::info!("Issued blind token for account {}", masked);

    Ok(Json(BlindTokenResponse {
        signed_blinded_message: signed,
    }))
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    Arc<AppState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = Arc::<AppState>::from_ref(state);

        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::Unauthorized);
        }

        let token = &auth_header[7..];
        let claims =
            crate::services::auth::decode_access_token(token, &state.settings.auth.jwt_secret)?;

        Ok(AuthUser {
            account_number: claims.sub,
            device_name: claims.device,
        })
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/account/generate",
    responses(
        (status = 200, description = "Account generated successfully", body = GenerateResponse)
    )
)]
pub async fn generate_account(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<GenerateResponse>> {
    let mut attempts = 0;
    let account = loop {
        let account_number = generate_account_number();

        match state.db.create_account(&account_number, 30).await {
            Ok(acc) => break acc,
            Err(AppError::Database(sqlx::Error::Database(db_err)))
                if db_err.is_unique_violation() && attempts < 10 =>
            {
                attempts += 1;
                continue;
            }
            Err(e) => return Err(e),
        }
    };

    let account_number = account.account_number.clone();

    let name = generate_device_name();

    state.db.add_device(&account_number, &name, None).await?;

    Ok(Json(GenerateResponse { account_number }))
}

#[utoipa::path(
    post,
    path = "/api/v1/account/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Account not found", body = ErrorResponse),
        (status = 400, description = "Invalid request or device limit reached", body = ErrorResponse)
    )
)]
pub async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    payload
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    if is_production() && payload.device_pubkey.is_none() {
        return Err(AppError::BadRequest(
            "device_pubkey required in production".to_string(),
        ));
    }

    if let Some(ref device_pubkey) = payload.device_pubkey {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(device_pubkey)
            .map_err(|_| AppError::BadRequest("invalid device_pubkey".to_string()))?;
        if decoded.len() != 32 {
            return Err(AppError::BadRequest("invalid device_pubkey".to_string()));
        }
        let provided_pubkey = headers
            .get("X-Marin-Attestation-Pub")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .ok_or(AppError::Unauthorized)?;
        if provided_pubkey != *device_pubkey {
            return Err(AppError::Unauthorized);
        }
    }

    let account = state
        .db
        .get_account(&payload.account_number)
        .await?
        .ok_or(AppError::AccountNotFound)?;

    if account.expiry_date < Utc::now().timestamp() {
        return Err(AppError::AccountExpired);
    }

    let devices = state.db.get_devices(&account.account_number).await?;

    let existing_device = if let Some(ref pubkey) = payload.device_pubkey {
        state
            .db
            .get_device_by_pubkey(&account.account_number, pubkey)
            .await?
    } else {
        None
    };

    let device_name = if let Some(existing) = existing_device {
        existing.name
    } else if let Some(pubkey) = payload.device_pubkey.as_deref() {
        if let Some(placeholder) = devices.iter().find(|d| d.attestation_pubkey.is_none()) {
            let updated = state
                .db
                .update_device_pubkey(&account.account_number, &placeholder.name, pubkey)
                .await?;
            if updated {
                placeholder.name.clone()
            } else {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "Failed to claim placeholder device"
                )));
            }
        } else if devices.len() >= 5 {
            if let Some(ref kick) = payload.kick_device {
                let removed = state
                    .db
                    .remove_device(&account.account_number, kick)
                    .await?;
                if !removed {
                    let common_devices = devices
                        .into_iter()
                        .map(|d| marinvpn_common::Device {
                            name: d.name,
                            created_date: format_utc_date(d.added_at),
                        })
                        .collect();
                    return Ok(Json(LoginResponse {
                        success: false,
                        auth_token: None,
                        refresh_token: None,
                        account_info: None,
                        current_device: None,
                        devices: Some(common_devices),
                        error_code: Some("DEVICE_NOT_FOUND".to_string()),
                        error: Some("Device not found".to_string()),
                    }));
                }

                let name = generate_device_name();
                state
                    .db
                    .add_device(&account.account_number, &name, Some(pubkey))
                    .await?;
                name
            } else {
                let common_devices = devices
                    .into_iter()
                    .map(|d| marinvpn_common::Device {
                        name: d.name,
                        created_date: format_utc_date(d.added_at),
                    })
                    .collect();
                return Ok(Json(LoginResponse {
                    success: false,
                    auth_token: None,
                    refresh_token: None,
                    account_info: None,
                    current_device: None,
                    devices: Some(common_devices),
                    error_code: Some("DEVICE_LIMIT".to_string()),
                    error: Some(
                        "Device limit reached (max 5). Remove a device to continue.".to_string(),
                    ),
                }));
            }
        } else {
            let name = generate_device_name();
            state
                .db
                .add_device(&account.account_number, &name, Some(pubkey))
                .await?;
            name
        }
    } else {
        let name = generate_device_name();
        state
            .db
            .add_device(&account.account_number, &name, None)
            .await?;
        name
    };

    let token = crate::services::auth::create_token(
        &account.account_number,
        &device_name,
        &state.settings.auth.jwt_secret,
    )?;
    let (refresh_token, refresh_exp) = crate::services::auth::create_refresh_token(
        &account.account_number,
        &device_name,
        &state.settings.auth.jwt_secret,
    )?;
    state
        .db
        .upsert_refresh_token(
            &account.account_number,
            &device_name,
            &refresh_token,
            refresh_exp,
        )
        .await?;

    Ok(Json(LoginResponse {
        success: true,
        auth_token: Some(token),
        refresh_token: Some(refresh_token),
        account_info: Some(account),
        current_device: Some(device_name),
        devices: None,
        error_code: None,
        error: None,
    }))
}

fn generate_device_name() -> String {
    let mut rng = rand::thread_rng();
    let adjectives = [
        "cold", "warm", "fast", "brave", "silent", "gentle", "wild", "smart",
    ];
    let nouns = [
        "chicken", "eagle", "tiger", "river", "mountain", "forest", "breeze", "storm",
    ];
    format!(
        "{} {}",
        adjectives[rng.gen_range(0..8)],
        nouns[rng.gen_range(0..8)]
    )
}

fn generate_account_number() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    let mut raw = String::with_capacity(16);
    for _ in 0..16 {
        let idx = rng.gen_range(0..ALPHABET.len());
        raw.push(ALPHABET[idx] as char);
    }
    format!(
        "{} {} {} {}",
        &raw[0..4],
        &raw[4..8],
        &raw[8..12],
        &raw[12..16]
    )
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = RefreshResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    )
)]
pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<RefreshRequest>,
) -> AppResult<Json<RefreshResponse>> {
    payload
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let claims = crate::services::auth::decode_refresh_token(
        &payload.refresh_token,
        &state.settings.auth.jwt_secret,
    )?;

    let provided_pubkey = headers
        .get("X-Marin-Attestation-Pub")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .ok_or(AppError::Unauthorized)?;

    let stored_pubkey = state
        .db
        .get_device_pubkey(&claims.sub, &claims.device)
        .await?;
    if stored_pubkey.as_deref() != Some(provided_pubkey.as_str()) {
        return Err(AppError::Unauthorized);
    }

    let new_access = crate::services::auth::create_token(
        &claims.sub,
        &claims.device,
        &state.settings.auth.jwt_secret,
    )?;
    let (new_refresh, refresh_exp) = crate::services::auth::create_refresh_token(
        &claims.sub,
        &claims.device,
        &state.settings.auth.jwt_secret,
    )?;

    let success = state
        .db
        .rotate_refresh_token(
            &claims.sub,
            &claims.device,
            &payload.refresh_token,
            &new_refresh,
            refresh_exp,
        )
        .await?;

    if !success {
        tracing::warn!(
            "Token rotation failed for {}: invalid old token or expired",
            claims.sub
        );
        return Err(AppError::Unauthorized);
    }

    Ok(Json(RefreshResponse {
        auth_token: new_access,
        refresh_token: new_refresh,
    }))
}

fn is_production() -> bool {
    let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".to_string());
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "".to_string());
    matches!(run_mode.to_lowercase().as_str(), "production" | "prod")
        || matches!(app_env.to_lowercase().as_str(), "production" | "prod")
}

#[utoipa::path(
    post,
    path = "/api/v1/account/devices",
    responses(
        (status = 200, description = "Devices retrieved successfully", body = Vec<Device>),
        (status = 401, description = "Account not found", body = ErrorResponse),
        (status = 403, description = "Account expired", body = ErrorResponse)
    )
)]
pub async fn get_devices(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> AppResult<Json<Vec<marinvpn_common::Device>>> {
    let account = state
        .db
        .get_account(&auth.account_number)
        .await?
        .ok_or(AppError::AccountNotFound)?;

    if account.expiry_date < Utc::now().timestamp() {
        return Err(AppError::AccountExpired);
    }

    let devices = state.db.get_devices(&account.account_number).await?;
    let common_devices = devices
        .into_iter()
        .map(|d| marinvpn_common::Device {
            name: d.name,
            created_date: format_utc_date(d.added_at),
        })
        .collect();
    Ok(Json(common_devices))
}

fn format_utc_date(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .unwrap_or_else(chrono::Utc::now)
        .format("%Y-%m-%d")
        .to_string()
}

#[utoipa::path(
    post,
    path = "/api/v1/account/devices/remove",
    request_body = RemoveDeviceRequest,
    responses(
        (status = 200, description = "Device removed successfully", body = bool),
        (status = 401, description = "Account not found", body = ErrorResponse)
    )
)]
pub async fn remove_device(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<RemoveDeviceRequest>,
) -> AppResult<Json<bool>> {
    payload
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    if auth.account_number != payload.account_number {
        return Err(AppError::Unauthorized);
    }

    let account = state
        .db
        .get_account(&payload.account_number)
        .await?
        .ok_or(AppError::AccountNotFound)?;

    let success = state
        .db
        .remove_device(&account.account_number, &payload.device_name)
        .await?;
    Ok(Json(success))
}
