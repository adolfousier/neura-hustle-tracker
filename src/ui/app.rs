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
    SelectingApp { selected_index: usize },
    SelectingCategory { selected_index: usize },
    CategoryMenu { app_name: String, selected_index: usize },
    Input { prompt: String, buffer: String, action: InputAction },
    CommandsPopup,
    HistoryPopup { view_mode: ViewMode, scroll_position: usize },
    BreakdownDashboard { view_mode: ViewMode, scroll_position: usize },
}

pub struct App {
    pub state: AppState,
    database: Database,
    monitor: AppMonitor,
    history: Vec<Session>,
    pub current_history: Vec<Session>,
    pub usage: Vec<(String, i64)>,
    pub daily_usage: Vec<(String, i64)>,
    pub weekly_usage: Vec<(String, i64)>,
    pub monthly_usage: Vec<(String, i64)>,
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
}

impl App {
    pub fn new(database: Database) -> Self {
        let monitor = AppMonitor::new();
        let last_input = Arc::new(Mutex::new(Local::now()));

        // Choose input monitoring method based on session type
        if monitor.uses_wayland() {
            Self::start_wayland_input_monitoring(Arc::clone(&last_input));
        } else {
            Self::start_x11_input_monitoring(Arc::clone(&last_input));
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
        }
    }

