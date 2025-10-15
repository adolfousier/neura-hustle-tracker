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

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Daily,
    Weekly,
    Monthly,
    History,
}

#[derive(Debug, Clone)]
pub enum InputAction {
    RenameApp { old_name: String },
    SetAppName,
}

#[derive(Debug, Clone)]
pub enum AppState {
    Dashboard { view_mode: ViewMode },
    ViewingLogs,
    SelectingApp { selected_index: usize },
    Input { prompt: String, buffer: String, action: InputAction },
    CommandsPopup,
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
    logs: Vec<String>,
    manual_app_name: Option<String>,
    current_app: String,
    current_window: Option<String>,
    current_session: Option<Session>,
    last_input: Arc<Mutex<DateTime<Local>>>,
}

impl App {
    pub fn new(database: Database) -> Self {
        let monitor = AppMonitor::new();
        let last_input = Arc::new(Mutex::new(Local::now()));
        let last_input_clone = Arc::clone(&last_input);
        std::thread::spawn(move || {
            let callback = move |event: rdev::Event| {
                match event.event_type {
                    EventType::KeyPress(_) | EventType::KeyRelease(_) | EventType::ButtonPress(_) | EventType::ButtonRelease(_) | EventType::MouseMove { .. } => {
                        *last_input_clone.lock().unwrap() = Local::now();
                    }
                    _ => {}
                }
            };
            if let Err(error) = listen(callback) {
                eprintln!("Error listening for input events: {:?}", error);
            }
        });
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
            logs: vec![],
            manual_app_name: None,
            current_app: "unknown".to_string(),
            current_window: None,
            current_session: None,
            last_input,
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
                    match &mut self.state {

                        AppState::ViewingLogs => {
                            match key.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Esc => self.state = AppState::Dashboard { view_mode: ViewMode::Daily },
                                _ => self.state = AppState::Dashboard { view_mode: ViewMode::Daily },
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
                                KeyCode::Esc => self.state = AppState::Dashboard { view_mode: ViewMode::Daily },
                                _ => {}
                            }
                        }

                        AppState::Input { buffer, .. } => {
                            match key.code {
                                KeyCode::Char(c) => buffer.push(c),
                                KeyCode::Backspace => { buffer.pop(); }
                                KeyCode::Enter => self.handle_input().await?,
                                KeyCode::Esc => self.state = AppState::Dashboard { view_mode: ViewMode::Daily },
                                _ => {}
                            }
                        }
                        AppState::Dashboard { view_mode } => {
                            match key.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Char('r') => self.start_app_selection(),
                                KeyCode::Char('m') => self.start_set_app_name(),
                                KeyCode::Char('u') => self.update_current_app(),
                                KeyCode::Char('l') => self.view_logs(),
                                KeyCode::Char('C') => self.state = AppState::CommandsPopup,  // Shift+C
                                KeyCode::Tab => {
                                     *view_mode = match view_mode {
                                         ViewMode::Daily => ViewMode::Weekly,
                                         ViewMode::Weekly => ViewMode::Monthly,
                                         ViewMode::Monthly => ViewMode::History,
                                         ViewMode::History => ViewMode::Daily,
                                     };
                                     self.update_history().await?;
                                 }
                                _ => {}
                            }
                        }
                        AppState::CommandsPopup => {
                            match key.code {
                                KeyCode::Esc => self.state = AppState::Dashboard { view_mode: ViewMode::Daily },
                                KeyCode::Char('q') => break,
                                KeyCode::Char('r') => {
                                    self.start_app_selection();
                                }
                                KeyCode::Char('m') => {
                                    self.start_set_app_name();
                                }
                                KeyCode::Char('u') => {
                                    self.update_current_app();
                                    self.state = AppState::Dashboard { view_mode: ViewMode::Daily };
                                }
                                KeyCode::Char('l') => {
                                    self.view_logs();
                                }
                                _ => {}
                            }
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
        }

        // Save current session on exit
        if let Some(mut session) = self.current_session.take() {
            session.duration = Local::now().signed_duration_since(session.start_time).num_seconds();
            if session.duration >= 10 {
                if let Err(e) = self.database.insert_session(&session).await {
                    log::error!("Failed to save session on exit: {}", e);
                    self.logs.push(format!("Failed to save session: {}", e));
                } else {
                    self.history = self.database.get_recent_sessions(30).await?;
                    self.usage = self.database.get_app_usage().await?;
                    self.logs.push(format!("Ended session: {} for {}s", session.app_name, session.duration));
                }
            } else {
                self.logs.push(format!("Skipped saving short session on exit: {} for {}s", session.app_name, session.duration));
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
                    format!("Tracking: {} for {}s | [Shift+C] Commands", display_name, duration)
                } else {
                    format!("Not tracking - Current app: {} | [Shift+C] Commands", self.current_app)
                }
            }
            AppState::ViewingLogs => "Viewing Logs - Press any key to return".to_string(),
            AppState::SelectingApp { .. } => "Rename Mode - Use arrow keys to select an app".to_string(),
            AppState::Input { action, .. } => {
                match action {
                    InputAction::RenameApp { .. } => "Rename Mode - Enter new name for the app".to_string(),
                    InputAction::SetAppName => "Manual App Name - Enter the app name to track".to_string(),
                }
            }
            AppState::CommandsPopup => "Commands Menu - Press key to execute or Esc to close".to_string(),
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

