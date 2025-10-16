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

    // Browser tracking
    pub browser_url: Option<String>,
    pub browser_page_title: Option<String>,
    pub browser_notification_count: Option<i32>,

    // Terminal tracking
    pub terminal_username: Option<String>,
    pub terminal_hostname: Option<String>,
    pub terminal_directory: Option<String>,
    pub terminal_project_name: Option<String>,

    // Editor tracking
    pub editor_filename: Option<String>,
    pub editor_filepath: Option<String>,
    pub editor_project_path: Option<String>,
    pub editor_language: Option<String>,

    // Multiplexer tracking
    pub tmux_window_name: Option<String>,
    pub tmux_pane_count: Option<i32>,
    pub terminal_multiplexer: Option<String>,

    // IDE tracking
    pub ide_project_name: Option<String>,
    pub ide_file_open: Option<String>,
    pub ide_workspace: Option<String>,

    // Metadata
    pub parsed_data: Option<sqlx::types::JsonValue>,
    pub parsing_success: Option<bool>,
}