use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, EnableMouseCapture, DisableMouseCapture};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;

use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Line;
use ratatui::widgets::{BarChart, Block, Borders, List, ListItem, Paragraph};
use ratatui::style::{Color, Style};
use ratatui::{Frame, Terminal};
use std::collections::BTreeMap;
use std::io;
use std::time::{Duration, Instant};

use chrono::Utc;

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
}

pub struct App {
    state: AppState,
    database: Database,
    monitor: AppMonitor,
    history: Vec<Session>,
    usage: Vec<(String, i64)>,
    daily_usage: Vec<(String, i64)>,
    weekly_usage: Vec<(String, i64)>,
    monthly_usage: Vec<(String, i64)>,
    logs: Vec<String>,
    manual_app_name: Option<String>,
    current_app: String,
    current_window: Option<String>,
    current_session: Option<Session>,
}

impl App {
    pub fn new(database: Database) -> Self {
        let monitor = AppMonitor::new();
        Self {
            state: AppState::Dashboard { view_mode: ViewMode::Daily },
            database,
            monitor,
            history: vec![],
            usage: vec![],
            daily_usage: vec![],
            weekly_usage: vec![],
            monthly_usage: vec![],
            logs: vec![],
            manual_app_name: None,
            current_app: "unknown".to_string(),
            current_window: None,
            current_session: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        log::info!("Starting UI...");

        // Start tracking initial app before enabling raw mode
        self.start_tracking().await?;

        // Load history and usage (load 30 sessions for display)
        self.history = self.database.get_recent_sessions(30).await.unwrap();
        self.usage = self.database.get_app_usage().await.unwrap();
        self.daily_usage = self.database.get_daily_usage().await.unwrap();
        self.weekly_usage = self.database.get_weekly_usage().await.unwrap();
        self.monthly_usage = self.database.get_monthly_usage().await.unwrap();

        eprintln!("Enabling raw mode...");
        enable_raw_mode().unwrap();
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut last_save = Instant::now();
        let save_interval = Duration::from_secs(600); // 10 minutes

        loop {
            terminal.draw(|f| self.draw(f))?;

            // Check for app or window change
            if let Ok(active_app) = self.monitor.get_active_app() {
                let active_window = self.monitor.get_active_window_name().ok();
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
                                KeyCode::Char('e') => self.end_session().await?,
                                KeyCode::Char('r') => self.start_app_selection(),
                                KeyCode::Char('m') => self.start_set_app_name(),
                                KeyCode::Char('u') => self.update_current_app(),
                                KeyCode::Char('l') => self.view_logs(),
                                KeyCode::Tab => {
                                    *view_mode = match view_mode {
                                        ViewMode::Daily => ViewMode::Weekly,
                                        ViewMode::Weekly => ViewMode::Monthly,
                                        ViewMode::Monthly => ViewMode::History,
                                        ViewMode::History => ViewMode::Daily,
                                    };
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
                    session.duration = Utc::now().signed_duration_since(session.start_time).num_seconds();
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
            session.duration = Utc::now().signed_duration_since(session.start_time).num_seconds();
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
            .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(3)].as_ref())
            .split(size);

        // Status bar
        let status = match &self.state {
            AppState::Dashboard { .. } => {
                if let Some(session) = &self.current_session {
                    let duration = Utc::now().signed_duration_since(session.start_time).num_seconds();
                    let display_name = self.manual_app_name.as_ref().unwrap_or(&session.app_name);
                    format!("Tracking: {} for {}s", display_name, duration)
                } else {
                    format!("Not tracking - Current app: {}", self.current_app)
                }
            }
            AppState::ViewingLogs => "Viewing Logs - Press any key to return".to_string(),
            AppState::SelectingApp { .. } => "Select app to rename (↑/↓ + Enter) or Esc to cancel".to_string(),
            AppState::Input { prompt, buffer, .. } => format!("{}: {}", prompt, buffer),
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
                let usage_items: Vec<ListItem> = self
                    .usage
                    .iter()
                    .enumerate()
                    .take(10)
                    .map(|(i, (app, duration))| {
                        let minutes = duration / 60;
                        let prefix = if i == *selected_index { "→ " } else { "  " };
                        let display = format!("{}{}: {}m", prefix, app, minutes);
                        ListItem::new(Line::from(display))
                    })
                    .collect();
                let usage_list = List::new(usage_items)
                    .block(Block::default().borders(Borders::ALL).title("App Usage - Select to Rename"));
                f.render_widget(usage_list, chunks[1]);
            }

            AppState::Input { .. } => {
                // Input is shown in status bar, show daily usage as default
                self.draw_dashboard(f, chunks[1], &ViewMode::Daily);
            }

            AppState::Dashboard { view_mode } => {
                self.draw_dashboard(f, chunks[1], view_mode);
            }
        }

        // Commands bar at bottom
        let commands = "[Tab] Switch View | [r] Rename app | [e] End | [l] Logs | [q] Quit";
        let commands_widget = Paragraph::new(commands)
            .block(Block::default().borders(Borders::ALL).title("Commands"));
        f.render_widget(commands_widget, chunks[2]);
    }

    fn draw_dashboard(&self, f: &mut Frame, area: ratatui::layout::Rect, view_mode: &ViewMode) {
        // Split into left (chart & timeline & stats) and right (history & pie) - 50/50
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        let (data, title) = match view_mode {
            ViewMode::Daily => (&self.daily_usage, "📊 Daily Usage"),
            ViewMode::Weekly => (&self.weekly_usage, "📊 Weekly Usage (7 days)"),
            ViewMode::Monthly => (&self.monthly_usage, "📊 Monthly Usage (30 days)"),
            ViewMode::History => (&self.daily_usage, "📊 Daily Usage"), // Default to daily when in history view
        };

        // Create bar chart data
        let bar_data: Vec<(&str, u64)> = data
            .iter()
            .take(10)
            .map(|(app, duration)| {
                let minutes = (duration / 60) as u64;
                (app.as_str(), minutes)
            })
            .collect();

        // LEFT SIDE: Bar Chart + Timeline + Stats
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),  // Bar chart
                Constraint::Percentage(30),  // Timeline
                Constraint::Percentage(30),  // Detailed stats
            ].as_ref())
            .split(main_chunks[0]);

        // RIGHT SIDE: Session History + Pie Chart
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
            .split(main_chunks[1]);

        // LEFT TOP: Bar chart
        if bar_data.is_empty() {
            let empty_msg = Paragraph::new("No data available yet. Start tracking!")
                .block(Block::default().borders(Borders::ALL).title(title));
            f.render_widget(empty_msg, left_chunks[0]);
        } else {
            let barchart = BarChart::default()
                .block(Block::default().borders(Borders::ALL).title(title))
                .bar_width(6)
                .bar_gap(1)
                .bar_style(Style::default().fg(Color::Cyan))
                .value_style(Style::default().fg(Color::White))
                .data(&bar_data);
            f.render_widget(barchart, left_chunks[0]);
        }

        // LEFT MIDDLE: Timeline
        self.draw_timeline(f, left_chunks[1]);

        // LEFT BOTTOM: Detailed stats list
        let stats_items: Vec<ListItem> = data
            .iter()
            .take(8)
            .map(|(app, duration)| {
                let hours = duration / 3600;
                let minutes = (duration % 3600) / 60;
                let display = if hours > 0 {
                    format!("  {} - {}h {}m", app, hours, minutes)
                } else {
                    format!("  {} - {}m", app, minutes)
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
        f.render_widget(stats_list, left_chunks[2]);

        // RIGHT TOP: Session History
        let history_items: Vec<ListItem> = self
            .history
            .iter()
            .take(20)
            .map(|session| {
                let minutes = session.duration / 60;
                let time = session.start_time.format("%H:%M");
                let display = format!("{} - {}: {}m", time, session.app_name, minutes);
                ListItem::new(Line::from(display))
            })
            .collect();
        let history_list = List::new(history_items)
            .block(Block::default().borders(Borders::ALL).title("📜 Session History"));
        f.render_widget(history_list, right_chunks[0]);

        // RIGHT BOTTOM: Pie Chart (Categories)
        self.draw_pie_chart(f, right_chunks[1], data);
    }

    fn categorize_app(app: &str) -> (&'static str, Color) {
        let app_lower = app.to_lowercase();
        if app_lower.contains("code") || app_lower.contains("vim") || app_lower.contains("nvim") ||
           app_lower.contains("terminal") || app_lower.contains("alacritty") || app_lower.contains("kitty") ||
           app_lower.contains("rust") || app_lower.contains("cargo") {
            ("💻 Development", Color::Cyan)
        } else if app_lower.contains("browser") || app_lower.contains("chrome") || app_lower.contains("firefox") ||
                  app_lower.contains("brave") || app_lower.contains("edge") {
            ("🌐 Browsing", Color::Blue)
        } else if app_lower.contains("slack") || app_lower.contains("zoom") || app_lower.contains("teams") ||
                  app_lower.contains("discord") || app_lower.contains("telegram") {
            ("💬 Communication", Color::Green)
        } else if app_lower.contains("spotify") || app_lower.contains("vlc") || app_lower.contains("music") {
            ("🎵 Media", Color::Magenta)
        } else if app_lower.contains("nautilus") || app_lower.contains("files") || app_lower.contains("dolphin") {
            ("📁 Files", Color::Yellow)
        } else {
            ("📦 Other", Color::Gray)
        }
    }

    fn draw_pie_chart(&self, f: &mut Frame, area: ratatui::layout::Rect, data: &[(String, i64)]) {
        // Calculate category totals - using BTreeMap for stable sorted order
        let mut categories: BTreeMap<&str, (i64, Color)> = BTreeMap::new();
        let total: i64 = data.iter().map(|(_, d)| d).sum();

        for (app, duration) in data {
            let (category, color) = Self::categorize_app(app);
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

        // Take last 8 sessions for timeline
        for session in self.history.iter().take(8) {
            let (_, color) = Self::categorize_app(&session.app_name);
            let time = session.start_time.format("%H:%M");
            let duration_min = session.duration / 60;
            let bar_length = (duration_min / 2).max(1).min(15) as usize; // Scale for visual
            let bar = "▓".repeat(bar_length);

            timeline_lines.push(Line::from(vec![
                ratatui::text::Span::raw(format!("{} ", time)),
                ratatui::text::Span::styled(bar, Style::default().fg(color)),
                ratatui::text::Span::raw(format!(" {} ({}m)", session.app_name, duration_min)),
            ]));
        }

        let timeline = Paragraph::new(timeline_lines)
            .block(Block::default().borders(Borders::ALL).title("⏱️  Timeline"));
        f.render_widget(timeline, area);
    }

    async fn start_tracking(&mut self) -> Result<()> {
        let app_name = self.manual_app_name.clone().unwrap_or_else(|| {
            let detected = self.monitor.get_active_app().unwrap_or("Unknown".to_string());
            self.current_app = detected.clone();
            detected
        });
        let window_name = self.monitor.get_active_window_name().ok();
        let start_time = Utc::now();
        let session = Session {
            id: None,
            app_name: app_name.clone(),
            window_name: window_name.clone(),
            start_time,
            duration: 0,
        };
        self.current_session = Some(session);
        self.current_window = window_name;
        self.logs.push(format!("Started tracking: {}", app_name));
        Ok(())
    }

    async fn switch_app(&mut self, new_app: String) -> Result<()> {
        // End current session
        if let Some(mut session) = self.current_session.take() {
            session.duration = Utc::now().signed_duration_since(session.start_time).num_seconds();
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
        let window_name = self.monitor.get_active_window_name().ok();
        let start_time = Utc::now();
        let session = Session {
            id: None,
            app_name: new_app.clone(),
            window_name: window_name.clone(),
            start_time,
            duration: 0,
        };
        self.current_session = Some(session);
        self.current_app = new_app.clone();
        self.current_window = window_name;
        self.logs.push(format!("Switched to: {}", new_app));
        Ok(())
    }

    async fn end_session(&mut self) -> Result<()> {
        if let Some(mut session) = self.current_session.take() {
            session.duration = Utc::now().signed_duration_since(session.start_time).num_seconds();
            if session.duration >= 10 {
                self.database.insert_session(&session).await.unwrap();
                self.history = self.database.get_recent_sessions(30).await.unwrap();
                self.usage = self.database.get_app_usage().await.unwrap();
                self.daily_usage = self.database.get_daily_usage().await.unwrap();
                self.weekly_usage = self.database.get_weekly_usage().await.unwrap();
                self.monthly_usage = self.database.get_monthly_usage().await.unwrap();
                self.logs.push(format!("Ended session: {} for {}s", session.app_name, session.duration));
            } else {
                self.logs.push(format!("Skipped ending short session: {} for {}s", session.app_name, session.duration));
            }
        }
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