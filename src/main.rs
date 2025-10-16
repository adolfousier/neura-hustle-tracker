mod config;
mod database;
mod models;
mod tracker;
mod ui;

use anyhow::Result;
use crate::config::settings::Settings;
use crate::database::connection::Database;
use crate::ui::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Configure logging to reduce zbus verbosity
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("neura_hustle_tracker=debug,warn")  // Only show our app at debug, others at warn
    ).init();
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