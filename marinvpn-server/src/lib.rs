use axum::{
    body::{to_bytes, Body},
    extract::State,
    routing::{get, post},
    Router,
};
use axum_prometheus::PrometheusMetricLayer;
use base64::Engine;
use once_cell::sync::Lazy;
use ring::signature::{UnparsedPublicKey, ED25519};
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod config;
pub mod error;
pub mod handlers;
pub mod models;
pub mod services;
pub mod vpn_config;

use marinvpn_common::{
    Account, AnonymousConfigRequest, BlindTokenRequest, BlindTokenResponse, ConfigRequest, Device,
    ErrorResponse, GenerateResponse, LoginRequest, LoginResponse, RefreshRequest, RefreshResponse,
    RemoveDeviceRequest, ReportRequest, VpnServer, WireGuardConfig,
};

pub struct AppState {
    pub db: services::db::Database,
    pub settings: config::Settings,
    pub vpn: services::vpn::VpnOrchestrator,
    pub signer: services::auth::BlindSigner,
    pub support_key: services::auth::SupportKey,
}

#[derive(Clone, Debug)]
struct AdminGuardConfig {
    admin_token: String,
    allowlist: Vec<String>,
    trusted_proxy_hops: u8,
    trusted_proxy_cidrs: Vec<String>,
}

static ADMIN_GUARD: Lazy<RwLock<AdminGuardConfig>> = Lazy::new(|| {
    RwLock::new(AdminGuardConfig {
        admin_token: String::new(),
        allowlist: Vec::new(),
        trusted_proxy_hops: 0,
        trusted_proxy_cidrs: Vec::new(),
    })
});

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::auth::generate_account,
        handlers::auth::login,
        handlers::auth::get_devices,
        handlers::auth::remove_device,
        handlers::auth::get_blind_public_key,
        handlers::auth::get_support_public_key,
        handlers::auth::issue_blind_token,
        handlers::auth::refresh_token,
        handlers::vpn::get_vpn_config,
        handlers::vpn::get_anonymous_config,
        handlers::vpn::report_problem,
        handlers::vpn::get_canary,
    ),
    components(
        schemas(
            Account,
            Device,
            VpnServer,
            LoginRequest,
            ConfigRequest,
            AnonymousConfigRequest,
            BlindTokenRequest,
            RemoveDeviceRequest,
            ReportRequest,
            LoginResponse,
            GenerateResponse,
            BlindTokenResponse,
            RefreshRequest,
            RefreshResponse,
            ErrorResponse,
            WireGuardConfig,
        )
    ),
    tags(
        (name = "MarinVPN", description = "Authentication and Configuration API")
    )
)]
struct ApiDoc;

