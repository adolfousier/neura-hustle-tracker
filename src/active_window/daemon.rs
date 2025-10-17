use anyhow::Result;
use chrono::Local;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

use crate::config::settings::Settings;
use crate::database::connection::Database;
use crate::models::session::Session;
use crate::tracker::monitor::AppMonitor;
use crate::tracker::parser;

pub struct Daemon {
    database: Database,
    monitor: AppMonitor,
    current_app: String,
    current_window: Option<String>,
    current_session: Option<Session>,
}

impl Daemon {
    pub fn new(database: Database) -> Self {
        let monitor = AppMonitor::new();

        Self {
            database,
            monitor,
            current_app: "unknown".to_string(),
            current_window: None,
            current_session: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        log::info!("Starting background daemon for window tracking...");

        // Set up signal handlers for graceful shutdown
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&shutdown_flag))?;
        signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown_flag))?;

        // Start tracking initial app
        self.start_tracking().await?;

        // Auto-save interval: 1 hour
        let mut last_save = tokio::time::Instant::now();
        let save_interval = Duration::from_secs(3600);

        loop {
            // Check for shutdown signal
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

            // Auto save every hour
            if last_save.elapsed() >= save_interval {
                if let Some(session) = &mut self.current_session {
                    session.duration = Local::now().signed_duration_since(session.start_time).num_seconds();
                    if let Err(e) = self.database.insert_session(session).await {
                        log::error!("Failed to auto save session: {}", e);
                    } else {
                        last_save = tokio::time::Instant::now();
                        log::info!("Auto-saved session: {} for {}s", session.app_name, session.duration);
                    }
                }
            }

            // Poll every 100ms for real-time tracking
            time::sleep(Duration::from_millis(100)).await;
        }

        // Save current session on exit
        if let Some(mut session) = self.current_session.take() {
            session.duration = Local::now().signed_duration_since(session.start_time).num_seconds();
            if let Err(e) = self.database.insert_session(&session).await {
                log::error!("Failed to save session on exit: {}", e);
            } else {
                log::info!("Saved session on exit: {} for {}s", session.app_name, session.duration);
            }
        }

        Ok(())
    }

    async fn start_tracking(&mut self) -> Result<()> {
        let app_name = match self.monitor.get_active_app_async().await {
            Ok(detected) => {
                self.current_app = detected.clone();
                detected
            }
            Err(e) => {
                log::error!("Window detection failed: {}", e);
                self.current_app = "Unknown".to_string();
                "Unknown".to_string()
            }
        };

        let window_name = self.monitor.get_active_window_name_async().await.ok();
        let start_time = Local::now();
        let (category_name, _) = Self::categorize_app(&app_name);

        let session = Self::create_session_with_parsing(
            app_name.clone(),
            window_name.clone(),
            start_time,
            category_name.to_string(),
        );

        self.current_session = Some(session);
        self.current_window = window_name;
        log::info!("Started tracking: {}", app_name);
        Ok(())
    }

    async fn switch_app(&mut self, new_app: String) -> Result<()> {
        // End current session
        if let Some(mut session) = self.current_session.take() {
            session.duration = Local::now().signed_duration_since(session.start_time).num_seconds();

            if let Err(e) = self.database.insert_session(&session).await {
                log::error!("Failed to save session: {}", e);
            } else {
                log::info!("Saved session: {} for {}s", session.app_name, session.duration);
            }
        }

        // Start new session
        let window_name = self.monitor.get_active_window_name_async().await.ok();
        let start_time = Local::now();
        let (category_name, _) = Self::categorize_app(&new_app);

        let session = Self::create_session_with_parsing(
            new_app.clone(),
            window_name.clone(),
            start_time,
            category_name.to_string(),
        );

        self.current_session = Some(session);
        self.current_app = new_app.clone();
        self.current_window = window_name;
        log::info!("Switched to: {}", new_app);
        Ok(())
    }

    fn categorize_app(app: &str) -> (&'static str, ()) {
        let app_lower = app.to_lowercase();
        if app_lower.contains("code") || app_lower.contains("vim") || app_lower.contains("nvim") ||
           app_lower.contains("terminal") || app_lower.contains("alacritty") || app_lower.contains("kitty") ||
           app_lower.contains("rust") || app_lower.contains("cargo") || app_lower.contains("editor") ||
           app_lower.contains("vscode") || app_lower.contains("vscodium") || app_lower.contains("gedit") ||
           app_lower.contains("nano") || app_lower.contains("emacs") || app_lower.contains("atom") ||
           app_lower.contains("sublime") {
            ("💻 Development", ())
        } else if app_lower.contains("browser") || app_lower.contains("chrome") || app_lower.contains("firefox") ||
                  app_lower.contains("brave") || app_lower.contains("edge") || app_lower.contains("chromium") {
            ("🌐 Browsing", ())
        } else if app_lower.contains("slack") || app_lower.contains("zoom") || app_lower.contains("teams") ||
                  app_lower.contains("discord") || app_lower.contains("telegram") || app_lower.contains("chat") ||
                  app_lower.contains("signal") || app_lower.contains("element") || app_lower.contains("video-call") ||
                  app_lower.contains("skype") || app_lower.contains("jitsi") {
            ("💬 Communication", ())
        } else if app_lower.contains("spotify") || app_lower.contains("vlc") || app_lower.contains("music") ||
                  app_lower.contains("media") || app_lower.contains("rhythmbox") || app_lower.contains("audacious") ||
                  app_lower.contains("clementine") {
            ("🎵 Media", ())
        } else if app_lower.contains("nautilus") || app_lower.contains("files") || app_lower.contains("dolphin") ||
                  app_lower.contains("file-manager") || app_lower.contains("thunar") || app_lower.contains("nemo") {
            ("📁 Files", ())
        } else if app_lower.contains("thunderbird") || app_lower.contains("evolution") || app_lower.contains("geary") ||
                  app_lower.contains("email") {
            ("📧 Email", ())
        } else if app_lower.contains("libreoffice") || app_lower.contains("soffice") {
            ("📄 Office", ())
        } else {
            ("📦 Other", ())
        }
    }

    fn create_session_with_parsing(
        app_name: String,
        window_name: Option<String>,
        start_time: chrono::DateTime<chrono::Local>,
        category: String,
    ) -> Session {
        let parsed = if let Some(ref win_name) = window_name {
            parser::parse_window_name(&app_name, win_name)
        } else {
            parser::ParsedSessionData::default()
        };

        let parsed_json = serde_json::to_value(&parsed).ok();

        Session {
            id: None,
            app_name,
            window_name,
            start_time,
            duration: 0,
            category: Some(category),
            browser_url: parsed.browser_url,
            browser_page_title: parsed.browser_page_title,
            browser_notification_count: parsed.browser_notification_count,
            terminal_username: parsed.terminal_username,
            terminal_hostname: parsed.terminal_hostname,
            terminal_directory: parsed.terminal_directory,
            terminal_project_name: parsed.terminal_project_name,
            editor_filename: parsed.editor_filename,
            editor_filepath: parsed.editor_filepath,
            editor_project_path: parsed.editor_project_path,
            editor_language: parsed.editor_language,
            tmux_window_name: parsed.tmux_window_name,
            tmux_pane_count: parsed.tmux_pane_count,
            terminal_multiplexer: parsed.terminal_multiplexer,
            ide_project_name: parsed.ide_project_name,
            ide_file_open: parsed.ide_file_open,
            ide_workspace: parsed.ide_workspace,
            parsed_data: parsed_json,
            parsing_success: Some(parsed.parsing_success),
        }
    }
}
