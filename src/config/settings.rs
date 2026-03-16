use anyhow::Result;
use rand::Rng;
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::config::docker::DockerManager;

/// Runtime settings resolved from environment variables and `.env` files.
#[derive(Debug)]
pub struct Settings {
    pub database_url: String,
}

impl Settings {
    /// Returns the application data directory.
    /// Linux:   ~/.local/share/hustle-tracker/
    /// macOS:   ~/Library/Application Support/hustle-tracker/
    /// Windows: %APPDATA%\hustle-tracker\
    pub fn data_dir() -> PathBuf {
        let base = dirs::data_dir()
            .unwrap_or_else(|| env::current_dir().unwrap());
        base.join("hustle-tracker")
    }

    /// Returns the directory where .env lives.
    /// Prefers CWD if .env already exists there (backward compat for `just run` workflow).
    /// Otherwise uses the stable data directory.
    fn env_dir() -> PathBuf {
        let cwd = env::current_dir().unwrap();
        if cwd.join(".env").exists() {
            return cwd;
        }
        Self::data_dir()
    }

    /// Ensures the data directory exists and returns it.
    fn ensure_data_dir() -> Result<PathBuf> {
        let dir = Self::env_dir();
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn new() -> Result<Self> {
        let data_dir = Self::ensure_data_dir()?;
        let env_path = data_dir.join(".env");
        dotenvy::from_path(&env_path).ok();

        if !env_path.exists() {
            log::info!("Database credentials not found or incomplete. Generating new credentials...");
            Self::generate_and_save_credentials(&env_path)?;
            dotenvy::from_path(&env_path)?;
        }

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable not set after credential generation"))?;

        Ok(Self { database_url })
    }

    /// Full initialization: ensure .env, ensure Docker DB is up, wait for readiness.
    pub async fn init() -> Result<Self> {
        let settings = Self::new()?;
        let data_dir = Self::env_dir();

        // Try connecting to the database with a short timeout
        let db_reachable = Self::check_db_reachable(&settings.database_url).await;

        if !db_reachable {
            eprintln!("Database not reachable. Attempting to start via Docker...");

            DockerManager::check_docker_available().map_err(|e| {
                anyhow::anyhow!(
                    "{}\n\n\
                     The database is not running and Docker is required to start it.\n\
                     Either install Docker or start PostgreSQL manually on port 52851.",
                    e
                )
            })?;

            DockerManager::start_database(&data_dir)?;

            eprintln!("Waiting for database to be ready...");
            DockerManager::wait_for_database(
                &settings.database_url,
                std::time::Duration::from_secs(30),
            )
            .await?;

            eprintln!("Database is ready.");
        }

        Ok(settings)
    }

    async fn check_db_reachable(url: &str) -> bool {
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            sqlx::postgres::PgPool::connect(url),
        )
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
    }

    fn generate_and_save_credentials(env_path: &std::path::Path) -> Result<()> {
        let (username, password) = Self::generate_credentials();
        let database_url = format!("postgres://{}:{}@localhost:52851/hustle-tracker", username, password);

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

        fs::write(env_path, env_content)?;

        log::info!("Generated new database credentials in .env file");
        log::info!("  Username: {}", username);
        log::info!("  Password: {} (saved in .env)", "*".repeat(password.len()));

        Ok(())
    }

    fn generate_credentials() -> (String, String) {
        let mut rng = rand::thread_rng();

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
