-- Migration: Add agent usage tracking to API keys
-- Track which agents use each key and enforce limits

-- Note: This migration is now a no-op as the columns already exist
-- The schema was manually updated. This file is kept for version tracking.
-- If running on a fresh database, these columns will be created in the initial schema.

-- No-op query to make migration succeed
SELECT 1;
