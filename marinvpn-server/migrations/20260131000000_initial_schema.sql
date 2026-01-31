-- Initial schema for MarinVPN

CREATE TABLE accounts (
    account_number TEXT PRIMARY KEY,
    expiry_date INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE devices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    name TEXT NOT NULL,
    added_at INTEGER NOT NULL,
    FOREIGN KEY (account_id) REFERENCES accounts (account_number) ON DELETE CASCADE,
    UNIQUE(account_id, name)
);

CREATE TABLE vpn_servers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    country TEXT NOT NULL,
    city TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    public_key TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1
);

-- Seed initial servers
INSERT INTO vpn_servers (country, city, endpoint, public_key) VALUES 
('Sweden', 'Stockholm', 'se-sto.marinvpn.net:51820', 'se_pub_key'),
('United States', 'New York', 'us-nyc.marinvpn.net:51820', 'us_pub_key'),
('Germany', 'Frankfurt', 'de-fra.marinvpn.net:51820', 'de_pub_key'),
('United Kingdom', 'London', 'gb-lon.marinvpn.net:51820', 'gb_pub_key'),
('Netherlands', 'Amsterdam', 'nl-ams.marinvpn.net:51820', 'nl_pub_key');
