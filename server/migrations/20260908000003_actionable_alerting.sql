CREATE TABLE IF NOT EXISTS notification_channels (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    webhook_url TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS notification_deliveries (
    id BIGSERIAL PRIMARY KEY,
    alert_id TEXT NOT NULL,
    channel_id TEXT REFERENCES notification_channels(id) ON DELETE SET NULL,
    channel_name TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK (event_type IN ('firing', 'resolved', 'test')),
    status TEXT NOT NULL CHECK (status IN ('pending', 'delivered', 'failed')),
    attempt_count INTEGER NOT NULL DEFAULT 0,
    response_status INTEGER,
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    delivered_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS idx_notification_deliveries_created
    ON notification_deliveries(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_notification_deliveries_alert
    ON notification_deliveries(alert_id);

CREATE TABLE IF NOT EXISTS alert_silences (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    reason TEXT,
    rule_id TEXT,
    agent_id TEXT,
    starts_at TIMESTAMP WITH TIME ZONE NOT NULL,
    ends_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_by TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CHECK (ends_at > starts_at)
);

CREATE INDEX IF NOT EXISTS idx_alert_silences_window
    ON alert_silences(starts_at, ends_at);
