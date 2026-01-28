-- Migration: Admin Users for Dashboard Access
-- Separate admin authentication from API keys

CREATE TABLE IF NOT EXISTS admin_users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    email TEXT,
    full_name TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_login_at TIMESTAMP,
    is_active BOOLEAN NOT NULL DEFAULT 1
);

-- Create default admin user (password: admin123 - CHANGE IN PRODUCTION!)
-- bcrypt hash for 'admin123'
INSERT INTO admin_users (username, password_hash, email, full_name) 
VALUES ('admin', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5NU7vL7i2Kzoa', 'admin@example.com', 'System Administrator');

CREATE INDEX IF NOT EXISTS idx_admin_users_username ON admin_users(username) WHERE is_active = 1;
