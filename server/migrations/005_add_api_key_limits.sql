-- Migration: Add agent limits and tracking to API keys
-- This adds the missing columns for agent management per API key

-- Add max_agents column (limit on number of agents that can use this key)
ALTER TABLE api_keys ADD COLUMN max_agents INTEGER DEFAULT NULL;

-- Add used_by_agents column (JSON array of agent IDs currently using this key)
ALTER TABLE api_keys ADD COLUMN used_by_agents TEXT DEFAULT '[]' NOT NULL;

-- Create index for searching by agent usage
CREATE INDEX IF NOT EXISTS idx_api_keys_used_by_agents ON api_keys(used_by_agents);
