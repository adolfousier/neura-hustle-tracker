use anyhow::Result;
use std::env;

#[derive(Debug)]
pub struct Settings {
    pub database_url: String,
}

impl Settings {
    pub fn new() -> Result<Self> {
        dotenvy::dotenv().ok();
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable not set"))?;
        Ok(Self { database_url })
    }
}