use crate::error::AppError;
use crate::models::{
    ConfigRequest, Device, GenerateResponse, LoginRequest, LoginResponse, RefreshRequest,
    RefreshResponse, RemoveDeviceRequest, ReportRequest, WireGuardConfig,
};
use base64::{prelude::BASE64_STANDARD, Engine};
use blake2::{Blake2s, Digest as BlakeDigest};
use boringtun::x25519::{PublicKey, StaticSecret};
use marinvpn_common::{AnonymousConfigRequest, BlindTokenRequest, BlindTokenResponse};
use ml_kem::kem::Decapsulate;
use ml_kem::{EncodedSizeUser, KemCore, MlKem768};
use num_bigint_dig::traits::ModInverse;
use num_integer::Integer;
use once_cell::sync::Lazy;
use rand::{thread_rng, Rng};
use reqwest::StatusCode;
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};
use rsa::traits::PublicKeyParts;
use rsa::{pkcs8::DecodePublicKey, BigUint, RsaPublicKey};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub struct AuthService;

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "User-Agent",
        reqwest::header::HeaderValue::from_static("MarinVPN-Core/1.0"),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(10))
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .build()
        .expect("Failed to build secure reqwest client")
});

fn api_base() -> Result<String, AppError> {
    let base = std::env::var("MARIN_API_URL");

    if is_production() {
        match base {
            Ok(url) => {
                if url.starts_with("http://") {
                    return Err(AppError::Config(
                        "MARIN_API_URL must be https in production".to_string(),
                    ));
                }
                Ok(url)
            }
            Err(_) => Err(AppError::Config(
                "MARIN_API_URL environment variable must be set in production".to_string(),
            )),
        }
    } else {
        Ok(base.unwrap_or_else(|_| "http://127.0.0.1:3000/api/v1".to_string()))
    }
}

fn api_url(path: &str) -> Result<String, AppError> {
    let base = api_base()?;
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    Ok(format!("{}/{}", base, path))
}

fn is_production() -> bool {
    let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".to_string());
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "".to_string());
    matches!(run_mode.to_lowercase().as_str(), "production" | "prod")
        || matches!(app_env.to_lowercase().as_str(), "production" | "prod")
}

fn device_keypair() -> Result<Ed25519KeyPair, AppError> {
    if let Some(encoded) = crate::storage::load_device_attestation_key() {
        let raw = BASE64_STANDARD
            .decode(encoded)
            .map_err(|_| AppError::Crypto("Invalid device attestation key in storage".to_string()))?;
        return Ed25519KeyPair::from_pkcs8(raw.as_slice())
            .map_err(|_| AppError::Crypto("Invalid device attestation key in storage".to_string()));
    }

    let rng = SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng)
        .map_err(|_| AppError::Crypto("Failed to generate device attestation key".to_string()))?;
    crate::storage::save_device_attestation_key(&BASE64_STANDARD.encode(pkcs8.as_ref()));
    Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())
        .map_err(|_| AppError::Crypto("Failed to load generated device attestation key".to_string()))
}

fn device_pubkey_b64() -> Result<String, AppError> {
    let key = device_keypair()?;
    Ok(BASE64_STANDARD.encode(key.public_key().as_ref()))
}

fn body_hash_hex(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    hex::encode(hash)
}

fn request_with_attestation(
    method: &str,
    path: &str,
    body: Option<Vec<u8>>,
) -> Result<reqwest::RequestBuilder, AppError> {
    let url = api_url(path)?;
    let hash = match body.as_ref() {
        Some(bytes) => body_hash_hex(bytes),
        None => body_hash_hex(&[]),
    };

    let timestamp = chrono::Utc::now().timestamp().to_string();
    let nonce: String = {
        let mut rng = rand::thread_rng();
        let n: [u8; 16] = rng.gen();
        hex::encode(n)
    };

    let mut rb = match method {
        "GET" => CLIENT.get(url),
        "POST" => CLIENT.post(url),
        _ => return Err(AppError::Config(format!("Unsupported HTTP method: {}", method))),
    };

    if let Some(bytes) = body {
        rb = rb
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(bytes);
    }

    let message = format!("{}:{}:{}:{}:{}", timestamp, nonce, method, path, hash);
    let key = device_keypair()?;
    let signature = key.sign(message.as_bytes());
    let signature_b64 = BASE64_STANDARD.encode(signature.as_ref());
    let pubkey_b64 = device_pubkey_b64()?;

    Ok(rb
        .header(
            "X-Marin-Attestation",
            format!("{}:{}:{}", timestamp, nonce, signature_b64),
        )
        .header("X-Marin-Attestation-Body", hash)
        .header("X-Marin-Attestation-Pub", pubkey_b64))
}

