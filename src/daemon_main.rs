mod config;
mod daemon;
mod models;

use anyhow::Result;
use crate::daemon::active_window::daemon::Daemon;
use crate::daemon::database::connection::Database;
use crate::config::settings::Settings;
use dotenvy::dotenv;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};

/// A writer that flushes after every write to ensure logs appear immediately
struct FlushingWriter {
    inner: Arc<Mutex<std::fs::File>>,
}

impl FlushingWriter {
    fn new(file: std::fs::File) -> Self {
        Self {
            inner: Arc::new(Mutex::new(file)),
        }
    }
}

impl Write for FlushingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut file = self.inner.lock().unwrap();
        let result = file.write(buf);
        file.flush()?; // Flush after every write
        result
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut file = self.inner.lock().unwrap();
        file.flush()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    dotenv().ok();

    // Check if debug logging is enabled via .env
    let debug_enabled = env::var("DEBUG_LOGS_ENABLED")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    if debug_enabled {
        // Enable debug logging to daemon.log file with immediate flushing
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("daemon.log")
            .expect("Failed to open daemon.log file");

        let flushing_writer = FlushingWriter::new(log_file);

        env_logger::Builder::from_env(
            env_logger::Env::default()
                .default_filter_or("neura_hustle_tracker=debug")
        )
        .target(env_logger::Target::Pipe(Box::new(flushing_writer)))
        .init();

        log::info!("=== DAEMON DEBUG LOGGING ENABLED ===");
        log::info!("Writing logs to daemon.log");
        log::info!("To disable: Remove DEBUG_LOGS_ENABLED from .env or set to false");
    } else {
        // No logging for regular users
        env_logger::Builder::from_env(
            env_logger::Env::default()
                .default_filter_or("off")
        )
        .init();
    }

    log::info!("Starting Neura Hustle Tracker Daemon");
    let settings = Settings::new()?;
    log::info!("Connecting to database...");
    let database = Database::new(&settings.database_url).await?;
    log::info!("Connected successfully. Creating tables...");
    database.create_table().await?;
    log::info!("Tables created. Starting daemon...");

    let mut daemon = Daemon::new(database);
    daemon.run().await?;

    Ok(())
}
