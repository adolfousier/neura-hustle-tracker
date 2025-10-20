CREATE TABLE IF NOT EXISTS sessions (
    id SERIAL PRIMARY KEY,
    app_name TEXT NOT NULL,
    window_name TEXT,
    start_time TIMESTAMP WITH TIME ZONE NOT NULL,
    duration BIGINT NOT NULL,
    category TEXT
);

ALTER TABLE sessions ADD COLUMN IF NOT EXISTS window_name TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS category TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS browser_url TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS browser_page_title TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS browser_notification_count INTEGER;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS terminal_username TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS terminal_hostname TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS terminal_directory TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS terminal_project_name TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS editor_filename TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS editor_filepath TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS editor_project_path TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS editor_language TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS tmux_window_name TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS tmux_pane_count INTEGER;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS terminal_multiplexer TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS ide_project_name TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS ide_file_open TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS ide_workspace TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS browser_page_title_renamed TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS browser_page_title_category TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS terminal_directory_renamed TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS terminal_directory_category TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS editor_filename_renamed TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS editor_filename_category TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS tmux_window_name_renamed TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS tmux_window_name_category TEXT;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS is_afk BOOLEAN DEFAULT FALSE;

ALTER TABLE sessions ADD COLUMN IF NOT EXISTS parsed_data JSONB;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS parsing_success BOOLEAN;
