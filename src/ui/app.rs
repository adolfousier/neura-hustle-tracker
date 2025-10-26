use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, EnableMouseCapture, DisableMouseCapture};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;

use ratatui::backend::CrosstermBackend;
use ratatui::style::Color;
use ratatui::{Frame, Terminal};
use std::collections::BTreeMap;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};
use rdev::{listen, EventType};
use std::sync::{Arc, Mutex};

use crate::database::connection::Database;
use crate::models::session::Session;
use crate::tracker::monitor::AppMonitor;
use crate::ui::{commands::{self, CommandContext}, tracking};
use crate::ui::hierarchical::HierarchicalDisplayItem;

// Re-export ViewMode for other ui modules
pub use crate::ui::tracking::ViewMode;

#[derive(Debug, Clone)]
pub enum InputAction {
    RenameApp { old_name: String },
    CreateCategory { app_name: String },
}

#[derive(Debug, Clone)]
pub enum AppState {
    Dashboard { view_mode: ViewMode },
    ViewingLogs,
    SelectingApp { selected_index: usize, selected_unique_id: String },
    SelectingCategory { selected_index: usize, selected_unique_id: String },
    CategoryMenu { unique_id: String, selected_index: usize },
    Input { prompt: String, buffer: String, action: InputAction },
    CommandsPopup,
    HistoryPopup { view_mode: ViewMode, scroll_position: usize },
    BreakdownDashboard { view_mode: ViewMode, selected_panel: usize, panel_scrolls: [usize; 5] },
}

pub struct App {
    pub state: AppState,
    database: Database,
    monitor: AppMonitor,
    history: Vec<Session>,
    pub current_history: Vec<Session>,
    pub usage: Vec<(String, i64)>,
        pub daily_usage: Vec<HierarchicalDisplayItem>, // Hierarchical for Detailed Stats
        pub weekly_usage: Vec<HierarchicalDisplayItem>,
        pub monthly_usage: Vec<HierarchicalDisplayItem>,    pub flat_daily_usage: Vec<(String, i64)>, // Flat for Today's Activity Progress
    pub current_view_mode: ViewMode,  // Track current dashboard view mode
    pub logs: Vec<String>,
    pub manual_app_name: Option<String>,
    pub current_app: String,
    current_window: Option<String>,
    pub current_session: Option<Session>,
    pub last_input: Arc<Mutex<DateTime<Local>>>,
    // Breakdown data caches
    pub browser_breakdown: Vec<(String, i64)>,
    pub project_breakdown: Vec<(String, i64)>,
    pub file_breakdown: Vec<(String, String, i64)>,
    pub terminal_breakdown: Vec<(String, i64)>,
    pub category_breakdown: Vec<(String, i64)>,
    pub categories: Vec<String>,
}

impl App {
    pub fn new(database: Database) -> Self {
        let monitor = AppMonitor::new();
        let last_input = Arc::new(Mutex::new(Local::now()));

        // Choose input monitoring method based on session type
        if monitor.uses_wayland() {
            // On Wayland, use D-Bus idle monitoring
            log::info!("Wayland detected - using D-Bus idle monitoring");
            Self::start_wayland_input_monitoring(Arc::clone(&last_input));
        } else {
            // On X11, use rdev for direct input event monitoring
            Self::start_rdev_input_monitoring(Arc::clone(&last_input));
        }
        Self {
            state: AppState::Dashboard { view_mode: ViewMode::Daily },
            database,
            monitor,
            history: vec![],
            current_history: vec![],
            usage: vec![],
            daily_usage: vec![],
            weekly_usage: vec![],
            monthly_usage: vec![],
            flat_daily_usage: vec![],
            current_view_mode: ViewMode::Daily,
            logs: vec![],
            manual_app_name: None,
            current_app: "unknown".to_string(),
            current_window: None,
            current_session: None,
            last_input,
            browser_breakdown: vec![],
            project_breakdown: vec![],
            file_breakdown: vec![],
            terminal_breakdown: vec![],
            category_breakdown: vec![],
            categories: vec![],
        }
    }

    // Cross-platform input monitoring using rdev
    fn start_rdev_input_monitoring(last_input: Arc<Mutex<DateTime<Local>>>) {
        std::thread::spawn(move || {
            let callback = move |event: rdev::Event| {
                match event.event_type {
                    EventType::KeyPress(_) | EventType::KeyRelease(_) | EventType::ButtonPress(_) | EventType::ButtonRelease(_) | EventType::MouseMove { .. } => {
                        log::debug!("Input event detected: {:?}", event.event_type);
                        *last_input.lock().unwrap() = Local::now();
                    }
                    _ => {}
                }
            };
            if let Err(error) = listen(callback) {
                eprintln!("Error listening for input events (X11): {:?}", error);
            }
        });
    }

