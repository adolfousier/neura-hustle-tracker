use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, EnableMouseCapture, DisableMouseCapture};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;

use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{Frame, Terminal};
use std::io;
use std::time::{Duration, Instant};

use chrono::Utc;

use crate::database::connection::Database;
use crate::models::session::Session;
use crate::tracker::monitor::AppMonitor;

#[derive(Debug, Clone)]
pub enum InputAction {
    Rename,
    SetAppName,
}

#[derive(Debug, Clone)]
pub enum AppState {
    Idle,
    ViewingHistory,
    ViewingLogs,
    Input { prompt: String, buffer: String, action: InputAction },
}

pub struct App {
    state: AppState,
    database: Database,
    monitor: AppMonitor,
    history: Vec<Session>,
    usage: Vec<(String, i64)>,
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
            state: AppState::Idle,
            database,
            monitor,
            history: vec![],
            usage: vec![],
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

        // Load history and usage
        self.history = self.database.get_recent_sessions(10).await.unwrap();
        self.usage = self.database.get_app_usage().await.unwrap();

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
                        AppState::ViewingHistory => {
                            match key.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Esc => self.state = AppState::Idle,
                                _ => self.state = AppState::Idle,
                            }
                        }
                        AppState::ViewingLogs => {
                            match key.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Esc => self.state = AppState::Idle,
                                _ => self.state = AppState::Idle,
                            }
                        }

                        AppState::Input { buffer, .. } => {
                            match key.code {
                                KeyCode::Char(c) => buffer.push(c),
                                KeyCode::Backspace => { buffer.pop(); }
                                KeyCode::Enter => self.handle_input(),
                                KeyCode::Esc => self.state = AppState::Idle,
                                _ => {}
                            }
                        }
                         _ => {
                            match key.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Char('e') => self.end_session().await?,
                                KeyCode::Char('r') => self.start_rename(),
                                KeyCode::Char('m') => self.start_set_app_name(),
                                KeyCode::Char('u') => self.update_current_app(),
                                KeyCode::Char('v') => self.view_history().await,
                                KeyCode::Char('l') => self.view_logs(),
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
                    self.history = self.database.get_recent_sessions(10).await?;
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

        let status = match &self.state {
            AppState::Idle => {
                if let Some(session) = &self.current_session {
                    let duration = Utc::now().signed_duration_since(session.start_time).num_seconds();
                    let display_name = if let Some(window) = &session.window_name {
                        format!("{} - {}", session.app_name, window)
                    } else {
                        session.app_name.clone()
                    };
                    format!("Tracking: {} for {}s", display_name, duration)
                } else {
                    format!("Not tracking - Current app: {}", self.current_app)
                }
            }
            AppState::ViewingHistory => "Viewing History - Press any key to return".to_string(),
            AppState::ViewingLogs => "Viewing Logs - Press any key to return".to_string(),
            AppState::Input { .. } => "Input mode - Enter to confirm, Esc to cancel".to_string(),
        };

        let status_widget = Paragraph::new(status)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status_widget, chunks[0]);

        match &self.state {
            AppState::ViewingHistory => {
                let items: Vec<ListItem> = self
                    .history
                    .iter()
                    .map(|s| {
                        let display = if let Some(window) = &s.window_name {
                            format!("{} ({}) - {}s", s.app_name, window, s.duration)
                        } else {
                            format!("{} - {}s", s.app_name, s.duration)
                        };
                        ListItem::new(Line::from(display))
                    })
                    .collect();
                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Recent Sessions"));
                f.render_widget(list, chunks[1]);
            }
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

            AppState::Input { prompt, buffer, .. } => {
                let input_display = format!("{}: {}", prompt, buffer);
                let input_widget = Paragraph::new(input_display)
                    .block(Block::default().borders(Borders::ALL).title("Input"));
                f.render_widget(input_widget, chunks[1]);
            }
            AppState::Idle => {
                let usage_items: Vec<ListItem> = self
                    .usage
                    .iter()
                    .take(10)
                    .map(|(app, duration)| {
                        let minutes = duration / 60;
                        let display = format!("{}: {}m", app, minutes);
                        ListItem::new(Line::from(display))
                    })
                    .collect();
                let usage_list = List::new(usage_items)
                    .block(Block::default().borders(Borders::ALL).title("App Usage"));
                f.render_widget(usage_list, chunks[1]);
            }
        }
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
        self.state = AppState::Idle; // Keep idle to show usage
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
                self.history = self.database.get_recent_sessions(10).await.unwrap();
                self.usage = self.database.get_app_usage().await.unwrap();
                self.logs.push(format!("Ended session: {} for {}s", session.app_name, session.duration));
            } else {
                self.logs.push(format!("Skipped ending short session: {} for {}s", session.app_name, session.duration));
            }
        }
        Ok(())
    }

    async fn view_history(&mut self) {
        self.history = self.database.get_recent_sessions(10).await.unwrap();
        // Sort by duration descending (most used first)
        self.history.sort_by(|a, b| b.duration.cmp(&a.duration));
        self.state = AppState::ViewingHistory;
    }



    fn start_set_app_name(&mut self) {
        self.state = AppState::Input {
            prompt: "Enter app name to track".to_string(),
            buffer: String::new(),
            action: InputAction::SetAppName,
        };
    }

    fn start_rename(&mut self) {
        if self.current_session.is_some() {
            self.state = AppState::Input {
                prompt: "Enter new session name".to_string(),
                buffer: String::new(),
                action: InputAction::Rename,
            };
        }
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

    fn handle_input(&mut self) {
        let (buffer, action) = if let AppState::Input { buffer, action, .. } = &self.state {
            (buffer.clone(), action.clone())
        } else {
            return;
        };

        match action {
            InputAction::Rename => {
                if let Some(session) = &mut self.current_session {
                    session.app_name = buffer;
                }
                self.state = AppState::Idle;
            }
            InputAction::SetAppName => {
                self.manual_app_name = Some(buffer.clone());
                self.current_app = buffer;
                self.state = AppState::Idle;
            }
        }
    }
}