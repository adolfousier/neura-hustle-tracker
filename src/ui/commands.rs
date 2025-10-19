use anyhow::Result;
use chrono::Local;
use crate::database::connection::Database;
use crate::models::session::Session;

/// Command execution context containing app state references
pub struct CommandContext<'a> {
    pub database: &'a Database,
    pub current_session: &'a mut Option<Session>,
    pub logs: &'a mut Vec<String>,
}

/// Result of executing a command that may require UI refresh
pub struct CommandResult {
    pub should_refresh: bool,
}

impl CommandResult {
    pub fn success_with_refresh() -> Self {
        Self {
            should_refresh: true,
        }
    }

    pub fn success_no_refresh() -> Self {
        Self {
            should_refresh: false,
        }
    }
}

/// Get predefined category options
pub fn get_category_options() -> Vec<String> {
    vec![
        "\u{1F4BB} Development".to_string(),
        "\u{1F310} Browsing".to_string(),
        "\u{1F4AC} Communication".to_string(),
        "\u{1F3B5} Media".to_string(),
        "\u{1F4C1} Files".to_string(),
        "\u{1F4E7} Email".to_string(),
        "\u{1F4C4} Office".to_string(),
        "\u{1F4E6} Other".to_string(),
        "\u{2795} Create New Category".to_string(),
    ]
}

/// Rename app command - renames an app while preserving its category
pub async fn execute_rename_app(
    ctx: &mut CommandContext<'_>,
    unique_id: &str,
    new_name: &str,
) -> Result<CommandResult> {
    if new_name.is_empty() {
        return Ok(CommandResult::success_no_refresh());
    }

    let (id_type, original_value) = unique_id.split_once(':').unwrap_or(("", unique_id));

    let result = match id_type {
        "app_name" => {
            // Get the original category before renaming
            let original_category = ctx.database.get_app_category_by_name(original_value).await?.unwrap_or_else(|| "Other".to_string());
            ctx.database.rename_app_with_category(original_value, new_name, &original_category).await
        },
        "browser_page_title" => ctx.database.rename_browser_page_title(original_value, new_name).await,
        "terminal_directory" => ctx.database.rename_terminal_directory(original_value, new_name).await,
        "editor_filename" => ctx.database.rename_editor_filename(original_value, new_name).await,
        "tmux_window_name" => ctx.database.rename_tmux_window_name(original_value, new_name).await,
        _ => {
            // Fallback for window_name or unknown types, treat as app_name
            let original_category = ctx.database.get_app_category_by_name(original_value).await?.unwrap_or_else(|| "Other".to_string());
            ctx.database.rename_app_with_category(original_value, new_name, &original_category).await
        }
    };

    if let Err(e) = result {
        let error_msg = format!("[{}] Failed to rename {}: {}", Local::now().format("%H:%M:%S"), unique_id, e);
        ctx.logs.push(error_msg.clone());
        return Ok(CommandResult::success_no_refresh());
    }

    // Update current session if it matches
    if let Some(session) = ctx.current_session {
        match id_type {
            "app_name" => {
                if session.app_name == original_value {
                    session.app_name = new_name.to_string();
                }
            },
            "browser_page_title" => {
                if session.browser_page_title.as_deref() == Some(original_value) {
                    session.browser_page_title_renamed = Some(new_name.to_string());
                }
            },
            "terminal_directory" => {
                if session.terminal_directory.as_deref() == Some(original_value) {
                    session.terminal_directory_renamed = Some(new_name.to_string());
                }
            },
            "editor_filename" => {
                if session.editor_filename.as_deref() == Some(original_value) {
                    session.editor_filename_renamed = Some(new_name.to_string());
                }
            },
            "tmux_window_name" => {
                if session.tmux_window_name.as_deref() == Some(original_value) {
                    session.tmux_window_name_renamed = Some(new_name.to_string());
                }
            },
            _ => {},
        }
    }

    let success_msg = format!(
        "[{}] Renamed '{}' to '{}'",
        Local::now().format("%H:%M:%S"),
        original_value,
        new_name
    );
    ctx.logs.push(success_msg.clone());

    Ok(CommandResult::success_with_refresh())
}

/// Update app category command - applies a predefined or custom category to an app
pub async fn execute_update_category(
    ctx: &mut CommandContext<'_>,
    unique_id: &str,
    category: &str,
) -> Result<CommandResult> {
    if category.is_empty() {
        return Ok(CommandResult::success_no_refresh());
    }

    let (id_type, original_value) = unique_id.split_once(':').unwrap_or(("", unique_id));

    let result = match id_type {
        "app_name" => ctx.database.update_app_category(original_value, category).await,
        "browser_page_title" => ctx.database.categorize_browser_page_title(original_value, category).await,
        "terminal_directory" => ctx.database.categorize_terminal_directory(original_value, category).await,
        "editor_filename" => ctx.database.categorize_editor_filename(original_value, category).await,
        "tmux_window_name" => ctx.database.categorize_tmux_window_name(original_value, category).await,
        _ => ctx.database.update_app_category(original_value, category).await, // Fallback to app_name
    };

    if let Err(e) = result {
        let error_msg = format!("[{}] Failed to update category for {}: {}", Local::now().format("%H:%M:%S"), unique_id, e);
        ctx.logs.push(error_msg.clone());
        return Ok(CommandResult::success_no_refresh());
    }

    // Update current session if it matches
    if let Some(session) = ctx.current_session {
        match id_type {
            "app_name" => {
                if session.app_name == original_value {
                    session.category = Some(category.to_string());
                }
            },
            "browser_page_title" => {
                if session.browser_page_title.as_deref() == Some(original_value) {
                    session.browser_page_title_category = Some(category.to_string());
                }
            },
            "terminal_directory" => {
                if session.terminal_directory.as_deref() == Some(original_value) {
                    session.terminal_directory_category = Some(category.to_string());
                }
            },
            "editor_filename" => {
                if session.editor_filename.as_deref() == Some(original_value) {
                    session.editor_filename_category = Some(category.to_string());
                }
            },
            "tmux_window_name" => {
                if session.tmux_window_name.as_deref() == Some(original_value) {
                    session.tmux_window_name_category = Some(category.to_string());
                }
            },
            _ => {},
        }
    }

    let success_msg = format!(
        "[{}] Updated category for '{}' to '{}'",
        Local::now().format("%H:%M:%S"),
        original_value,
        category
    );
    ctx.logs.push(success_msg.clone());

    Ok(CommandResult::success_with_refresh())
}

/// Create and apply custom category command
pub async fn execute_create_category(
    ctx: &mut CommandContext<'_>,
    unique_id: &str,
    custom_category: &str,
) -> Result<CommandResult> {
    if custom_category.is_empty() {
        return Ok(CommandResult::success_no_refresh());
    }

    // Apply custom category
    execute_update_category(ctx, unique_id, custom_category).await?;

    let success_msg = format!(
        "[{}] Created and applied category '{}' for '{}'",
        Local::now().format("%H:%M:%S"),
        custom_category,
        unique_id
    );
    ctx.logs.push(success_msg.clone());

    Ok(CommandResult::success_with_refresh())
}
