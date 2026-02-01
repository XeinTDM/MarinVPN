-- Add device attestation public key storage

ALTER TABLE devices ADD COLUMN attestation_pubkey TEXT;

CREATE INDEX IF NOT EXISTS idx_devices_attestation_pubkey ON devices(attestation_pubkey);
