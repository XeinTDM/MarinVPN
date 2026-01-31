use crate::models::{
    WireGuardConfig, Account, Device, LoginResponse, LoginRequest, 
    GenerateResponse, ConfigRequest, RemoveDeviceRequest, ReportRequest
};
use marinvpn_common::{BlindTokenRequest, BlindTokenResponse, AnonymousConfigRequest};
use boringtun::x25519::{StaticSecret, PublicKey};
use base64::{prelude::BASE64_STANDARD, Engine};
use rand::{thread_rng, Rng};
use once_cell::sync::Lazy;
use rsa::{RsaPublicKey, pkcs8::DecodePublicKey, BigUint};
use rsa::traits::PublicKeyParts;
use ml_kem::kem::Decapsulate;
use ml_kem::{MlKem768, KemCore, EncodedSizeUser};
use num_integer::Integer;
use num_bigint_dig::traits::ModInverse;

pub struct AuthService;

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", reqwest::header::HeaderValue::from_static("MarinVPN-Core/1.0"));

    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(10))
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .build()
        .expect("Failed to build secure reqwest client")
});
static API_BASE: Lazy<String> = Lazy::new(|| {
    std::env::var("MARIN_API_URL").unwrap_or_else(|_| "http://127.0.0.1:3000/api/v1".to_string())
});

impl AuthService {
    pub async fn secure_resolve(hostname: &str) -> Option<String> {
        let doh_url = "https://cloudflare-dns.com/dns-query";
        
        let rb = CLIENT.get(doh_url)
            .header("Accept", "application/dns-json")
            .query(&[("name", hostname), ("type", "A")]);
            
        let res = rb.send().await.ok()?;
        if !res.status().is_success() { return None; }
        
        let json: serde_json::Value = res.json().await.ok()?;
        let answers = json.get("Answer")?.as_array()?;
        
        for answer in answers {
            if let Some(data) = answer.get("data").and_then(|d| d.as_str()) {
                if data.split('.').count() == 4 {
                    tracing::info!("DoH: Resolved {} to {}", hostname, data);
                    return Some(data.to_string());
                }
            }
        }
        
        None
    }

    fn with_attestation(rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let secret = std::env::var("MARIN_ATTESTATION_SECRET")
            .unwrap_or_else(|_| "marinvpn_secure_attestation_2026_top_tier".to_string());
        let timestamp = chrono::Utc::now().timestamp().to_string();
        
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hasher.update(timestamp.as_bytes());
        let signature = hex::encode(hasher.finalize());
        
        rb.header("X-Marin-Attestation", format!("{}:{}", timestamp, signature))
    }
    pub async fn get_anonymous_config(
        location: &str, 
        token: &str, 
        dns_blocking: Option<crate::models::DnsBlockingState>,
        quantum_resistant: bool,
    ) -> Result<WireGuardConfig, String> {
        let rb = CLIENT.get(format!("{}/auth/blind-key", *API_BASE));
        let key_pem = Self::with_attestation(rb)
            .send()
            .await
            .map_err(|e| format!("Failed to get blind key: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read blind key: {}", e))?;

        let server_pub_key = RsaPublicKey::from_public_key_pem(&key_pem)
            .map_err(|e| format!("Invalid server public key: {}", e))?;

        let mut rng = thread_rng();
        let m_bytes: [u8; 32] = rng.gen();
        
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, &m_bytes);
        let hashed_m = BigUint::from_bytes_be(&hasher.finalize());

        let n = server_pub_key.n();
        let e = server_pub_key.e();
        let mut r;
        loop {
            let r_bytes: [u8; 32] = rng.gen();
            r = BigUint::from_bytes_be(&r_bytes);
            if r < *n && r.clone().gcd(n) == BigUint::from(1u32) {
                break;
            }
        }

        let r_pow_e = r.modpow(e, n);
        let m_prime = (hashed_m.clone() * r_pow_e) % n;
        let m_prime_base64 = BASE64_STANDARD.encode(m_prime.to_bytes_be());

        let rb = CLIENT.post(format!("{}/auth/issue-token", *API_BASE))
            .header("Authorization", format!("Bearer {}", token))
            .json(&BlindTokenRequest { blinded_message: m_prime_base64 });

