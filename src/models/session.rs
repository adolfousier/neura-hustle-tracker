use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<i32>,
    pub app_name: String,
    pub window_name: Option<String>,
    pub start_time: DateTime<Local>,
    pub duration: i64, // in seconds
    pub category: Option<String>,
}