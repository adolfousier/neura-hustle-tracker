mod config;
mod database;
mod models;
mod tracker;
mod ui;

use anyhow::Result;
use crate::config::settings::Settings;
use crate::database::connection::Database;
use crate::ui::app::App;
use dotenvy::dotenv;
use std::env;
use std::fs::OpenOptions;
use clap::{Arg, Command};

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("Neura Hustle Tracker")
        .version("0.3.2")
        .author("Your Name")
        .about("Track your application usage")
        .arg(
            Arg::new("test-idle")
                .long("test-idle")
                .help("Test D-Bus idle detection instead of running the full UI")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

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
            .open("app.log")
            .expect("Failed to open log file");

        env_logger::Builder::from_env(
            env_logger::Env::default()
                .default_filter_or("neura_hustle_tracker=debug")
        )
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();

        log::info!("=== DEBUG LOGGING ENABLED ===");
        log::info!("Writing logs to app.log");
        log::info!("To disable: Remove DEBUG_LOGS_ENABLED from .env or set to false");
    } else {
        // No logging for regular users
        env_logger::Builder::from_env(
            env_logger::Env::default()
                .default_filter_or("off")
        )
        .init();
    }

    // Check if we're running idle test mode
    if matches.get_flag("test-idle") {
        println!("Testing Wayland D-Bus idle detection...");
        test_idle_detection().await?;
        return Ok(());
    }

    log::info!("Starting Neura Hustle Tracker");
    let settings = Settings::new().unwrap();
    log::info!("Connecting to database...");
    let database = Database::new(&settings.database_url).await.unwrap();
    log::info!("Connected successfully. Creating tables...");

    log::info!("Tables created. Starting application...");

    let mut app = App::new(database);
    app.run().await?;

    Ok(())
}

async fn test_idle_detection() -> Result<()> {
    println!("Testing Wayland D-Bus idle detection...");

    // Import the idle detection function from the app module
    use crate::ui::app::App;

    match App::check_wayland_idle_time().await {
        Ok(idle_time) => {
            println!("✅ Success! Idle time: {} seconds", idle_time);
        }
        Err(e) => {
            println!("❌ Failed: {}", e);
        }
    }

    Ok(())
}
