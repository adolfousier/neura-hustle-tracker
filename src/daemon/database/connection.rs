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



    pub async fn get_browser_page_title_rename(&self, title: &str) -> Result<Option<String>> {
        let renamed: Option<(String,)> = sqlx::query_as(
            "SELECT browser_page_title_renamed FROM sessions WHERE browser_page_title = $1 AND browser_page_title_renamed IS NOT NULL LIMIT 1"
        )
        .bind(title)
        .fetch_optional(&self.pool)
        .await?;
        Ok(renamed.map(|(r,)| r))
    }

    pub async fn get_browser_page_title_category(&self, title: &str) -> Result<Option<String>> {
        let category: Option<(String,)> = sqlx::query_as(
            "SELECT browser_page_title_category FROM sessions WHERE browser_page_title = $1 AND browser_page_title_category IS NOT NULL LIMIT 1"
        )
        .bind(title)
        .fetch_optional(&self.pool)
        .await?;
        Ok(category.map(|(c,)| c))
    }

    pub async fn get_terminal_directory_rename(&self, dir: &str) -> Result<Option<String>> {
        let renamed: Option<(String,)> = sqlx::query_as(
            "SELECT terminal_directory_renamed FROM sessions WHERE terminal_directory = $1 AND terminal_directory_renamed IS NOT NULL LIMIT 1"
        )
        .bind(dir)
        .fetch_optional(&self.pool)
        .await?;
        Ok(renamed.map(|(r,)| r))
    }

    pub async fn get_terminal_directory_category(&self, dir: &str) -> Result<Option<String>> {
        let category: Option<(String,)> = sqlx::query_as(
            "SELECT terminal_directory_category FROM sessions WHERE terminal_directory = $1 AND terminal_directory_category IS NOT NULL LIMIT 1"
        )
        .bind(dir)
        .fetch_optional(&self.pool)
        .await?;
        Ok(category.map(|(c,)| c))
    }

    pub async fn get_editor_filename_rename(&self, filename: &str) -> Result<Option<String>> {
        let renamed: Option<(String,)> = sqlx::query_as(
            "SELECT editor_filename_renamed FROM sessions WHERE editor_filename = $1 AND editor_filename_renamed IS NOT NULL LIMIT 1"
        )
        .bind(filename)
        .fetch_optional(&self.pool)
        .await?;
        Ok(renamed.map(|(r,)| r))
    }

    pub async fn get_editor_filename_category(&self, filename: &str) -> Result<Option<String>> {
        let category: Option<(String,)> = sqlx::query_as(
            "SELECT editor_filename_category FROM sessions WHERE editor_filename = $1 AND editor_filename_category IS NOT NULL LIMIT 1"
        )
        .bind(filename)
        .fetch_optional(&self.pool)
        .await?;
        Ok(category.map(|(c,)| c))
    }

    pub async fn get_tmux_window_name_rename(&self, name: &str) -> Result<Option<String>> {
        let renamed: Option<(String,)> = sqlx::query_as(
            "SELECT tmux_window_name_renamed FROM sessions WHERE tmux_window_name = $1 AND tmux_window_name_renamed IS NOT NULL LIMIT 1"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(renamed.map(|(r,)| r))
    }

    pub async fn get_tmux_window_name_category(&self, name: &str) -> Result<Option<String>> {
        let category: Option<(String,)> = sqlx::query_as(
            "SELECT tmux_window_name_category FROM sessions WHERE tmux_window_name = $1 AND tmux_window_name_category IS NOT NULL LIMIT 1"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(category.map(|(c,)| c))
    }

    pub async fn apply_renames_and_categories(&self, session: &mut Session) -> Result<()> {
        if let Some(title) = &session.browser_page_title {
            session.browser_page_title_renamed = self.get_browser_page_title_rename(title).await?;
            session.browser_page_title_category = self.get_browser_page_title_category(title).await?;
        }
        if let Some(dir) = &session.terminal_directory {
            session.terminal_directory_renamed = self.get_terminal_directory_rename(dir).await?;
            session.terminal_directory_category = self.get_terminal_directory_category(dir).await?;
        }
        if let Some(filename) = &session.editor_filename {
            session.editor_filename_renamed = self.get_editor_filename_rename(filename).await?;
            session.editor_filename_category = self.get_editor_filename_category(filename).await?;
        }
        if let Some(name) = &session.tmux_window_name {
            session.tmux_window_name_renamed = self.get_tmux_window_name_rename(name).await?;
            session.tmux_window_name_category = self.get_tmux_window_name_category(name).await?;
        }
        Ok(())
    }

    pub async fn insert_session(&self, session: &Session) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO sessions (
                app_name, window_name, start_time, duration, category,
                browser_url, browser_page_title, browser_notification_count,
                browser_page_title_renamed, browser_page_title_category,
                terminal_username, terminal_hostname, terminal_directory, terminal_project_name,
                terminal_directory_renamed, terminal_directory_category,
                editor_filename, editor_filepath, editor_project_path, editor_language,
                editor_filename_renamed, editor_filename_category,
                tmux_window_name, tmux_pane_count, terminal_multiplexer,
                tmux_window_name_renamed, tmux_window_name_category,
                ide_project_name, ide_file_open, ide_workspace,
                parsed_data, parsing_success, is_afk
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8,
                $9, $10,
                $11, $12, $13, $14,
                $15, $16,
                $17, $18, $19, $20,
                $21, $22,
                $23, $24, $25,
                $26, $27,
                $28, $29, $30,
                $31, $32, $33
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
        .bind(&session.browser_page_title_renamed)
        .bind(&session.browser_page_title_category)
        // Terminal
        .bind(&session.terminal_username)
        .bind(&session.terminal_hostname)
        .bind(&session.terminal_directory)
        .bind(&session.terminal_project_name)
        .bind(&session.terminal_directory_renamed)
        .bind(&session.terminal_directory_category)
        // Editor
        .bind(&session.editor_filename)
        .bind(&session.editor_filepath)
        .bind(&session.editor_project_path)
        .bind(&session.editor_language)
        .bind(&session.editor_filename_renamed)
        .bind(&session.editor_filename_category)
        // Multiplexer
        .bind(&session.tmux_window_name)
        .bind(session.tmux_pane_count)
        .bind(&session.terminal_multiplexer)
        .bind(&session.tmux_window_name_renamed)
        .bind(&session.tmux_window_name_category)
        // IDE
        .bind(&session.ide_project_name)
        .bind(&session.ide_file_open)
        .bind(&session.ide_workspace)
        // Metadata
        .bind(&session.parsed_data)
        .bind(session.parsing_success)
        // AFK tracking
        .bind(session.is_afk)
        .fetch_one(&self.pool)
        .await?;
        Ok(id.0)
    }
}
