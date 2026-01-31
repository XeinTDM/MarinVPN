use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::util::ServiceExt;
use marinvpn_server::{AppState, api_routes};
use std::sync::Arc;
use marinvpn_common::{GenerateResponse, LoginRequest, LoginResponse};

async fn setup_app() -> axum::Router {
    let db_url = "sqlite::memory:";
    let db = marinvpn_server::services::db::Database::new(db_url).await.expect("Failed to create memory DB");
    
    let mut settings = marinvpn_server::config::Settings::new().unwrap();
    settings.database.url = db_url.to_string();
    
    let vpn = marinvpn_server::services::vpn::VpnOrchestrator::new("wg0".to_string());
    let state = Arc::new(AppState { db, settings, vpn });
    
    api_routes().with_state(state)
}

#[tokio::test]
async fn test_generate_and_login() {
    let app = setup_app().await;

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/account/generate")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let gen_res: GenerateResponse = serde_json::from_slice(&body).unwrap();
    assert!(!gen_res.account_number.is_empty());

    let login_req = LoginRequest {
        account_number: gen_res.account_number.clone(),
        device_name: Some("Test Device".to_string()),
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/account/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&login_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let login_res: LoginResponse = serde_json::from_slice(&body).unwrap();
    assert!(login_res.success);
    assert_eq!(login_res.account_info.unwrap().account_number, gen_res.account_number);
    assert_eq!(login_res.current_device, Some("Test Device".to_string()));
}
