use axum::{
    extract::State,
    routing::{get, post},
    Router,
};
use axum_prometheus::PrometheusMetricLayer;
use std::sync::Arc;
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
    ErrorResponse, GenerateResponse, LoginRequest, LoginResponse, RemoveDeviceRequest,
    ReportRequest, VpnServer, WireGuardConfig,
};

pub struct AppState {
    pub db: services::db::Database,
    pub settings: config::Settings,
    pub vpn: services::vpn::VpnOrchestrator,
    pub signer: services::auth::BlindSigner,
    pub support_key: services::auth::SupportKey,
}

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

    axum::serve(listener, app)
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
    let secret = &state.settings.auth.attestation_secret;

    let path = req.uri().path();
    if path == "/health" || path.starts_with("/swagger-ui") || path.starts_with("/api-docs") {
        return Ok(next.run(req).await);
    }

    let attestation = req
        .headers()
        .get("X-Marin-Attestation")
        .and_then(|h| h.to_str().ok())
        .ok_or(error::AppError::Unauthorized)?;

    let parts: Vec<&str> = attestation.split(':').collect();
    if parts.len() != 3 {
        tracing::warn!(
            "Blocked request with invalid attestation format from {}",
            path
        );
        return Err(error::AppError::Unauthorized);
    }

    let timestamp_str = parts[0];
    let nonce = parts[1];
    let provided_sig = parts[2];

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

    if state.db.is_attestation_id_used(nonce).await? {
        tracing::warn!(
            "Blocked replayed client request (nonce: {}) to {}",
            nonce,
            path
        );
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

    use blake2::{Blake2s, Digest};
    let mut hasher = Blake2s::new();
    hasher.update(secret.as_bytes());
    hasher.update(timestamp_str.as_bytes());
    hasher.update(nonce.as_bytes());
    hasher.update(req.method().as_str().as_bytes());
    hasher.update(path.as_bytes());
    let expected_sig = hex::encode(hasher.finalize());

    if provided_sig != expected_sig {
        tracing::warn!(
            "Blocked unauthorized client request to {} (Signature mismatch)",
            path
        );
        return Err(error::AppError::Unauthorized);
    }

    Ok(next.run(req).await)
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