fn json_body<T: Serialize>(payload: &T) -> Result<Vec<u8>, AppError> {
    serde_json::to_vec(payload).map_err(|e| AppError::Serialization(e))
}

impl AuthService {
    async fn send_authed_with_refresh<F>(
        token: &str,
        make_req: F,
    ) -> Result<reqwest::Response, AppError>
    where
        F: Fn(&str) -> Result<reqwest::RequestBuilder, AppError>,
    {
        let res = make_req(token)?.send().await?;

        if res.status() != StatusCode::UNAUTHORIZED {
            return Ok(res);
        }

        let refresh = crate::storage::load_config().refresh_token;
        let Some(refresh_token) = refresh else {
            return Err(AppError::SessionExpired);
        };

        let refreshed = match Self::refresh_auth(&refresh_token).await {
            Ok(tokens) => tokens,
            Err(err) => {
                let _ = crate::storage::update_auth_tokens(None, None);
                return Err(err);
            }
        };
        let _ = crate::storage::update_auth_tokens(
            Some(refreshed.auth_token.clone()),
            Some(refreshed.refresh_token.clone()),
        );

        make_req(&refreshed.auth_token)?.send().await.map_err(AppError::from)
    }

    pub async fn secure_resolve(hostname: &str) -> Option<String> {
        let doh_url = "https://cloudflare-dns.com/dns-query";

        let rb = CLIENT
            .get(doh_url)
            .header("Accept", "application/dns-json")
            .query(&[("name", hostname), ("type", "A")]);

        let res = rb.send().await.ok()?;
        if !res.status().is_success() {
            return None;
        }

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

    pub async fn get_anonymous_config(
        location: &str,
        token: &str,
        dns_blocking: Option<crate::models::DnsBlockingState>,
        quantum_resistant: bool,
    ) -> Result<WireGuardConfig, AppError> {
        let rb = request_with_attestation("GET", "/api/v1/auth/blind-key", None)?;
        let key_pem = rb
            .send()
            .await?
            .text()
            .await
            .map_err(|e| AppError::Network(e))?;

        let server_pub_key = RsaPublicKey::from_public_key_pem(&key_pem)
            .map_err(|e| AppError::Crypto(format!("Invalid server public key: {}", e)))?;

        // Scope RNG usage
        let m_bytes: [u8; 32] = {
            let mut rng = thread_rng();
            rng.gen()
        };

        let mut hasher = Blake2s::new();
        hasher.update(b"MARIN_VPN_BLIND_SIG_V1");
        hasher.update(m_bytes);
        let hashed_m = BigUint::from_bytes_be(&hasher.finalize());

        let n = server_pub_key.n();
        let e = server_pub_key.e();
        let mut r;
        {
            let mut rng = thread_rng();
            loop {
                let r_bytes: [u8; 32] = rng.gen();
                r = BigUint::from_bytes_be(&r_bytes);
                if r > BigUint::from(1u32) && r < *n && r.clone().gcd(n) == BigUint::from(1u32) {
                    break;
                }
            }
        }

        let r_pow_e = r.modpow(e, n);
        let m_prime = (hashed_m.clone() * r_pow_e) % n;
        let m_prime_base64 = BASE64_STANDARD.encode(m_prime.to_bytes_be());

        let blind_req = BlindTokenRequest {
            blinded_message: m_prime_base64,
        };
        let res = Self::send_authed_with_refresh(token, |t| {
            request_with_attestation(
                "POST",
                "/api/v1/auth/issue-token",
                Some(json_body(&blind_req)?),
            )
            .map(|rb| rb.header("Authorization", format!("Bearer {}", t)))
        })
        .await?;

        if !res.status().is_success() {
            return Err(AppError::Api {
                status: res.status(),
                message: "Failed to issue blind token".to_string(),
            });
        }

        let blind_resp = res.json::<BlindTokenResponse>().await?;

        let s_prime_bytes = BASE64_STANDARD
            .decode(&blind_resp.signed_blinded_message)
            .map_err(|_| AppError::Crypto("Invalid base64 in signed blinded message".to_string()))?;
        let s_prime = BigUint::from_bytes_be(&s_prime_bytes);

        let r_inv_bi = r.mod_inverse(n).ok_or(AppError::Crypto("Failed to compute mod inverse".to_string()))?;
        let r_inv = r_inv_bi.to_biguint().ok_or(AppError::Crypto("Inverse is negative".to_string()))?;
        let s = (s_prime * r_inv) % n;

        if s.modpow(e, n) != hashed_m {
            return Err(AppError::Crypto("Blind signature verification failed locally!".to_string()));
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
            message: BASE64_STANDARD.encode(m_bytes),
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

        let rb = request_with_attestation(
            "POST",
            "/api/v1/vpn/config-anonymous",
            Some(json_body(&anon_req)?),
        )?;

        let res = rb.send().await?;

        if !res.status().is_success() {
            return Err(AppError::Api {
                status: res.status(),
                message: res.text().await.unwrap_or_default(),
            });
        }

        let mut config = res.json::<WireGuardConfig>().await?;

        if let (Some(sk), Some(ct_b64)) = (pqc_sk, &config.pqc_ciphertext) {
            let ct_bytes = BASE64_STANDARD
                .decode(ct_b64)
                .map_err(|_| AppError::Crypto("Invalid PQC ciphertext".to_string()))?;
            let ct = ml_kem::Ciphertext::<MlKem768>::try_from(ct_bytes.as_slice())
                .map_err(|_| AppError::Crypto("Invalid PQC CT length".to_string()))?;
            let ss = sk
                .decapsulate(&ct)
                .map_err(|_| AppError::Crypto("PQC Decapsulation failed".to_string()))?;
            config.preshared_key = Some(BASE64_STANDARD.encode(ss.as_slice()));
        }

        config.private_key = priv_base64;

        Ok(config)
    }

    pub async fn login(
        account_number: &str,
        kick_device: Option<String>,
    ) -> Result<LoginResponse, AppError> {
        let login_req = LoginRequest {
            account_number: account_number.to_string(),
            device_pubkey: Some(device_pubkey_b64()?),
            kick_device,
        };
        let rb = request_with_attestation(
            "POST",
            "/api/v1/account/login",
            Some(json_body(&login_req)?),
        )?;

        let res = rb.send().await?;

        if !res.status().is_success() {
             return Err(AppError::Api {
                status: res.status(),
                message: res.text().await.unwrap_or_default(),
            });
        }

        let data = res.json::<LoginResponse>().await?;

        Ok(data)
    }

    pub async fn refresh_auth(refresh_token: &str) -> Result<RefreshResponse, AppError> {
        let req = RefreshRequest {
            refresh_token: refresh_token.to_string(),
        };
        let rb = request_with_attestation("POST", "/api/v1/auth/refresh", Some(json_body(&req)?))?;

        let res = rb.send().await?;

        if !res.status().is_success() {
            return Err(AppError::Api {
                status: res.status(),
                message: res.text().await.unwrap_or_default(),
            });
        }

        Ok(res.json::<RefreshResponse>().await?)
    }

    pub async fn get_devices(account_number: &str, token: &str) -> Result<Vec<Device>, AppError> {
        let login_req = LoginRequest {
            account_number: account_number.to_string(),
            device_pubkey: None,
            kick_device: None,
        };
        let res = Self::send_authed_with_refresh(token, |t| {
            request_with_attestation(
                "POST",
                "/api/v1/account/devices",
                Some(json_body(&login_req)?),
            )
            .map(|rb| rb.header("Authorization", format!("Bearer {}", t)))
        })
        .await?;

        if !res.status().is_success() {
             return Err(AppError::Api {
                status: res.status(),
                message: res.text().await.unwrap_or_default(),
            });
        }

        let devices = res.json::<Vec<Device>>().await?;

        Ok(devices)
    }

    pub async fn remove_device(
        account_number: &str,
        device_name: &str,
        token: &str,
    ) -> Result<bool, AppError> {
        let remove_req = RemoveDeviceRequest {
            account_number: account_number.to_string(),
            device_name: device_name.to_string(),
        };
        let res = Self::send_authed_with_refresh(token, |t| {
            request_with_attestation(
                "POST",
                "/api/v1/account/devices/remove",
                Some(json_body(&remove_req)?),
            )
            .map(|rb| rb.header("Authorization", format!("Bearer {}", t)))
        })
        .await?;

        if !res.status().is_success() {
             return Err(AppError::Api {
                status: res.status(),
                message: res.text().await.unwrap_or_default(),
            });
        }

        let success = res.json::<bool>().await?;

        Ok(success)
    }

    pub async fn report_problem(
        account_number: &str,
        message: &str,
        token: &str,
    ) -> Result<bool, AppError> {
        let rb = request_with_attestation("GET", "/api/v1/auth/support-key", None)?;
        let key_pem = rb
            .send()
            .await?
            .text()
            .await
            .map_err(|e| AppError::Network(e))?;

        let pub_key = RsaPublicKey::from_public_key_pem(&key_pem)
            .map_err(|e| AppError::Crypto(format!("Invalid support public key: {}", e)))?;

        let mut rng = thread_rng();
        let enc_data = if !message.is_empty() {
            let msg_bytes = message.as_bytes();
            let max_chunk = 400;
            let mut encrypted_chunks = Vec::new();

            for chunk in msg_bytes.chunks(max_chunk) {
                let enc = pub_key
                    .encrypt(&mut rng, rsa::Oaep::new::<Sha256>(), chunk)
                    .map_err(|e| AppError::Crypto(format!("Encryption failed: {}", e)))?;
                encrypted_chunks.push(BASE64_STANDARD.encode(enc));
            }
            encrypted_chunks.join("|")
        } else {
            String::new()
        };

        let report_req = ReportRequest {
            account_number: account_number.to_string(),
            message: enc_data,
            is_encrypted: true,
        };
        let res = Self::send_authed_with_refresh(token, |t| {
            request_with_attestation("POST", "/api/v1/vpn/report", Some(json_body(&report_req)?))
                .map(|rb| rb.header("Authorization", format!("Bearer {}", t)))
        })
        .await?;

        if !res.status().is_success() {
             return Err(AppError::Api {
                status: res.status(),
                message: res.text().await.unwrap_or_default(),
            });
        }

        let success = res.json::<bool>().await?;

        Ok(success)
    }

    pub async fn generate_account_number() -> Result<String, AppError> {
        let rb = request_with_attestation("POST", "/api/v1/account/generate", None)?;
        let res = rb.send().await?;

        if !res.status().is_success() {
             return Err(AppError::Api {
                status: res.status(),
                message: res.text().await.unwrap_or_default(),
            });
        }

        let data = res.json::<GenerateResponse>().await?;

        Ok(data.account_number)
    }

    pub async fn get_config(
        account_number: &str,
        location: &str,
        token: &str,
        dns_blocking: Option<crate::models::DnsBlockingState>,
        quantum_resistant: bool,
    ) -> Result<WireGuardConfig, AppError> {
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

        let cfg_req = ConfigRequest {
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
        };
        let res = Self::send_authed_with_refresh(token, |t| {
            request_with_attestation("POST", "/api/v1/vpn/config", Some(json_body(&cfg_req)?))
                .map(|rb| rb.header("Authorization", format!("Bearer {}", t)))
        })
        .await?;

        if !res.status().is_success() {
            let status = res.status();
            let err_body = res.text().await.unwrap_or_default();
            return Err(AppError::Api {
                status,
                message: err_body,
            });
        }

        let mut config = res.json::<WireGuardConfig>().await?;

        if let (Some(sk), Some(ct_b64)) = (pqc_sk, &config.pqc_ciphertext) {
            let ct_bytes = BASE64_STANDARD
                .decode(ct_b64)
                .map_err(|_| AppError::Crypto("Invalid PQC ciphertext".to_string()))?;
            let ct = ml_kem::Ciphertext::<MlKem768>::try_from(ct_bytes.as_slice())
                .map_err(|_| AppError::Crypto("Invalid PQC CT length".to_string()))?;
            let ss = sk
                .decapsulate(&ct)
                .map_err(|_| AppError::Crypto("PQC Decapsulation failed".to_string()))?;
            config.preshared_key = Some(BASE64_STANDARD.encode(ss.as_slice()));
        }

        config.private_key = priv_base64;

        Ok(config)
    }
}
