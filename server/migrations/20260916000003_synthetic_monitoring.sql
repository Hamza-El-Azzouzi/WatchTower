ALTER TABLE agents DROP CONSTRAINT agents_agent_type_check;
ALTER TABLE agents ADD CONSTRAINT agents_agent_type_check CHECK(agent_type IN ('server','database','synthetic'));
CREATE TABLE synthetic_checks (
 id TEXT PRIMARY KEY,
 owner_api_key_id BIGINT NOT NULL REFERENCES api_keys(id),
 agent_id TEXT NOT NULL UNIQUE REFERENCES agents(id) ON DELETE CASCADE,
 spec JSONB NOT NULL,
 enabled BOOLEAN NOT NULL DEFAULT true,
 deleted_at TIMESTAMPTZ,
 next_run_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 lease_token TEXT,
 leased_until TIMESTAMPTZ,
 consecutive_failures INTEGER NOT NULL DEFAULT 0,
 active_alert_id TEXT,
 last_result JSONB,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX synthetic_checks_due_idx ON synthetic_checks(next_run_at) WHERE enabled;
CREATE INDEX synthetic_checks_owner_idx ON synthetic_checks(owner_api_key_id);
CREATE TABLE synthetic_results (
 id BIGSERIAL PRIMARY KEY,
 check_id TEXT NOT NULL REFERENCES synthetic_checks(id) ON DELETE CASCADE,
 result JSONB NOT NULL,
 checked_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX synthetic_results_history_idx ON synthetic_results(check_id,id DESC);
