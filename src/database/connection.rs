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
        Ok(())
    }

    pub async fn insert_session(&self, session: &Session) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO sessions (app_name, window_name, start_time, duration, category) VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(&session.app_name)
        .bind(&session.window_name)
        .bind(session.start_time)
        .bind(session.duration)
        .bind(&session.category)
        .fetch_one(&self.pool)
        .await?;
        Ok(id.0)
    }

    pub async fn get_recent_sessions(&self, limit: i64) -> Result<Vec<Session>> {
        let sessions = sqlx::query_as::<_, Session>(
            "SELECT id, app_name, window_name, start_time, duration, category FROM sessions ORDER BY start_time DESC LIMIT $1",
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

    pub async fn update_session_duration(&self, session_id: i32, new_duration: i64) -> Result<()> {
        sqlx::query("UPDATE sessions SET duration = $1 WHERE id = $2")
            .bind(new_duration)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_daily_usage(&self) -> Result<Vec<(String, i64)>> {
        let rows = sqlx::query!(
            "SELECT app_name, SUM(duration)::bigint as total_duration FROM sessions WHERE start_time >= date_trunc('day', CURRENT_TIMESTAMP) AND start_time < date_trunc('day', CURRENT_TIMESTAMP) + INTERVAL '1 day' GROUP BY app_name ORDER BY total_duration DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| (r.app_name, r.total_duration.unwrap_or(0))).collect())
    }

    pub async fn get_weekly_usage(&self) -> Result<Vec<(String, i64)>> {
        let rows = sqlx::query!(
            "SELECT app_name, SUM(duration)::bigint as total_duration FROM sessions WHERE start_time >= date_trunc('day', CURRENT_TIMESTAMP) - INTERVAL '6 days' GROUP BY app_name ORDER BY total_duration DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| (r.app_name, r.total_duration.unwrap_or(0))).collect())
    }

    pub async fn get_monthly_usage(&self) -> Result<Vec<(String, i64)>> {
        let rows = sqlx::query!(
            "SELECT app_name, SUM(duration)::bigint as total_duration FROM sessions WHERE start_time >= date_trunc('day', CURRENT_TIMESTAMP) - INTERVAL '29 days' GROUP BY app_name ORDER BY total_duration DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| (r.app_name, r.total_duration.unwrap_or(0))).collect())
    }

    pub async fn get_daily_sessions(&self) -> Result<Vec<Session>> {
        let rows = sqlx::query_as::<_, Session>(
            "SELECT id, app_name, window_name, start_time, duration, category FROM sessions WHERE start_time >= date_trunc('day', CURRENT_TIMESTAMP) AND start_time < date_trunc('day', CURRENT_TIMESTAMP) + INTERVAL '1 day' ORDER BY start_time DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_weekly_sessions(&self) -> Result<Vec<Session>> {
        let rows = sqlx::query_as::<_, Session>(
            "SELECT id, app_name, window_name, start_time, duration, category FROM sessions WHERE start_time >= date_trunc('day', CURRENT_TIMESTAMP) - INTERVAL '6 days' ORDER BY start_time DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_monthly_sessions(&self) -> Result<Vec<Session>> {
        let rows = sqlx::query_as::<_, Session>(
            "SELECT id, app_name, window_name, start_time, duration, category FROM sessions WHERE start_time >= date_trunc('day', CURRENT_TIMESTAMP) - INTERVAL '29 days' ORDER BY start_time DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
