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
        // Enable debug logging to daemon.log file
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

        log::info!("=== DEBUG LOGGING ENABLED ===");
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
    log::info!("Database URL: {}", settings.database_url);
    log::info!("Environment variables loaded: POSTGRES_USERNAME={}, POSTGRES_PASSWORD=***", 
               env::var("POSTGRES_USERNAME").unwrap_or_else(|_| "NOT_SET".to_string()));
    let database = match Database::new(&settings.database_url).await {
        Ok(db) => {
            log::info!("Database connection successful");
            db
        }
        Err(e) => {
            if debug_enabled {
                log::error!("Database connection failed: {}", e);
                log::error!("Full error details: {:?}", e);
            }
            eprintln!("❌ Daemon failed to connect to database. Please check:");
            eprintln!("  - Database is running (make daemon-status)");
            eprintln!("  - .env file has correct DATABASE_URL");
            eprintln!("  - Port number in DATABASE_URL is valid");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    log::info!("Connected successfully. Creating tables...");

    log::info!("Tables created. Starting daemon...");

    let mut daemon = Daemon::new(database);
    daemon.run().await?;

    Ok(())
}
