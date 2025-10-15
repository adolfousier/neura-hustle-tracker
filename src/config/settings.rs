use anyhow::Result;
use rand::Rng;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct Settings {
    pub database_url: String,
}

impl Settings {
    pub fn new() -> Result<Self> {
        // Try to load existing .env
        dotenvy::dotenv().ok();

        // Check if we need to generate credentials
        let needs_generation = Self::needs_credential_generation();

        if needs_generation {
            log::info!("Database credentials not found or incomplete. Generating new credentials...");
            Self::generate_and_save_credentials()?;
            // Reload .env after generation
            dotenvy::dotenv()?;
        }

        // Now try to get DATABASE_URL (should exist after generation if it was needed)
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable not set after credential generation"))?;

        Ok(Self { database_url })
    }

    fn needs_credential_generation() -> bool {
        // Check if .env file exists
        let env_path = Path::new(".env");
        if !env_path.exists() {
            return true;
        }

        // Check if required variables are set
        let database_url = env::var("DATABASE_URL").unwrap_or_default();
        let postgres_username = env::var("POSTGRES_USERNAME").unwrap_or_default();
        let postgres_password = env::var("POSTGRES_PASSWORD").unwrap_or_default();

        database_url.is_empty() || postgres_username.is_empty() || postgres_password.is_empty()
    }

    fn generate_and_save_credentials() -> Result<()> {
        let (username, password) = Self::generate_credentials();
        let database_url = format!("postgres://{}:{}@localhost:5432/time_tracker", username, password);

        let env_content = format!(
            "# Auto-generated database credentials\n\
             # These credentials were automatically created for you.\n\
             # You can modify them if needed, but make sure to update all three values together.\n\
             # If you delete this file, new credentials will be generated on next run.\n\
             \n\
             POSTGRES_USERNAME={}\n\
             POSTGRES_PASSWORD={}\n\
             \n\
             # Full database URL\n\
             DATABASE_URL={}\n",
            username, password, database_url
        );

        fs::write(".env", env_content)?;

        log::info!("✓ Generated new database credentials in .env file");
        log::info!("  Username: {}", username);
        log::info!("  Password: {} (saved in .env)", "*".repeat(password.len()));

        Ok(())
    }

    fn generate_credentials() -> (String, String) {
        let mut rng = rand::thread_rng();

        // Generate username: timetracker_<8 random chars>
        let random_suffix: String = (0..8)
            .map(|_| {
                let idx = rng.gen_range(0..36);
                if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'a' + (idx - 10)) as char
                }
            })
            .collect();
        let username = format!("timetracker_{}", random_suffix);

        // Generate password: 32 random alphanumeric chars
        let password: String = (0..32)
            .map(|_| {
                let idx = rng.gen_range(0..62);
                if idx < 10 {
                    (b'0' + idx) as char
                } else if idx < 36 {
                    (b'a' + (idx - 10)) as char
                } else {
                    (b'A' + (idx - 36)) as char
                }
            })
            .collect();

        (username, password)
    }
}
