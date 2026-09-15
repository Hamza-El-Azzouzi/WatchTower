-- api_keys.id is BIGSERIAL (BIGINT), so the referencing column must use the
-- same type. Reading the previous INTEGER column as i64 caused SQLx to panic
-- on authenticated requests once an agent row already existed.
ALTER TABLE agents
    DROP CONSTRAINT IF EXISTS agents_api_key_id_fkey;

ALTER TABLE agents
    ALTER COLUMN api_key_id TYPE BIGINT
    USING api_key_id::BIGINT;

ALTER TABLE agents
    ADD CONSTRAINT agents_api_key_id_fkey
    FOREIGN KEY (api_key_id)
    REFERENCES api_keys(id)
    ON DELETE SET NULL;
