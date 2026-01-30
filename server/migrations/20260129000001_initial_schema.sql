-- Admin Users Table
CREATE TABLE IF NOT EXISTS admin_users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    full_name VARCHAR(255),
    email VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_login_at TIMESTAMP WITH TIME ZONE,
    is_active BOOLEAN DEFAULT true
);

-- API Keys Table (with agent limits)
CREATE TABLE IF NOT EXISTS api_keys (
    id BIGSERIAL PRIMARY KEY,
    key TEXT UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    max_agents INTEGER DEFAULT NULL,  -- NULL means unlimited
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by VARCHAR(255) NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE,
    last_used_at TIMESTAMP WITH TIME ZONE,
    revoked BOOLEAN DEFAULT false,
    revoked_at TIMESTAMP WITH TIME ZONE,
    revoked_by VARCHAR(255)
);

-- Create indexes for API keys
CREATE INDEX IF NOT EXISTS idx_api_keys_key ON api_keys(key) WHERE NOT revoked;
CREATE INDEX IF NOT EXISTS idx_api_keys_expires_at ON api_keys(expires_at);
CREATE INDEX IF NOT EXISTS idx_api_keys_revoked ON api_keys(revoked);

-- Agents Table
CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    agent_type TEXT NOT NULL CHECK (agent_type IN ('server', 'database')),
    hostname TEXT,
    ip_address TEXT,
    os TEXT,
    api_key_id INTEGER REFERENCES api_keys(id) ON DELETE SET NULL,
    first_seen TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_seen TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    status TEXT DEFAULT 'healthy' CHECK (status IN ('healthy', 'degraded', 'unreachable'))
);

-- Create indexes for agents
CREATE INDEX IF NOT EXISTS idx_agents_api_key_id ON agents(api_key_id);
CREATE INDEX IF NOT EXISTS idx_agents_last_seen ON agents(last_seen);
CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);

-- Metrics Table (Raw Data - kept for 24 hours)
CREATE TABLE IF NOT EXISTS metrics (
    id BIGSERIAL PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    metric_name TEXT NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    unit TEXT,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for metrics
CREATE INDEX IF NOT EXISTS idx_metrics_agent_timestamp ON metrics(agent_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_metrics_metric_name ON metrics(metric_name);
CREATE INDEX IF NOT EXISTS idx_metrics_timestamp ON metrics(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_metrics_created_at ON metrics(created_at);

-- Aggregated Metrics (1-minute aggregates - kept for 7 days)
CREATE TABLE IF NOT EXISTS metrics_1min (
    id BIGSERIAL PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    metric_name TEXT NOT NULL,
    min_value DOUBLE PRECISION NOT NULL,
    max_value DOUBLE PRECISION NOT NULL,
    avg_value DOUBLE PRECISION NOT NULL,
    sum_value DOUBLE PRECISION NOT NULL,
    count INTEGER NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(agent_id, metric_name, timestamp)
);

-- Create indexes for 1-minute aggregates
CREATE INDEX IF NOT EXISTS idx_metrics_1min_agent_timestamp ON metrics_1min(agent_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_metrics_1min_metric_name ON metrics_1min(metric_name);
CREATE INDEX IF NOT EXISTS idx_metrics_1min_timestamp ON metrics_1min(timestamp DESC);

-- Aggregated Metrics (1-hour aggregates - kept for 30 days)
CREATE TABLE IF NOT EXISTS metrics_1hour (
    id BIGSERIAL PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    metric_name TEXT NOT NULL,
    min_value DOUBLE PRECISION NOT NULL,
    max_value DOUBLE PRECISION NOT NULL,
    avg_value DOUBLE PRECISION NOT NULL,
    sum_value DOUBLE PRECISION NOT NULL,
    count INTEGER NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(agent_id, metric_name, timestamp)
);

-- Create indexes for 1-hour aggregates
CREATE INDEX IF NOT EXISTS idx_metrics_1hour_agent_timestamp ON metrics_1hour(agent_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_metrics_1hour_metric_name ON metrics_1hour(metric_name);
CREATE INDEX IF NOT EXISTS idx_metrics_1hour_timestamp ON metrics_1hour(timestamp DESC);

-- Logs Table
CREATE TABLE IF NOT EXISTS logs (
    id BIGSERIAL PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    level TEXT NOT NULL CHECK (level IN ('DEBUG', 'INFO', 'WARN', 'ERROR', 'FATAL')),
    source TEXT,
    message TEXT NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for logs
CREATE INDEX IF NOT EXISTS idx_logs_agent_timestamp ON logs(agent_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_logs_level ON logs(level);
CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_logs_created_at ON logs(created_at);

-- Alerts Table
CREATE TABLE IF NOT EXISTS alerts (
    id BIGSERIAL PRIMARY KEY,
    alert_id TEXT UNIQUE NOT NULL,
    rule_name TEXT NOT NULL,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    message TEXT NOT NULL,
    triggered_at TIMESTAMP WITH TIME ZONE NOT NULL,
    resolved_at TIMESTAMP WITH TIME ZONE,
    state TEXT NOT NULL CHECK (state IN ('firing', 'resolved')),
    metadata JSONB
);

-- Create indexes for alerts
CREATE INDEX IF NOT EXISTS idx_alerts_agent_id ON alerts(agent_id);
CREATE INDEX IF NOT EXISTS idx_alerts_state ON alerts(state);
CREATE INDEX IF NOT EXISTS idx_alerts_triggered_at ON alerts(triggered_at DESC);

-- Alert Rules Table
CREATE TABLE IF NOT EXISTS alert_rules (
    id BIGSERIAL PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    description TEXT,
    condition TEXT NOT NULL,
    duration_seconds INTEGER NOT NULL DEFAULT 0,
    severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create default admin user (password: admin123 - CHANGE THIS IN PRODUCTION!)
-- Password hash for 'admin123'
INSERT INTO admin_users (username, password_hash, full_name, email) 
VALUES ('admin', '$2b$12$mFYO.MsW/zucU1fIDg/vD.Eqyfso4Phf2YzGGFJZcX7yCyDb/cOIu', 'Administrator', 'admin@monitoring.local')
ON CONFLICT (username) DO NOTHING;
