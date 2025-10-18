use anyhow::Result;
use chrono::Local;
use crate::database::connection::Database;
use crate::models::session::Session;
use crate::tracker::monitor::AppMonitor;
use crate::ui::session;

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Daily,
    Weekly,
    Monthly,
}

/// Context for tracking operations
pub struct TrackingContext<'a> {
    pub monitor: &'a AppMonitor,
    pub database: &'a Database,
    pub manual_app_name: Option<String>,
}

/// Result of starting a new tracking session
pub struct TrackingResult {
    pub session: Session,
    pub app_name: String,
    pub window_name: Option<String>,
    pub log_message: String,
}

/// Result of switching to a new app
pub struct SwitchResult {
    pub new_session: Session,
    pub saved_session: Option<Session>,
    pub app_name: String,
    pub window_name: Option<String>,
    pub logs: Vec<String>,
}

/// Result of refreshing dashboard data
pub struct RefreshData {
    pub usage: Vec<(String, i64)>,
    pub daily_usage: Vec<(String, i64)>,
    pub weekly_usage: Vec<(String, i64)>,
    pub monthly_usage: Vec<(String, i64)>,
    pub history: Vec<Session>,
    pub current_history: Vec<Session>,
}

pub async fn start_tracking(
    ctx: &TrackingContext<'_>,
    categorize_fn: fn(&str) -> (&'static str, ratatui::style::Color),
) -> Result<TrackingResult> {
    let app_name = if let Some(manual_name) = &ctx.manual_app_name {
        manual_name.clone()
    } else {
        match ctx.monitor.get_active_app_async().await {
            Ok(detected) => detected,
            Err(e) => {
                let error_msg = format!("Window detection failed: {}", e);
                eprintln!("{}", error_msg);
                "Unknown".to_string()
            }
        }
    };

    let window_name = ctx.monitor.get_active_window_name_async().await.ok();
    let start_time = Local::now();
    let (category_name, _) = categorize_fn(&app_name);

    let session = session::create_session_with_parsing(
        app_name.clone(),
        window_name.clone(),
        start_time,
        category_name.to_string(),
    );

    let log_message = format!("[{}] Started tracking: {}", Local::now().format("%H:%M:%S"), app_name);

    Ok(TrackingResult {
        session,
        app_name: app_name.clone(),
        window_name,
        log_message,
    })
}

pub async fn switch_app(
    ctx: &TrackingContext<'_>,
    current_session: Option<Session>,
    new_app: String,
    categorize_fn: fn(&str) -> (&'static str, ratatui::style::Color),
) -> Result<SwitchResult> {
    let mut logs = Vec::new();
    let saved_session;

    // End current session
    if let Some(mut session) = current_session {
        session.duration = Local::now().signed_duration_since(session.start_time).num_seconds();

        // Save ALL sessions regardless of duration
        if let Err(e) = ctx.database.insert_session(&session).await {
            log::error!("Failed to save session: {}", e);
            logs.push(format!("[{}] Failed to save session: {}", Local::now().format("%H:%M:%S"), e));
            saved_session = None;
        } else {
            logs.push(format!("[{}] Saved session: {} for {}s", Local::now().format("%H:%M:%S"), session.app_name, session.duration));
            saved_session = Some(session);
        }
    } else {
        saved_session = None;
    }

    // Start new session
    let window_name = ctx.monitor.get_active_window_name_async().await.ok();
    let start_time = Local::now();
    let (category_name, _) = categorize_fn(&new_app);

    let new_session = session::create_session_with_parsing(
        new_app.clone(),
        window_name.clone(),
        start_time,
        category_name.to_string(),
    );

    logs.push(format!("[{}] Switched to: {}", Local::now().format("%H:%M:%S"), new_app));

    Ok(SwitchResult {
        new_session,
        saved_session,
        app_name: new_app,
        window_name,
        logs,
    })
}

pub async fn refresh_all_data(database: &Database, view_mode: &ViewMode) -> Result<RefreshData> {
    let usage = database.get_app_usage().await?;
    let daily_usage = database.get_daily_usage().await.unwrap_or_default();
    let weekly_usage = database.get_weekly_usage().await.unwrap_or_default();
    let monthly_usage = database.get_monthly_usage().await.unwrap_or_default();
    let history = database.get_recent_sessions(30).await.unwrap_or_default();

    let current_history = match view_mode {
        ViewMode::Daily => database.get_daily_sessions().await.unwrap_or_default(),
        ViewMode::Weekly => database.get_weekly_sessions().await.unwrap_or_default(),
        ViewMode::Monthly => database.get_monthly_sessions().await.unwrap_or_default(),
    };

    Ok(RefreshData {
        usage,
        daily_usage,
        weekly_usage,
        monthly_usage,
        history,
        current_history,
    })
}
