-- Production alert delivery, maintenance windows, and correlated incident events.

ALTER TABLE notification_channels
    ALTER COLUMN webhook_url DROP NOT NULL;

ALTER TABLE notification_channels
    ADD COLUMN IF NOT EXISTS channel_type TEXT NOT NULL DEFAULT 'generic_webhook',
    ADD COLUMN IF NOT EXISTS email_to TEXT,
    ADD COLUMN IF NOT EXISTS smtp_host TEXT,
    ADD COLUMN IF NOT EXISTS smtp_port INTEGER NOT NULL DEFAULT 587,
    ADD COLUMN IF NOT EXISTS smtp_username TEXT,
    ADD COLUMN IF NOT EXISTS smtp_password TEXT,
    ADD COLUMN IF NOT EXISTS smtp_from TEXT,
    ADD COLUMN IF NOT EXISTS smtp_tls BOOLEAN NOT NULL DEFAULT true;

ALTER TABLE notification_channels DROP CONSTRAINT IF EXISTS notification_channels_type_check;
ALTER TABLE notification_channels ADD CONSTRAINT notification_channels_type_check
    CHECK (channel_type IN ('generic_webhook', 'slack', 'discord', 'email'));

ALTER TABLE notification_channels DROP CONSTRAINT IF EXISTS notification_channels_destination_check;
ALTER TABLE notification_channels ADD CONSTRAINT notification_channels_destination_check CHECK (
    (channel_type IN ('generic_webhook', 'slack', 'discord') AND webhook_url IS NOT NULL)
    OR
    (channel_type = 'email' AND email_to IS NOT NULL AND smtp_host IS NOT NULL AND smtp_from IS NOT NULL)
);

ALTER TABLE notification_deliveries
    ADD COLUMN IF NOT EXISTS payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS next_attempt_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    ADD COLUMN IF NOT EXISTS last_attempt_at TIMESTAMP WITH TIME ZONE,
    ADD COLUMN IF NOT EXISTS max_attempts INTEGER NOT NULL DEFAULT 5;

CREATE INDEX IF NOT EXISTS idx_notification_deliveries_retry
    ON notification_deliveries(next_attempt_at)
    WHERE status IN ('pending', 'failed');

CREATE INDEX IF NOT EXISTS idx_alert_silences_agent
    ON alert_silences(agent_id, starts_at, ends_at);
CREATE INDEX IF NOT EXISTS idx_alert_silences_rule
    ON alert_silences(rule_id, starts_at, ends_at);

CREATE TABLE IF NOT EXISTS incident_events (
    id BIGSERIAL PRIMARY KEY,
    event_id TEXT UNIQUE NOT NULL,
    alert_id TEXT,
    rule_id TEXT,
    agent_id TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK (
        event_type IN ('pending', 'firing', 'recovery', 'acknowledged', 'process_spike', 'log_error')
    ),
    severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_incident_events_agent_time
    ON incident_events(agent_id, occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_incident_events_alert
    ON incident_events(alert_id, occurred_at DESC);
