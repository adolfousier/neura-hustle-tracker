use anyhow::Result;
use sqlx::postgres::PgPool;
use sqlx::PgPool as Pool;
use crate::models::session::Session;

struct EmbeddedMigration {
    version: i64,
    description: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[EmbeddedMigration] = &[
    EmbeddedMigration {
        version: 20231019000000,
        description: "20231019000000_initial_schema.sql",
        sql: include_str!("migrations/20231019000000_initial_schema.sql"),
    },
    EmbeddedMigration {
        version: 20251020000000,
        description: "20251020000000_add_parsed_data_columns.sql",
        sql: include_str!("migrations/20251020000000_add_parsed_data_columns.sql"),
    },
    EmbeddedMigration {
        version: 20251030000000,
        description: "20251030000000_add_idle_tracking.sql",
        sql: include_str!("migrations/20251030000000_add_idle_tracking.sql"),
    },
    EmbeddedMigration {
        version: 20251103000000,
        description: "20251103000000_add_idle_accumulation.sql",
        sql: include_str!("migrations/20251103000000_add_idle_accumulation.sql"),
    },
    EmbeddedMigration {
        version: 20251128000000,
        description: "20251128000000_add_app_renames.sql",
        sql: include_str!("migrations/20251128000000_add_app_renames.sql"),
    },
];

pub struct Database {
    pool: Pool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;

        Self::run_migrations(&pool).await?;

        Ok(Self { pool })
    }

    async fn run_migrations(pool: &Pool) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS _sqlx_migrations (
                version BIGINT NOT NULL PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMPTZ NOT NULL DEFAULT now(),
                success BOOLEAN NOT NULL,
                checksum BYTEA NOT NULL,
                execution_time BIGINT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await?;

        for migration in MIGRATIONS {
            let already_applied: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = $1)"
            )
            .bind(migration.version)
            .fetch_one(pool)
            .await?;

            if !already_applied {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(migration.sql.as_bytes());
                let checksum = hasher.finalize().to_vec();

                let start = std::time::Instant::now();
                match sqlx::raw_sql(migration.sql).execute(pool).await {
                    Ok(_) => {
                        let execution_time = start.elapsed().as_millis() as i64;
                        sqlx::query(
                            r#"
                            INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
                            VALUES ($1, $2, true, $3, $4)
                            "#,
                        )
                        .bind(migration.version)
                        .bind(migration.description)
                        .bind(&checksum)
                        .bind(execution_time)
                        .execute(pool)
                        .await?;

                        log::info!("Applied migration: {} ({}ms)", migration.description, execution_time);
                    }
                    Err(e) => {
                        sqlx::query(
                            r#"
                            INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
                            VALUES ($1, $2, false, $3, 0)
                            "#,
                        )
                        .bind(migration.version)
                        .bind(migration.description)
                        .bind(&checksum)
                        .execute(pool)
                        .await
                        .ok();

                        return Err(anyhow::anyhow!("Migration failed: {}: {}", migration.description, e));
                    }
                }
            }
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
                parsed_data, parsing_success, is_afk, is_idle, idle_accumulation_secs
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
                $31, $32,
                $33, $34, $35
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
        .bind(session.is_idle)
        .bind(session.idle_accumulation_secs)
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
                browser_page_title_renamed, browser_page_title_category,
                terminal_username, terminal_hostname, terminal_directory, terminal_project_name,
                terminal_directory_renamed, terminal_directory_category,
                editor_filename, editor_filepath, editor_project_path, editor_language,
                editor_filename_renamed, editor_filename_category,
                tmux_window_name, tmux_pane_count, terminal_multiplexer,
                tmux_window_name_renamed, tmux_window_name_category,
                ide_project_name, ide_file_open, ide_workspace,
                parsed_data, parsing_success, is_afk, is_idle, idle_accumulation_secs
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
            "SELECT app_name, SUM(duration)::BIGINT as total_duration FROM sessions WHERE is_afk IS NOT TRUE AND is_idle IS NOT TRUE GROUP BY app_name ORDER BY total_duration DESC",
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

