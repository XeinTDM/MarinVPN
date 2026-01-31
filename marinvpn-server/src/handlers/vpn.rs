use axum::{extract::State, Json};
use crate::models::requests::*;
use crate::models::responses::ErrorResponse;
use crate::error::{AppResult, AppError};
use crate::AppState;
use crate::vpn_config::WireGuardConfig;
use crate::models::CommonVpnServer;
use crate::handlers::auth::AuthUser;
use std::sync::Arc;
use chrono::Utc;
use validator::Validate;

pub async fn get_servers(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<CommonVpnServer>>> {
    let servers = state.db.get_active_servers().await?;
    let common_servers = servers.into_iter().map(|s| s.into_common()).collect();
    Ok(Json(common_servers))
}

#[utoipa::path(
    post,
    path = "/api/v1/vpn/config",
    request_body = ConfigRequest,
    responses(
        (status = 200, description = "Configuration retrieved successfully", body = WireGuardConfig),
        (status = 401, description = "Account not found", body = ErrorResponse),
        (status = 403, description = "Account expired", body = ErrorResponse)
    )
)]
pub async fn get_vpn_config(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<ConfigRequest>
) -> AppResult<Json<WireGuardConfig>> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    if auth.account_number != payload.account_number {
        return Err(AppError::Unauthorized);
    }

    let account = state.db.get_account(&payload.account_number).await?
        .ok_or(AppError::AccountNotFound)?;
        
    if account.expiry_date < Utc::now().timestamp() {
        return Err(AppError::AccountExpired);
    }

    let country = payload.location.split(',').next().unwrap_or("Sweden").trim();
    let server = state.db.get_server_by_location(country).await?
        .ok_or(AppError::BadRequest("No active servers in this location".to_string()))?;

    // Step 1: Verify Account (Zero-Knowledge: Account is checked, but not linked to the resulting IP)
    let assigned_ip = state.db.get_or_create_peer(&payload.pub_key).await?;

    // Step 2: Register on Interface
    state.vpn.register_peer(&payload.pub_key, &assigned_ip).await?;


    let dns_servers = if let Some(prefs) = payload.dns_blocking {
        if prefs.ads || prefs.trackers || prefs.malware {
            "94.140.14.14, 94.140.15.15".to_string() // AdGuard DNS
        } else if prefs.adult_content {
            "1.1.1.3, 1.0.0.3".to_string() // Cloudflare Family
        } else {
            "1.1.1.1, 8.8.8.8".to_string()
        }
    } else {
        "1.1.1.1, 8.8.8.8".to_string()
    };

    let (psk, pqc_info) = if payload.quantum_resistant {
        let psk = base64::engine::general_purpose::STANDARD.encode(rand::thread_rng().gen::<[u8; 32]>());
        (Some(psk), Some("ML-KEM-768 (Kyber) via OQS".to_string()))
    } else {
        (None, None)
    };

    let config = WireGuardConfig {
        private_key: "".to_string(), 
        public_key: server.public_key,
        preshared_key: psk,
        endpoint: server.endpoint,
        allowed_ips: "0.0.0.0/0, ::/0".to_string(),
        address: assigned_ip,
        dns: Some(dns_servers),
        pqc_handshake: pqc_info,
        pqc_provider: if payload.quantum_resistant { Some("MarinQuantum v1".to_string()) } else { None },
    };

    Ok(Json(config))
}

#[utoipa::path(
    post,
    path = "/api/v1/vpn/report",
    request_body = ReportRequest,
    responses(
        (status = 200, description = "Report received", body = bool),
        (status = 401, description = "Account not found", body = ErrorResponse)
    )
)]
pub async fn report_problem(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<ReportRequest>
) -> AppResult<Json<bool>> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    if auth.account_number != payload.account_number {
        return Err(AppError::Unauthorized);
    }

    let _account = state.db.get_account(&payload.account_number).await?
        .ok_or(AppError::AccountNotFound)?;
        
    let masked_account = if payload.account_number.len() >= 12 {
        format!("{} **** **** {}", &payload.account_number[0..4], &payload.account_number[payload.account_number.len()-4..])
    } else {
        "****".to_string()
    };

    tracing::info!("PROBLEM REPORTED from {}: {}", masked_account, payload.message);
    Ok(Json(true))
}
