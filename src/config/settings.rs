use anyhow::Result;
use rand::Rng;
use std::env;
use std::fs;

#[derive(Debug)]
pub struct Settings {
    pub database_url: String,
}

impl Settings {
    fn get_env_path() -> std::path::PathBuf {
        std::env::current_dir().unwrap().join(".env")
    }

    pub fn new() -> Result<Self> {
        // Try to load existing .env from project root
        let env_path = Self::get_env_path();
        dotenvy::from_path(&env_path).ok();

        // Check if we need to generate credentials
        let needs_generation = Self::needs_credential_generation();

        if needs_generation {
            log::info!("Database credentials not found or incomplete. Generating new credentials...");
            Self::generate_and_save_credentials()?;
            // Reload .env after generation
            dotenvy::from_path(&Self::get_env_path())?;
        }

        // Now try to get DATABASE_URL (should exist after generation if it was needed)
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable not set after credential generation"))?;

        Ok(Self { database_url })
    }

    fn needs_credential_generation() -> bool {
        // Only generate if .env file does not exist
        let env_path = Self::get_env_path();
        !env_path.exists()
    }

    fn generate_and_save_credentials() -> Result<()> {
        let (username, password) = Self::generate_credentials();
        let database_url = format!("postgres://{}:{}@localhost:5432/hustle-tracker", username, password);

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

        let env_path = Self::get_env_path();
        fs::write(env_path, env_content)?;

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
