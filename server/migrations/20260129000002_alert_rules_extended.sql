-- Extend alert_rules table to store full AlertRule structure
-- Add missing columns that the Rust AlertRule struct needs

-- Add metric column
ALTER TABLE alert_rules ADD COLUMN IF NOT EXISTS metric TEXT;

-- Add threshold columns
ALTER TABLE alert_rules ADD COLUMN IF NOT EXISTS threshold DOUBLE PRECISION DEFAULT 0;
ALTER TABLE alert_rules ADD COLUMN IF NOT EXISTS threshold_percent DOUBLE PRECISION;

-- Add rule_id (the string ID used in code, separate from database serial ID)
ALTER TABLE alert_rules ADD COLUMN IF NOT EXISTS rule_id TEXT UNIQUE;

-- Add channels as JSON array
ALTER TABLE alert_rules ADD COLUMN IF NOT EXISTS channels JSONB DEFAULT '[]'::jsonb;

-- Add agent_filter for filtering specific agents
ALTER TABLE alert_rules ADD COLUMN IF NOT EXISTS agent_filter TEXT;

-- Add cooldown_seconds
ALTER TABLE alert_rules ADD COLUMN IF NOT EXISTS cooldown_seconds INTEGER DEFAULT 0;

-- Rename condition to condition_expr if it exists (idempotent)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'alert_rules' AND column_name = 'condition') THEN
        ALTER TABLE alert_rules RENAME COLUMN condition TO condition_expr;
    END IF;
END $$;

-- Ensure condition_expr column is nullable (legacy column, no longer required)
ALTER TABLE alert_rules ALTER COLUMN condition_expr DROP NOT NULL;

-- Add condition_type column
ALTER TABLE alert_rules ADD COLUMN IF NOT EXISTS condition_type TEXT DEFAULT 'greater_than';

-- Update check constraint for severity to be case-insensitive
ALTER TABLE alert_rules DROP CONSTRAINT IF EXISTS alert_rules_severity_check;
ALTER TABLE alert_rules ADD CONSTRAINT alert_rules_severity_check 
    CHECK (LOWER(severity) IN ('info', 'warning', 'critical'));

-- Extend alerts table to store full Alert structure
ALTER TABLE alerts ADD COLUMN IF NOT EXISTS rule_id TEXT;
ALTER TABLE alerts ADD COLUMN IF NOT EXISTS agent_name TEXT;
ALTER TABLE alerts ADD COLUMN IF NOT EXISTS metric TEXT;
ALTER TABLE alerts ADD COLUMN IF NOT EXISTS current_value DOUBLE PRECISION;
ALTER TABLE alerts ADD COLUMN IF NOT EXISTS threshold DOUBLE PRECISION;
ALTER TABLE alerts ADD COLUMN IF NOT EXISTS condition_type TEXT;
ALTER TABLE alerts ADD COLUMN IF NOT EXISTS acknowledged BOOLEAN DEFAULT false;
ALTER TABLE alerts ADD COLUMN IF NOT EXISTS acknowledged_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE alerts ADD COLUMN IF NOT EXISTS acknowledged_by TEXT;
ALTER TABLE alerts ADD COLUMN IF NOT EXISTS last_notification_at TIMESTAMP WITH TIME ZONE;

-- Update alerts state check to include 'pending'
ALTER TABLE alerts DROP CONSTRAINT IF EXISTS alerts_state_check;
ALTER TABLE alerts ADD CONSTRAINT alerts_state_check 
    CHECK (LOWER(state) IN ('pending', 'firing', 'resolved'));

-- Create indexes for new columns
CREATE INDEX IF NOT EXISTS idx_alert_rules_rule_id ON alert_rules(rule_id);
CREATE INDEX IF NOT EXISTS idx_alert_rules_enabled ON alert_rules(enabled);
CREATE INDEX IF NOT EXISTS idx_alerts_rule_id ON alerts(rule_id);
CREATE INDEX IF NOT EXISTS idx_alerts_acknowledged ON alerts(acknowledged);
