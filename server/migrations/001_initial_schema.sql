-- Agents table
CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    last_seen TIMESTAMP NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Metrics table (time-series data)
CREATE TABLE IF NOT EXISTS metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    metric_name TEXT NOT NULL,
    value REAL NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- Index for fast metric queries
CREATE INDEX IF NOT EXISTS idx_metrics_agent_time ON metrics(agent_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_metrics_name_time ON metrics(metric_name, timestamp DESC);

-- Alert rules table
CREATE TABLE IF NOT EXISTS alert_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    metric TEXT NOT NULL,
    condition TEXT NOT NULL,
    threshold REAL NOT NULL,
    threshold_percent REAL,
    duration_seconds INTEGER NOT NULL,
    severity TEXT NOT NULL,
    channels TEXT, -- JSON array
    agent_filter TEXT,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    cooldown_seconds INTEGER NOT NULL DEFAULT 300,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Alerts table
CREATE TABLE IF NOT EXISTS alerts (
    id TEXT PRIMARY KEY,
    rule_id TEXT NOT NULL,
    rule_name TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    state TEXT NOT NULL,
    metric TEXT NOT NULL,
    current_value REAL NOT NULL,
    threshold REAL NOT NULL,
    condition TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    triggered_at TIMESTAMP NOT NULL,
    resolved_at TIMESTAMP,
    acknowledged BOOLEAN NOT NULL DEFAULT 0,
    acknowledged_at TIMESTAMP,
    acknowledged_by TEXT,
    last_notified_at TIMESTAMP,
    FOREIGN KEY (rule_id) REFERENCES alert_rules(id) ON DELETE CASCADE,
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- Index for alert queries
CREATE INDEX IF NOT EXISTS idx_alerts_state ON alerts(state);
CREATE INDEX IF NOT EXISTS idx_alerts_agent ON alerts(agent_id);
CREATE INDEX IF NOT EXISTS idx_alerts_triggered ON alerts(triggered_at DESC);
