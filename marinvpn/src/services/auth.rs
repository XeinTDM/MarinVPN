use crate::models::{
    WireGuardConfig, Account, Device, LoginResponse, LoginRequest, 
    GenerateResponse, ConfigRequest, RemoveDeviceRequest, ReportRequest
};
use boringtun::x25519::{StaticSecret, PublicKey};
use base64::{prelude::BASE64_STANDARD, Engine};
use rand::thread_rng;
use once_cell::sync::Lazy;

pub struct AuthService;

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent("MarinVPN-Desktop/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .build()
        .expect("Failed to build secure reqwest client")
});
static API_BASE: Lazy<String> = Lazy::new(|| {
    std::env::var("MARIN_API_URL").unwrap_or_else(|_| "http://127.0.0.1:3000/api/v1".to_string())
});

impl AuthService {
    pub async fn login(account_number: &str, device_name: Option<String>) -> Result<(Account, String, String), String> {
        let res = CLIENT.post(format!("{}/account/login", *API_BASE))
            .json(&LoginRequest { 
                account_number: account_number.to_string(),
                device_name 
            })
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        let data: LoginResponse = res.json()
            .await
            .map_err(|e| format!("Server error: {}", e))?;

        if data.success {
            Ok((
                data.account_info.unwrap(), 
                data.current_device.unwrap_or_default(),
                data.auth_token.unwrap_or_default()
            ))
        } else {
            Err(data.error.unwrap_or_else(|| "Unknown error".to_string()))
        }
    }

    pub async fn get_devices(account_number: &str, token: &str) -> Result<Vec<Device>, String> {
        let res = CLIENT.post(format!("{}/account/devices", *API_BASE))
            .header("Authorization", format!("Bearer {}", token))
            .json(&LoginRequest { 
                account_number: account_number.to_string(),
                device_name: None 
            })
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        if !res.status().is_success() {
             return Err(format!("Server error: {}", res.status()));
        }

        let devices: Vec<Device> = res.json()
            .await
            .map_err(|e| format!("Server error: {}", e))?;

        Ok(devices)
    }

    pub async fn remove_device(account_number: &str, device_name: &str, token: &str) -> Result<bool, String> {
        let res = CLIENT.post(format!("{}/account/devices/remove", *API_BASE))
            .header("Authorization", format!("Bearer {}", token))
            .json(&RemoveDeviceRequest { 
                account_number: account_number.to_string(),
                device_name: device_name.to_string() 
            })
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        let success: bool = res.json()
            .await
            .map_err(|e| format!("Server error: {}", e))?;

        Ok(success)
    }

    pub async fn report_problem(account_number: &str, message: &str, token: &str) -> Result<bool, String> {
        let res = CLIENT.post(format!("{}/vpn/report", *API_BASE))
            .header("Authorization", format!("Bearer {}", token))
            .json(&ReportRequest { 
                account_number: account_number.to_string(),
                message: message.to_string() 
            })
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        let success: bool = res.json()
            .await
            .map_err(|e| format!("Server error: {}", e))?;

        Ok(success)
    }

    pub async fn generate_account_number() -> Result<String, String> {
        let res = CLIENT.post(format!("{}/account/generate", *API_BASE))
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        let data: GenerateResponse = res.json()
            .await
            .map_err(|e| format!("Server error: {}", e))?;

        Ok(data.account_number)
    }

    pub async fn get_config(
        account_number: &str, 
        location: &str, 
        token: &str, 
        dns_blocking: Option<crate::models::DnsBlockingState>,
        quantum_resistant: bool,
    ) -> Result<WireGuardConfig, String> {
        // EPHEMERAL IDENTITY: Generate a fresh keypair for every single session.
        // This prevents the server from linking multiple connections to the same device.
        let private_key = StaticSecret::random_from_rng(thread_rng());
        let public_key = PublicKey::from(&private_key);
        
        let priv_base64 = BASE64_STANDARD.encode(private_key.to_bytes());
        let pub_base64 = BASE64_STANDARD.encode(public_key.as_bytes());

        let res = CLIENT.post(format!("{}/vpn/config", *API_BASE))
            .header("Authorization", format!("Bearer {}", token))
            .json(&ConfigRequest { 
                account_number: account_number.to_string(),
                location: location.to_string(),
                pub_key: pub_base64,
                dns_blocking,
                quantum_resistant,
            })
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let err_body = res.text().await.unwrap_or_default();
            return Err(format!("Server error ({}): {}", status, err_body));
        }

        let mut config: WireGuardConfig = res.json()
            .await
            .map_err(|e| format!("Server error: {}", e))?;

        config.private_key = priv_base64;

        Ok(config)
    }
}