                        let display = format!("{}{:<30} {}", prefix, app, time_display);

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
                    InputAction::SetAppName => "🏷️  Set App Name",
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
                self.draw_dashboard(f, chunks[1], &ViewMode::Daily);

                // Draw popup overlay
                let popup_area = Self::centered_rect(60, 50, size);
                f.render_widget(ratatui::widgets::Clear, popup_area);

                let commands_text = vec![
                    Line::from(""),
                    Line::from("  [Tab]  Switch View (Daily/Weekly/Monthly/History)"),
                    Line::from("  [r]    Rename app/tab"),
                    Line::from("  [m]    Manually set app name"),
                    Line::from("  [u]    Update current app detection"),
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
            ViewMode::History => (&self.daily_usage, "📊 Daily Usage"), // Default to daily when in history view
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

                    Bar::default()
                        .value(*value_minutes)
                        .label(Line::from(format!("{}", app)))
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
        let max_items = (area.height.saturating_sub(2) as usize).min(15).max(3);

        let stats_items: Vec<ListItem> = data
            .iter()
            .take(max_items)
            .map(|(app, duration)| {
                let hours = duration / 3600;
                let minutes = (duration % 3600) / 60;

                // Truncate app name if terminal is narrow
                let app_display = if area.width < 40 {
                    if app.len() > 15 {
                        format!("{}...", &app[..12])
                    } else {
                        app.clone()
                    }
                } else {
                    app.clone()
                };

                let display = if hours > 0 {
                    format!("  {} - {}h {}m", app_display, hours, minutes)
                } else {
                    format!("  {} - {}m", app_display, minutes)
                };
                ListItem::new(Line::from(display))
            })
            .collect();

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
        let max_items = (area.height.saturating_sub(2) as usize).min(30).max(5);

        let history_items: Vec<ListItem> = self
            .current_history
            .iter()
            .take(max_items)
            .map(|session| {
                let minutes = session.duration / 60;
                let time = session.start_time.format("%H:%M");

                // Create display name with window name if available
                let display_name = if let Some(window_name) = &session.window_name {
                    if area.width < 40 {
                        // Truncate both app and window names for narrow terminals
                        let app_short = if session.app_name.len() > 8 {
                            format!("{}...", &session.app_name[..5])
                        } else {
                            session.app_name.clone()
                        };
                        let window_short = if window_name.len() > 8 {
                            format!("{}...", &window_name[..5])
                        } else {
                            window_name.clone()
                        };
                        format!("{} ({})", app_short, window_short)
                    } else {
                        format!("{} ({})", session.app_name, window_name)
                    }
                } else {
                    // Fallback to just app name if no window name
                    if area.width < 40 {
                        if session.app_name.len() > 12 {
                            format!("{}...", &session.app_name[..9])
                        } else {
                            session.app_name.clone()
                        }
                    } else {
                        session.app_name.clone()
                    }
                };

                let display = format!("{} - {}: {}m", time, display_name, minutes);
                ListItem::new(Line::from(display))
            })
            .collect();
        let history_list = List::new(history_items)
            .block(Block::default().borders(Borders::ALL).title("📜 Session History"));
        f.render_widget(history_list, area);
    }

