-- Add is_idle column to track sessions with 10+ minutes of no input
-- IDLE = AFK session that lasted 10+ minutes with zero keyboard/mouse activity
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS is_idle BOOLEAN DEFAULT FALSE;

-- Create index for faster queries filtering out idle time
CREATE INDEX IF NOT EXISTS idx_sessions_is_idle ON sessions(is_idle);
