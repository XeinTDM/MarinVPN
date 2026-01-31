use sqlx::sqlite::SqlitePool;
use crate::models::{Account, Device, VpnServer};
use crate::error::{AppResult};
use chrono::Utc;
use tracing::info;
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2, Algorithm, Version, Params
};

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
    ephemeral_pool: SqlitePool,
    salt: String,
}

impl Database {
    fn hash_account(&self, account_number: &str) -> String {
        let salt = SaltString::encode_b64(self.salt.as_bytes()).unwrap();
        
        let params = Params::new(15360, 2, 1, None).unwrap();
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        
        let hash = argon2.hash_password(account_number.as_bytes(), &salt)
            .expect("Failed to hash account number")
            .to_string();
        
        hash
    }

    pub async fn new(url: &str, salt: &str) -> AppResult<Self> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(25)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .idle_timeout(std::time::Duration::from_secs(600))
            .connect(url).await?;
        
        sqlx::query("PRAGMA journal_mode=WAL;").execute(&pool).await?;
        sqlx::query("PRAGMA synchronous=NORMAL;").execute(&pool).await?;

        let ephemeral_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(25)
            .connect("sqlite::memory:").await?;

        sqlx::migrate!("./migrations").run(&pool).await?;
        
        sqlx::query("CREATE TABLE IF NOT EXISTS peers (id INTEGER PRIMARY KEY, pub_key TEXT UNIQUE, assigned_ip TEXT UNIQUE, registered_at INTEGER)")
            .execute(&ephemeral_pool).await?;
        
        sqlx::query("CREATE TABLE IF NOT EXISTS used_tokens (message TEXT PRIMARY KEY, used_at INTEGER)")
            .execute(&ephemeral_pool).await?;