    // X11 input monitoring using rdev
    fn start_x11_input_monitoring(last_input: Arc<Mutex<DateTime<Local>>>) {
        std::thread::spawn(move || {
            let callback = move |event: rdev::Event| {
                match event.event_type {
                    EventType::KeyPress(_) | EventType::KeyRelease(_) | EventType::ButtonPress(_) | EventType::ButtonRelease(_) | EventType::MouseMove { .. } => {
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

    // Wayland input monitoring using D-Bus idle monitoring
    fn start_wayland_input_monitoring(last_input: Arc<Mutex<DateTime<Local>>>) {
        tokio::spawn(async move {
            loop {
                match Self::check_wayland_idle_time().await {
                    Ok(idle_seconds) => {
                        // If idle time is less than 10 seconds, consider it as recent activity
                        if idle_seconds < 10 {
                            *last_input.lock().unwrap() = Local::now();
                        }
                    }
                    Err(e) => {
                        log::debug!("Failed to check Wayland idle time: {}", e);
                        // Fallback: update every 60 seconds if we can't monitor properly
                        *last_input.lock().unwrap() = Local::now();
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });
    }

    // Check idle time using GNOME Session Manager D-Bus interface
    async fn check_wayland_idle_time() -> Result<u32> {
        let connection = zbus::Connection::session().await?;

        // Try GNOME Session Manager idle time
        match connection.call_method(
            Some("org.gnome.SessionManager"),
            "/org/gnome/SessionManager/Presence",
            Some("org.gnome.SessionManager.Presence"),
            "GetIdleTime",
            &(),
        ).await {
            Ok(response) => {
                let idle_time: u32 = response.body().deserialize()?;
                Ok(idle_time / 1000) // Convert milliseconds to seconds
            }
            Err(_) => {
                // Fallback: try Screen Saver interface
                match connection.call_method(
                    Some("org.gnome.ScreenSaver"),
                    "/org/gnome/ScreenSaver",
                    Some("org.gnome.ScreenSaver"),
                    "GetActiveTime",
                    &(),
                ).await {
                    Ok(_response) => {
                        // If screen saver is not active, assume recent activity
                        Ok(0) // Consider it active if we can call this
                    }
                    Err(e) => Err(anyhow::anyhow!("No suitable D-Bus interface found: {}", e))
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
        self.daily_usage = self.database.get_daily_usage().await.unwrap();
        self.weekly_usage = self.database.get_weekly_usage().await.unwrap();
        self.monthly_usage = self.database.get_monthly_usage().await.unwrap();
        self.current_history = self.history.clone();

        eprintln!("Enabling raw mode...");
        enable_raw_mode().unwrap();
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut last_save = Instant::now();
        let save_interval = Duration::from_secs(3600); // 1 hour

        let mut last_data_refresh = Instant::now();
        let data_refresh_interval = Duration::from_secs(5); // Refresh dashboard data every 5 seconds for near real-time updates

        loop {
            terminal.draw(|f| self.draw(f))?;

            // Check for shutdown signal (SIGTERM/SIGINT)
            if shutdown_flag.load(Ordering::Relaxed) {
                log::info!("Received shutdown signal, saving and exiting...");
                break;
            }

            // Check for app or window change
            if let Ok(active_app) = self.monitor.get_active_app_async().await {
                let active_window = self.monitor.get_active_window_name_async().await.ok();
                if active_app != self.current_app || active_window != self.current_window {
                    self.switch_app(active_app.clone()).await?;
                    self.current_app = active_app;
                    self.current_window = active_window;
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
                                     scroll_position: 0
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
                                     scroll_position: 0
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
                             AppState::SelectingApp { selected_index } => {
                                 match key.code {
                                     KeyCode::Up => {
                                         if *selected_index > 0 {
                                             *selected_index -= 1;
                                         }
                                     }
                                     KeyCode::Down => {
                                         if *selected_index < self.usage.len().saturating_sub(1) {
                                             *selected_index += 1;
                                         }
                                     }
                                     KeyCode::Enter => {
                                         if let Some((app_name, _)) = self.usage.get(*selected_index) {
                                             self.start_rename_app(app_name.clone());
                                         }
                                     }
                                     KeyCode::Esc => self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() },
                                     _ => {}
                                 }
                             }
                             AppState::SelectingCategory { selected_index } => {
                                 match key.code {
                                     KeyCode::Up => {
                                         if *selected_index > 0 {
                                             *selected_index -= 1;
                                         }
                                     }
                                     KeyCode::Down => {
                                         if *selected_index < self.usage.len().saturating_sub(1) {
                                             *selected_index += 1;
                                         }
                                     }
                                     KeyCode::Enter => {
                                         if let Some((app_name, _)) = self.usage.get(*selected_index) {
                                             self.start_category_menu(app_name.clone());
                                         }
                                     }
                                     KeyCode::Esc => self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() },
                                     _ => {}
                                 }
                             }
                             AppState::CategoryMenu { app_name, selected_index } => {
                                 let categories = Self::get_category_options();
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
                                             let app = app_name.clone();
                                             let cat = category.clone();
                                             self.handle_category_selection(app, cat).await?;
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
                             AppState::BreakdownDashboard { view_mode, scroll_position } => {
                                 match key.code {
                                     KeyCode::Esc => self.state = AppState::Dashboard { view_mode: view_mode.clone() },
                                     KeyCode::Char('q') => break,
                                     KeyCode::Up => {
                                         *scroll_position = scroll_position.saturating_sub(1);
                                     }
                                     KeyCode::Down => {
                                         *scroll_position = scroll_position.saturating_add(1);
                                     }
                                     KeyCode::PageUp => {
                                         *scroll_position = scroll_position.saturating_sub(5);
                                     }
                                     KeyCode::PageDown => {
                                         *scroll_position = scroll_position.saturating_add(5);
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
                self.daily_usage = self.database.get_daily_usage().await.unwrap_or_default();
                self.weekly_usage = self.database.get_weekly_usage().await.unwrap_or_default();
                self.monthly_usage = self.database.get_monthly_usage().await.unwrap_or_default();

                // Update current_history based on current view mode
                if let AppState::Dashboard { ref view_mode } = self.state {
                    self.current_history = match view_mode {
                        ViewMode::Daily => self.database.get_daily_sessions().await.unwrap_or_default(),
                        ViewMode::Weekly => self.database.get_weekly_sessions().await.unwrap_or_default(),
                        ViewMode::Monthly => self.database.get_monthly_sessions().await.unwrap_or_default(),
                    };
                }

                // Update current session duration in history for real-time display
                if let Some(current_session) = &self.current_session {
                    let current_duration = Local::now().signed_duration_since(current_session.start_time).num_seconds();
                    // Update the most recent session in history if it matches the current one
                    if let Some(latest_session) = self.current_history.first_mut() {
                        if latest_session.app_name == current_session.app_name &&
                           latest_session.start_time == current_session.start_time {
                            latest_session.duration = current_duration;
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

        disable_raw_mode().unwrap();
        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
        Ok(())
    }

    fn draw(&self, f: &mut Frame) {
        // Delegate to render module
        crate::ui::render::draw(self, f);
    }

    pub fn draw_dashboard(&self, f: &mut Frame, area: ratatui::layout::Rect, view_mode: &ViewMode) {
        crate::ui::render::draw_dashboard(self, f, area, view_mode);
    }

    pub fn draw_bar_chart(&self, f: &mut Frame, area: ratatui::layout::Rect, title: &str, bar_data: &[(&str, u64)]) {
        crate::ui::render::draw_bar_chart(self, f, area, title, bar_data);
    }

    pub fn draw_history(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        crate::ui::render::draw_history(self, f, area);
    }

    pub fn draw_pie_chart(&self, f: &mut Frame, area: ratatui::layout::Rect, data: &[(String, i64)]) {
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
    ) {
        crate::ui::render::draw_file_breakdown_section(self, f, area, scroll_position);
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
        if !self.usage.is_empty() {
            self.state = AppState::SelectingApp { selected_index: 0 };
        }
    }

    fn start_rename_app(&mut self, old_name: String) {
        self.state = AppState::Input {
            prompt: format!("Rename '{}' to", old_name),
            buffer: String::new(),
            action: InputAction::RenameApp { old_name },
        };
    }

    fn start_category_selection(&mut self) {
        if !self.usage.is_empty() {
            self.state = AppState::SelectingCategory { selected_index: 0 };
        }
    }

    fn start_category_menu(&mut self, app_name: String) {
        self.state = AppState::CategoryMenu { app_name, selected_index: 0 };
    }

    pub fn get_category_options() -> Vec<String> {
        commands::get_category_options()
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

    pub fn categorize_app(app: &str) -> (&'static str, Color) {
        let app_lower = app.to_lowercase();
        if app_lower.contains("code") || app_lower.contains("vim") || app_lower.contains("nvim") ||
           app_lower.contains("terminal") || app_lower.contains("alacritty") || app_lower.contains("kitty") ||
           app_lower.contains("rust") || app_lower.contains("cargo") || app_lower.contains("editor") ||
           app_lower.contains("vscode") || app_lower.contains("vscodium") || app_lower.contains("gedit") ||
           app_lower.contains("nano") || app_lower.contains("emacs") || app_lower.contains("atom") ||
           app_lower.contains("sublime") || app_lower.contains("console") || app_lower.contains("iterm") {
            ("💻 Development", Color::Yellow)
        } else if app_lower.contains("browser") || app_lower.contains("chrome") || app_lower.contains("firefox") ||
                  app_lower.contains("brave") || app_lower.contains("edge") || app_lower.contains("chromium") {
            ("🌐 Browsing", Color::Blue)
        } else if app_lower.contains("slack") || app_lower.contains("zoom") || app_lower.contains("teams") ||
                  app_lower.contains("discord") || app_lower.contains("telegram") || app_lower.contains("chat") ||
                  app_lower.contains("signal") || app_lower.contains("element") || app_lower.contains("video-call") ||
                  app_lower.contains("skype") || app_lower.contains("jitsi") {
            ("💬 Communication", Color::Green)
        } else if app_lower.contains("spotify") || app_lower.contains("vlc") || app_lower.contains("music") ||
                  app_lower.contains("media") || app_lower.contains("rhythmbox") || app_lower.contains("audacious") ||
                  app_lower.contains("clementine") {
            ("🎵 Media", Color::Magenta)
        } else if app_lower.contains("nautilus") || app_lower.contains("files") || app_lower.contains("dolphin") ||
                  app_lower.contains("file-manager") || app_lower.contains("thunar") || app_lower.contains("nemo") {
            ("📁 Files", Color::Cyan)
        } else if app_lower.contains("thunderbird") || app_lower.contains("evolution") || app_lower.contains("geary") ||
                  app_lower.contains("email") {
            ("📧 Email", Color::LightYellow)
        } else if app_lower.contains("libreoffice") || app_lower.contains("soffice") {
            ("📄 Office", Color::LightBlue)
        } else {
            ("📦 Other", Color::White)
        }
    }

    pub fn category_from_string(category: &str) -> (&'static str, Color) {
        match category {
            "💻 Development" => ("💻 Development", Color::Yellow),
            "🌐 Browsing" => ("🌐 Browsing", Color::Blue),
            "💬 Communication" => ("💬 Communication", Color::Green),
            "🎵 Media" => ("🎵 Media", Color::Magenta),
            "📁 Files" => ("📁 Files", Color::Cyan),
            "📧 Email" => ("📧 Email", Color::LightYellow),
            "📄 Office" => ("📄 Office", Color::LightBlue),
            _ => ("📦 Other", Color::White),
        }
    }

    pub fn get_app_category(&self, app: &str) -> (&'static str, Color) {
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
        self.daily_usage = self.database.get_daily_usage().await?;
        self.weekly_usage = self.database.get_weekly_usage().await?;
        self.monthly_usage = self.database.get_monthly_usage().await?;
        self.history = self.database.get_recent_sessions(30).await?;

        // Update current_history based on current view mode
        self.current_history = match &self.current_view_mode {
            ViewMode::Daily => self.database.get_daily_sessions().await.unwrap_or_default(),
            ViewMode::Weekly => self.database.get_weekly_sessions().await.unwrap_or_default(),
            ViewMode::Monthly => self.database.get_monthly_sessions().await.unwrap_or_default(),
        };
        Ok(())
    }

    fn load_breakdown_data_from_history(&mut self) {
        // Aggregate breakdown data from current_history (which is already filtered by view mode)

        // Browser breakdown
        let mut browser_map: BTreeMap<String, i64> = BTreeMap::new();
        for session in &self.current_history {
            if let Some(url) = &session.browser_url {
                // Extract domain from URL
                let domain = url.split('/').nth(2).unwrap_or(url).to_string();
                *browser_map.entry(domain).or_insert(0) += session.duration;
            }
        }
        self.browser_breakdown = browser_map.into_iter().collect();
        self.browser_breakdown.sort_by(|a, b| b.1.cmp(&a.1));

        // Project breakdown
        let mut project_map: BTreeMap<String, i64> = BTreeMap::new();
        for session in &self.current_history {
            if let Some(project) = &session.terminal_project_name {
                *project_map.entry(project.clone()).or_insert(0) += session.duration;
            } else if let Some(project) = &session.ide_project_name {
                *project_map.entry(project.clone()).or_insert(0) += session.duration;
            }
        }
        self.project_breakdown = project_map.into_iter().collect();
        self.project_breakdown.sort_by(|a, b| b.1.cmp(&a.1));

        // File breakdown
        let mut file_map: BTreeMap<(String, String), i64> = BTreeMap::new();
        for session in &self.current_history {
            if let (Some(filename), Some(language)) = (&session.editor_filename, &session.editor_language) {
                *file_map.entry((filename.clone(), language.clone())).or_insert(0) += session.duration;
            }
        }
        self.file_breakdown = file_map.into_iter().map(|((f, l), d)| (f, l, d)).collect();
        self.file_breakdown.sort_by(|a, b| b.2.cmp(&a.2));

        // Terminal breakdown
        let mut terminal_map: BTreeMap<String, i64> = BTreeMap::new();
        for session in &self.current_history {
            if let Some(dir) = &session.terminal_directory {
                *terminal_map.entry(dir.clone()).or_insert(0) += session.duration;
            }
        }
        self.terminal_breakdown = terminal_map.into_iter().collect();
        self.terminal_breakdown.sort_by(|a, b| b.1.cmp(&a.1));

        // Category breakdown
        let mut category_map: BTreeMap<String, i64> = BTreeMap::new();
        for session in &self.current_history {
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
                // Get the original category before creating context
                let original_category = self.get_app_category(&old_name).0.to_string();

                // Create command context for executing commands
                let mut ctx = CommandContext {
                    database: &self.database,
                    current_session: &mut self.current_session,
                    logs: &mut self.logs,
                };

                // Execute rename command using commands module
                let result = commands::execute_rename_app(&mut ctx, &old_name, &buffer, &original_category).await?;

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
                }

                self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() };
            }
        }
        Ok(())
    }
}
