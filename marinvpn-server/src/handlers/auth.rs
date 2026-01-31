use axum::{extract::State, Json};
use rand::Rng;
use crate::models::{Device};
use marinvpn_common::{
    LoginRequest, LoginResponse, GenerateResponse, RemoveDeviceRequest, 
    BlindTokenRequest, BlindTokenResponse, ErrorResponse
};
use crate::error::{AppResult, AppError};
use crate::AppState;
use std::sync::Arc;
use chrono::Utc;
use validator::Validate;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
    extract::FromRef,
};

pub struct AuthUser {
    pub account_number: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/blind-key",
    responses(
        (status = 200, description = "Public key for blind signatures", body = String)
    )
)]
pub async fn get_blind_public_key(
    State(state): State<Arc<AppState>>,
) -> String {
    state.signer.get_public_key_pem()
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
    Json(payload): Json<BlindTokenRequest>
) -> AppResult<Json<BlindTokenResponse>> {
    let account = state.db.get_account(&auth.account_number).await?
        .ok_or(AppError::Unauthorized)?;

    if account.expiry_date < Utc::now().timestamp() {
        return Err(AppError::AccountExpired);
    }

    let signed = state.signer.sign_blinded(&payload.blinded_message)?;
    
    tracing::info!("Issued blind token for account {}", auth.account_number);
    
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
        
        let auth_header = parts.headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::Unauthorized);
        }

        let token = &auth_header[7..];
        let claims = crate::services::auth::decode_token(token, &state.settings.auth.jwt_secret)?;

        Ok(AuthUser {
            account_number: claims.sub,
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
    let account_number = {
        let mut rng = rand::thread_rng();
        format!("{:04} {:04} {:04} {:04}", 
            rng.gen_range(0..10000), rng.gen_range(0..10000), 
            rng.gen_range(0..10000), rng.gen_range(0..10000)
        )
    };

    let account = state.db.create_account(&account_number, 30).await?;
    
    let name = {
        let mut rng = rand::thread_rng();
        let adjectives = ["cold", "warm", "fast", "brave", "silent", "gentle", "wild", "smart"];
        let nouns = ["chicken", "eagle", "tiger", "river", "mountain", "forest", "breeze", "storm"];
        format!("{} {}", adjectives[rng.gen_range(0..8)], nouns[rng.gen_range(0..8)])
    };
    
    state.db.add_device(&account.account_number, &name).await?;

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
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let account = state.db.get_account(&payload.account_number).await?
        .ok_or(AppError::AccountNotFound)?;

    let devices = state.db.get_devices(&account.account_number).await?;
    
    let device_name = if let Some(ref name) = payload.device_name {
        if !devices.iter().any(|d| d.name == *name) {
            state.db.add_device(&account.account_number, name).await?;
        }
        name.clone()
    } else {
        devices.first().map(|d| d.name.clone()).unwrap_or_else(|| "Default Device".to_string())
    };

    let token = crate::services::auth::create_token(&account.account_number, &state.settings.auth.jwt_secret)?;

    Ok(Json(LoginResponse {
        success: true,
        auth_token: Some(token),
        account_info: Some(account),
        current_device: Some(device_name),
        error: None,
    }))
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
    let account = state.db.get_account(&auth.account_number).await?
        .ok_or(AppError::AccountNotFound)?;
        
    if account.expiry_date < Utc::now().timestamp() {
        return Err(AppError::AccountExpired);
    }

    let devices = state.db.get_devices(&account.account_number).await?;
    let common_devices = devices.into_iter().map(|d| marinvpn_common::Device {
        name: d.name,
        added_at: d.added_at,
    }).collect();
    Ok(Json(common_devices))
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
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    if auth.account_number != payload.account_number {
        return Err(AppError::Unauthorized);
    }

    let account = state.db.get_account(&payload.account_number).await?
        .ok_or(AppError::AccountNotFound)?;

    let success = state.db.remove_device(&account.account_number, &payload.device_name).await?;
    Ok(Json(success))
}
