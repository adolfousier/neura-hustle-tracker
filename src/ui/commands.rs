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
    old_name: &str,
    new_name: &str,
    original_category: &str,
) -> Result<CommandResult> {
    if new_name.is_empty() {
        return Ok(CommandResult::success_no_refresh());
    }

    // Rename app in database while preserving category
    if let Err(e) = ctx.database.rename_app_with_category(old_name, new_name, original_category).await {
        let error_msg = format!("[{}] Failed to rename app: {}", Local::now().format("%H:%M:%S"), e);
        ctx.logs.push(error_msg.clone());
        return Ok(CommandResult::success_no_refresh());
    }

    // Update current session if it matches
    if let Some(session) = ctx.current_session {
        if session.app_name == old_name {
            session.app_name = new_name.to_string();
            session.category = Some(original_category.to_string());
        }
    }

    let success_msg = format!(
        "[{}] Renamed '{}' to '{}' (preserved category: {})",
        Local::now().format("%H:%M:%S"),
        old_name,
        new_name,
        original_category
    );
    ctx.logs.push(success_msg.clone());

    Ok(CommandResult::success_with_refresh())
}

/// Update app category command - applies a predefined or custom category to an app
pub async fn execute_update_category(
    ctx: &mut CommandContext<'_>,
    app_name: &str,
    category: &str,
) -> Result<CommandResult> {
    if category.is_empty() {
        return Ok(CommandResult::success_no_refresh());
    }

    // Apply category
    if let Err(e) = ctx.database.update_app_category(app_name, category).await {
        let error_msg = format!("[{}] Failed to update category: {}", Local::now().format("%H:%M:%S"), e);
        ctx.logs.push(error_msg.clone());
        return Ok(CommandResult::success_no_refresh());
    }

    // Update current session if it matches
    if let Some(session) = ctx.current_session {
        if session.app_name == app_name {
            session.category = Some(category.to_string());
        }
    }

    let success_msg = format!(
        "[{}] Updated category for '{}' to '{}'",
        Local::now().format("%H:%M:%S"),
        app_name,
        category
    );
    ctx.logs.push(success_msg.clone());

    Ok(CommandResult::success_with_refresh())
}

/// Create and apply custom category command
pub async fn execute_create_category(
    ctx: &mut CommandContext<'_>,
    app_name: &str,
    custom_category: &str,
) -> Result<CommandResult> {
    if custom_category.is_empty() {
        return Ok(CommandResult::success_no_refresh());
    }

    // Apply custom category
    if let Err(e) = ctx.database.update_app_category(app_name, custom_category).await {
        let error_msg = format!("[{}] Failed to create category: {}", Local::now().format("%H:%M:%S"), e);
        ctx.logs.push(error_msg.clone());
        return Ok(CommandResult::success_no_refresh());
    }

    // Update current session if it matches
    if let Some(session) = ctx.current_session {
        if session.app_name == app_name {
            session.category = Some(custom_category.to_string());
        }
    }

    let success_msg = format!(
        "[{}] Created and applied category '{}' for '{}'",
        Local::now().format("%H:%M:%S"),
        custom_category,
        app_name
    );
    ctx.logs.push(success_msg.clone());

    Ok(CommandResult::success_with_refresh())
}