    // Pattern matching for determining category from app name
    fn categorize_app(app: &str) -> (&'static str, Color) {
        let app_lower = app.to_lowercase();
        if app_lower.contains("code") || app_lower.contains("vim") || app_lower.contains("nvim") ||
           app_lower.contains("terminal") || app_lower.contains("alacritty") || app_lower.contains("kitty") ||
           app_lower.contains("rust") || app_lower.contains("cargo") {
            ("💻 Development", Color::Cyan)
        } else if app_lower.contains("browser") || app_lower.contains("chrome") || app_lower.contains("firefox") ||
                  app_lower.contains("brave") || app_lower.contains("edge") {
            ("🌐 Browsing", Color::LightBlue)
        } else if app_lower.contains("slack") || app_lower.contains("zoom") || app_lower.contains("teams") ||
                  app_lower.contains("discord") || app_lower.contains("telegram") {
            ("💬 Communication", Color::LightGreen)
        } else if app_lower.contains("spotify") || app_lower.contains("vlc") || app_lower.contains("music") {
            ("🎵 Media", Color::LightMagenta)
        } else if app_lower.contains("nautilus") || app_lower.contains("files") || app_lower.contains("dolphin") {
            ("📁 Files", Color::LightYellow)
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
            "💻 Development" => ("💻 Development", Color::Cyan),
            "🌐 Browsing" => ("🌐 Browsing", Color::LightBlue),
            "💬 Communication" => ("💬 Communication", Color::LightGreen),
            "🎵 Media" => ("🎵 Media", Color::LightMagenta),
            "📁 Files" => ("📁 Files", Color::LightYellow),
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
        // Show recent activity timeline with color-coded apps
        let mut timeline_lines = vec![];
        timeline_lines.push(Line::from(""));

        // Adaptive: show more items if we have height, fewer if narrow
        let max_timeline_items = (area.height.saturating_sub(3) as usize).min(12).max(4);
        let max_bar_length = if area.width < 60 { 10 } else if area.width < 100 { 15 } else { 20 };

        for session in self.current_history.iter().take(max_timeline_items) {
            // Use stored category if available, otherwise pattern match
            let (_, color) = if let Some(stored_category) = &session.category {
                Self::category_from_string(stored_category)
            } else {
                Self::categorize_app(&session.app_name)
            };
            let time = session.start_time.format("%H:%M");
            let duration_min = session.duration / 60;

            // Adaptive bar length based on available width
            let bar_length = ((duration_min / 2).max(1) as usize).min(max_bar_length);
            let bar = "▓".repeat(bar_length);

            // Truncate app name if terminal is narrow
            let app_display = if area.width < 60 {
                if session.app_name.len() > 12 {
                    format!("{}...", &session.app_name[..9])
                } else {
                    session.app_name.clone()
                }
            } else {
                session.app_name.clone()
            };

            timeline_lines.push(Line::from(vec![
                ratatui::text::Span::raw(format!("{} ", time)),
                ratatui::text::Span::styled(bar, Style::default().fg(color)),
                ratatui::text::Span::raw(format!(" {} ({}m)", app_display, duration_min)),
            ]));
        }

        let timeline = Paragraph::new(timeline_lines)
            .block(Block::default().borders(Borders::ALL).title("⏱️  Timeline"));
        f.render_widget(timeline, area);
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
                    self.logs.push(error_msg.clone());
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
        let session = Session {
            id: None,
            app_name: app_name.clone(),
            window_name: window_name.clone(),
            start_time,
            duration: 0,
            category: Some(category_name.to_string()),
        };
        self.current_session = Some(session);
        self.current_window = window_name;
        self.logs.push(format!("Started tracking: {}", app_name));
        Ok(())
    }

    async fn switch_app(&mut self, new_app: String) -> Result<()> {
        // End current session
        if let Some(mut session) = self.current_session.take() {
            session.duration = Local::now().signed_duration_since(session.start_time).num_seconds();
            if session.duration >= 10 {
                if let Err(e) = self.database.insert_session(&session).await {
                    log::error!("Failed to save session: {}", e);
                } else {
                    self.usage = self.database.get_app_usage().await?;
                }
            } else {
                self.logs.push(format!("Skipped saving short session: {} for {}s", session.app_name, session.duration));
            }
        }
        // Start new session
        let window_name = self.monitor.get_active_window_name_async().await.ok();
        let start_time = Local::now();
        // Determine category from original app name
        let (category_name, _) = Self::categorize_app(&new_app);
        let session = Session {
            id: None,
            app_name: new_app.clone(),
            window_name: window_name.clone(),
            start_time,
            duration: 0,
            category: Some(category_name.to_string()),
        };
        self.current_session = Some(session);
        self.current_app = new_app.clone();
        self.current_window = window_name;
        self.logs.push(format!("Switched to: {}", new_app));
        Ok(())
    }


    fn start_set_app_name(&mut self) {
        self.state = AppState::Input {
            prompt: "Enter app name to track".to_string(),
            buffer: String::new(),
            action: InputAction::SetAppName,
        };
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

    fn update_current_app(&mut self) {
        if self.manual_app_name.is_none() {
            self.current_app = self.monitor.get_active_app().unwrap_or("unknown".to_string());
            self.current_window = self.monitor.get_active_window_name().ok();
        }
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
                ViewMode::History => self.database.get_recent_sessions(30).await?,
            };
        }
        Ok(())
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
                    // Rename app in database
                    if let Err(e) = self.database.rename_app(&old_name, &buffer).await {
                        self.logs.push(format!("Failed to rename app: {}", e));
                    } else {
                        // Update current session if it matches
                        if let Some(session) = &mut self.current_session {
                            if session.app_name == old_name {
                                session.app_name = buffer.clone();
                            }
                        }
                        // Refresh ALL usage data (all time, daily, weekly, monthly, history)
                        self.usage = self.database.get_app_usage().await?;
                        self.daily_usage = self.database.get_daily_usage().await?;
                        self.weekly_usage = self.database.get_weekly_usage().await?;
                        self.monthly_usage = self.database.get_monthly_usage().await?;
                        self.history = self.database.get_recent_sessions(30).await?;
                        self.logs.push(format!("Renamed '{}' to '{}'", old_name, buffer));
                    }
                }
                self.state = AppState::Dashboard { view_mode: ViewMode::Daily };
            }
            InputAction::SetAppName => {
                self.manual_app_name = Some(buffer.clone());
                self.current_app = buffer;
                self.state = AppState::Dashboard { view_mode: ViewMode::Daily };
            }
        }
        Ok(())
    }
}