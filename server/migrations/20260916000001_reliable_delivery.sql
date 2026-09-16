CREATE TABLE agent_delivery_streams (
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    stream_id TEXT NOT NULL,
    last_sequence BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (agent_id, stream_id)
);
ALTER TABLE agents ADD COLUMN IF NOT EXISTS delivery_status JSONB NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE agents ADD COLUMN IF NOT EXISTS last_successful_upload TIMESTAMPTZ;
ALTER TABLE agents ADD COLUMN IF NOT EXISTS last_heartbeat_at TIMESTAMPTZ;
CREATE TABLE agent_enrollment_tokens (
 token_hash TEXT PRIMARY KEY, agent_id TEXT NOT NULL, expires_at TIMESTAMPTZ NOT NULL,
 consumed_at TIMESTAMPTZ, created_by TEXT NOT NULL
);
CREATE TABLE agent_key_rotations (
 agent_id TEXT PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
 old_key_id BIGINT NOT NULL REFERENCES api_keys(id), new_key_id BIGINT NOT NULL REFERENCES api_keys(id),
 expires_at TIMESTAMPTZ NOT NULL
);
CREATE TABLE agent_signed_configs (
 agent_id TEXT PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
 revision BIGINT NOT NULL, envelope JSONB NOT NULL, updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_by TEXT NOT NULL
);
