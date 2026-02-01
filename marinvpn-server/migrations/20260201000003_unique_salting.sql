-- Add security columns for unique salting strategy
-- We use a prefix lookup to allow unique salts without knowing the salt beforehand.

ALTER TABLE accounts ADD COLUMN prefix TEXT;
ALTER TABLE accounts ADD COLUMN salt TEXT;
ALTER TABLE accounts ADD COLUMN verifier TEXT;

CREATE INDEX idx_accounts_prefix ON accounts(prefix);
