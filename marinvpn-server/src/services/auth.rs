use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};
use crate::error::{AppResult, AppError};
use rsa::{RsaPrivateKey, pkcs8::EncodePublicKey, BigUint};
use rsa::traits::{PublicKeyParts, PrivateKeyParts};
use base64::Engine;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // account_number
    pub exp: usize,
}

pub struct BlindSigner {
    key: RsaPrivateKey,
}

impl BlindSigner {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let key = RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate RSA key");
        Self { key }
    }

    pub fn get_public_key_pem(&self) -> String {
        self.key.to_public_key().to_public_key_pem(rsa::pkcs8::LineEnding::LF).unwrap()
    }

    pub fn sign_blinded(&self, blinded_message_base64: &str) -> AppResult<String> {
        let blinded_bytes = base64::engine::general_purpose::STANDARD
            .decode(blinded_message_base64)
            .map_err(|_| AppError::BadRequest("Invalid base64 in blinded message".to_string()))?;
        
        let m = BigUint::from_bytes_be(&blinded_bytes);
        
        // RSA Blind Signing: s' = (m')^d mod n
        let s_prime = m.modpow(self.key.d(), self.key.n());
        
        Ok(base64::engine::general_purpose::STANDARD.encode(s_prime.to_bytes_be()))
    }

    pub fn verify(&self, message_base64: &str, signature_base64: &str) -> bool {
        let m_bytes = match base64::engine::general_purpose::STANDARD.decode(message_base64) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let s_bytes = match base64::engine::general_purpose::STANDARD.decode(signature_base64) {
            Ok(b) => b,
            Err(_) => return false,
        };

        let m = BigUint::from_bytes_be(&m_bytes);
        let s = BigUint::from_bytes_be(&s_bytes);
        
        // Verification: s^e mod n == m
        let m_check = s.modpow(self.key.e(), self.key.n());
        m_check == m
    }
}

pub fn create_token(account_number: &str, secret: &str) -> AppResult<String> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(30))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: account_number.to_owned(),
        exp: expiration as usize,
    };

    let header = Header::new(Algorithm::HS256);
    encode(&header, &claims, &EncodingKey::from_secret(secret.as_bytes()))
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