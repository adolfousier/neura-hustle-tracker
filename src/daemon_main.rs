mod config;
mod database;
mod models;
mod tracker;
mod active_window;

use anyhow::Result;
use crate::active_window::daemon::Daemon;
use crate::config::settings::Settings;
use crate::database::connection::Database;
use dotenvy::dotenv;
use std::env;
use std::fs::OpenOptions;

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
        // Enable debug logging to app.log file
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("daemon.log")
            .expect("Failed to open log file");

        env_logger::Builder::from_env(
            env_logger::Env::default()
                .default_filter_or("neura_hustle_tracker=debug")
        )
        .target(env_logger::Target::Pipe(Box::new(log_file)))
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
