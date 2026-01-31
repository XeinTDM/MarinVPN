use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    compression::CompressionLayer,
    timeout::TimeoutLayer,
};
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use axum_prometheus::PrometheusMetricLayer;

pub mod error;
pub mod handlers;
pub mod models;
pub mod services;
pub mod vpn_config;
pub mod config;

pub struct AppState {
    pub db: services::db::Database,
    pub settings: config::Settings,
    pub vpn: services::vpn::VpnOrchestrator,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::auth::generate_account,
        handlers::auth::login,
        handlers::auth::get_devices,
        handlers::auth::remove_device,
        handlers::vpn::get_vpn_config,
        handlers::vpn::report_problem,
    ),
    components(
        schemas(
            models::Account,
            models::Device,
            models::VpnServer,
            models::requests::LoginRequest,
            models::requests::ConfigRequest,
            models::requests::RemoveDeviceRequest,
            models::requests::ReportRequest,
            models::responses::LoginResponse,
            models::responses::GenerateResponse,
            models::responses::ErrorResponse,
            vpn_config::WireGuardConfig,
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
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| format!("marinvpn_server={},tower_http=info", settings.server.log_level).into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = services::db::Database::new(&settings.database.url).await.expect("Failed to initialize database");
    let vpn_iface = std::env::var("WG_INTERFACE").unwrap_or_else(|_| "wg0".to_string());
    let vpn_orchestrator = services::vpn::VpnOrchestrator::new(vpn_iface);

    let state = Arc::new(AppState { 
        db, 
        settings: settings.clone(),
        vpn: vpn_orchestrator,
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
        .route("/metrics", get(move || {
            let handle = metric_handle.clone();
            async move { handle.render() }
        }))
        .nest("/api/v1", api_routes())
        .layer(prometheus_layer)
        .layer(GovernorLayer { config: governor_config })
        .layer(TraceLayer::new_for_http())
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
    
    tracing::info!("MarinVPN High-Performance Server listening on http://{}", addr);

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
        .route("/account/devices/remove", post(handlers::auth::remove_device))
        .route("/vpn/servers", get(handlers::vpn::get_servers))
        .route("/vpn/config", post(handlers::vpn::get_vpn_config))
        .route("/vpn/report", post(handlers::vpn::report_problem))
}

async fn health_check() -> &'static str {
    "OK"
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
