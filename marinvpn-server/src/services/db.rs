use crate::error::{AppError, AppResult};
use crate::models::{Account, Device, VpnServer};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Algorithm, Argon2, Params, Version,
};
use blake2::{Blake2s, Digest};
use chrono::{TimeZone, Utc};
use rand::Rng;
use sqlx::{postgres::PgPoolOptions, Error, PgPool};
use tracing::info;

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
    salt: String,
}

impl Database {
    fn hash_account_legacy(&self, account_number: &str) -> AppResult<String> {
        let salt = SaltString::encode_b64(self.salt.as_bytes())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Salt encoding failed: {}", e)))?;

        let normalized: String = account_number
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_uppercase();

        let params = Params::new(15360, 2, 1, None)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Argon2 params failed: {}", e)))?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        let hash = argon2
            .hash_password(normalized.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Hashing failed: {}", e)))?
            .to_string();

        Ok(hash)
    }

    fn hash_account_v2(account_number: &str, salt: &str) -> AppResult<String> {
        let salt_string = SaltString::from_b64(salt)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid salt: {}", e)))?;

        let normalized: String = account_number
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_uppercase();

        let params = Params::new(15360, 2, 1, None)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Argon2 params failed: {}", e)))?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        let hash = argon2
            .hash_password(normalized.as_bytes(), &salt_string)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Hashing failed: {}", e)))?
            .to_string();

        Ok(hash)
    }

    async fn resolve_account_pk(&self, account_number: &str) -> AppResult<String> {
        let normalized: String = account_number
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_uppercase();

        let prefix = if normalized.len() >= 8 {
            &normalized[..8]
        } else {
            &normalized
        };

        let candidates: Vec<(String, Option<String>)> =
            sqlx::query_as("SELECT account_number, salt FROM accounts WHERE prefix = $1")
                .bind(prefix)
                .fetch_all(&self.pool)
                .await?;

        for (db_hash, db_salt) in candidates {
            if let Some(salt) = db_salt {
                let h = Self::hash_account_v2(account_number, &salt)?;
                if h == db_hash {
                    return Ok(h);
                }
            } else {
                let h = self.hash_account_legacy(account_number)?;
                if h == db_hash {
                    return Ok(h);
                }
            }
        }

        self.hash_account_legacy(account_number)
    }

    pub async fn new(url: &str, salt: &str) -> AppResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(25)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .idle_timeout(std::time::Duration::from_secs(600))
            .connect(url)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self {
            pool,
            salt: salt.to_string(),
        })
    }

    fn map_id_to_ip(id: i64) -> String {
        const POOL_SIZE: u32 = 16_580_608;
        let id_wrapped = (id as u32).wrapping_rem(POOL_SIZE);

        let z = (id_wrapped % 253) + 2; // 2 to 254
        let y = (id_wrapped / 253) % 256; // 0 to 255
        let x = (id_wrapped / (253 * 256)) % 256; // 0 to 255

        format!("10.{}.{}.{}/32", x, y, z)
    }

    fn hash_refresh_token(token: &str) -> String {
        let mut hasher = Blake2s::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub async fn cleanup_stale_sessions(&self, max_age_secs: i64) -> AppResult<Vec<String>> {
        let cutoff = Utc::now().timestamp() - max_age_secs;
        let now = Utc::now().timestamp();

        let stale_peers: Vec<(String,)> =
            sqlx::query_as("SELECT pub_key FROM peers WHERE registered_at < $1")
                .bind(cutoff)
                .fetch_all(&self.pool)
                .await?;

        let pub_keys: Vec<String> = stale_peers.into_iter().map(|(pk,)| pk).collect();

        if !pub_keys.is_empty() {
            info!(
                "Cleaning up {} stale VPN sessions from shared session store",
                pub_keys.len()
            );
            sqlx::query("DELETE FROM peers WHERE registered_at < $1")
                .bind(cutoff)
                .execute(&self.pool)
                .await?;
        }

        sqlx::query("DELETE FROM used_tokens WHERE used_at < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM attestation_ids WHERE used_at < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM refresh_tokens WHERE expires_at < $1")
            .bind(now)
            .execute(&self.pool)
            .await?;

        Ok(pub_keys)
    }

    pub async fn is_attestation_id_used(&self, id: &str) -> AppResult<bool> {
        let row: Option<(String,)> = sqlx::query_as("SELECT id FROM attestation_ids WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }

    pub async fn mark_attestation_id_used(&self, id: &str) -> AppResult<()> {
        let now = Utc::now().timestamp();
        sqlx::query("INSERT INTO attestation_ids (id, used_at) VALUES ($1, $2)")
            .bind(id)
            .bind(now)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn is_token_used(&self, message: &str) -> AppResult<bool> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT message FROM used_tokens WHERE message = $1")
                .bind(message)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.is_some())
    }

    pub async fn mark_token_used(&self, message: &str) -> AppResult<()> {
        let now = Utc::now().timestamp();
        sqlx::query("INSERT INTO used_tokens (message, used_at) VALUES ($1, $2)")
            .bind(message)
            .bind(now)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_account(
        &self,
        account_number: &str,
        expiry_days: i64,
    ) -> AppResult<Account> {
        let now = Utc::now().timestamp();
        let expiry = now + (expiry_days * 24 * 60 * 60);

        let salt = SaltString::generate(&mut rand::thread_rng());
        let salt_str = salt.as_str().to_string();

        let hashed = Self::hash_account_v2(account_number, &salt_str)?;

        let normalized: String = account_number
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_uppercase();

        let prefix = if normalized.len() >= 8 {
            &normalized[..8]
        } else {
            &normalized
        };

        let account = Account {
            account_number: account_number.to_string(),
            expiry_date: expiry,
            created_at: now,
        };

        sqlx::query(
            "INSERT INTO accounts (account_number, expiry_date, created_at, prefix, salt) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&hashed)
        .bind(account.expiry_date)
        .bind(account.created_at)
        .bind(prefix)
        .bind(&salt_str)
        .execute(&self.pool)
        .await?;

        Ok(account)
    }

    pub async fn get_account(&self, account_number: &str) -> AppResult<Option<Account>> {
        let hashed = self.resolve_account_pk(account_number).await?;
        let row: Option<(i64, i64)> = sqlx::query_as(
            "SELECT expiry_date, created_at FROM accounts WHERE account_number = $1",
        )
        .bind(&hashed)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(expiry, created)| Account {
            account_number: account_number.to_string(),
            expiry_date: expiry,
            created_at: created,
        }))
    }

    pub async fn add_device(
        &self,
        account_id: &str,
        name: &str,
        attestation_pubkey: Option<&str>,
    ) -> AppResult<Device> {
        let today = Utc::now().date_naive();
        let now = Utc
            .from_utc_datetime(&today.and_hms_opt(0, 0, 0).unwrap())
            .timestamp();
        let hashed = self.resolve_account_pk(account_id).await?;
        sqlx::query(
            "INSERT INTO devices (account_id, name, added_at, attestation_pubkey) VALUES ($1, $2, $3, $4)",
        )
            .bind(&hashed)
            .bind(name)
            .bind(now)
            .bind(attestation_pubkey)
            .execute(&self.pool)
            .await?;

        Ok(Device {
            id: None,
            account_id: account_id.to_string(),
            name: name.to_string(),
            added_at: now,
            attestation_pubkey: attestation_pubkey.map(|v| v.to_string()),
        })
    }

    pub async fn get_device_by_pubkey(
        &self,
        account_id: &str,
        attestation_pubkey: &str,
    ) -> AppResult<Option<Device>> {
        let hashed = self.resolve_account_pk(account_id).await?;
        let row: Option<(String, i64, Option<String>)> = sqlx::query_as(
            "SELECT name, added_at, attestation_pubkey FROM devices WHERE account_id = $1 AND attestation_pubkey = $2",
        )
        .bind(&hashed)
        .bind(attestation_pubkey)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(name, added_at, attestation_pubkey)| Device {
            id: None,
            account_id: account_id.to_string(),
            name,
            added_at,
            attestation_pubkey,
        }))
    }

    pub async fn upsert_refresh_token(
        &self,
        account_id: &str,
        device_name: &str,
        refresh_token: &str,
        expires_at: i64,
    ) -> AppResult<()> {
        let now = Utc::now().timestamp();
        let hashed_account = self.resolve_account_pk(account_id).await?;
        let token_hash = Self::hash_refresh_token(refresh_token);

        sqlx::query(
            "INSERT INTO refresh_tokens (account_id, device_name, token_hash, issued_at, expires_at) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (account_id, device_name) DO UPDATE SET token_hash = $3, issued_at = $4, expires_at = $5",
        )
        .bind(&hashed_account)
        .bind(device_name)
        .bind(token_hash)
        .bind(now)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn rotate_refresh_token(
        &self,
        account_id: &str,
        device_name: &str,
        old_token: &str,
        new_token: &str,
        new_expires_at: i64,
    ) -> AppResult<bool> {
        let now = Utc::now().timestamp();
        let hashed_account = self.resolve_account_pk(account_id).await?;
        let old_hash = Self::hash_refresh_token(old_token);
        let new_hash = Self::hash_refresh_token(new_token);

        let res = sqlx::query(
            "UPDATE refresh_tokens 
             SET token_hash = $1, issued_at = $2, expires_at = $3 
             WHERE account_id = $4 AND device_name = $5 AND token_hash = $6 AND expires_at >= $7",
        )
        .bind(&new_hash)
        .bind(now)
        .bind(new_expires_at)
        .bind(&hashed_account)
        .bind(device_name)
        .bind(&old_hash)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(res.rows_affected() > 0)
    }

    pub async fn validate_refresh_token(
        &self,
        account_id: &str,
        device_name: &str,
        refresh_token: &str,
    ) -> AppResult<bool> {
        let hashed_account = self.resolve_account_pk(account_id).await?;
        let token_hash = Self::hash_refresh_token(refresh_token);
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT expires_at FROM refresh_tokens WHERE account_id = $1 AND device_name = $2 AND token_hash = $3",
        )
        .bind(&hashed_account)
        .bind(device_name)
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((expires_at,)) = row {
            Ok(expires_at >= Utc::now().timestamp())
        } else {
            Ok(false)
        }
    }

    pub async fn revoke_refresh_tokens(
        &self,
        account_id: &str,
        device_name: &str,
    ) -> AppResult<()> {
        let hashed_account = self.resolve_account_pk(account_id).await?;
        sqlx::query("DELETE FROM refresh_tokens WHERE account_id = $1 AND device_name = $2")
            .bind(&hashed_account)
            .bind(device_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_devices(&self, account_id: &str) -> AppResult<Vec<Device>> {
        let hashed = self.resolve_account_pk(account_id).await?;
        let rows: Vec<(String, i64, Option<String>)> = sqlx::query_as(
            "SELECT name, added_at, attestation_pubkey FROM devices WHERE account_id = $1",
        )
        .bind(&hashed)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(name, added, attestation_pubkey)| Device {
                id: None,
                account_id: account_id.to_string(),
                name,
                added_at: added,
                attestation_pubkey,
            })
            .collect())
    }

    pub async fn remove_device(&self, account_id: &str, name: &str) -> AppResult<bool> {
        let hashed = self.resolve_account_pk(account_id).await?;
        let res = sqlx::query("DELETE FROM devices WHERE account_id = $1 AND name = $2")
            .bind(&hashed)
            .bind(name)
            .execute(&self.pool)
            .await?;
        let _ = self.revoke_refresh_tokens(account_id, name).await;
        Ok(res.rows_affected() > 0)
    }

    pub async fn update_device_pubkey(
        &self,
        account_id: &str,
        name: &str,
        attestation_pubkey: &str,
    ) -> AppResult<bool> {
        let hashed = self.resolve_account_pk(account_id).await?;
        let res = sqlx::query(
            "UPDATE devices SET attestation_pubkey = $1 WHERE account_id = $2 AND name = $3",
        )
        .bind(attestation_pubkey)
        .bind(&hashed)
        .bind(name)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn get_device_pubkey(
        &self,
        account_id: &str,
        name: &str,
    ) -> AppResult<Option<String>> {
        let hashed = self.resolve_account_pk(account_id).await?;
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT attestation_pubkey FROM devices WHERE account_id = $1 AND name = $2",
        )
        .bind(&hashed)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|(pk,)| pk))
    }

    pub async fn update_server_health(
        &self,
        endpoint: &str,
        load: i64,
        latency: i64,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE vpn_servers SET current_load = $1, avg_latency = $2 WHERE endpoint = $3",
        )
        .bind(load)
        .bind(latency)
        .bind(endpoint)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_servers_by_location(&self, country: &str) -> AppResult<Vec<VpnServer>> {
        Ok(sqlx::query_as::<_, VpnServer>(
            "SELECT * FROM vpn_servers WHERE country = $1 AND is_active = true",
        )
        .bind(country)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_active_servers(&self) -> AppResult<Vec<VpnServer>> {
        Ok(
            sqlx::query_as::<_, VpnServer>("SELECT * FROM vpn_servers WHERE is_active = true")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn get_or_create_peer(&self, pub_key: &str) -> AppResult<String> {
        let mut tx = self.pool.begin().await?;

        let existing: Option<(String,)> =
            sqlx::query_as("SELECT assigned_ip FROM peers WHERE pub_key = $1")
                .bind(pub_key)
                .fetch_optional(&mut *tx)
                .await?;

        if let Some((ip,)) = existing {
            tx.commit().await?;
            return Ok(ip);
        }

        let now = Utc::now().timestamp();

        let insert_result = sqlx::query_scalar::<_, i64>(
            "INSERT INTO peers (pub_key, registered_at) VALUES ($1, $2) RETURNING id",
        )
        .bind(pub_key)
        .bind(now)
        .fetch_one(&mut *tx)
        .await;

        let row_id = match insert_result {
            Ok(id) => id,
            Err(Error::Database(db_err)) if db_err.is_unique_violation() => {
                tx.rollback().await?;
                let existing: (String,) =
                    sqlx::query_as("SELECT assigned_ip FROM peers WHERE pub_key = $1")
                        .bind(pub_key)
                        .fetch_one(&self.pool)
                        .await?;
                return Ok(existing.0);
            }
            Err(e) => {
                tx.rollback().await?;
                return Err(e.into());
            }
        };

        let mut allocated = false;
        let mut assigned_ip = String::new();

        for offset in 0..10 {
            let candidate_id = row_id.wrapping_add(offset);
            let candidate_ip = Self::map_id_to_ip(candidate_id);

            match sqlx::query("UPDATE peers SET assigned_ip = $1 WHERE id = $2")
                .bind(&candidate_ip)
                .bind(row_id)
                .execute(&mut *tx)
                .await
            {
                Ok(_) => {
                    allocated = true;
                    assigned_ip = candidate_ip;
                    if offset > 0 {
                        info!(
                            "Allocated IP {} with offset {} due to collision",
                            assigned_ip, offset
                        );
                    } else {
                        info!("Allocating anonymous IP {} for public key", assigned_ip);
                    }
                    break;
                }
                Err(Error::Database(db_err)) if db_err.is_unique_violation() => {
                    tracing::warn!("IP collision for {}, retrying...", candidate_ip);
                    continue;
                }
                Err(e) => {
                    tx.rollback().await?;
                    return Err(e.into());
                }
            }
        }

        if !allocated {
            tx.rollback().await?;
            return Err(AppError::BadRequest(
                "Failed to allocate IP address: Pool saturated or high collision rate".to_string(),
            ));
        }

        tx.commit().await?;
        Ok(assigned_ip)
    }

    pub async fn panic_wipe(&self) -> AppResult<()> {
        info!("CRITICAL: Panic wipe triggered. Clearing all ephemeral session data.");
        sqlx::query("DELETE FROM peers").execute(&self.pool).await?;
        sqlx::query("DELETE FROM used_tokens")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM attestation_ids")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM refresh_tokens")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
