use axum::{extract::State, Json};
use marinvpn_common::{ConfigRequest, AnonymousConfigRequest, ErrorResponse, WireGuardConfig, ReportRequest};
use crate::error::{AppResult, AppError};
use crate::AppState;
use crate::models::CommonVpnServer;
use crate::handlers::auth::AuthUser;
use std::sync::Arc;
use chrono::Utc;
use validator::Validate;
use base64::Engine;
use rand::Rng;
use ml_kem::kem::Encapsulate;
use ml_kem::{MlKem768Params, EncodedSizeUser};

fn encapsulate_pqc(pk_base64: &str) -> Option<(String, String)> {
    let pk_bytes = base64::engine::general_purpose::STANDARD.decode(pk_base64).ok()?;
    let pk = <ml_kem::kem::EncapsulationKey<MlKem768Params> as EncodedSizeUser>::from_bytes(pk_bytes.as_slice().try_into().ok()?);
    
    let mut rng = rand::thread_rng();
    let (ct, ss) = pk.encapsulate(&mut rng).ok()?;
    
    let ss_bytes: &[u8] = ss.as_slice();
    let ct_bytes: &[u8] = ct.as_slice();

    Some((
        base64::engine::general_purpose::STANDARD.encode(ss_bytes),
        base64::engine::general_purpose::STANDARD.encode(ct_bytes)
    ))
}

pub async fn get_servers(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<CommonVpnServer>>> {
    let servers = state.db.get_active_servers().await?;
    let common_servers = servers.into_iter().map(|s| s.into_common()).collect();
    Ok(Json(common_servers))
}

#[utoipa::path(
    post,
    path = "/api/v1/vpn/config-anonymous",
    request_body = AnonymousConfigRequest,
    responses(
        (status = 200, description = "Configuration retrieved successfully", body = WireGuardConfig),
        (status = 401, description = "Invalid token or signature", body = ErrorResponse)
    )
)]
pub async fn get_anonymous_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AnonymousConfigRequest>
) -> AppResult<Json<WireGuardConfig>> {
    payload.validate().map_err(|e: validator::ValidationErrors| AppError::BadRequest(e.to_string()))?;

    if !state.signer.verify(&payload.message, &payload.signature) {
        return Err(AppError::Unauthorized);
    }

    if state.db.is_token_used(&payload.message).await? {
        return Err(AppError::BadRequest("Token already used".to_string()));
    }

    state.db.mark_token_used(&payload.message).await?;

    let country = payload.location.split(',').next().unwrap_or("Sweden").trim();
    let servers = state.db.get_servers_by_location(country).await?;
    let server = servers.into_iter()
        .min_by(|a, b| a.health_score().partial_cmp(&b.health_score()).unwrap())
        .ok_or(AppError::BadRequest("No active servers in this location".to_string()))?;

    let assigned_ip = state.db.get_or_create_peer(&payload.pub_key).await?;
    state.vpn.register_peer(&payload.pub_key, &assigned_ip).await?;

    let (psk, pqc_info, pqc_ct) = if payload.quantum_resistant {
        if let Some(ref pk_b64) = payload.pqc_public_key {
            if let Some((ss_b64, ct_b64)) = encapsulate_pqc(pk_b64) {
                (Some(ss_b64), Some("ML-KEM-768 Hybrid".to_string()), Some(ct_b64))
            } else {
                (None, Some("PQC Error".to_string()), None)
            }
        } else {
            let psk = base64::engine::general_purpose::STANDARD.encode(rand::thread_rng().gen::<[u8; 32]>());
            (Some(psk), Some("ML-KEM-768 (Fallback to random PSK)".to_string()), None)
        }
    } else {
        (None, None, None)
    };

    let obfuscation_key = base64::engine::general_purpose::STANDARD.encode(rand::thread_rng().gen::<[u8; 32]>());

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
        pqc_ciphertext: pqc_ct,
        obfuscation_key: Some(obfuscation_key),
    };

    Ok(Json(config))
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
    let servers = state.db.get_servers_by_location(country).await?;
    let server = servers.into_iter()
        .min_by(|a, b| a.health_score().partial_cmp(&b.health_score()).unwrap())
        .ok_or(AppError::BadRequest("No active servers in this location".to_string()))?;

    let assigned_ip = state.db.get_or_create_peer(&payload.pub_key).await?;
    state.vpn.register_peer(&payload.pub_key, &assigned_ip).await?;


    let dns_servers = if let Some(ref prefs) = payload.dns_blocking {
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

    let (psk, pqc_info, pqc_ct) = if payload.quantum_resistant {
        if let Some(ref pk_b64) = payload.pqc_public_key {
            if let Some((ss_b64, ct_b64)) = encapsulate_pqc(pk_b64) {
                (Some(ss_b64), Some("ML-KEM-768 Hybrid".to_string()), Some(ct_b64))
            } else {
                (None, Some("PQC Error".to_string()), None)
            }
        } else {
            let psk = base64::engine::general_purpose::STANDARD.encode(rand::thread_rng().gen::<[u8; 32]>());
            (Some(psk), Some("ML-KEM-768 (Fallback to random PSK)".to_string()), None)
        }
    } else {
        (None, None, None)
    };

    let obfuscation_key = base64::engine::general_purpose::STANDARD.encode(rand::thread_rng().gen::<[u8; 32]>());

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
        pqc_ciphertext: pqc_ct,
        obfuscation_key: Some(obfuscation_key),
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
    payload.validate().map_err(|e: validator::ValidationErrors| AppError::BadRequest(e.to_string()))?;

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

    let cleaned_message = if payload.message.len() > 500 {
        format!("{}... [TRUNCATED]", &payload.message[0..500])
    } else {
        payload.message.clone()
    };

    tracing::info!("PROBLEM REPORTED from {}: (Message length: {} bytes)", masked_account, cleaned_message.len());
    // TODO: the message would be encrypted for the support team
    // or sent to a secure processing queue, not logged to stdout.
    Ok(Json(true))
}

pub async fn trigger_panic(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> AppResult<Json<bool>> {
    let panic_key = &state.settings.auth.panic_key;
    let provided_key = headers.get("X-Panic-Key")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    if provided_key != panic_key {
        return Err(AppError::Unauthorized);
    }

    state.db.panic_wipe().await?;
    state.vpn.remove_all_peers().await?;
    
    tracing::error!("EMERGENCY PANIC WIPE COMPLETED. All ephemeral session data and peers removed.");
    Ok(Json(true))
}