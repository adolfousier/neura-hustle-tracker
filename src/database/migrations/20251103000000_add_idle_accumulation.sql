-- Add idle_accumulation_secs to track accumulated idle time per session
-- This prevents activity percentage from resetting to 100% when user becomes active again
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS idle_accumulation_secs BIGINT DEFAULT 0;

-- Create index for potential future queries
CREATE INDEX IF NOT EXISTS idx_sessions_idle_accumulation ON sessions(idle_accumulation_secs);
