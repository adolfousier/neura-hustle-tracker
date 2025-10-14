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
    env_logger::init();
    log::info!("Starting time tracker app");
    let settings = Settings::new()?;
    log::info!("Connecting to database...");
    let database = Database::new(&settings.database_url).await?;
    log::info!("Connected successfully. Creating tables...");
    database.create_table().await?;
    log::info!("Tables created. Starting application...");

    let mut app = App::new(database).await?;
    app.run().await?;

    Ok(())
}