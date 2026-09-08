ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS key_hash TEXT;
ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS key_prefix TEXT;
ALTER TABLE api_keys ALTER COLUMN key DROP NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_api_keys_key_hash
    ON api_keys(key_hash) WHERE key_hash IS NOT NULL;

-- Legacy keys were stored in plaintext and must be rotated. Preserve only a display prefix.
UPDATE api_keys
SET key_prefix = LEFT(key, 12), revoked = true, revoked_at = COALESCE(revoked_at, NOW()), key = NULL
WHERE key_hash IS NULL AND key IS NOT NULL;
