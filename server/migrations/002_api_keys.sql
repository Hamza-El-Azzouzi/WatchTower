-- Migration: API Keys for Authentication
-- This migration adds support for API key authentication

CREATE TABLE IF NOT EXISTS api_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    expires_at TIMESTAMP,
    last_used_at TIMESTAMP,
    revoked BOOLEAN NOT NULL DEFAULT 0,
    revoked_at TIMESTAMP,
    created_by TEXT DEFAULT 'system'
);

CREATE INDEX IF NOT EXISTS idx_api_keys_key ON api_keys(key) WHERE revoked = 0;
CREATE INDEX IF NOT EXISTS idx_api_keys_active ON api_keys(revoked, expires_at);