        Ok(Self { pool, ephemeral_pool, salt: salt.to_string() })
    }

    pub async fn cleanup_stale_sessions(&self, max_age_secs: i64) -> AppResult<Vec<String>> {
        let cutoff = Utc::now().timestamp() - max_age_secs;
        
        let stale_peers: Vec<(String,)> = sqlx::query_as("SELECT pub_key FROM peers WHERE registered_at < ?")
            .bind(cutoff)
            .fetch_all(&self.ephemeral_pool).await?;
        
        let pub_keys: Vec<String> = stale_peers.into_iter().map(|(pk,)| pk).collect();

        if !pub_keys.is_empty() {
            info!("Cleaning up {} stale VPN sessions from ephemeral storage", pub_keys.len());
            sqlx::query("DELETE FROM peers WHERE registered_at < ?")
                .bind(cutoff)
                .execute(&self.ephemeral_pool).await?;
            
            sqlx::query("DELETE FROM used_tokens WHERE used_at < ?")
                .bind(cutoff)
                .execute(&self.ephemeral_pool).await?;
        }

        Ok(pub_keys)
    }

    pub async fn is_token_used(&self, message: &str) -> AppResult<bool> {
        let row: Option<(String,)> = sqlx::query_as("SELECT message FROM used_tokens WHERE message = ?")
            .bind(message)
            .fetch_optional(&self.ephemeral_pool).await?;
        Ok(row.is_some())
    }

    pub async fn mark_token_used(&self, message: &str) -> AppResult<()> {
        let now = Utc::now().timestamp();
        sqlx::query("INSERT INTO used_tokens (message, used_at) VALUES (?, ?)")
            .bind(message)
            .bind(now)
            .execute(&self.ephemeral_pool).await?;
        Ok(())
    }

    pub async fn create_account(&self, account_number: &str, expiry_days: i64) -> AppResult<Account> {
        let now = Utc::now().timestamp();
        let expiry = now + (expiry_days * 24 * 60 * 60);
        let hashed = self.hash_account(account_number);
        
        let account = Account {
            account_number: account_number.to_string(),
            expiry_date: expiry,
            created_at: now,
        };

        sqlx::query("INSERT INTO accounts (account_number, expiry_date, created_at) VALUES (?, ?, ?)")
            .bind(&hashed)
            .bind(account.expiry_date)
            .bind(account.created_at)
            .execute(&self.pool).await?;

        Ok(account)
    }

    pub async fn get_account(&self, account_number: &str) -> AppResult<Option<Account>> {
        let hashed = self.hash_account(account_number);
        let row: Option<(i64, i64)> = sqlx::query_as("SELECT expiry_date, created_at FROM accounts WHERE account_number = ?")
            .bind(&hashed)
            .fetch_optional(&self.pool).await?;

        Ok(row.map(|(expiry, created)| Account {
            account_number: account_number.to_string(),
            expiry_date: expiry,
            created_at: created,
        }))
    }

    pub async fn add_device(&self, account_id: &str, name: &str) -> AppResult<Device> {
        let now = Utc::now().timestamp();
        let hashed = self.hash_account(account_id);
        sqlx::query("INSERT INTO devices (account_id, name, added_at) VALUES (?, ?, ?)")
            .bind(&hashed)
            .bind(name)
            .bind(now)
            .execute(&self.pool).await?;

        Ok(Device { id: None, account_id: account_id.to_string(), name: name.to_string(), added_at: now })
    }

    pub async fn get_devices(&self, account_id: &str) -> AppResult<Vec<Device>> {
        let hashed = self.hash_account(account_id);
        let rows: Vec<(String, i64)> = sqlx::query_as("SELECT name, added_at FROM devices WHERE account_id = ?")
            .bind(&hashed)
            .fetch_all(&self.pool).await?;

        Ok(rows.into_iter().map(|(name, added)| Device {
            id: None,
            account_id: account_id.to_string(),
            name,
            added_at: added,
        }).collect())
    }

    pub async fn remove_device(&self, account_id: &str, name: &str) -> AppResult<bool> {
        let hashed = self.hash_account(account_id);
        let res = sqlx::query("DELETE FROM devices WHERE account_id = ? AND name = ?")
            .bind(&hashed)
            .bind(name)
            .execute(&self.pool).await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn update_server_health(&self, endpoint: &str, load: i64, latency: i64) -> AppResult<()> {
        sqlx::query("UPDATE vpn_servers SET current_load = ?, avg_latency = ? WHERE endpoint = ?")
            .bind(load)
            .bind(latency)
            .bind(endpoint)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_servers_by_location(&self, country: &str) -> AppResult<Vec<VpnServer>> {
        Ok(sqlx::query_as::<_, VpnServer>("SELECT * FROM vpn_servers WHERE country = ? AND is_active = 1")
            .bind(country)
            .fetch_all(&self.pool).await?)
    }

    pub async fn get_active_servers(&self) -> AppResult<Vec<VpnServer>> {
        Ok(sqlx::query_as::<_, VpnServer>("SELECT * FROM vpn_servers WHERE is_active = 1")
            .fetch_all(&self.pool).await?)
    }

    pub async fn get_or_create_peer(&self, pub_key: &str) -> AppResult<String> {
        let mut tx = self.ephemeral_pool.begin().await?;

        let existing: Option<(String,)> = sqlx::query_as("SELECT assigned_ip FROM peers WHERE pub_key = ?")
            .bind(pub_key)
            .fetch_optional(&mut *tx).await?;

        if let Some((ip,)) = existing {
            tx.commit().await?;
            return Ok(ip);
        }

        let res: Option<(i64,)> = sqlx::query_as(
            "SELECT (p1.id + 1) FROM peers p1 WHERE NOT EXISTS (SELECT 1 FROM peers p2 WHERE p2.id = p1.id + 1) ORDER BY p1.id LIMIT 1"
        ).fetch_optional(&mut *tx).await?;

        let next_id = res.map(|(id,)| id).unwrap_or(1);
        
        let x = (next_id >> 16) & 0xFF;
        let y = (next_id >> 8) & 0xFF;
        let z = next_id & 0xFF;
        
        let assigned_ip = format!("10.{}.{}.{}/32", x, y, z);

        info!("Allocating anonymous IP {} for public key", assigned_ip);

        let now = Utc::now().timestamp();
        sqlx::query("INSERT INTO peers (id, pub_key, assigned_ip, registered_at) VALUES (?, ?, ?, ?)")
            .bind(next_id)
            .bind(pub_key)
            .bind(&assigned_ip)
            .bind(now)
            .execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(assigned_ip)
    }

    pub async fn panic_wipe(&self) -> AppResult<()> {
        info!("CRITICAL: Panic wipe triggered. Clearing all ephemeral data.");
        sqlx::query("DELETE FROM peers").execute(&self.ephemeral_pool).await?;
        sqlx::query("DELETE FROM used_tokens").execute(&self.ephemeral_pool).await?;
        Ok(())
    }
}