pub async fn run() {
    dotenvy::dotenv().ok();

    let settings = config::Settings::new().expect("Failed to load configuration");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "marinvpn_server={},tower_http=info",
                    settings.server.log_level
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = services::db::Database::new(&settings.database.url, &settings.auth.account_salt)
        .await
        .expect("Failed to initialize database");
    let vpn_iface = std::env::var("WG_INTERFACE").unwrap_or_else(|_| "marinvpn0".to_string());
    let vpn_orchestrator = services::vpn::VpnOrchestrator::new(vpn_iface);
    let signer = services::auth::BlindSigner::new();
    let support_key = services::auth::SupportKey::new();

    let state = Arc::new(AppState {
        db,
        settings: settings.clone(),
        vpn: vpn_orchestrator,
        signer,
        support_key,
    });

    {
        let mut guard = ADMIN_GUARD.write().expect("admin guard lock poisoned");
        guard.admin_token = settings.server.admin_token.clone();
        guard.allowlist = settings.server.metrics_allowlist.clone();
        guard.trusted_proxy_hops = settings.server.trusted_proxy_hops;
        guard.trusted_proxy_cidrs = settings.server.trusted_proxy_cidrs.clone();
    }

    #[cfg(unix)]
    {
        tokio::spawn(async move {
            use tokio::signal::unix::{signal, SignalKind};
            if let Ok(mut hup) = signal(SignalKind::hangup()) {
                loop {
                    hup.recv().await;
                    if let Some(new_cfg) = reload_admin_guard_from_env() {
                        let mut guard = ADMIN_GUARD.write().expect("admin guard lock poisoned");
                        *guard = new_cfg;
                        tracing::info!("Reloaded admin guard configuration from env");
                    }
                }
            }
        });
    }

    let cleanup_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            tracing::info!("Starting periodic cleanup of stale VPN sessions...");
            match cleanup_state.db.cleanup_stale_sessions(86400).await {
                Ok(stale_keys) => {
                    for key in stale_keys {
                        let _ = cleanup_state.vpn.remove_peer(&key).await;
                    }
                }
                Err(e) => tracing::error!("Failed to cleanup stale sessions: {}", e),
            }
        }
    });

    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(6)
            .burst_size(10)
            .finish()
            .unwrap(),
    );

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/health", get(health_check))
        .route(
            "/metrics",
            get(move || {
                let handle = metric_handle.clone();
                async move { handle.render() }
            }),
        )
        .nest("/api/v1", api_routes())
        .layer(prometheus_layer)
        .layer(GovernorLayer {
            config: governor_config,
        })
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::http::Request<_>| {
                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri().path(),
                    )
                })
                .on_request(|_request: &axum::http::Request<_>, _span: &tracing::Span| {
                    // Minimal logging on request
                })
                .on_response(
                    |response: &axum::http::Response<_>,
                     latency: Duration,
                     _span: &tracing::Span| {
                        tracing::info!(
                            status = %response.status(),
                            latency = ?latency,
                            "finished processing request"
                        )
                    },
                ),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            verify_client_attestation,
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .with_state(state);

    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!(
        "MarinVPN High-Performance Server listening on http://{}",
        addr
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/account/generate", post(handlers::auth::generate_account))
        .route("/account/login", post(handlers::auth::login))
        .route("/account/devices", post(handlers::auth::get_devices))
        .route(
            "/account/devices/remove",
            post(handlers::auth::remove_device),
        )
        .route("/auth/blind-key", get(handlers::auth::get_blind_public_key))
        .route(
            "/auth/support-key",
            get(handlers::auth::get_support_public_key),
        )
        .route("/auth/issue-token", post(handlers::auth::issue_blind_token))
        .route("/auth/refresh", post(handlers::auth::refresh_token))
        .route("/vpn/servers", get(handlers::vpn::get_servers))
        .route("/vpn/config", post(handlers::vpn::get_vpn_config))
        .route(
            "/vpn/config-anonymous",
            post(handlers::vpn::get_anonymous_config),
        )
        .route("/vpn/report", post(handlers::vpn::report_problem))
        .route("/vpn/panic", post(handlers::vpn::trigger_panic))
        .route("/canary", get(handlers::vpn::get_canary))
}

async fn health_check() -> &'static str {
    "OK"
}

