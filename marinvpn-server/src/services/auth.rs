use crate::error::{AppError, AppResult};
use base64::Engine;
use blake2::{Blake2s, Digest};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rsa::{
    pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding},
    traits::{PrivateKeyParts, PublicKeyParts},
    BigUint, RsaPrivateKey,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub device: String,
    pub kind: String,
}

pub struct BlindSigner {
    key: RsaPrivateKey,
}

impl BlindSigner {
    pub fn new() -> Self {
        let key_path = resolve_key_path("blind_signer.pem");
        ensure_key_dir(&key_path);

        if let Ok(pem) = fs::read_to_string(&key_path) {
            if let Ok(key) = RsaPrivateKey::from_pkcs8_pem(&pem) {
                tracing::info!(
                    "Loaded existing Blind Signer RSA key from {}",
                    key_path.display()
                );
                return Self { key };
            }
        }

        let mut rng = rand::thread_rng();
        let key = RsaPrivateKey::new(&mut rng, 4096).expect("failed to generate 4096-bit RSA key");

        if let Ok(pem) = key.to_pkcs8_pem(LineEnding::LF) {
            let _ = write_private_key(&key_path, pem.as_bytes());
            tracing::info!(
                "Generated and saved new Blind Signer RSA key to {}",
                key_path.display()
            );
        }

        Self { key }
    }

    pub fn get_public_key_pem(&self) -> String {
        self.key
            .to_public_key()
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .unwrap()
    }

    pub fn sign_blinded(&self, blinded_message_base64: &str) -> AppResult<String> {
        let blinded_bytes = base64::engine::general_purpose::STANDARD
            .decode(blinded_message_base64)
            .map_err(|_| AppError::BadRequest("Invalid base64".to_string()))?;

        let m = BigUint::from_bytes_be(&blinded_bytes);
        let n = self.key.n();

        if m < BigUint::from(2u32) || m >= *n {
            return Err(AppError::BadRequest("Message out of range".to_string()));
        }

        let s_prime = m.modpow(self.key.d(), n);

        Ok(base64::engine::general_purpose::STANDARD.encode(s_prime.to_bytes_be()))
    }

    pub fn verify(&self, message_base64: &str, signature_base64: &str) -> bool {
        let m_raw = match base64::engine::general_purpose::STANDARD.decode(message_base64) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let s_bytes = match base64::engine::general_purpose::STANDARD.decode(signature_base64) {
            Ok(b) => b,
            Err(_) => return false,
        };

        let mut hasher = Blake2s::new();
        hasher.update(b"MARIN_VPN_BLIND_SIG_V1");
        hasher.update(&m_raw);
        let hashed_m = BigUint::from_bytes_be(&hasher.finalize());

        let s = BigUint::from_bytes_be(&s_bytes);
        let n = self.key.n();

        if s >= *n {
            return false;
        }

        let m_check = s.modpow(self.key.e(), n);

        m_check == hashed_m
    }
}

impl Default for BlindSigner {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SupportKey {
    key: RsaPrivateKey,
}

impl SupportKey {
    pub fn new() -> Self {
        let key_path = resolve_key_path("support_key.pem");
        ensure_key_dir(&key_path);

        if let Ok(pem) = fs::read_to_string(&key_path) {
            if let Ok(key) = RsaPrivateKey::from_pkcs8_pem(&pem) {
                tracing::info!(
                    "Loaded existing Support RSA key from {}",
                    key_path.display()
                );
                return Self { key };
            }
        }

        let mut rng = rand::thread_rng();
        let key = RsaPrivateKey::new(&mut rng, 4096).expect("failed to generate 4096-bit RSA key");

        if let Ok(pem) = key.to_pkcs8_pem(LineEnding::LF) {
            let _ = write_private_key(&key_path, pem.as_bytes());
            tracing::info!(
                "Generated and saved new Support RSA key to {}",
                key_path.display()
            );
        }

        Self { key }
    }

    pub fn get_public_key_pem(&self) -> String {
        self.key
            .to_public_key()
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .unwrap()
    }
}

impl Default for SupportKey {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_key_path(filename: &str) -> PathBuf {
    if let Ok(dir) = std::env::var("MARIN_KEY_DIR") {
        return PathBuf::from(dir).join(filename);
    }

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(filename)
}

fn ensure_key_dir(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

fn write_private_key(path: &Path, data: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = fs::OpenOptions::new();
        options.create(true).write(true).truncate(true).mode(0o600);
        let mut file = options.open(path)?;
        use std::io::Write;
        file.write_all(data)?;
        return Ok(());
    }

    #[cfg(not(unix))]
    {
        fs::write(path, data)
    }
}

pub fn create_token(account_number: &str, device: &str, secret: &str) -> AppResult<String> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(15))
        .expect("valid timestamp")
        .timestamp();

    create_token_with_exp(account_number, device, secret, expiration, "access")
}

pub fn create_refresh_token(
    account_number: &str,
    device: &str,
    secret: &str,
) -> AppResult<(String, i64)> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(30))
        .expect("valid timestamp")
        .timestamp();

    let token = create_token_with_exp(account_number, device, secret, expiration, "refresh")?;
    Ok((token, expiration))
}

fn create_token_with_exp(
    account_number: &str,
    device: &str,
    secret: &str,
    exp: i64,
    kind: &str,
) -> AppResult<String> {
    let normalized: String = account_number
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase();

    let claims = Claims {
        sub: normalized,
        exp: exp as usize,
        device: device.to_string(),
        kind: kind.to_string(),
    };

    let header = Header::new(Algorithm::HS256);
    encode(
        &header,
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to create token")))
}

pub fn decode_token(token: &str, secret: &str) -> AppResult<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized)
}

pub fn decode_access_token(token: &str, secret: &str) -> AppResult<Claims> {
    let claims = decode_token(token, secret)?;
    if claims.kind != "access" {
        return Err(AppError::Unauthorized);
    }
    Ok(claims)
}

pub fn decode_refresh_token(token: &str, secret: &str) -> AppResult<Claims> {
    let claims = decode_token(token, secret)?;
    if claims.kind != "refresh" {
        return Err(AppError::Unauthorized);
    }
    Ok(claims)
}
