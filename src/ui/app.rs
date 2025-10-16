use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, EnableMouseCapture, DisableMouseCapture};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;

use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Bar, BarChart, BarGroup, Block, Borders, List, ListItem, Paragraph};
use ratatui::style::{Color, Style};
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
use crate::tracker::parser;

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone)]
pub enum InputAction {
    RenameApp { old_name: String },
}

#[derive(Debug, Clone)]
pub enum AppState {
    Dashboard { view_mode: ViewMode },
    ViewingLogs,
    SelectingApp { selected_index: usize },
    Input { prompt: String, buffer: String, action: InputAction },
    CommandsPopup,
    HistoryPopup { view_mode: ViewMode, scroll_position: usize },
    BreakdownDashboard { view_mode: ViewMode, scroll_position: usize },
}

pub struct App {
    state: AppState,
    database: Database,
    monitor: AppMonitor,
    history: Vec<Session>,
    current_history: Vec<Session>,
    usage: Vec<(String, i64)>,
    daily_usage: Vec<(String, i64)>,
    weekly_usage: Vec<(String, i64)>,
    monthly_usage: Vec<(String, i64)>,
    current_view_mode: ViewMode,  // Track current dashboard view mode
    logs: Vec<String>,
    manual_app_name: Option<String>,
    current_app: String,
    current_window: Option<String>,
    current_session: Option<Session>,
    last_input: Arc<Mutex<DateTime<Local>>>,
    // Breakdown data caches
    browser_breakdown: Vec<(String, i64)>,
    project_breakdown: Vec<(String, i64)>,
    file_breakdown: Vec<(String, String, i64)>,
    terminal_breakdown: Vec<(String, i64)>,
    category_breakdown: Vec<(String, i64)>,
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
        let size = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
            .split(size);

        // Status bar with Shift+C indicator
        let status = match &self.state {
            AppState::Dashboard { .. } => {
                if let Some(session) = &self.current_session {
                    let duration = Local::now().signed_duration_since(session.start_time).num_seconds();
                    let display_name = self.manual_app_name.as_ref().unwrap_or(&session.app_name);
                    format!("Tracking: {} for {}s | [Shift+C] Commands | [h] History", display_name, duration)
                } else {
                    format!("Not tracking - Current app: {} | [Shift+C] Commands | [h] History", self.current_app)
                }
            }
            AppState::ViewingLogs => "Viewing Logs - Press any key to return".to_string(),
            AppState::SelectingApp { .. } => "Rename Mode - Use arrow keys to select an app".to_string(),
            AppState::Input { action, .. } => {
                match action {
                    InputAction::RenameApp { .. } => "Rename Mode - Enter new name for the app".to_string(),
                }
            }
            AppState::CommandsPopup => "Commands Menu - Press key to execute or Esc to close".to_string(),
            AppState::HistoryPopup { .. } => "Session History - Use ↑/↓/PgUp/PgDn to scroll, Esc to close".to_string(),
            AppState::BreakdownDashboard { .. } => "📊 Activity Breakdown Dashboard - [↑/↓/PgUp/PgDn] Scroll | [Esc] Close".to_string(),
        };