async fn verify_client_attestation(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, error::AppError> {
    let (req_parts, body) = req.into_parts();
    let path = req_parts.uri.path();
    if path == "/health" {
        let req = axum::extract::Request::from_parts(req_parts, body);
        return Ok(next.run(req).await);
    }

    if path == "/metrics" || path.starts_with("/swagger-ui") || path.starts_with("/api-docs") {
        let (admin_token, allowlist, trusted_proxy_hops, trusted_proxy_cidrs) = {
            let guard = ADMIN_GUARD.read().expect("admin guard lock poisoned");
            (
                guard.admin_token.clone(),
                guard.allowlist.clone(),
                guard.trusted_proxy_hops,
                guard.trusted_proxy_cidrs.clone(),
            )
        };

        if !allowlist.is_empty()
            && !is_metrics_ip_allowed(
                &req_parts,
                &allowlist,
                trusted_proxy_hops,
                &trusted_proxy_cidrs,
            )
        {
            return Err(error::AppError::Unauthorized);
        }

        let provided = req_parts
            .headers
            .get("X-Admin-Token")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .or_else(|| {
                req_parts
                    .headers
                    .get(axum::http::header::AUTHORIZATION)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|v| v.strip_prefix("Bearer "))
                    .map(|s| s.to_string())
            })
            .ok_or(error::AppError::Unauthorized)?;

        use subtle::ConstantTimeEq;
        if admin_token
            .as_bytes()
            .ct_eq(provided.as_bytes())
            .unwrap_u8()
            == 0
        {
            return Err(error::AppError::Unauthorized);
        }

        let req = axum::extract::Request::from_parts(req_parts, body);
        return Ok(next.run(req).await);
    }

    let body_bytes = to_bytes(body, state.settings.server.max_body_bytes)
        .await
        .map_err(|_| error::AppError::Unauthorized)?;
    let body_hash = {
        use sha2::Digest;
        hex::encode(sha2::Sha256::digest(&body_bytes))
    };

    let attestation = req_parts
        .headers
        .get("X-Marin-Attestation")
        .and_then(|h| h.to_str().ok())
        .ok_or(error::AppError::Unauthorized)?;
    let provided_body_hash = req_parts
        .headers
        .get("X-Marin-Attestation-Body")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let provided_pubkey = req_parts
        .headers
        .get("X-Marin-Attestation-Pub")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let att_parts: Vec<&str> = attestation.split(':').collect();
    if att_parts.len() != 3 {
        tracing::warn!(
            "Blocked request with invalid attestation format from {}",
            path
        );
        return Err(error::AppError::Unauthorized);
    }

    let timestamp_str = att_parts[0];
    let nonce = att_parts[1];
    let provided_sig = att_parts[2];

    let timestamp = timestamp_str
        .parse::<i64>()
        .map_err(|_| error::AppError::Unauthorized)?;
    let now = chrono::Utc::now().timestamp();

    if (now - timestamp).abs() > 60 {
        tracing::warn!(
            "Blocked request with expired attestation (diff: {}s) to {}",
            now - timestamp,
            path
        );
        return Err(error::AppError::Unauthorized);
    }

    if is_production() && provided_body_hash.is_none() {
        tracing::warn!("Blocked request missing attestation body hash to {}", path);
        return Err(error::AppError::Unauthorized);
    }

    if let Some(ref provided) = provided_body_hash {
        if provided != &body_hash {
            tracing::warn!("Blocked request with body hash mismatch to {}", path);
            return Err(error::AppError::Unauthorized);
        }
    }

    if state.db.is_attestation_id_used(nonce).await? {
        tracing::warn!(
            "Blocked replayed client request (nonce: {}) to {}",
            nonce,
            path
        );
        return Err(error::AppError::Unauthorized);
    }

    let mut device_pubkey = None;
    if let Some(auth_header) = req_parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            if let Ok(claims) =
                crate::services::auth::decode_access_token(token, &state.settings.auth.jwt_secret)
            {
                device_pubkey = state
                    .db
                    .get_device_pubkey(&claims.sub, &claims.device)
                    .await?;
                if is_production() && device_pubkey.is_none() {
                    tracing::warn!(
                        "Blocked request with no device pubkey on file for {}",
                        claims.sub
                    );
                    return Err(error::AppError::Unauthorized);
                }
                if let (Some(ref stored), Some(ref provided)) = (&device_pubkey, &provided_pubkey) {
                    if stored != provided {
                        tracing::warn!(
                            "Blocked request with mismatched device pubkey for {}",
                            claims.sub
                        );
                        return Err(error::AppError::Unauthorized);
                    }
                }
            }
        }
    }

    if device_pubkey.is_none() {
        device_pubkey = provided_pubkey.clone();
    }

    if is_production() && device_pubkey.is_none() {
        tracing::warn!("Blocked request missing device pubkey to {}", path);
        return Err(error::AppError::Unauthorized);
    }

    if let Some(ref pubkey_b64) = device_pubkey {
        let pubkey_bytes = base64::engine::general_purpose::STANDARD
            .decode(pubkey_b64)
            .map_err(|_| error::AppError::Unauthorized)?;
        let sig_bytes = base64::engine::general_purpose::STANDARD
            .decode(provided_sig)
            .map_err(|_| error::AppError::Unauthorized)?;

        let message = format!(
            "{}:{}:{}:{}:{}",
            timestamp_str,
            nonce,
            req_parts.method.as_str(),
            path,
            body_hash
        );

        let verifier = UnparsedPublicKey::new(&ED25519, pubkey_bytes);
        if verifier.verify(message.as_bytes(), &sig_bytes).is_err() {
            tracing::warn!(
                "Blocked unauthorized client request to {} (Signature mismatch)",
                path
            );
            return Err(error::AppError::Unauthorized);
        }
    } else {
        tracing::warn!("Blocked request missing device pubkey to {}", path);
        return Err(error::AppError::Unauthorized);
    }

    if let Err(e) = state.db.mark_attestation_id_used(nonce).await {
        if let error::AppError::Database(sqlx::Error::Database(db_err)) = &e {
            if db_err.is_unique_violation() {
                return Err(error::AppError::Unauthorized);
            }
        }
        return Err(e);
    }

    let req = axum::extract::Request::from_parts(req_parts, Body::from(body_bytes));
    Ok(next.run(req).await)
}

