-- Add peers table to track assigned IPs
CREATE TABLE peers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_number TEXT NOT NULL,
    pub_key TEXT NOT NULL,
    assigned_ip TEXT NOT NULL,
    last_seen INTEGER,
    FOREIGN KEY (account_number) REFERENCES accounts (account_number) ON DELETE CASCADE,
    UNIQUE(account_number, pub_key)
);