        let status_widget = Paragraph::new(status)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status_widget, chunks[0]);

        // Main content area
        match &self.state {
            AppState::ViewingLogs => {
                let log_items: Vec<ListItem> = self
                    .logs
                    .iter()
                    .rev()
                    .take(20)
                    .map(|log| ListItem::new(Line::from(log.clone())))
                    .collect();
                let log_list = List::new(log_items)
                    .block(Block::default().borders(Borders::ALL).title("Logs"));
                f.render_widget(log_list, chunks[1]);
            }

            AppState::SelectingApp { selected_index } => {
                // Full-screen app selection view
                let max_items = (chunks[1].height.saturating_sub(2) as usize).min(20).max(5);
                let usage_items: Vec<ListItem> = self
                    .usage
                    .iter()
                    .enumerate()
                    .take(max_items)
                    .map(|(i, (app, duration))| {
                        let hours = duration / 3600;
                        let minutes = (duration % 3600) / 60;
                        let prefix = if i == *selected_index { "→ " } else { "  " };

                        let time_display = if hours > 0 {
                            format!("{}h {}m", hours, minutes)
                        } else {
                            format!("{}m", minutes)
                        };

                        let clean_app = Self::clean_app_name(app);
                        let display = format!("{}{:<30} {}", prefix, clean_app, time_display);

                        let style = if i == *selected_index {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        };

                        ListItem::new(Line::from(display)).style(style)
                    })
                    .collect();

                let usage_list = List::new(usage_items)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .title("📝 Select App to Rename (↑/↓ to navigate, Enter to select, Esc to cancel)"));
                f.render_widget(usage_list, chunks[1]);
            }

            AppState::Input { prompt, buffer, action } => {
                // Full-screen input view with centered input box
                let input_area = Self::centered_rect(70, 30, chunks[1]);

                // Clear background
                f.render_widget(ratatui::widgets::Clear, input_area);

                // Determine title based on action
                let title = match action {
                    InputAction::RenameApp { .. } => "✏️  Rename App",
                };

                // Create input text with cursor
                let input_text = vec![
                    Line::from(""),
                    Line::from(prompt.clone()).style(Style::default().fg(Color::Cyan)),
                    Line::from(""),
                    Line::from(vec![
                        ratatui::text::Span::styled("  ", Style::default()),
                        ratatui::text::Span::styled(buffer.clone(), Style::default().fg(Color::White)),
                        ratatui::text::Span::styled("█", Style::default().fg(Color::Yellow)),
                    ]),
                    Line::from(""),
                    Line::from("  Press Enter to confirm, Esc to cancel").style(Style::default().fg(Color::Gray)),
                ];

                let input_widget = Paragraph::new(input_text)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .style(Style::default().bg(Color::Black)));

                f.render_widget(input_widget, input_area);
            }

            AppState::Dashboard { view_mode } => {
                self.draw_dashboard(f, chunks[1], view_mode);
            }

            AppState::CommandsPopup => {
                // Show dashboard in background
                self.draw_dashboard(f, chunks[1], &self.current_view_mode);

                // Draw popup overlay
                let popup_area = Self::centered_rect(60, 50, size);
                f.render_widget(ratatui::widgets::Clear, popup_area);

                let commands_text = vec![
                    Line::from(""),
                    Line::from("  [Tab]  Switch View (Daily/Weekly/Monthly)"),
                    Line::from("  [h]    View session history (scrollable popup)"),
                    Line::from("  [b]    View activity breakdowns (scrollable popup)"),
                    Line::from("  [r]    Rename app/tab"),
                    Line::from("  [l]    View logs"),
                    Line::from("  [q]    Quit application (auto-saves)"),
                    Line::from(""),
                    Line::from("  Press Esc to close this menu"),
                ];

                let popup = Paragraph::new(commands_text)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .title("📋 Commands Menu")
                        .style(Style::default().bg(Color::Black)));
                f.render_widget(popup, popup_area);
            }

            AppState::HistoryPopup { view_mode, scroll_position } => {
                // Show dashboard in background
                self.draw_dashboard(f, chunks[1], view_mode);

                // Draw popup overlay
                let popup_area = Self::centered_rect(80, 70, size);
                f.render_widget(ratatui::widgets::Clear, popup_area);

                // Calculate how many items can fit in the popup
                let max_visible_items = (popup_area.height.saturating_sub(4) as usize).max(10);

                // Create history list items
                let mut history_items: Vec<ListItem> = Vec::new();

                // Get the visible slice of history based on scroll position
                let start_idx = *scroll_position;
                let end_idx = (start_idx + max_visible_items).min(self.current_history.len());

                for (idx, session) in self.current_history[start_idx..end_idx].iter().enumerate() {
                    let minutes = session.duration / 60;
                    let time = session.start_time.format("%Y-%m-%d %H:%M");

                    // Create display name with window name if available
                    let clean_app = Self::clean_app_name(&session.app_name);
                    let display_name = if let Some(window_name) = &session.window_name {
                        format!("{} ({})", clean_app, window_name)
                    } else {
                        clean_app
                    };

                    let display = format!("{}  {} - {}m", time, display_name, minutes);
                    let style = if idx == 0 && start_idx == 0 {
                        Style::default().fg(Color::Yellow)  // Highlight first (most recent)
                    } else {
                        Style::default()
                    };

                    history_items.push(ListItem::new(Line::from(display)).style(style));
                }

                // Add indicator if there are more items to scroll
                let scroll_indicator = if self.current_history.len() > max_visible_items {
                    format!(" (Showing {}-{} of {} sessions)", start_idx + 1, end_idx, self.current_history.len())
                } else {
                    format!(" ({} sessions)", self.current_history.len())
                };

                let history_list = List::new(history_items)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .title(format!("📜 Session History{}", scroll_indicator))
                        .style(Style::default().bg(Color::Black)));
                f.render_widget(history_list, popup_area);
            }

            AppState::BreakdownDashboard { view_mode, scroll_position } => {
                // Show dashboard in background
                self.draw_dashboard(f, chunks[1], view_mode);

                // Draw popup overlay (90% width, 85% height)
                let popup_area = Self::centered_rect(90, 85, size);
                f.render_widget(ratatui::widgets::Clear, popup_area);

                // Main popup container
                let popup_block = Block::default()
                    .borders(Borders::ALL)
                    .title("📊 Activity Breakdown Dashboard")
                    .style(Style::default().bg(Color::Black));
                f.render_widget(popup_block, popup_area);

                // Inner area for grid layout
                let inner_area = popup_area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });

                // Create grid layout: 2 columns x 3 rows
                let rows = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(33),
                        Constraint::Percentage(33),
                        Constraint::Percentage(34),
                    ].as_ref())
                    .split(inner_area);

                // Row 1: Categories | Browser Services
                let row1_cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(rows[0]);

                // Row 2: Projects | Files
                let row2_cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(rows[1]);

                // Row 3: Terminal Sessions (full width)
                let row3_area = rows[2];

                // Render each breakdown section
                self.draw_breakdown_section(f, row1_cols[0], "📦 Categories", &self.category_breakdown, Color::Magenta, *scroll_position, true);
                self.draw_breakdown_section(f, row1_cols[1], "🌐 Browser Services", &self.browser_breakdown, Color::Blue, *scroll_position, false);
                self.draw_breakdown_section(f, row2_cols[0], "📁 Projects", &self.project_breakdown, Color::Yellow, *scroll_position, false);

                // Files breakdown with language info
                self.draw_file_breakdown_section(f, row2_cols[1], *scroll_position);

                self.draw_breakdown_section(f, row3_area, "💻 Terminal Sessions", &self.terminal_breakdown, Color::Green, *scroll_position, false);
            }
        }
    }

    // Helper function to create centered rectangle
    fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    fn draw_dashboard(&self, f: &mut Frame, area: ratatui::layout::Rect, view_mode: &ViewMode) {
        // Adaptive layout based on terminal size
        let use_vertical_layout = area.width < 120 || area.height < 30;

        let (data, title) = match view_mode {
            ViewMode::Daily => (&self.daily_usage, "📊 Daily Usage"),
            ViewMode::Weekly => (&self.weekly_usage, "📊 Weekly Usage (7 days)"),
            ViewMode::Monthly => (&self.monthly_usage, "📊 Monthly Usage (30 days)"),
        };

        // Create bar chart data - limit based on space
        let max_bars = if area.width < 80 { 5 } else if area.width < 120 { 8 } else { 10 };
        let bar_data: Vec<(&str, u64)> = data
            .iter()
            .take(max_bars)
            .map(|(app, duration)| {
                let minutes = (duration / 60) as u64;
                (app.as_str(), minutes)
            })
            .collect();

        if use_vertical_layout {
            // VERTICAL LAYOUT for small terminals
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(10),  // Bar chart
                    Constraint::Min(8),   // Timeline
                    Constraint::Min(8),   // AFK
                    Constraint::Min(8),   // Stats
                    Constraint::Min(10),  // History
                    Constraint::Min(8),   // Categories
                ].as_ref())
                .split(area);

            self.draw_bar_chart(f, chunks[0], title, &bar_data);
            self.draw_timeline(f, chunks[1]);
            self.draw_afk(f, chunks[2]);
            self.draw_stats(f, chunks[3], data);
            self.draw_history(f, chunks[4]);
            self.draw_pie_chart(f, chunks[5], data);
        } else {
            // HORIZONTAL LAYOUT for larger terminals (50/50 split)
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ].as_ref())
                .split(area);

            // LEFT SIDE: Bar Chart + Timeline/AFK + Stats
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),   // Bar chart
                    Constraint::Percentage(30),   // Timeline + AFK
                    Constraint::Percentage(30),   // Detailed stats
                ].as_ref())
                .split(main_chunks[0]);

            // RIGHT SIDE: Session History + Pie Chart
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(70),
                    Constraint::Percentage(30),
                ].as_ref())
                .split(main_chunks[1]);

            self.draw_bar_chart(f, left_chunks[0], title, &bar_data);
            let timeline_afk_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(left_chunks[1]);
            self.draw_timeline(f, timeline_afk_chunks[0]);
            self.draw_afk(f, timeline_afk_chunks[1]);
            self.draw_stats(f, left_chunks[2], data);
            self.draw_history(f, right_chunks[0]);
            self.draw_pie_chart(f, right_chunks[1], data);
        }
    }

    fn draw_bar_chart(&self, f: &mut Frame, area: ratatui::layout::Rect, title: &str, bar_data: &[(&str, u64)]) {
        if bar_data.is_empty() {
            let empty_msg = Paragraph::new("No data available yet. Start tracking!")
                .block(Block::default().borders(Borders::ALL).title(title));
            f.render_widget(empty_msg, area);
        } else {
            // Adaptive bar width based on terminal width
            let bar_width = if area.width < 60 { 3 } else if area.width < 100 { 5 } else { 6 };
            let bar_gap = if area.width < 60 { 0 } else { 1 };

            // Find max value in minutes
            let max_minutes = bar_data.iter().map(|(_, v)| *v).max().unwrap_or(0);

            // Calculate scale: minimum 8h (480 min), or max_value + 2h (120 min)
            // This ensures bars never reach the top
            let scale_minutes = if max_minutes <= 480 {
                480  // 8h default for regular workday
            } else {
                // Round up to next hour and add 2h buffer
                ((max_minutes / 60) + 3) * 60
            };

            let scale_hours = scale_minutes / 60;

            // Create bars with category-based colors and hour labels
            let bars: Vec<Bar> = bar_data
                .iter()
                .map(|(app, value_minutes)| {
                    let (_, color) = self.get_app_category(app);
                    let hours = value_minutes / 60;
                    let mins = value_minutes % 60;

                    // Format label: show hours only, or hours + minutes
                    let value_label = if mins == 0 {
                        format!("{}h", hours)
                    } else if hours == 0 {
                        format!("{}m", mins)
                    } else {
                        format!("{}h{}m", hours, mins)
                    };

                    let clean_app = Self::clean_app_name(app);
                    Bar::default()
                        .value(*value_minutes)
                        .label(Line::from(clean_app))
                        .text_value(value_label)
                        .style(Style::default().fg(color))
                        .value_style(Style::default().fg(Color::White))
                })
                .collect();

            let chart_title = format!("{} (scale: 0-{}h)", title, scale_hours);

            let barchart = BarChart::default()
                .block(Block::default().borders(Borders::ALL).title(chart_title))
                .bar_width(bar_width)
                .bar_gap(bar_gap)
                .max(scale_minutes)  // Set max scale directly instead of padding bar
                .data(BarGroup::default().bars(&bars));
            f.render_widget(barchart, area);
        }
    }

    fn draw_stats(&self, f: &mut Frame, area: ratatui::layout::Rect, data: &[(String, i64)]) {
        // Adaptive number of items based on available height
        let max_items = (area.height.saturating_sub(3) as usize).min(15).max(3);

        let mut stats_items: Vec<ListItem> = Vec::new();

        // Add top margin
        stats_items.push(ListItem::new(Line::from("")));

        stats_items.extend(data
            .iter()
            .take(max_items)
            .map(|(app, duration)| {
                let hours = duration / 3600;
                let minutes = (duration % 3600) / 60;

                // Clean and truncate app name if terminal is narrow
                let clean_app = Self::clean_app_name(app);
                let app_display = if area.width < 40 {
                    if clean_app.len() > 15 {
                        format!("{}...", &clean_app[..12])
                    } else {
                        clean_app
                    }
                } else {
                    clean_app
                };

                let display = if hours > 0 {
                    format!("  {} - {}h {}m", app_display, hours, minutes)
                } else {
                    format!("  {} - {}m", app_display, minutes)
                };
                ListItem::new(Line::from(display))
            })
            .collect::<Vec<ListItem>>());

        let total_duration: i64 = data.iter().map(|(_, d)| d).sum();
        let total_hours = total_duration / 3600;
        let total_minutes = (total_duration % 3600) / 60;
        let stats_title = if total_hours > 0 {
            format!("📈 Detailed Stats (Total: {}h {}m)", total_hours, total_minutes)
        } else {
            format!("📈 Detailed Stats (Total: {}m)", total_minutes)
        };

        let stats_list = List::new(stats_items)
            .block(Block::default().borders(Borders::ALL).title(stats_title));
        f.render_widget(stats_list, area);
    }

    fn draw_history(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        // Adaptive number of items based on available height
        let max_items = (area.height.saturating_sub(3) as usize).min(30).max(5);

        let mut history_items: Vec<ListItem> = Vec::new();

        // Add top margin
        history_items.push(ListItem::new(Line::from("")));

        // Add current session first with real-time duration
        if let Some(current_session) = &self.current_session {
            let current_duration = Local::now().signed_duration_since(current_session.start_time).num_seconds();
            let minutes = current_duration / 60;
            let time = current_session.start_time.format("%H:%M");

            // Create display name with window name if available
            let clean_app = Self::clean_app_name(&current_session.app_name);
            let display_name = if let Some(window_name) = &current_session.window_name {
                if area.width < 40 {
                    // Truncate both app and window names for narrow terminals
                    let app_short = if clean_app.len() > 8 {
                        format!("{}...", &clean_app[..5])
                    } else {
                        clean_app.clone()
                    };
                    let window_short = if window_name.len() > 8 {
                        format!("{}...", &window_name[..5])
                    } else {
                        window_name.clone()
                    };
                    format!("{} ({})", app_short, window_short)
                } else {
                    format!("{} ({})", clean_app, window_name)
                }
            } else {
                // Fallback to just app name if no window name
                if area.width < 40 {
                    if clean_app.len() > 12 {
                        format!("{}...", &clean_app[..9])
                    } else {
                        clean_app
                    }
                } else {
                    clean_app
                }
            };

            let display = format!("{} - {}: {}m [LIVE]", time, display_name, minutes);
            history_items.push(ListItem::new(Line::from(display)).style(Style::default().fg(Color::Green)));
        }

        // Add historical sessions
        let remaining_slots = max_items.saturating_sub(history_items.len());
        history_items.extend(
            self.current_history
                .iter()
                .take(remaining_slots)
                .map(|session| {
                    let minutes = session.duration / 60;
                    let time = session.start_time.format("%H:%M");

                    // Create display name with window name if available
                    let clean_app = Self::clean_app_name(&session.app_name);
                    let display_name = if let Some(window_name) = &session.window_name {
                        if area.width < 40 {
                            // Truncate both app and window names for narrow terminals
                            let app_short = if clean_app.len() > 8 {
                                format!("{}...", &clean_app[..5])
                            } else {
                                clean_app.clone()
                            };
                            let window_short = if window_name.len() > 8 {
                                format!("{}...", &window_name[..5])
                            } else {
                                window_name.clone()
                            };
                            format!("{} ({})", app_short, window_short)
                        } else {
                            format!("{} ({})", clean_app, window_name)
                        }
                    } else {
                        // Fallback to just app name if no window name
                        if area.width < 40 {
                            if clean_app.len() > 12 {
                                format!("{}...", &clean_app[..9])
                            } else {
                                clean_app
                            }
                        } else {
                            clean_app
                        }
                    };

                    let display = format!("{} - {}: {}m", time, display_name, minutes);
                    ListItem::new(Line::from(display))
                })
                .collect::<Vec<ListItem>>()
        );

        let history_list = List::new(history_items)
            .block(Block::default().borders(Borders::ALL).title("📜 Session History"));
        f.render_widget(history_list, area);
    }

    // Pattern matching for determining category from app name
    fn categorize_app(app: &str) -> (&'static str, Color) {
        let app_lower = app.to_lowercase();
        if app_lower.contains("code") || app_lower.contains("vim") || app_lower.contains("nvim") ||
           app_lower.contains("terminal") || app_lower.contains("alacritty") || app_lower.contains("kitty") ||
           app_lower.contains("rust") || app_lower.contains("cargo") || app_lower.contains("editor") ||
           app_lower.contains("vscode") || app_lower.contains("vscodium") || app_lower.contains("gedit") ||
           app_lower.contains("nano") || app_lower.contains("emacs") || app_lower.contains("atom") ||
           app_lower.contains("sublime") {
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

    // Get category and color for an app, using stored category if available
    fn get_app_category(&self, app: &str) -> (&'static str, Color) {
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

    // Convert stored category string back to emoji+name and color
    fn category_from_string(category: &str) -> (&'static str, Color) {
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

    fn draw_pie_chart(&self, f: &mut Frame, area: ratatui::layout::Rect, data: &[(String, i64)]) {
        // Calculate category totals - using BTreeMap for stable sorted order
        let mut categories: BTreeMap<&str, (i64, Color)> = BTreeMap::new();
        let total: i64 = data.iter().map(|(_, d)| d).sum();

        for (app, duration) in data {
            let (category, color) = self.get_app_category(app);
            let entry = categories.entry(category).or_insert((0, color));
            entry.0 += duration;
        }

        // Create pie chart representation as text
        let mut pie_lines = vec![];
        pie_lines.push(Line::from(""));

        // Sort by duration descending for consistent display
        let mut sorted_cats: Vec<_> = categories.iter().collect();
        sorted_cats.sort_by(|a, b| b.1.0.cmp(&a.1.0));

        for (category, (duration, color)) in sorted_cats {
            if total > 0 {
                let percentage = (*duration as f64 / total as f64 * 100.0) as u64;
                let bar_length = (percentage / 5).max(1) as usize; // Scale down for display
                let bar = "█".repeat(bar_length);
                let hours = duration / 3600;
                let minutes = (duration % 3600) / 60;
                let time_str = if hours > 0 {
                    format!("{}h {}m", hours, minutes)
                } else {
                    format!("{}m", minutes)
                };

                pie_lines.push(Line::from(vec![
                    ratatui::text::Span::styled(format!("{} ", category), Style::default().fg(*color)),
                    ratatui::text::Span::styled(bar, Style::default().fg(*color)),
                    ratatui::text::Span::raw(format!(" {}% ({})", percentage, time_str)),
                ]));
            }
        }

        let pie_chart = Paragraph::new(pie_lines)
            .block(Block::default().borders(Borders::ALL).title("🥧 Categories"));
        f.render_widget(pie_chart, area);
    }

    fn draw_timeline(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        // Real-time progress bars showing % of day for each app
        let mut progress_lines = vec![];

        if self.daily_usage.is_empty() {
            progress_lines.push(Line::from("No activity data yet today"));
            let progress = Paragraph::new(progress_lines)
                .block(Block::default().borders(Borders::ALL).title("📊 Today's Activity Progress"));
            f.render_widget(progress, area);
            return;
        }

        // Calculate total seconds in the day so far
        let now = Local::now();
        let start_of_day = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap();
        let seconds_since_midnight = now.signed_duration_since(start_of_day).num_seconds() as f64;

        // Sort apps by usage time (descending)
        let mut sorted_apps: Vec<_> = self.daily_usage.iter().collect();
        sorted_apps.sort_by(|a, b| b.1.cmp(&a.1));

        // Limit to top apps that fit in the area
        let max_items = (area.height.saturating_sub(4) as usize).min(10).max(3);
        let top_apps = &sorted_apps[..sorted_apps.len().min(max_items)];

        // Add top margin (consistent with other cards)
        progress_lines.push(Line::from(""));

        for (app_name, total_seconds) in top_apps {
            let clean_app_name = Self::clean_app_name(app_name);
            let (_, color) = self.get_app_category(app_name);

            // Calculate percentage of day
            let percentage = if seconds_since_midnight > 0.0 {
                ((*total_seconds as f64 / seconds_since_midnight) * 100.0).min(100.0)
            } else {
                0.0
            };

            // Create progress bar (only filled portion visible)
            let bar_width = (area.width.saturating_sub(20) as usize).max(10); // Reserve space for labels
            let filled_width = ((percentage / 100.0) * bar_width as f64) as usize;

            let mut bar_chars = String::new();
            for i in 0..bar_width {
                if i < filled_width {
                    bar_chars.push('█');
                } else {
                    bar_chars.push(' ');
                }
            }

            // Create the progress line
            let progress_line = vec![
                ratatui::text::Span::styled(format!("{:<12}", clean_app_name), Style::default().fg(color)),
                ratatui::text::Span::styled(format!("{:>5.1}%", percentage), Style::default().fg(Color::Cyan)),
                ratatui::text::Span::raw(" "),
                ratatui::text::Span::styled(bar_chars, Style::default().fg(color)),
            ];

            progress_lines.push(Line::from(progress_line));
        }



        let progress = Paragraph::new(progress_lines)
            .block(Block::default().borders(Borders::ALL).title("📊 Today's Activity Progress"));
        f.render_widget(progress, area);
    }

    // Helper function to clean app names by removing gnome- prefixes
    fn clean_app_name(app_name: &str) -> String {
        if app_name.starts_with("gnome-") {
            app_name.strip_prefix("gnome-").unwrap_or(app_name).to_string()
        } else {
            app_name.to_string()
        }
    }

    fn draw_afk(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let afk_threshold_secs = 300; // 5 minutes
        let is_afk = self.is_afk(afk_threshold_secs);
        let status = if is_afk { "AFK" } else { "Active" };
        let color = if is_afk { Color::Red } else { Color::Green };
        let last_input = *self.last_input.lock().unwrap();
        let idle_duration = Local::now().signed_duration_since(last_input).num_seconds();
        let idle_minutes = idle_duration / 60;
        let idle_seconds = idle_duration % 60;

        let afk_lines = vec![
            Line::from(""),
            Line::from(vec![
                ratatui::text::Span::styled("Status: ", Style::default()),
                ratatui::text::Span::styled(status, Style::default().fg(color)),
            ]),
            Line::from(""),
            Line::from(format!("Idle for: {}m {}s", idle_minutes, idle_seconds)),
            Line::from(""),
            Line::from("Detects keyboard/mouse activity"),
            Line::from("AFK if idle > 5 minutes"),
        ];

        let afk_paragraph = Paragraph::new(afk_lines)
            .block(Block::default().borders(Borders::ALL).title("🚫 AFK Status"));
        f.render_widget(afk_paragraph, area);
    }

    // Helper for rendering breakdown sections
    fn draw_breakdown_section(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        title: &str,
        data: &[(String, i64)],
        color: Color,
        _scroll_position: usize,
        is_category: bool,
    ) {
        let max_items = (area.height.saturating_sub(3) as usize).max(3);
        let mut items: Vec<ListItem> = Vec::new();

        if data.is_empty() {
            items.push(ListItem::new(Line::from("  No data available")));
        } else {
            for (name, duration) in data.iter().take(max_items) {
                let hours = duration / 3600;
                let minutes = (duration % 3600) / 60;
                let time_str = if hours > 0 {
                    format!("{}h {}m", hours, minutes)
                } else {
                    format!("{}m", minutes)
                };

                // For categories, extract color from category name
                let item_color = if is_category {
                    Self::category_from_string(name).1
                } else {
                    color
                };

                let display = format!("  {}  {}", name, time_str);
                items.push(ListItem::new(Line::from(display)).style(Style::default().fg(item_color)));
            }
        }

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(list, area);
    }

    // Helper for rendering file breakdown section (special case with language)
    fn draw_file_breakdown_section(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        _scroll_position: usize,
    ) {
        let max_items = (area.height.saturating_sub(3) as usize).max(3);
        let mut items: Vec<ListItem> = Vec::new();

        if self.file_breakdown.is_empty() {
            items.push(ListItem::new(Line::from("  No file data available")));
        } else {
            for (filename, language, duration) in self.file_breakdown.iter().take(max_items) {
                let hours = duration / 3600;
                let minutes = (duration % 3600) / 60;
                let time_str = if hours > 0 {
                    format!("{}h {}m", hours, minutes)
                } else {
                    format!("{}m", minutes)
                };

                let display = format!("  {} ({})  {}", filename, language, time_str);
                items.push(ListItem::new(Line::from(display)).style(Style::default().fg(Color::Cyan)));
            }
        }

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("📝 Files Edited"));
        f.render_widget(list, area);
    }

    // Helper function to create a session with parsed data
    fn create_session_with_parsing(app_name: String, window_name: Option<String>, start_time: DateTime<Local>, category: String) -> Session {
        // Parse window name if available
        let parsed = if let Some(ref win_name) = window_name {
            parser::parse_window_name(&app_name, win_name)
        } else {
            parser::ParsedSessionData::default()
        };

        // Convert parsed data to JSON
        let parsed_json = serde_json::to_value(&parsed).ok();

        Session {
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
            // Terminal fields
            terminal_username: parsed.terminal_username,
            terminal_hostname: parsed.terminal_hostname,
            terminal_directory: parsed.terminal_directory,
            terminal_project_name: parsed.terminal_project_name,
            // Editor fields
            editor_filename: parsed.editor_filename,
            editor_filepath: parsed.editor_filepath,
            editor_project_path: parsed.editor_project_path,
            editor_language: parsed.editor_language,
            // Multiplexer fields
            tmux_window_name: parsed.tmux_window_name,
            tmux_pane_count: parsed.tmux_pane_count,
            terminal_multiplexer: parsed.terminal_multiplexer,
            // IDE fields
            ide_project_name: parsed.ide_project_name,
            ide_file_open: parsed.ide_file_open,
            ide_workspace: parsed.ide_workspace,
            // Metadata
            parsed_data: parsed_json,
            parsing_success: Some(parsed.parsing_success),
        }
    }

    async fn start_tracking(&mut self) -> Result<()> {
        let app_name = if let Some(manual_name) = self.manual_app_name.clone() {
            manual_name
        } else {
            match self.monitor.get_active_app_async().await {
                Ok(detected) => {
                    self.current_app = detected.clone();
                    detected
                }
                Err(e) => {
                    let error_msg = format!("Window detection failed: {}", e);
                     self.logs.push(format!("[{}] {}", Local::now().format("%H:%M:%S"), error_msg));
                    eprintln!("{}", error_msg);
                    self.current_app = "Unknown".to_string();
                    "Unknown".to_string()
                }
            }
        };
        let window_name = self.monitor.get_active_window_name_async().await.ok();
        let start_time = Local::now();
        // Determine category from original app name
        let (category_name, _) = Self::categorize_app(&app_name);

        // Create session with parsed data
        let session = Self::create_session_with_parsing(
            app_name.clone(),
            window_name.clone(),
            start_time,
            category_name.to_string(),
        );

        self.current_session = Some(session);
        self.current_window = window_name;
        self.logs.push(format!("[{}] Started tracking: {}", Local::now().format("%H:%M:%S"), app_name));
        Ok(())
    }

    async fn switch_app(&mut self, new_app: String) -> Result<()> {
        // End current session
        if let Some(mut session) = self.current_session.take() {
            session.duration = Local::now().signed_duration_since(session.start_time).num_seconds();

            // Save ALL sessions regardless of duration
            if let Err(e) = self.database.insert_session(&session).await {
                log::error!("Failed to save session: {}", e);
                self.logs.push(format!("[{}] Failed to save session: {}", Local::now().format("%H:%M:%S"), e));
            } else {
                // Refresh all usage data after saving session
                self.usage = self.database.get_app_usage().await?;
                self.daily_usage = self.database.get_daily_usage().await.unwrap_or_default();
                self.weekly_usage = self.database.get_weekly_usage().await.unwrap_or_default();
                self.monthly_usage = self.database.get_monthly_usage().await.unwrap_or_default();
                self.history = self.database.get_recent_sessions(30).await.unwrap_or_default();

                // Update current_history based on current view mode
                if let AppState::Dashboard { ref view_mode } = self.state {
                    self.current_history = match view_mode {
                        ViewMode::Daily => self.database.get_daily_sessions().await.unwrap_or_default(),
                        ViewMode::Weekly => self.database.get_weekly_sessions().await.unwrap_or_default(),
                        ViewMode::Monthly => self.database.get_monthly_sessions().await.unwrap_or_default(),
                    };
                }
                self.logs.push(format!("[{}] Saved session: {} for {}s", Local::now().format("%H:%M:%S"), session.app_name, session.duration));
            }
        }
        // Start new session
        let window_name = self.monitor.get_active_window_name_async().await.ok();
        let start_time = Local::now();
        // Determine category from original app name
        let (category_name, _) = Self::categorize_app(&new_app);

        // Create session with parsed data
        let session = Self::create_session_with_parsing(
            new_app.clone(),
            window_name.clone(),
            start_time,
            category_name.to_string(),
        );

        self.current_session = Some(session);
        self.current_app = new_app.clone();
        self.current_window = window_name;
        self.logs.push(format!("[{}] Switched to: {}", Local::now().format("%H:%M:%S"), new_app));
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

    fn view_logs(&mut self) {
        self.state = AppState::ViewingLogs;
    }

    fn is_afk(&self, threshold_secs: i64) -> bool {
        let last = *self.last_input.lock().unwrap();
        Local::now().signed_duration_since(last).num_seconds() > threshold_secs
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
                if !buffer.is_empty() {
                    // Get the original category before renaming
                    let original_category = self.get_app_category(&old_name).0.to_string();

                    // Rename app in database while preserving category
                    if let Err(e) = self.database.rename_app_with_category(&old_name, &buffer, &original_category).await {
                        self.logs.push(format!("[{}] Failed to rename app: {}", Local::now().format("%H:%M:%S"), e));
                    } else {
                        // Update current session if it matches
                        if let Some(session) = &mut self.current_session {
                            if session.app_name == old_name {
                                session.app_name = buffer.clone();
                                session.category = Some(original_category.clone());
                            }
                        }
                        // Refresh ALL usage data (all time, daily, weekly, monthly, history)
                        self.usage = self.database.get_app_usage().await?;
                        self.daily_usage = self.database.get_daily_usage().await?;
                        self.weekly_usage = self.database.get_weekly_usage().await?;
                        self.monthly_usage = self.database.get_monthly_usage().await?;
                        self.history = self.database.get_recent_sessions(30).await?;

                        // Update current_history based on current view mode
                        if let AppState::Dashboard { ref view_mode } = self.state {
                            self.current_history = match view_mode {
                                ViewMode::Daily => self.database.get_daily_sessions().await.unwrap_or_default(),
                                ViewMode::Weekly => self.database.get_weekly_sessions().await.unwrap_or_default(),
                                ViewMode::Monthly => self.database.get_monthly_sessions().await.unwrap_or_default(),
                            };
                        }
                        self.logs.push(format!("[{}] Renamed '{}' to '{}' (preserved category: {})", Local::now().format("%H:%M:%S"), old_name, buffer, original_category));
                    }
                }
                self.state = AppState::Dashboard { view_mode: self.current_view_mode.clone() };
            }
        }
        Ok(())
    }
}