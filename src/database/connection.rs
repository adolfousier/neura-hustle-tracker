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
                duration BIGINT NOT NULL
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
        Ok(())
    }

    pub async fn insert_session(&self, session: &Session) -> Result<i32> {
        let id: (i32,) = sqlx::query_as(
            "INSERT INTO sessions (app_name, window_name, start_time, duration) VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(&session.app_name)
        .bind(&session.window_name)
        .bind(session.start_time)
        .bind(session.duration)
        .fetch_one(&self.pool)
        .await?;
        Ok(id.0)
    }

    pub async fn get_recent_sessions(&self, limit: i64) -> Result<Vec<Session>> {
        let sessions = sqlx::query_as::<_, Session>(
            "SELECT id, app_name, window_name, start_time, duration FROM sessions ORDER BY start_time DESC LIMIT $1",
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

    pub async fn rename_app(&self, old_name: &str, new_name: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET app_name = $1 WHERE app_name = $2")
            .bind(new_name)
            .bind(old_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_daily_usage(&self) -> Result<Vec<(String, i64)>> {
        let rows = sqlx::query!(
            "SELECT app_name, SUM(duration)::bigint as total_duration FROM sessions WHERE DATE(start_time) = CURRENT_DATE GROUP BY app_name ORDER BY total_duration DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| (r.app_name, r.total_duration.unwrap_or(0))).collect())
    }

    pub async fn get_weekly_usage(&self) -> Result<Vec<(String, i64)>> {
        let rows = sqlx::query!(
            "SELECT app_name, SUM(duration)::bigint as total_duration FROM sessions WHERE DATE(start_time) >= CURRENT_DATE - INTERVAL '7 days' GROUP BY app_name ORDER BY total_duration DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| (r.app_name, r.total_duration.unwrap_or(0))).collect())
    }

    pub async fn get_monthly_usage(&self) -> Result<Vec<(String, i64)>> {
        let rows = sqlx::query!(
            "SELECT app_name, SUM(duration)::bigint as total_duration FROM sessions WHERE DATE(start_time) >= CURRENT_DATE - INTERVAL '30 days' GROUP BY app_name ORDER BY total_duration DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| (r.app_name, r.total_duration.unwrap_or(0))).collect())
    }
}