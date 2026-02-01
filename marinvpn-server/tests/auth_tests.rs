use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use marinvpn_common::{GenerateResponse, LoginRequest, LoginResponse};
use marinvpn_server::{api_routes, AppState};
use std::sync::Arc;
use tower::util::ServiceExt;

async fn setup_app() -> Option<axum::Router> {
    let db_url = match std::env::var("TEST_DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("TEST_DATABASE_URL not set; skipping integration test.");
            return None;
        }
    };
    let db = marinvpn_server::services::db::Database::new(&db_url, "test_salt")
        .await
        .expect("Failed to create test DB");

    let mut settings = marinvpn_server::config::Settings::new().unwrap();
    settings.database.url = db_url.to_string();

    let vpn = marinvpn_server::services::vpn::VpnOrchestrator::new("wg0".to_string());
    let signer = marinvpn_server::services::auth::BlindSigner::new();
    let support_key = marinvpn_server::services::auth::SupportKey::new();
    let state = Arc::new(AppState {
        db,
        settings,
        vpn,
        signer,
        support_key,
    });

    Some(api_routes().with_state(state))
}

#[tokio::test]
async fn test_generate_and_login() {
    let Some(app) = setup_app().await else {
        return;
    };

    let response = app
        .clone()
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

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let gen_res: GenerateResponse = serde_json::from_slice(&body).unwrap();
    assert!(!gen_res.account_number.is_empty());

    let login_req = LoginRequest {
        account_number: gen_res.account_number.clone(),
        device_pubkey: Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string()),
        kick_device: None,
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

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let login_res: LoginResponse = serde_json::from_slice(&body).unwrap();
    assert!(login_res.success);
    assert_eq!(
        login_res.account_info.unwrap().account_number,
        gen_res.account_number
    );
    assert!(login_res.current_device.unwrap_or_default().len() > 0);
    assert!(login_res.auth_token.unwrap_or_default().len() > 10);
    assert!(login_res.refresh_token.unwrap_or_default().len() > 10);
}
