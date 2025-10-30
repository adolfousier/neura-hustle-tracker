use chrono::{DateTime, Local};
use crate::models::session::Session;
use crate::ui::parser;
use crate::database::connection::Database;
use anyhow::Result;

pub async fn create_session_with_parsing(db: &Database, app_name: String, window_name: Option<String>, start_time: DateTime<Local>, category: String) -> Result<Session> {
    create_session_with_parsing_and_afk(db, app_name, window_name, start_time, category, Some(false)).await
}

pub async fn create_session_with_parsing_and_afk(db: &Database, app_name: String, window_name: Option<String>, start_time: DateTime<Local>, category: String, is_afk: Option<bool>) -> Result<Session> {
    // Parse window name if available
    let parsed = if let Some(ref win_name) = window_name {
        parser::parse_window_name(&app_name, win_name)
    } else {
        parser::ParsedSessionData::default()
    };

    // Convert parsed data to JSON
    let parsed_json = serde_json::to_value(&parsed).ok();

    let mut session = Session {
        id: None,
        app_name,
        window_name,
        start_time,
        duration: 0,
        category: Some(category),
        // Browser fields
        browser_url: parsed.browser_url,
        browser_page_title: parsed.browser_page_title,
        browser_notification_count: parsed.browser_notification_count,
        browser_page_title_renamed: None,
        browser_page_title_category: None,
        // Terminal fields
        terminal_username: parsed.terminal_username,
        terminal_hostname: parsed.terminal_hostname,
        terminal_directory: parsed.terminal_directory,
        terminal_project_name: parsed.terminal_project_name,
        terminal_directory_renamed: None,
        terminal_directory_category: None,
        // Editor fields
        editor_filename: parsed.editor_filename,
        editor_filepath: parsed.editor_filepath,
        editor_project_path: parsed.editor_project_path,
        editor_language: parsed.editor_language,
        editor_filename_renamed: None,
        editor_filename_category: None,
        // Multiplexer fields
        tmux_window_name: parsed.tmux_window_name,
        tmux_pane_count: parsed.tmux_pane_count,
        terminal_multiplexer: parsed.terminal_multiplexer,
        tmux_window_name_renamed: None,
        tmux_window_name_category: None,
        // IDE fields
        ide_project_name: parsed.ide_project_name,
        ide_file_open: parsed.ide_file_open,
        ide_workspace: parsed.ide_workspace,
        // Metadata
        parsed_data: parsed_json,
        parsing_success: Some(parsed.parsing_success),
        // AFK tracking
        is_afk,
        is_idle: Some(false),  // Default to not idle for new sessions
    };

    let id = db.insert_session(&session).await?;
    session.id = Some(id);
    Ok(session)
}