    pub async fn rename_browser_page_title(&self, old_title: &str, new_title: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET browser_page_title_renamed = $1 WHERE browser_page_title = $2")
            .bind(new_title)
            .bind(old_title)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn categorize_browser_page_title(&self, title: &str, category: &str) -> Result<()> {
        let result = sqlx::query("UPDATE sessions SET browser_page_title_category = $1 WHERE browser_page_title = $2")
            .bind(category)
            .bind(title)
            .execute(&self.pool)
            .await?;
        log::info!("Updated browser_page_title_category: title='{}', category='{}', rows_affected={}", title, category, result.rows_affected());
        Ok(())
    }

    pub async fn rename_terminal_directory(&self, old_dir: &str, new_dir: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET terminal_directory_renamed = $1 WHERE terminal_directory = $2")
            .bind(new_dir)
            .bind(old_dir)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn categorize_terminal_directory(&self, dir: &str, category: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET terminal_directory_category = $1 WHERE terminal_directory = $2")
            .bind(category)
            .bind(dir)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn rename_editor_filename(&self, old_filename: &str, new_filename: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET editor_filename_renamed = $1 WHERE editor_filename = $2")
            .bind(new_filename)
            .bind(old_filename)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn categorize_editor_filename(&self, filename: &str, category: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET editor_filename_category = $1 WHERE editor_filename = $2")
            .bind(category)
            .bind(filename)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn rename_tmux_window_name(&self, old_name: &str, new_name: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET tmux_window_name_renamed = $1 WHERE tmux_window_name = $2")
            .bind(new_name)
            .bind(old_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn categorize_tmux_window_name(&self, name: &str, category: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET tmux_window_name_category = $1 WHERE tmux_window_name = $2")
            .bind(category)
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_app_category_by_name(&self, app_name: &str) -> Result<Option<String>> {
        let category: Option<(String,)> = sqlx::query_as("SELECT category FROM sessions WHERE app_name = $1 AND category IS NOT NULL LIMIT 1")
            .bind(app_name)
            .fetch_optional(&self.pool)
            .await?;
        Ok(category.map(|(c,)| c))
    }

    pub async fn set_app_rename(&self, original_app_name: &str, renamed_app_name: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO app_renames (original_app_name, renamed_app_name)
            VALUES ($1, $2)
            ON CONFLICT (original_app_name)
            DO UPDATE SET renamed_app_name = $2, updated_at = CURRENT_TIMESTAMP
            "#
        )
        .bind(original_app_name)
        .bind(renamed_app_name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_app_rename(&self, original_app_name: &str) -> Result<Option<String>> {
        let rename: Option<(String,)> = sqlx::query_as(
            "SELECT renamed_app_name FROM app_renames WHERE original_app_name = $1"
        )
        .bind(original_app_name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(rename.map(|(r,)| r))
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
        // Get local midnight (start of today in local timezone)
        let now = chrono::Local::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_local_timezone(chrono::Local).unwrap();

        let rows: Vec<(String, Option<i64>)> = sqlx::query_as(
            "SELECT app_name, SUM(duration)::bigint as total_duration FROM sessions WHERE start_time >= $1 AND is_afk IS NOT TRUE AND is_idle IS NOT TRUE GROUP BY app_name ORDER BY total_duration DESC"
        )
        .bind(today_start)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(app_name, total_duration)| (app_name, total_duration.unwrap_or(0))).collect())
    }

    pub async fn get_daily_sessions(&self) -> Result<Vec<Session>> {
        // Get local midnight (start of today in local timezone)
        let now = chrono::Local::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_local_timezone(chrono::Local).unwrap();

        let rows = sqlx::query_as::<_, Session>(
            r#"
            SELECT
                id, app_name, window_name, start_time, duration, category,
                browser_url, browser_page_title, browser_notification_count,
                browser_page_title_renamed, browser_page_title_category,
                terminal_username, terminal_hostname, terminal_directory, terminal_project_name,
                terminal_directory_renamed, terminal_directory_category,
                editor_filename, editor_filepath, editor_project_path, editor_language,
                editor_filename_renamed, editor_filename_category,
                tmux_window_name, tmux_pane_count, terminal_multiplexer,
                tmux_window_name_renamed, tmux_window_name_category,
                ide_project_name, ide_file_open, ide_workspace,
                parsed_data, parsing_success, is_afk, is_idle, idle_accumulation_secs
            FROM sessions
            WHERE start_time >= $1 AND is_afk IS NOT TRUE AND is_idle IS NOT TRUE
            ORDER BY start_time DESC
            "#,
        )
        .bind(today_start)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_weekly_sessions(&self) -> Result<Vec<Session>> {
        // Get local midnight 7 days ago (start of week in local timezone)
        let now = chrono::Local::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_local_timezone(chrono::Local).unwrap();
        let week_start = today_start - chrono::Duration::days(6);

        let rows = sqlx::query_as::<_, Session>(
            r#"
            SELECT
                id, app_name, window_name, start_time, duration, category,
                browser_url, browser_page_title, browser_notification_count,
                browser_page_title_renamed, browser_page_title_category,
                terminal_username, terminal_hostname, terminal_directory, terminal_project_name,
                terminal_directory_renamed, terminal_directory_category,
                editor_filename, editor_filepath, editor_project_path, editor_language,
                editor_filename_renamed, editor_filename_category,
                tmux_window_name, tmux_pane_count, terminal_multiplexer,
                tmux_window_name_renamed, tmux_window_name_category,
                ide_project_name, ide_file_open, ide_workspace,
                parsed_data, parsing_success, is_afk, is_idle, idle_accumulation_secs
            FROM sessions
            WHERE start_time >= $1 AND is_afk IS NOT TRUE AND is_idle IS NOT TRUE
            ORDER BY start_time DESC
            "#,
        )
        .bind(week_start)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_monthly_sessions(&self) -> Result<Vec<Session>> {
        // Get local midnight 30 days ago (start of month in local timezone)
        let now = chrono::Local::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_local_timezone(chrono::Local).unwrap();
        let month_start = today_start - chrono::Duration::days(29);

        let rows = sqlx::query_as::<_, Session>(
            r#"
            SELECT
                id, app_name, window_name, start_time, duration, category,
                browser_url, browser_page_title, browser_notification_count,
                browser_page_title_renamed, browser_page_title_category,
                terminal_username, terminal_hostname, terminal_directory, terminal_project_name,
                terminal_directory_renamed, terminal_directory_category,
                editor_filename, editor_filepath, editor_project_path, editor_language,
                editor_filename_renamed, editor_filename_category,
                tmux_window_name, tmux_pane_count, terminal_multiplexer,
                tmux_window_name_renamed, tmux_window_name_category,
                ide_project_name, ide_file_open, ide_workspace,
                parsed_data, parsing_success, is_afk, is_idle, idle_accumulation_secs
            FROM sessions
            WHERE start_time >= $1 AND is_afk IS NOT TRUE AND is_idle IS NOT TRUE
            ORDER BY start_time DESC
            "#,
        )
        .bind(month_start)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
    pub async fn get_custom_categories(&self) -> Result<Vec<String>> {
        let categories: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT category FROM sessions WHERE category IS NOT NULL AND category NOT IN ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind("💻 Development")
        .bind("🌐 Browsing")
        .bind("💬 Communication")
        .bind("🎵 Media")
        .bind("📁 Files")
        .bind("📧 Email")
        .bind("📄 Office")
        .bind("📦 Other")
        .fetch_all(&self.pool)
        .await?;
        Ok(categories.into_iter().map(|(c,)| c).collect())
    }
}