fn is_metrics_ip_allowed(
    req_parts: &axum::http::request::Parts,
    allowlist: &[String],
    trusted_proxy_hops: u8,
    trusted_proxy_cidrs: &[String],
) -> bool {
    let ip = match extract_client_ip(req_parts, trusted_proxy_hops, trusted_proxy_cidrs) {
        Some(addr) => addr,
        None => return false,
    };

    allowlist
        .iter()
        .any(|allowed| match allowed.parse::<IpAddr>() {
            Ok(addr) => addr == ip,
            Err(_) => match allowed.parse::<ipnet::IpNet>() {
                Ok(net) => net.contains(&ip),
                Err(_) => false,
            },
        })
}

fn extract_client_ip(
    req_parts: &axum::http::request::Parts,
    trusted_proxy_hops: u8,
    trusted_proxy_cidrs: &[String],
) -> Option<IpAddr> {
    let peer_ip = req_parts
        .extensions
        .get::<axum::extract::connect_info::ConnectInfo<std::net::SocketAddr>>()
        .map(|c| c.0.ip());

    if trusted_proxy_hops > 0
        && peer_ip.is_some()
        && is_ip_in_cidrs(peer_ip.unwrap(), trusted_proxy_cidrs)
    {
        if let Some(forwarded) = req_parts
            .headers
            .get("X-Forwarded-For")
            .and_then(|h| h.to_str().ok())
        {
            let parts: Vec<&str> = forwarded.split(',').map(|s| s.trim()).collect();
            if !parts.is_empty() {
                let idx = parts
                    .len()
                    .saturating_sub(trusted_proxy_hops as usize)
                    .saturating_sub(1);
                if let Some(ip_str) = parts.get(idx).or_else(|| parts.first()) {
                    if let Ok(ip) = ip_str.parse::<IpAddr>() {
                        return Some(ip);
                    }
                }
            }
        }
    }

    peer_ip
}

fn is_ip_in_cidrs(ip: IpAddr, cidrs: &[String]) -> bool {
    if cidrs.is_empty() {
        return false;
    }
    cidrs.iter().any(|allowed| match allowed.parse::<IpAddr>() {
        Ok(addr) => addr == ip,
        Err(_) => match allowed.parse::<ipnet::IpNet>() {
            Ok(net) => net.contains(&ip),
            Err(_) => false,
        },
    })
}

#[cfg(unix)]
fn reload_admin_guard_from_env() -> Option<AdminGuardConfig> {
    let token = std::env::var("APP__SERVER__ADMIN_TOKEN").ok()?;
    let allowlist = std::env::var("APP__SERVER__METRICS_ALLOWLIST")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let trusted_proxy_hops = std::env::var("APP__SERVER__TRUSTED_PROXY_HOPS")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(0);
    let trusted_proxy_cidrs = std::env::var("APP__SERVER__TRUSTED_PROXY_CIDRS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    Some(AdminGuardConfig {
        admin_token: token,
        allowlist,
        trusted_proxy_hops,
        trusted_proxy_cidrs,
    })
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutting down gracefully...");
}

fn is_production() -> bool {
    let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".to_string());
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "".to_string());
    matches!(run_mode.to_lowercase().as_str(), "production" | "prod")
        || matches!(app_env.to_lowercase().as_str(), "production" | "prod")
}