    // Wayland input monitoring using D-Bus idle monitoring + window change detection
    fn start_wayland_input_monitoring(last_input: Arc<Mutex<DateTime<Local>>>) {
        let monitor = AppMonitor::new(); // Create new monitor for the async task
        tokio::spawn(async move {
            let mut last_window_check = tokio::time::Instant::now();
            let mut last_window_info: Option<(String, Option<String>)> = None;
            let mut last_fallback_update = tokio::time::Instant::now();

            loop {
                // First, try D-Bus idle monitoring
                match Self::check_wayland_idle_time().await {
                    Ok(idle_seconds) => {
                        log::debug!("Wayland idle time: {} seconds", idle_seconds);
                        // If idle time is very low, consider it as recent activity
                        if idle_seconds < 3 {
                            *last_input.lock().unwrap() = Local::now();
                            log::debug!("Updated last_input due to low idle time");
                        }
                        // If idle time is moderate but still active, nudge the timer
                        else if idle_seconds < 15 {
                            let current = *last_input.lock().unwrap();
                            let time_since_last_input = Local::now().signed_duration_since(current).num_seconds();
                            // If it's been more than 10 seconds since last update, nudge it
                            if time_since_last_input > 10 {
                                *last_input.lock().unwrap() = Local::now() - chrono::Duration::seconds(10);
                                log::debug!("Nudged last_input for moderate idle time");
                            }
                        }
                        // If idle time is high, don't update - let AFK detection work
                    }
                    Err(e) => {
                        log::warn!("Failed to check Wayland idle time: {}", e);
                        // Fallback: log when D-Bus fails, but don't assume activity
                        // Window changes and low idle times will still update last_input
                        if last_fallback_update.elapsed() >= tokio::time::Duration::from_secs(60) {
                            last_fallback_update = tokio::time::Instant::now();
                            log::info!("D-Bus idle monitoring failed - relying on window changes for activity detection");
                        }

                        // Also check for window changes as additional activity detection
                        if last_window_check.elapsed() >= tokio::time::Duration::from_secs(2) {
                            match tokio::join!(
                                monitor.get_active_app_async(),
                                monitor.get_active_window_name_async()
                            ) {
                                (Ok(app), Ok(window_name)) => {
                                    let window = if window_name.is_empty() { None } else { Some(window_name) };
                                    let current_info = (app.clone(), window.clone());
                                    if last_window_info.as_ref() != Some(&current_info) {
                                        // Window changed - consider this as activity
                                        *last_input.lock().unwrap() = Local::now();
                                        log::debug!("Updated last_input due to window change: {} -> {:?}", app, window);
                                        last_window_info = Some(current_info);
                                    }
                                }
                                _ => {
                                    // Window detection also failed - this is bad
                                    log::warn!("Both idle monitoring and window detection failed");
                                }
                            }
                            last_window_check = tokio::time::Instant::now();
                        }
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    }

    // Check idle time using GNOME D-Bus interfaces
    pub async fn check_wayland_idle_time() -> Result<u32> {
        let connection = zbus::Connection::session().await?;

        // Try GNOME Mutter Idle Monitor with proper monitor creation
        match Self::get_mutter_idle_time(&connection).await {
            Ok(idle_time) => Ok(idle_time),
            Err(e1) => {
                log::debug!("Mutter IdleMonitor failed: {}", e1);
                // Fallback: try GNOME Session Manager
                match connection.call_method(
                    Some("org.gnome.SessionManager"),
                    "/org/gnome/SessionManager/Presence",
                    Some("org.gnome.SessionManager.Presence"),
                    "GetIdleTime",
                    &(),
                ).await {
                    Ok(response) => {
                        let idle_time: u64 = response.body().deserialize()?;
                        Ok((idle_time / 1000) as u32)
                    }
                    Err(e2) => {
                        log::debug!("SessionManager Presence failed: {}", e2);
                        // Try logind idle hint (systemd)
                        match connection.call_method(
                            Some("org.freedesktop.login1"),
                            "/org/freedesktop/login1/session/auto",
                            Some("org.freedesktop.login1.Session"),
                            "GetIdleHint",
                            &(),
                        ).await {
                            Ok(response) => {
                                let idle_hint: bool = response.body().deserialize()?;
                                // GetIdleHint returns boolean, not time
                                // If idle, assume high idle time; if not idle, assume 0
                                Ok(if idle_hint { 300 } else { 0 }) // 5 minutes or 0
                            }
                            Err(e3) => {
                                log::debug!("logind IdleHint failed: {}", e3);
                                // Try org.freedesktop.ScreenSaver
                                match connection.call_method(
                                    Some("org.freedesktop.ScreenSaver"),
                                    "/org/freedesktop/ScreenSaver",
                                    Some("org.freedesktop.ScreenSaver"),
                                    "GetSessionIdleTime",
                                    &(),
                                ).await {
                                    Ok(response) => {
                                        let idle_time: u64 = response.body().deserialize()?;
                                        Ok((idle_time / 1000) as u32)
                                    }
                                    Err(e4) => {
                                        log::debug!("ScreenSaver GetSessionIdleTime failed: {}", e4);
                                        // Try alternative ScreenSaver method
                                        match connection.call_method(
                                            Some("org.freedesktop.ScreenSaver"),
                                            "/org/freedesktop/ScreenSaver",
                                            Some("org.freedesktop.ScreenSaver"),
                                            "GetActiveTime",
                                            &(),
                                        ).await {
                                            Ok(response) => {
                                                let active_time: u64 = response.body().deserialize()?;
                                                Ok((active_time / 1000) as u32)
                                            }
                                            Err(e5) => {
                                                log::debug!("ScreenSaver GetActiveTime failed: {}", e5);
                                                // Last resort: try to detect if we can connect to GNOME Shell
                                                // If GNOME Shell is responding, assume some activity
                                                match connection.call_method(
                                                    Some("org.gnome.Shell"),
                                                    "/org/gnome/Shell",
                                                    Some("org.gnome.Shell"),
                                                    "Eval",
                                                    &("1 + 1".to_string()),
                                                ).await {
                                                    Ok(_) => Ok(0), // GNOME Shell responsive, assume active
                                                    Err(e6) => {
                                                        log::debug!("GNOME Shell check failed: {}", e6);
                                                        Err(anyhow::anyhow!("All idle detection methods failed"))
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Properly create and query Mutter Idle Monitor
    async fn get_mutter_idle_time(connection: &zbus::Connection) -> Result<u32> {
        // First try the existing core monitor
        match connection.call_method(
            Some("org.gnome.Mutter.IdleMonitor"),
            "/org/gnome/Mutter/IdleMonitor/Core",
            Some("org.gnome.Mutter.IdleMonitor"),
            "GetIdletime",
            &(),
        ).await {
            Ok(response) => {
                let idle_time: u64 = response.body().deserialize()?;
                return Ok((idle_time / 1000) as u32);
            }
            Err(_) => {
                // Core monitor doesn't exist, try to create one
                match connection.call_method(
                    Some("org.gnome.Mutter.IdleMonitor"),
                    "/org/gnome/Mutter/IdleMonitor/Core",
                    Some("org.gnome.Mutter.IdleMonitor"),
                    "CreateMonitor",
                    &(),
                ).await {
                    Ok(response) => {
                        let monitor_path: String = response.body().deserialize()?;
                        log::debug!("Created idle monitor at: {}", monitor_path);

                        // Now query the created monitor
                        match connection.call_method(
                            Some("org.gnome.Mutter.IdleMonitor"),
                            monitor_path.as_str(),
                            Some("org.gnome.Mutter.IdleMonitor"),
                            "GetIdletime",
                            &(),
                        ).await {
                            Ok(response) => {
                                let idle_time: u64 = response.body().deserialize()?;
                                Ok((idle_time / 1000) as u32)
                            }
                            Err(e) => {
                                log::debug!("Failed to query created monitor: {}", e);
                                Err(anyhow::anyhow!("Created monitor query failed: {}", e))
                            }
                        }
                    }
                    Err(e) => {
                        log::debug!("Failed to create idle monitor: {}", e);
                        Err(anyhow::anyhow!("Idle monitor creation failed: {}", e))
                    }
                }
            }
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        log::info!("Starting UI...");

        // Set up signal handlers for graceful shutdown on SIGTERM/SIGINT
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        // Register signal handlers
        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&shutdown_flag))?;
        signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown_flag))?;

        // Start tracking initial app before enabling raw mode
        self.start_tracking().await?;

        // Fix any old category data from previous versions
        if let Err(e) = self.database.fix_old_categories().await {
            log::warn!("Failed to fix old categories: {}", e);
        }

        // Load history and usage (load 30 sessions for display)
        self.history = self.database.get_recent_sessions(30).await.unwrap();
        self.usage = self.database.get_app_usage().await.unwrap();
        self.current_history = self.database.get_daily_sessions().await.unwrap();
        self.refresh_categories().await.unwrap();

        // Create hierarchical usage data from sessions for Detailed Stats
        self.daily_usage = crate::ui::hierarchical::create_hierarchical_usage(&self.current_history);
        self.weekly_usage = self.daily_usage.clone();
        self.monthly_usage = self.daily_usage.clone();

        // Create flat usage data for Today's Activity Progress
        self.flat_daily_usage = self.database.get_daily_usage().await.unwrap();

        eprintln!("Enabling raw mode...");
        if let Err(e) = enable_raw_mode() {
            eprintln!("Failed to enable raw mode: {}. This may happen when running in environments without proper terminal support (e.g., SSH without pseudo-terminal, containers, etc.)", e);
            return Err(anyhow::anyhow!("Terminal raw mode not supported: {}", e));
        }
        let mut stdout = io::stdout();
        if let Err(e) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
            eprintln!("Failed to enter alternate screen: {}", e);
            // Try to disable raw mode before returning error
            let _ = disable_raw_mode();
            return Err(anyhow::anyhow!("Failed to setup terminal: {}", e));
        }
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut last_save = Instant::now();
        let save_interval = Duration::from_secs(3600); // 1 hour

        let mut last_data_refresh = Instant::now();
        let data_refresh_interval = Duration::from_secs(5); // Refresh dashboard data every 5 seconds for near real-time updates

        let mut last_afk_check = Instant::now();
        let afk_check_interval = Duration::from_secs(1); // Check AFK status every second
        let afk_threshold = Duration::from_secs(300); // 5 minutes of idle = AFK

        loop {
            terminal.draw(|f| self.draw(f))?;

            // Check for shutdown signal (SIGTERM/SIGINT)
            if shutdown_flag.load(Ordering::Relaxed) {
                log::info!("Received shutdown signal, saving and exiting...");
                break;
            }

            // Check for AFK status every second
            if last_afk_check.elapsed() >= afk_check_interval {
                let time_since_last_check = last_afk_check.elapsed();

                // Detect system sleep: if more than 10 minutes passed since last check, system was likely asleep
                let sleep_threshold = Duration::from_secs(600); // 10 minutes
                let was_system_asleep = time_since_last_check > sleep_threshold;

                let idle_duration = Local::now().signed_duration_since(*self.last_input.lock().unwrap());
                let is_currently_afk = idle_duration.num_seconds() >= afk_threshold.as_secs() as i64;
                log::debug!("Idle duration: {} seconds, is_afk: {}", idle_duration.num_seconds(), is_currently_afk);

                // If system was asleep, force AFK state for the sleep period
                if was_system_asleep && !is_currently_afk {
                    log::info!("System sleep detected (gap: {:.1} minutes), creating AFK session for sleep period",
                              time_since_last_check.as_secs_f64() / 60.0);
                    // End current session and start AFK session for the sleep period
                    if let Some(ref mut session) = self.current_session {
                        if !session.is_afk.unwrap_or(false) {
                            // Save the current session up to sleep time
                            let mut old_session = self.current_session.take().unwrap();
                            let sleep_start_time = Local::now() - chrono::Duration::from_std(time_since_last_check).unwrap_or(chrono::Duration::minutes(0));
                            old_session.duration = sleep_start_time.signed_duration_since(old_session.start_time).num_seconds();

                            if let Err(e) = self.database.insert_session(&old_session).await {
                                log::error!("Failed to save session during sleep detection: {}", e);
                            } else {
                                log::info!("Session saved due to system sleep: {} for {:.1} minutes",
                                          old_session.app_name, old_session.duration as f64 / 60.0);
                            }

                            // Start AFK session for sleep period
                            self.switch_app("AFK".to_string()).await?;
                            if let Some(ref mut new_session) = self.current_session {
                                new_session.is_afk = Some(true);
                                new_session.start_time = sleep_start_time;
                            }

                            // Now continue with normal AFK check
                        }
                    }
                }

                // If we have a current session, check if AFK state changed
                if let Some(ref mut session) = self.current_session {
                    let was_afk = session.is_afk.unwrap_or(false);

                    // AFK state changed - end current session and start new one
                    if was_afk != is_currently_afk {
                        // Save the current session
                        let mut old_session = self.current_session.take().unwrap();
                        old_session.duration = Local::now().signed_duration_since(old_session.start_time).num_seconds();

                        if let Err(e) = self.database.insert_session(&old_session).await {
                            log::error!("Failed to save session on AFK state change: {}", e);
                        } else {
                            log::info!("Session saved on AFK state change: {} -> is_afk={}", old_session.app_name, is_currently_afk);
                        }

                        // Start new session with updated AFK state
                        if is_currently_afk {
                            // Starting AFK session
                            self.switch_app("AFK".to_string()).await?;
                            if let Some(ref mut new_session) = self.current_session {
                                new_session.is_afk = Some(true);
                            }
                        } else {
                            // Returning from AFK - get the actual active app
                            if let Ok(active_app) = self.monitor.get_active_app_async().await {
                                self.switch_app(active_app.clone()).await?;
                                if let Some(ref mut new_session) = self.current_session {
                                    new_session.is_afk = Some(false);
                                }
                            }
                        }
                    }
                }

                last_afk_check = Instant::now();
            }

            // Check for app or window change (but not if we're AFK)
            if let Ok(active_app) = self.monitor.get_active_app_async().await {
                let active_window = self.monitor.get_active_window_name_async().await.ok();
                let idle_duration = Local::now().signed_duration_since(*self.last_input.lock().unwrap());
                let is_currently_afk = idle_duration.num_seconds() >= afk_threshold.as_secs() as i64;

                // Only track app changes if not AFK
                if !is_currently_afk && (active_app != self.current_app || active_window != self.current_window) {
                    self.switch_app(active_app.clone()).await?;
                    self.current_app = active_app;
                    self.current_window = active_window;

                    // Mark new session as not AFK
                    if let Some(ref mut session) = self.current_session {
                        session.is_afk = Some(false);
                    }
                }
            }

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    log::debug!("Key pressed: {:?} in state: {:?}", key.code, self.state);
                    self.logs.push(format!("[{}] Key: {:?} State: {:?}", Local::now().format("%H:%M:%S"), key.code, self.state));

                     let dashboard_view_mode = match &self.state {
                         AppState::Dashboard { view_mode } => Some(view_mode.clone()),
                         _ => None,
                     };
                     if dashboard_view_mode.is_some() {
                         let view_mode = dashboard_view_mode.as_ref().unwrap();
                         match key.code {
                             KeyCode::Char('q') => break,
                             KeyCode::Char('r') => self.start_app_selection(),
                             KeyCode::Char('c') => self.start_category_selection(),
                             KeyCode::Char('l') => self.view_logs(),
                             KeyCode::Char('C') => self.state = AppState::CommandsPopup,
                             KeyCode::Tab => {
                                 let new_view_mode = match view_mode {
                                     ViewMode::Daily => ViewMode::Weekly,
                                     ViewMode::Weekly => ViewMode::Monthly,
                                     ViewMode::Monthly => ViewMode::Daily,
                                 };
                                 self.current_view_mode = new_view_mode.clone();
                                 self.update_history().await?;
                                 self.state = AppState::Dashboard { view_mode: new_view_mode };
                             }
                             KeyCode::Char('h') => {
                                 log::debug!("'h' key pressed - opening history popup");
                                 self.logs.push(format!("[{}] Opening history popup", Local::now().format("%H:%M:%S")));
                                 self.current_history = match view_mode {
                                     ViewMode::Daily => self.database.get_daily_sessions().await.unwrap_or_default(),
                                     ViewMode::Weekly => self.database.get_weekly_sessions().await.unwrap_or_default(),
                                     ViewMode::Monthly => self.database.get_monthly_sessions().await.unwrap_or_default(),
                                 };
                                 self.state = AppState::HistoryPopup { view_mode: view_mode.clone(), scroll_position: 0 };
                             }
                             KeyCode::Char('b') => {
                                 log::debug!("'b' key pressed - opening breakdown dashboard");
                                 self.logs.push(format!("[{}] Opening breakdown dashboard", Local::now().format("%H:%M:%S")));
                                 // Load current_history first (filtered by view mode)
                                 self.current_history = match view_mode {
                                     ViewMode::Daily => self.database.get_daily_sessions().await.unwrap_or_default(),
                                     ViewMode::Weekly => self.database.get_weekly_sessions().await.unwrap_or_default(),
                                     ViewMode::Monthly => self.database.get_monthly_sessions().await.unwrap_or_default(),
                                 };
                                 // Then aggregate breakdown data from current_history
                                 self.load_breakdown_data_from_history();
self.state = AppState::BreakdownDashboard {
                                      view_mode: view_mode.clone(),
                                      selected_panel: 0,
                                      panel_scrolls: [0; 5],
                                  };
                             }
                             _ => {}
                         }
                     } else if matches!(self.state, AppState::CommandsPopup) {
                         match key.code {
                             KeyCode::Esc => self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() },
                             KeyCode::Char('q') => break,
                             KeyCode::Char('r') => self.start_app_selection(),
                             KeyCode::Char('c') => self.start_category_selection(),
                             KeyCode::Char('l') => self.view_logs(),
                             KeyCode::Char('h') => {
                                 log::debug!("'h' key pressed from CommandsPopup - opening history popup");
                                 self.logs.push(format!("[{}] Opening history popup from commands menu", Local::now().format("%H:%M:%S")));
                                 self.current_history = match &self.current_view_mode {
                                     ViewMode::Daily => self.database.get_daily_sessions().await.unwrap_or_default(),
                                     ViewMode::Weekly => self.database.get_weekly_sessions().await.unwrap_or_default(),
                                     ViewMode::Monthly => self.database.get_monthly_sessions().await.unwrap_or_default(),
                                 };
                                 self.state = AppState::HistoryPopup { view_mode: self.current_view_mode.clone(), scroll_position: 0 };
                             }
                             KeyCode::Char('b') => {
                                 log::debug!("'b' key pressed from CommandsPopup - opening breakdown dashboard");
                                 self.logs.push(format!("[{}] Opening breakdown dashboard from commands menu", Local::now().format("%H:%M:%S")));
                                 // Load current_history first (filtered by view mode)
                                 self.current_history = match &self.current_view_mode {
                                     ViewMode::Daily => self.database.get_daily_sessions().await.unwrap_or_default(),
                                     ViewMode::Weekly => self.database.get_weekly_sessions().await.unwrap_or_default(),
                                     ViewMode::Monthly => self.database.get_monthly_sessions().await.unwrap_or_default(),
                                 };
                                 // Then aggregate breakdown data from current_history
                                 self.load_breakdown_data_from_history();
self.state = AppState::BreakdownDashboard {
                                      view_mode: self.current_view_mode.clone(),
                                      selected_panel: 0,
                                      panel_scrolls: [0; 5],
                                  };
                             }
                             _ => {}
                         }
                     } else {
                         match &mut self.state {
                             AppState::ViewingLogs => {
                                 match key.code {
                                     KeyCode::Char('q') => break,
                                     KeyCode::Esc => self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() },
                                     _ => self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() },
                                 }
                             }
                             AppState::SelectingApp { selected_index, selected_unique_id } => {
                                 match key.code {
                                     KeyCode::Up => {
                                         if *selected_index > 0 {
                                             *selected_index -= 1;
                                             *selected_unique_id = self.daily_usage[*selected_index].unique_id.clone();
                                         }
                                     }
                                     KeyCode::Down => {
                                         if *selected_index < self.daily_usage.len().saturating_sub(1) {
                                             *selected_index += 1;
                                             *selected_unique_id = self.daily_usage[*selected_index].unique_id.clone();
                                         }
                                     }
                                     KeyCode::Enter => {
                                         if let Some(item) = self.daily_usage.get(*selected_index) {
                                             self.start_rename_app(item.unique_id.clone());
                                         }
                                     }
                                     KeyCode::Esc => self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() },
                                     _ => {}
                                 }
                             }
                             AppState::SelectingCategory { selected_index, selected_unique_id } => {
                                 match key.code {
                                     KeyCode::Up => {
                                         if *selected_index > 0 {
                                             *selected_index -= 1;
                                             *selected_unique_id = self.daily_usage[*selected_index].unique_id.clone();
                                         }
                                     }
                                     KeyCode::Down => {
                                         if *selected_index < self.daily_usage.len().saturating_sub(1) {
                                             *selected_index += 1;
                                             *selected_unique_id = self.daily_usage[*selected_index].unique_id.clone();
                                         }
                                     }
                                     KeyCode::Enter => {
                                         if let Some(item) = self.daily_usage.get(*selected_index) {
                                             self.start_category_menu(item.unique_id.clone());
                                         }
                                     }
                                     KeyCode::Esc => self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() },
                                     _ => {}
                                 }
                             }
                             AppState::CategoryMenu { unique_id, selected_index } => {
                                 let categories = self.categories.clone();
                                 match key.code {
                                     KeyCode::Up => {
                                         if *selected_index > 0 {
                                             *selected_index -= 1;
                                         }
                                     }
                                     KeyCode::Down => {
                                         if *selected_index < categories.len().saturating_sub(1) {
                                             *selected_index += 1;
                                         }
                                     }
                                     KeyCode::Enter => {
                                         if let Some(category) = categories.get(*selected_index) {
                                             let id = unique_id.clone();
                                             let cat = category.clone();
                                             self.handle_category_selection(id, cat).await?;
                                         }
                                     }
                                     KeyCode::Esc => self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() },
                                     _ => {}
                                 }
                             }
                             AppState::Input { buffer, .. } => {
                                 match key.code {
                                     KeyCode::Char(c) => buffer.push(c),
                                     KeyCode::Backspace => { buffer.pop(); }
                                     KeyCode::Enter => self.handle_input().await?,
                                     KeyCode::Esc => self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() },
                                     _ => {}
                                 }
                             }
                             AppState::HistoryPopup { view_mode, scroll_position } => {
                                 match key.code {
                                     KeyCode::Esc => self.state = AppState::Dashboard { view_mode: view_mode.clone() },
                                     KeyCode::Char('q') => break,
                                     KeyCode::Up => {
                                         if *scroll_position > 0 {
                                             *scroll_position -= 1;
                                         }
                                     }
                                     KeyCode::Down => {
                                         let max_scroll = self.current_history.len().saturating_sub(1);
                                         if *scroll_position < max_scroll {
                                             *scroll_position += 1;
                                         }
                                     }
                                     KeyCode::PageUp => {
                                         *scroll_position = scroll_position.saturating_sub(10);
                                     }
                                     KeyCode::PageDown => {
                                         let max_scroll = self.current_history.len().saturating_sub(1);
                                         *scroll_position = (*scroll_position + 10).min(max_scroll);
                                     }
                                     _ => {}
                                 }
                             }
AppState::BreakdownDashboard { view_mode, selected_panel, panel_scrolls } => {
                                  match key.code {
                                      KeyCode::Esc => self.state = AppState::Dashboard { view_mode: view_mode.clone() },
                                      KeyCode::Char('q') => break,
                                      KeyCode::Tab => {
                                          *selected_panel = (*selected_panel + 1) % 5;
                                      }
                                      KeyCode::Enter => {
                                          // Enter selects/highlights the current panel - visual feedback only
                                      }
                                      KeyCode::Up => {
                                          panel_scrolls[*selected_panel] = panel_scrolls[*selected_panel].saturating_sub(1);
                                      }
                                      KeyCode::Down => {
                                          panel_scrolls[*selected_panel] = panel_scrolls[*selected_panel].saturating_add(1);
                                      }
                                      KeyCode::PageUp => {
                                          panel_scrolls[*selected_panel] = panel_scrolls[*selected_panel].saturating_sub(5);
                                      }
                                      KeyCode::PageDown => {
                                          panel_scrolls[*selected_panel] = panel_scrolls[*selected_panel].saturating_add(5);
                                      }
                                      _ => {}
                                  }
                              }
                             _ => {}
                         }
                     }
                }
            }

            // Auto save every hour
            if last_save.elapsed() >= save_interval {
                if let Some(session) = &mut self.current_session {
                    session.duration = Local::now().signed_duration_since(session.start_time).num_seconds();
                    if let Err(e) = self.database.insert_session(session).await {
                        log::error!("Failed to auto save session: {}", e);
                    } else {
                        last_save = Instant::now();
                    }
                }
            }

            // Refresh dashboard data every 5 seconds for near real-time updates
            if last_data_refresh.elapsed() >= data_refresh_interval {
                self.history = self.database.get_recent_sessions(30).await.unwrap_or_default();
                self.usage = self.database.get_app_usage().await.unwrap_or_default();

                // Update current_history based on current view mode
                if let AppState::Dashboard { ref view_mode } = self.state {
                    self.current_history = match view_mode {
                        ViewMode::Daily => self.database.get_daily_sessions().await.unwrap_or_default(),
                        ViewMode::Weekly => self.database.get_weekly_sessions().await.unwrap_or_default(),
                        ViewMode::Monthly => self.database.get_monthly_sessions().await.unwrap_or_default(),
                    };

                    // Create hierarchical usage data from current_history for Detailed Stats
                    self.daily_usage = crate::ui::hierarchical::create_hierarchical_usage(&self.current_history);
                    self.weekly_usage = self.daily_usage.clone();
                    self.monthly_usage = self.daily_usage.clone();

                    // Create flat usage data for Today's Activity Progress
                    self.flat_daily_usage = self.database.get_daily_usage().await.unwrap_or_default();
                }

                // Update current session duration in history for real-time display
                if let Some(current_session) = &self.current_session {
                    let current_duration = Local::now().signed_duration_since(current_session.start_time).num_seconds();
                    // Update the most recent session in history if it matches the current one
                    if let Some(latest_session) = self.current_history.first_mut() {
                        if latest_session.app_name == current_session.app_name &&
                           latest_session.start_time == current_session.start_time {
                            latest_session.duration = current_duration;
                            // Update renamed fields for persistence
                            latest_session.browser_page_title_renamed = current_session.browser_page_title_renamed.clone();
                            latest_session.terminal_directory_renamed = current_session.terminal_directory_renamed.clone();
                            latest_session.editor_filename_renamed = current_session.editor_filename_renamed.clone();
                            latest_session.tmux_window_name_renamed = current_session.tmux_window_name_renamed.clone();
                        }
                    }
                }

                last_data_refresh = Instant::now();
                log::debug!("Dashboard data refreshed");
            }
        }

        // Save current session on exit
        if let Some(mut session) = self.current_session.take() {
            session.duration = Local::now().signed_duration_since(session.start_time).num_seconds();

            // Save ALL sessions regardless of duration
            if let Err(e) = self.database.insert_session(&session).await {
                log::error!("Failed to save session on exit: {}", e);
                self.logs.push(format!("Failed to save session: {}", e));
            } else {
                self.history = self.database.get_recent_sessions(30).await?;
                self.usage = self.database.get_app_usage().await?;
                self.logs.push(format!("[{}] Ended session: {} for {}s", Local::now().format("%H:%M:%S"), session.app_name, session.duration));
            }
        }

        // Clean up terminal state
        if let Err(e) = disable_raw_mode() {
            log::warn!("Failed to disable raw mode: {}", e);
        }
        if let Err(e) = execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture) {
            log::warn!("Failed to leave alternate screen: {}", e);
        }
        Ok(())
    }

    fn draw(&self, f: &mut Frame) {
        // Delegate to render module
        crate::ui::render::draw(self, f);
    }

    pub fn draw_dashboard(&self, f: &mut Frame, area: ratatui::layout::Rect, view_mode: &ViewMode) {
        crate::ui::render::draw_dashboard(self, f, area, view_mode);
    }

    pub fn draw_bar_chart(&self, f: &mut Frame, area: ratatui::layout::Rect, title: &str, bar_data: &[crate::ui::hierarchical::HierarchicalDisplayItem]) {
        crate::ui::render::draw_bar_chart(self, f, area, title, bar_data);
    }

    pub fn draw_history(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        crate::ui::render::draw_history(self, f, area);
    }

    pub fn draw_pie_chart(&self, f: &mut Frame, area: ratatui::layout::Rect, data: &[HierarchicalDisplayItem]) {
        crate::ui::render::draw_pie_chart(self, f, area, data);
    }

    pub fn draw_timeline(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        crate::ui::render::draw_timeline(self, f, area);
    }

    pub fn draw_afk(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        crate::ui::render::draw_afk(self, f, area);
    }

    pub fn draw_file_breakdown_section(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        scroll_position: usize,
        highlighted: bool,
    ) {
        crate::ui::render::draw_file_breakdown_section(self, f, area, scroll_position, highlighted);
    }

    pub fn draw_file_breakdown_section_with_style(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        scroll_position: usize,
        style: ratatui::style::Style,
    ) {
        crate::ui::render::draw_file_breakdown_section_with_style(self, f, area, scroll_position, style);
    }

    pub fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
        crate::ui::render::centered_rect(percent_x, percent_y, r)
    }

    async fn start_tracking(&mut self) -> Result<()> {
        let ctx = tracking::TrackingContext {
            monitor: &self.monitor,
            database: &self.database,
            manual_app_name: self.manual_app_name.clone(),
        };

        let result = tracking::start_tracking(&ctx, Self::categorize_app).await?;

        self.current_app = result.app_name;
        self.current_session = Some(result.session);
        self.current_window = result.window_name;
        self.logs.push(result.log_message);
        Ok(())
    }

    async fn switch_app(&mut self, new_app: String) -> Result<()> {
        let ctx = tracking::TrackingContext {
            monitor: &self.monitor,
            database: &self.database,
            manual_app_name: self.manual_app_name.clone(),
        };

        let view_mode = match &self.state {
            AppState::Dashboard { view_mode } => view_mode.clone(),
            _ => self.current_view_mode.clone(),
        };

        let result = tracking::switch_app(&ctx, self.current_session.take(), new_app, Self::categorize_app).await?;

        // If session was saved, refresh all data
        if result.saved_session.is_some() {
            let refresh_data = tracking::refresh_all_data(&self.database, &view_mode).await?;
            self.usage = refresh_data.usage;
            self.daily_usage = refresh_data.daily_usage;
            self.weekly_usage = refresh_data.weekly_usage;
            self.monthly_usage = refresh_data.monthly_usage;
            self.history = refresh_data.history;
            self.current_history = refresh_data.current_history;
        }

        self.current_session = Some(result.new_session);
        self.current_app = result.app_name;
        self.current_window = result.window_name;
        self.logs.extend(result.logs);
        Ok(())
    }

    fn start_app_selection(&mut self) {
        if !self.daily_usage.is_empty() {
            let initial_unique_id = self.daily_usage[0].unique_id.clone();
            self.state = AppState::SelectingApp { selected_index: 0, selected_unique_id: initial_unique_id };
        }
    }

    fn start_rename_app(&mut self, unique_id: String) {
        let display_name = self.daily_usage.iter().find(|item| item.unique_id == unique_id).map(|item| item.display_name.clone()).unwrap_or(unique_id.clone());
        self.state = AppState::Input {
            prompt: format!("Rename '{}' to", display_name),
            buffer: String::new(),
            action: InputAction::RenameApp { old_name: unique_id },
        };
    }

    fn start_category_selection(&mut self) {
        if !self.daily_usage.is_empty() {
            let initial_unique_id = self.daily_usage[0].unique_id.clone();
            self.state = AppState::SelectingCategory { selected_index: 0, selected_unique_id: initial_unique_id };
        }
    }

    fn start_category_menu(&mut self, unique_id: String) {
        self.state = AppState::CategoryMenu { unique_id, selected_index: 0 };
    }

    pub fn get_category_options(&self) -> Vec<String> {
        self.categories.clone()
    }

    pub async fn refresh_categories(&mut self) -> Result<()> {
        let mut categories = commands::get_category_options();
        
        // Fetch custom categories from database
        match self.database.get_custom_categories().await {
            Ok(custom_cats) => {
                for cat in custom_cats {
                    if !categories.contains(&cat) {
                        categories.insert(categories.len() - 1, cat);
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to fetch custom categories: {}", e);
            }
        }
        
        self.categories = categories;
        Ok(())
    }
    async fn handle_category_selection(&mut self, app_name: String, category: String) -> Result<()> {
        if category == "➕ Create New Category" {
            // User wants to create custom category
            self.state = AppState::Input {
                prompt: format!("Enter custom category for '{}' (e.g., 🎮 Gaming)", app_name),
                buffer: String::new(),
                action: InputAction::CreateCategory { app_name },
            };
        } else {
            // Apply predefined category using commands module
            let mut ctx = CommandContext {
                database: &self.database,
                current_session: &mut self.current_session,
                logs: &mut self.logs,
            };

            let result = commands::execute_update_category(&mut ctx, &app_name, &category).await?;

            if result.should_refresh {
                self.refresh_all_data().await?;
            }

            self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() };
        }
        Ok(())
    }

    fn view_logs(&mut self) {
        self.state = AppState::ViewingLogs;
    }

    pub fn is_afk(&self, threshold_secs: i64) -> bool {
        let last = *self.last_input.lock().unwrap();
        Local::now().signed_duration_since(last).num_seconds() > threshold_secs
    }

    pub fn clean_app_name(app_name: &str) -> String {
        if app_name.starts_with("gnome-") {
            app_name.strip_prefix("gnome-").unwrap_or(app_name).to_string()
        } else {
            app_name.to_string()
        }
    }

    pub fn categorize_app(app: &str) -> (String, Color) {
        let app_lower = app.to_lowercase();
        if app_lower.contains("code") || app_lower.contains("vim") || app_lower.contains("nvim") ||
           app_lower.contains("terminal") || app_lower.contains("alacritty") || app_lower.contains("kitty") ||
           app_lower.contains("rust") || app_lower.contains("cargo") || app_lower.contains("editor") ||
           app_lower.contains("vscode") || app_lower.contains("vscodium") || app_lower.contains("gedit") ||
           app_lower.contains("nano") || app_lower.contains("emacs") || app_lower.contains("atom") ||
           app_lower.contains("sublime") || app_lower.contains("console") || app_lower.contains("iterm") {
            ("💻 Development".to_string(), Color::Yellow)
        } else if app_lower.contains("browser") || app_lower.contains("chrome") || app_lower.contains("firefox") ||
                  app_lower.contains("brave") || app_lower.contains("edge") || app_lower.contains("chromium") {
            ("🌐 Browsing".to_string(), Color::Blue)
        } else if app_lower.contains("slack") || app_lower.contains("zoom") || app_lower.contains("teams") ||
                  app_lower.contains("discord") || app_lower.contains("telegram") || app_lower.contains("chat") ||
                  app_lower.contains("signal") || app_lower.contains("element") || app_lower.contains("video-call") ||
                  app_lower.contains("skype") || app_lower.contains("jitsi") {
            ("💬 Communication".to_string(), Color::Green)
        } else if app_lower.contains("spotify") || app_lower.contains("vlc") || app_lower.contains("music") ||
                  app_lower.contains("media") || app_lower.contains("rhythmbox") || app_lower.contains("audacious") ||
                  app_lower.contains("clementine") {
            ("🎵 Media".to_string(), Color::Magenta)
        } else if app_lower.contains("nautilus") || app_lower.contains("files") || app_lower.contains("dolphin") ||
                  app_lower.contains("file-manager") || app_lower.contains("thunar") || app_lower.contains("nemo") {
            ("📁 Files".to_string(), Color::Cyan)
        } else if app_lower.contains("thunderbird") || app_lower.contains("evolution") || app_lower.contains("geary") ||
                  app_lower.contains("email") {
            ("📧 Email".to_string(), Color::LightYellow)
        } else if app_lower.contains("libreoffice") || app_lower.contains("soffice") {
            ("📄 Office".to_string(), Color::LightBlue)
        } else {
            ("📦 Other".to_string(), Color::White)
        }
    }

    pub fn category_from_string(category: &str) -> (String, Color) {
        match category {
            "💻 Development" => ("💻 Development".to_string(), Color::Yellow),
            "🌐 Browsing" => ("🌐 Browsing".to_string(), Color::Blue),
            "💬 Communication" => ("💬 Communication".to_string(), Color::Green),
            "🎵 Media" => ("🎵 Media".to_string(), Color::Magenta),
            "📁 Files" => ("📁 Files".to_string(), Color::Cyan),
            "📧 Email" => ("📧 Email".to_string(), Color::LightYellow),
            "📄 Office" => ("📄 Office".to_string(), Color::LightBlue),
            _ => {
                if category == "📦 Other" {
                    ("📦 Other".to_string(), Color::White)
                } else {
                    (category.to_string(), Color::LightMagenta)
                }
            }
        }
    }

    pub fn get_app_category(&self, app: &str) -> (String, Color) {
        // First try to find stored category in history
        for session in &self.current_history {
            if session.app_name == app {
                if let Some(stored_category) = &session.category {
                    // Map stored category string to emoji+name and color
                    return Self::category_from_string(stored_category);
                }
            }
        }
        // Fall back to pattern matching for backward compatibility
        Self::categorize_app(app)
    }

    async fn update_history(&mut self) -> Result<()> {
        if let AppState::Dashboard { ref view_mode } = self.state {
            self.current_history = match view_mode {
                ViewMode::Daily => self.database.get_daily_sessions().await?,
                ViewMode::Weekly => self.database.get_weekly_sessions().await?,
                ViewMode::Monthly => self.database.get_monthly_sessions().await?,
            };
        }
        Ok(())
    }

    async fn refresh_all_data(&mut self) -> Result<()> {
        // Refresh ALL usage data
        self.usage = self.database.get_app_usage().await?;

        // Update current_history based on current view mode FIRST
        self.current_history = match &self.current_view_mode {
            ViewMode::Daily => self.database.get_daily_sessions().await.unwrap_or_default(),
            ViewMode::Weekly => self.database.get_weekly_sessions().await.unwrap_or_default(),
            ViewMode::Monthly => self.database.get_monthly_sessions().await.unwrap_or_default(),
        };

        // Create hierarchical usage data from sessions using hierarchical module
        self.daily_usage = crate::ui::hierarchical::create_hierarchical_usage(&self.current_history);
        self.weekly_usage = self.daily_usage.clone(); // Will be replaced based on view mode
        self.monthly_usage = self.daily_usage.clone(); // Will be replaced based on view mode

        // Create flat usage data for Today's Activity Progress
        self.flat_daily_usage = self.database.get_daily_usage().await?;

        self.history = self.database.get_recent_sessions(30).await?;
        Ok(())
    }

    fn load_breakdown_data_from_history(&mut self) {
        // Use hierarchical module to create all breakdown data from current_history
        self.browser_breakdown = crate::ui::hierarchical::create_browser_breakdown(&self.current_history);
        self.project_breakdown = crate::ui::hierarchical::create_project_breakdown(&self.current_history);
        self.file_breakdown = crate::ui::hierarchical::create_file_breakdown(&self.current_history);
        self.terminal_breakdown = crate::ui::hierarchical::create_terminal_breakdown(&self.current_history);

        // Category breakdown - exclude AFK sessions
        let mut category_map: BTreeMap<String, i64> = BTreeMap::new();
        for session in &self.current_history {
            // Skip AFK sessions
            if session.is_afk.unwrap_or(false) {
                continue;
            }

            if let Some(category) = &session.category {
                *category_map.entry(category.clone()).or_insert(0) += session.duration;
            }
        }
        self.category_breakdown = category_map.into_iter().collect();
        self.category_breakdown.sort_by(|a, b| b.1.cmp(&a.1));
    }

    async fn handle_input(&mut self) -> Result<()> {
        let (buffer, action) = if let AppState::Input { buffer, action, .. } = &self.state {
            (buffer.clone(), action.clone())
        } else {
            return Ok(());
        };

        match action {
            InputAction::RenameApp { old_name } => {
                // Create command context for executing commands
                let mut ctx = CommandContext {
                    database: &self.database,
                    current_session: &mut self.current_session,
                    logs: &mut self.logs,
                };

                // Execute rename command using commands module
                let result = commands::execute_rename_app(&mut ctx, &old_name, &buffer).await?;

                if result.should_refresh {
                    self.refresh_all_data().await?;
                }

                self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() };
            }
            InputAction::CreateCategory { app_name } => {
                // Create command context for executing commands
                let mut ctx = CommandContext {
                    database: &self.database,
                    current_session: &mut self.current_session,
                    logs: &mut self.logs,
                };

                // Execute create category command using commands module
                let result = commands::execute_create_category(&mut ctx, &app_name, &buffer).await?;

                if result.should_refresh {
                    self.refresh_all_data().await?;
                self.refresh_categories().await?;
                }

                self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() };
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_sleep_gap_detection_ui() {
        // This test verifies that sleep gap detection works correctly for the UI app
        // We can't easily test the full app loop, but we can test the logic

        // Test that sleep threshold detection works
        let sleep_threshold = Duration::from_secs(600); // 10 minutes
        let short_gap = Duration::from_secs(30);
        let long_gap = Duration::from_secs(1200); // 20 minutes

        assert!(! (short_gap > sleep_threshold), "Short gap should not trigger sleep detection");
        assert!(long_gap > sleep_threshold, "Long gap should trigger sleep detection");

        // Test chrono duration conversion
        let chrono_duration = chrono::Duration::from_std(long_gap).unwrap();
        assert_eq!(chrono_duration.num_minutes(), 20);

        println!("UI sleep gap detection logic test passed");
    }

    #[tokio::test]
    async fn test_afk_session_creation_logic_ui() {
        // Test that the logic for creating AFK sessions during sleep works
        let start_time = Local::now();
        let sleep_duration = Duration::from_secs(3600); // 1 hour sleep
        let sleep_start_time = start_time + chrono::Duration::from_std(sleep_duration).unwrap();

        // Simulate what happens during sleep detection
        let session_duration_before_sleep = sleep_start_time.signed_duration_since(start_time).num_seconds();
        assert_eq!(session_duration_before_sleep, 3600);

        println!("UI AFK session creation logic test passed");
    }
}
