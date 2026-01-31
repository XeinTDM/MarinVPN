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

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub struct BlindSigner {
    key: RsaPrivateKey,
}

impl BlindSigner {
    pub fn new() -> Self {
        let key_path = "blind_signer.pem";

        if let Ok(pem) = fs::read_to_string(key_path) {
            if let Ok(key) = RsaPrivateKey::from_pkcs8_pem(&pem) {
                tracing::info!("Loaded existing Blind Signer RSA key from {}", key_path);
                return Self { key };
            }
        }

        let mut rng = rand::thread_rng();
        let key = RsaPrivateKey::new(&mut rng, 4096).expect("failed to generate 4096-bit RSA key");

        if let Ok(pem) = key.to_pkcs8_pem(LineEnding::LF) {
            let _ = fs::write(key_path, pem.as_bytes());
            tracing::info!(
                "Generated and saved new Blind Signer RSA key to {}",
                key_path
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

        // Basic range checks to prevent trivial/out-of-bounds messages
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

        // Use a domain separator to prevent cross-protocol attacks
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
        let key_path = "support_key.pem";

        if let Ok(pem) = fs::read_to_string(key_path) {
            if let Ok(key) = RsaPrivateKey::from_pkcs8_pem(&pem) {
                tracing::info!("Loaded existing Support RSA key from {}", key_path);
                return Self { key };
            }
        }

        let mut rng = rand::thread_rng();
        let key = RsaPrivateKey::new(&mut rng, 4096).expect("failed to generate 4096-bit RSA key");

        if let Ok(pem) = key.to_pkcs8_pem(LineEnding::LF) {
            let _ = fs::write(key_path, pem.as_bytes());
            tracing::info!("Generated and saved new Support RSA key to {}", key_path);
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

pub fn create_token(account_number: &str, secret: &str) -> AppResult<String> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(30))
        .expect("valid timestamp")
        .timestamp();

    let normalized: String = account_number
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    let claims = Claims {
        sub: normalized,
        exp: expiration as usize,
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
