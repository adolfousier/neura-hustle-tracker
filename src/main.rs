mod config;
mod database;
mod models;
mod tracker;
mod ui;

use anyhow::Result;
use crate::config::settings::Settings;
use crate::database::connection::Database;
use crate::ui::app::App;
use std::fs::OpenOptions;

#[tokio::main]
async fn main() -> Result<()> {
    // Configure logging to write to app.log file (prevents TUI corruption)
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("app.log")
        .expect("Failed to open log file");

    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("neura_hustle_tracker=info,warn")  // Only show our app at info, others at warn
    )
    .target(env_logger::Target::Pipe(Box::new(log_file)))
    .init();

    log::info!("Starting time tracker app");
    let settings = Settings::new().unwrap();
    log::info!("Connecting to database...");
    let database = Database::new(&settings.database_url).await.unwrap();
    log::info!("Connected successfully. Creating tables...");
    database.create_table().await.unwrap();
    log::info!("Tables created. Starting application...");

    let mut app = App::new(database);
    app.run().await?;

    Ok(())
}