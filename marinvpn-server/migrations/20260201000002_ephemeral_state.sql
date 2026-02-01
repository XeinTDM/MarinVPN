-- Persistent shared state for replay protection and peer allocation

CREATE TABLE IF NOT EXISTS peers (
    id BIGSERIAL PRIMARY KEY,
    pub_key TEXT NOT NULL UNIQUE,
    assigned_ip TEXT UNIQUE,
    registered_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS used_tokens (
    message TEXT PRIMARY KEY,
    used_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS attestation_ids (
    id TEXT PRIMARY KEY,
    used_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS refresh_tokens (
    account_id TEXT NOT NULL,
    device_name TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    issued_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    PRIMARY KEY (account_id, device_name)
);

CREATE INDEX IF NOT EXISTS idx_peers_registered_at ON peers(registered_at);
CREATE INDEX IF NOT EXISTS idx_used_tokens_used_at ON used_tokens(used_at);
CREATE INDEX IF NOT EXISTS idx_attestation_ids_used_at ON attestation_ids(used_at);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires_at ON refresh_tokens(expires_at);
