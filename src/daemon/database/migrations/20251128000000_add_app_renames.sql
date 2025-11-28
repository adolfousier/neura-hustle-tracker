-- Create app_renames table to store persistent app name rename mappings
-- This allows the daemon to apply renames to new sessions automatically
CREATE TABLE IF NOT EXISTS app_renames (
    id SERIAL PRIMARY KEY,
    original_app_name TEXT NOT NULL UNIQUE,
    renamed_app_name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create index on original_app_name for fast lookups
CREATE INDEX IF NOT EXISTS idx_app_renames_original ON app_renames(original_app_name);
