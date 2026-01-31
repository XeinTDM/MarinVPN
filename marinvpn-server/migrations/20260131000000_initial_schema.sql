-- Initial schema for MarinVPN

CREATE TABLE IF NOT EXISTS accounts (
    account_number TEXT PRIMARY KEY,
    expiry_date INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS devices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    name TEXT NOT NULL,
    added_at INTEGER NOT NULL,
    FOREIGN KEY (account_id) REFERENCES accounts (account_number) ON DELETE CASCADE,
    UNIQUE(account_id, name)
);

CREATE TABLE IF NOT EXISTS vpn_servers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    country TEXT NOT NULL,
    city TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    public_key TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
        current_load INTEGER NOT NULL DEFAULT 0,
        avg_latency INTEGER NOT NULL DEFAULT 0
    );
    
    CREATE INDEX IF NOT EXISTS idx_devices_account ON devices(account_id);
    CREATE INDEX IF NOT EXISTS idx_servers_location ON vpn_servers(country, is_active);
    