        let res = Self::with_attestation(rb)
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Failed to issue blind token: {}", res.status()));
        }

        let blind_resp = res.json::<BlindTokenResponse>()
            .await
            .map_err(|e| format!("Invalid server response: {}", e))?;

        let s_prime_bytes = BASE64_STANDARD.decode(&blind_resp.signed_blinded_message)
            .map_err(|_| "Invalid base64 in signed blinded message")?;
        let s_prime = BigUint::from_bytes_be(&s_prime_bytes);

        let r_inv_bi = r.mod_inverse(n).ok_or("Failed to compute mod inverse")?;
        let r_inv = r_inv_bi.to_biguint().ok_or("Inverse is negative")?;
        let s = (s_prime * r_inv) % n;

        if s.modpow(e, n) != hashed_m {
            return Err("Blind signature verification failed locally!".to_string());
        }

        let private_key = StaticSecret::random_from_rng(thread_rng());
        let public_key = PublicKey::from(&private_key);
        let priv_base64 = BASE64_STANDARD.encode(private_key.to_bytes());
        let pub_base64 = BASE64_STANDARD.encode(public_key.as_bytes());

        let (pqc_sk, pqc_pk_b64) = if quantum_resistant {
            let mut rng = thread_rng();
            let (sk, pk) = MlKem768::generate(&mut rng);
            (Some(sk), Some(BASE64_STANDARD.encode(pk.as_bytes())))
        } else {
            (None, None)
        };

        let anon_req = AnonymousConfigRequest {
            message: BASE64_STANDARD.encode(&m_bytes),
            signature: BASE64_STANDARD.encode(s.to_bytes_be()),
            location: location.to_string(),
            pub_key: pub_base64,
            dns_blocking: dns_blocking.map(|d| marinvpn_common::DnsBlockingState {
                ads: d.ads,
                trackers: d.trackers,
                malware: d.malware,
                gambling: d.gambling,
                adult_content: d.adult_content,
                social_media: d.social_media,
            }),
            quantum_resistant,
            pqc_public_key: pqc_pk_b64,
        };

        let rb = CLIENT.post(format!("{}/vpn/config-anonymous", *API_BASE))
            .json(&anon_req);

        let res = Self::with_attestation(rb)
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Server error ({}): {}", res.status(), res.text().await.unwrap_or_default()));
        }

        let mut config = res.json::<WireGuardConfig>()
            .await
            .map_err(|e| format!("Server error decoding config: {}", e))?;

        if let (Some(sk), Some(ct_b64)) = (pqc_sk, &config.pqc_ciphertext) {
            let ct_bytes = BASE64_STANDARD.decode(ct_b64).map_err(|_| "Invalid PQC ciphertext")?;
            let ct = ml_kem::Ciphertext::<MlKem768>::try_from(ct_bytes.as_slice()).map_err(|_| "Invalid PQC CT length")?;
            let ss = sk.decapsulate(&ct).map_err(|_| "PQC Decapsulation failed")?;
            config.preshared_key = Some(BASE64_STANDARD.encode(ss.as_slice()));
        }

        config.private_key = priv_base64;
        Ok(config)
    }

    pub async fn login(account_number: &str, device_name: Option<String>) -> Result<(Account, String, String), String> {
        let rb = CLIENT.post(format!("{}/account/login", *API_BASE))
            .json(&LoginRequest { 
                account_number: account_number.to_string(),
                device_name 
            });

        let res = Self::with_attestation(rb)
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        let data = res.json::<LoginResponse>()
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
        let rb = CLIENT.post(format!("{}/account/devices", *API_BASE))
            .header("Authorization", format!("Bearer {}", token))
            .json(&LoginRequest { 
                account_number: account_number.to_string(),
                device_name: None 
            });

        let res = Self::with_attestation(rb)
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        if !res.status().is_success() {
             return Err(format!("Server error: {}", res.status()));
        }

        let devices = res.json::<Vec<Device>>()
            .await
            .map_err(|e| format!("Server error: {}", e))?;

        Ok(devices)
    }

    pub async fn remove_device(account_number: &str, device_name: &str, token: &str) -> Result<bool, String> {
        let rb = CLIENT.post(format!("{}/account/devices/remove", *API_BASE))
            .header("Authorization", format!("Bearer {}", token))
            .json(&RemoveDeviceRequest { 
                account_number: account_number.to_string(),
                device_name: device_name.to_string() 
            });

        let res = Self::with_attestation(rb)
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        let success = res.json::<bool>()
            .await
            .map_err(|e| format!("Server error: {}", e))?;

        Ok(success)
    }

    pub async fn report_problem(account_number: &str, message: &str, token: &str) -> Result<bool, String> {
        let rb = CLIENT.post(format!("{}/vpn/report", *API_BASE))
            .header("Authorization", format!("Bearer {}", token))
            .json(&ReportRequest { 
                account_number: account_number.to_string(),
                message: message.to_string() 
            });

        let res = Self::with_attestation(rb)
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        let success = res.json::<bool>()
            .await
            .map_err(|e| format!("Server error: {}", e))?;

        Ok(success)
    }

    pub async fn generate_account_number() -> Result<String, String> {
        let rb = CLIENT.post(format!("{}/account/generate", *API_BASE));
        let res = Self::with_attestation(rb)
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        let data = res.json::<GenerateResponse>()
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
        let private_key = StaticSecret::random_from_rng(thread_rng());
        let public_key = PublicKey::from(&private_key);
        
        let priv_base64 = BASE64_STANDARD.encode(private_key.to_bytes());
        let pub_base64 = BASE64_STANDARD.encode(public_key.as_bytes());

        let (pqc_sk, pqc_pk_b64) = if quantum_resistant {
            let mut rng = thread_rng();
            let (sk, pk) = MlKem768::generate(&mut rng);
            (Some(sk), Some(BASE64_STANDARD.encode(pk.as_bytes())))
        } else {
            (None, None)
        };

        let rb = CLIENT.post(format!("{}/vpn/config", *API_BASE))
            .header("Authorization", format!("Bearer {}", token))
            .json(&ConfigRequest { 
                account_number: account_number.to_string(),
                location: location.to_string(),
                pub_key: pub_base64,
                dns_blocking: dns_blocking.map(|d| marinvpn_common::DnsBlockingState {
                    ads: d.ads,
                    trackers: d.trackers,
                    malware: d.malware,
                    gambling: d.gambling,
                    adult_content: d.adult_content,
                    social_media: d.social_media,
                }),
                quantum_resistant,
                pqc_public_key: pqc_pk_b64,
            });

        let res = Self::with_attestation(rb)
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let err_body = res.text().await.unwrap_or_default();
            return Err(format!("Server error ({}): {}", status, err_body));
        }

        let mut config = res.json::<WireGuardConfig>()
            .await
            .map_err(|e| format!("Server error: {}", e))?;

        if let (Some(sk), Some(ct_b64)) = (pqc_sk, &config.pqc_ciphertext) {
            let ct_bytes = BASE64_STANDARD.decode(ct_b64).map_err(|_| "Invalid PQC ciphertext")?;
            let ct = ml_kem::Ciphertext::<MlKem768>::try_from(ct_bytes.as_slice()).map_err(|_| "Invalid PQC CT length")?;
            let ss = sk.decapsulate(&ct).map_err(|_| "PQC Decapsulation failed")?;
            config.preshared_key = Some(BASE64_STANDARD.encode(ss.as_slice()));
        }

        config.private_key = priv_base64;

        Ok(config)
    }

        if !res.status().is_success() {
            let status = res.status();
            let err_body = res.text().await.unwrap_or_default();
            return Err(format!("Server error ({}): {}", status, err_body));
        }

        let mut config = res.json::<WireGuardConfig>()
            .await
            .map_err(|e| format!("Server error: {}", e))?;

        if let (Some(sk), Some(ct_b64)) = (pqc_sk, &config.pqc_ciphertext) {
            let ct_bytes = BASE64_STANDARD.decode(ct_b64).map_err(|_| "Invalid PQC ciphertext")?;
            let ct = ml_kem::Ciphertext::<MlKem768>::try_from(ct_bytes.as_slice()).map_err(|_| "Invalid PQC CT length")?;
            let ss = sk.decapsulate(&ct).map_err(|_| "PQC Decapsulation failed")?;
            config.preshared_key = Some(BASE64_STANDARD.encode(ss.as_slice()));
        }

        config.private_key = priv_base64;

        Ok(config)
    }
}