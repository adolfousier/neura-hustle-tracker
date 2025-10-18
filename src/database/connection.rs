use anyhow::Result;
use sqlx::postgres::PgPool;
use sqlx::PgPool as Pool;
use crate::models::session::Session;

pub struct Database {
    pool: Pool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn create_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id SERIAL PRIMARY KEY,
                app_name TEXT NOT NULL,
                window_name TEXT,
                start_time TIMESTAMP WITH TIME ZONE NOT NULL,
                duration BIGINT NOT NULL,
                category TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Add window_name column if it doesn't exist (for migration)
        sqlx::query(
            r#"
            ALTER TABLE sessions ADD COLUMN IF NOT EXISTS window_name TEXT
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Add category column if it doesn't exist (for migration)
        sqlx::query(
            r#"
            ALTER TABLE sessions ADD COLUMN IF NOT EXISTS category TEXT
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Enhanced tracking columns - Browser
        sqlx::query(
            r#"
            ALTER TABLE sessions ADD COLUMN IF NOT EXISTS browser_url TEXT,
            ADD COLUMN IF NOT EXISTS browser_page_title TEXT,
            ADD COLUMN IF NOT EXISTS browser_notification_count INTEGER
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Enhanced tracking columns - Terminal
        sqlx::query(
            r#"
            ALTER TABLE sessions ADD COLUMN IF NOT EXISTS terminal_username TEXT,
            ADD COLUMN IF NOT EXISTS terminal_hostname TEXT,
            ADD COLUMN IF NOT EXISTS terminal_directory TEXT,
            ADD COLUMN IF NOT EXISTS terminal_project_name TEXT
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Enhanced tracking columns - Editor
        sqlx::query(
            r#"
            ALTER TABLE sessions ADD COLUMN IF NOT EXISTS editor_filename TEXT,
            ADD COLUMN IF NOT EXISTS editor_filepath TEXT,
            ADD COLUMN IF NOT EXISTS editor_project_path TEXT,
            ADD COLUMN IF NOT EXISTS editor_language TEXT
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Enhanced tracking columns - Multiplexer
        sqlx::query(
            r#"
            ALTER TABLE sessions ADD COLUMN IF NOT EXISTS tmux_window_name TEXT,
            ADD COLUMN IF NOT EXISTS tmux_pane_count INTEGER,
            ADD COLUMN IF NOT EXISTS terminal_multiplexer TEXT
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Enhanced tracking columns - IDE
        sqlx::query(
            r#"
            ALTER TABLE sessions ADD COLUMN IF NOT EXISTS ide_project_name TEXT,
            ADD COLUMN IF NOT EXISTS ide_file_open TEXT,
            ADD COLUMN IF NOT EXISTS ide_workspace TEXT
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Metadata columns
        sqlx::query(
            r#"
            ALTER TABLE sessions ADD COLUMN IF NOT EXISTS parsed_data JSONB,
            ADD COLUMN IF NOT EXISTS parsing_success BOOLEAN DEFAULT TRUE
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn insert_session(&self, session: &Session) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO sessions (
                app_name, window_name, start_time, duration, category,
                browser_url, browser_page_title, browser_notification_count,
                terminal_username, terminal_hostname, terminal_directory, terminal_project_name,
                editor_filename, editor_filepath, editor_project_path, editor_language,
                tmux_window_name, tmux_pane_count, terminal_multiplexer,
                ide_project_name, ide_file_open, ide_workspace,
                parsed_data, parsing_success
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8,
                $9, $10, $11, $12,
                $13, $14, $15, $16,
                $17, $18, $19,
                $20, $21, $22,
                $23, $24
            ) RETURNING id
            "#,
        )
        .bind(&session.app_name)
        .bind(&session.window_name)
        .bind(session.start_time)
        .bind(session.duration)
        .bind(&session.category)
        // Browser
        .bind(&session.browser_url)
        .bind(&session.browser_page_title)
        .bind(session.browser_notification_count)
        // Terminal
        .bind(&session.terminal_username)
        .bind(&session.terminal_hostname)
        .bind(&session.terminal_directory)
        .bind(&session.terminal_project_name)
        // Editor
        .bind(&session.editor_filename)
        .bind(&session.editor_filepath)
        .bind(&session.editor_project_path)
        .bind(&session.editor_language)
        // Multiplexer
        .bind(&session.tmux_window_name)
        .bind(session.tmux_pane_count)
        .bind(&session.terminal_multiplexer)
        // IDE
        .bind(&session.ide_project_name)
        .bind(&session.ide_file_open)
        .bind(&session.ide_workspace)
        // Metadata
        .bind(&session.parsed_data)
        .bind(session.parsing_success)
        .fetch_one(&self.pool)
        .await?;
        Ok(id.0)
    }

    pub async fn get_recent_sessions(&self, limit: i64) -> Result<Vec<Session>> {
        let sessions = sqlx::query_as::<_, Session>(
            r#"
            SELECT
                id, app_name, window_name, start_time, duration, category,
                browser_url, browser_page_title, browser_notification_count,
                terminal_username, terminal_hostname, terminal_directory, terminal_project_name,
                editor_filename, editor_filepath, editor_project_path, editor_language,
                tmux_window_name, tmux_pane_count, terminal_multiplexer,
                ide_project_name, ide_file_open, ide_workspace,
                parsed_data, parsing_success
            FROM sessions
            ORDER BY start_time DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(sessions)
    }

    pub async fn get_app_usage(&self) -> Result<Vec<(String, i64)>> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT app_name, SUM(duration)::BIGINT as total_duration FROM sessions GROUP BY app_name ORDER BY total_duration DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn rename_app_with_category(&self, old_name: &str, new_name: &str, category: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET app_name = $1, category = $2 WHERE app_name = $3")
            .bind(new_name)
            .bind(category)
            .bind(old_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_app_category(&self, app_name: &str, category: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET category = $1 WHERE app_name = $2")
            .bind(category)
            .bind(app_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn fix_old_categories(&self) -> Result<()> {
        // Fix any sessions with old category names that should be Development
        sqlx::query("UPDATE sessions SET category = $1 WHERE category IN ($2, $3)")
            .bind("💻 Development")
            .bind("🖥️  Terminal")
            .bind("📝 Editor")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_daily_usage(&self) -> Result<Vec<(String, i64)>> {
        let rows: Vec<(String, Option<i64>)> = sqlx::query_as(
            "SELECT app_name, SUM(duration)::bigint as total_duration FROM sessions WHERE start_time >= date_trunc('day', CURRENT_TIMESTAMP) AND start_time < date_trunc('day', CURRENT_TIMESTAMP) + INTERVAL '1 day' GROUP BY app_name ORDER BY total_duration DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(app_name, total_duration)| (app_name, total_duration.unwrap_or(0))).collect())
    }

    pub async fn get_weekly_usage(&self) -> Result<Vec<(String, i64)>> {
        let rows: Vec<(String, Option<i64>)> = sqlx::query_as(
            "SELECT app_name, SUM(duration)::bigint as total_duration FROM sessions WHERE start_time >= date_trunc('day', CURRENT_TIMESTAMP) - INTERVAL '6 days' GROUP BY app_name ORDER BY total_duration DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(app_name, total_duration)| (app_name, total_duration.unwrap_or(0))).collect())
    }

    pub async fn get_monthly_usage(&self) -> Result<Vec<(String, i64)>> {
        let rows: Vec<(String, Option<i64>)> = sqlx::query_as(
            "SELECT app_name, SUM(duration)::bigint as total_duration FROM sessions WHERE start_time >= date_trunc('day', CURRENT_TIMESTAMP) - INTERVAL '29 days' GROUP BY app_name ORDER BY total_duration DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(app_name, total_duration)| (app_name, total_duration.unwrap_or(0))).collect())
    }

    pub async fn get_daily_sessions(&self) -> Result<Vec<Session>> {
        let rows = sqlx::query_as::<_, Session>(
            r#"
            SELECT
                id, app_name, window_name, start_time, duration, category,
                browser_url, browser_page_title, browser_notification_count,
                terminal_username, terminal_hostname, terminal_directory, terminal_project_name,
                editor_filename, editor_filepath, editor_project_path, editor_language,
                tmux_window_name, tmux_pane_count, terminal_multiplexer,
                ide_project_name, ide_file_open, ide_workspace,
                parsed_data, parsing_success
            FROM sessions
            WHERE start_time >= date_trunc('day', CURRENT_TIMESTAMP)
            AND start_time < date_trunc('day', CURRENT_TIMESTAMP) + INTERVAL '1 day'
            ORDER BY start_time DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_weekly_sessions(&self) -> Result<Vec<Session>> {
        let rows = sqlx::query_as::<_, Session>(
            r#"
            SELECT
                id, app_name, window_name, start_time, duration, category,
                browser_url, browser_page_title, browser_notification_count,
                terminal_username, terminal_hostname, terminal_directory, terminal_project_name,
                editor_filename, editor_filepath, editor_project_path, editor_language,
                tmux_window_name, tmux_pane_count, terminal_multiplexer,
                ide_project_name, ide_file_open, ide_workspace,
                parsed_data, parsing_success
            FROM sessions
            WHERE start_time >= date_trunc('day', CURRENT_TIMESTAMP) - INTERVAL '6 days'
            ORDER BY start_time DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_monthly_sessions(&self) -> Result<Vec<Session>> {
        let rows = sqlx::query_as::<_, Session>(
            r#"
            SELECT
                id, app_name, window_name, start_time, duration, category,
                browser_url, browser_page_title, browser_notification_count,
                terminal_username, terminal_hostname, terminal_directory, terminal_project_name,
                editor_filename, editor_filepath, editor_project_path, editor_language,
                tmux_window_name, tmux_pane_count, terminal_multiplexer,
                ide_project_name, ide_file_open, ide_workspace,
                parsed_data, parsing_success
            FROM sessions
            WHERE start_time >= date_trunc('day', CURRENT_TIMESTAMP) - INTERVAL '29 days'
            ORDER BY start_time DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

