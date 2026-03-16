use anyhow::{Result, bail};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// The compose.yml content, embedded at compile time.
const COMPOSE_YML: &str = include_str!("../../compose.yml");

pub struct DockerManager;

impl DockerManager {
    /// Checks if `docker` CLI is available.
    pub fn check_docker_available() -> Result<()> {
        let output = Command::new("docker").arg("version").output();
        match output {
            Ok(o) if o.status.success() => Ok(()),
            _ => bail!(
                "Docker is not installed or not running.\n\
                 Install Docker: https://docs.docker.com/get-docker/"
            ),
        }
    }

    /// Determines the correct compose command.
    /// Tries `docker compose` (v2 plugin) first, falls back to standalone `docker-compose`.
    fn get_compose_command() -> Result<(String, Vec<String>)> {
        // Try docker compose (v2 plugin)
        let output = Command::new("docker")
            .args(["compose", "version"])
            .output();
        if let Ok(o) = output
            && o.status.success() {
                return Ok(("docker".to_string(), vec!["compose".to_string()]));
            }
        // Try standalone docker-compose
        let output = Command::new("docker-compose").arg("version").output();
        if let Ok(o) = output
            && o.status.success() {
                return Ok(("docker-compose".to_string(), vec![]));
            }
        bail!(
            "'docker compose' plugin is not available.\n\
             Install it: sudo apt-get install docker-compose-plugin\n\
             See: https://docs.docker.com/compose/install/"
        )
    }

    /// Ensures compose.yml exists in the given directory.
    /// Writes the embedded version if missing.
    pub fn ensure_compose_file(dir: &Path) -> Result<std::path::PathBuf> {
        let compose_path = dir.join("compose.yml");
        if !compose_path.exists() {
            std::fs::write(&compose_path, COMPOSE_YML)?;
            log::info!("Created compose.yml at {}", compose_path.display());
        }
        Ok(compose_path)
    }

    /// Starts PostgreSQL via docker compose.
    /// The .env file must already exist in the given directory.
    pub fn start_database(data_dir: &Path) -> Result<()> {
        let (cmd, base_args) = Self::get_compose_command()?;

        let compose_path = Self::ensure_compose_file(data_dir)?;
        let env_path = data_dir.join(".env");

        let mut args = base_args;
        args.extend([
            "--file".to_string(),
            compose_path.to_string_lossy().to_string(),
            "--env-file".to_string(),
            env_path.to_string_lossy().to_string(),
            "--project-name".to_string(),
            "hustle-tracker".to_string(),
            "up".to_string(),
            "-d".to_string(),
        ]);

        let output = Command::new(&cmd).args(&args).output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to start database:\n{}", stderr);
        }

        Ok(())
    }

    /// Waits for PostgreSQL to accept connections, polling with retries.
    pub async fn wait_for_database(database_url: &str, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_secs(1);

        loop {
            if let Ok(pool) = sqlx::postgres::PgPool::connect(database_url).await {
                if sqlx::query("SELECT 1").execute(&pool).await.is_ok() {
                    pool.close().await;
                    return Ok(());
                }
                pool.close().await;
            }

            if start.elapsed() >= timeout {
                bail!(
                    "Database did not become ready within {} seconds.\n\
                     Check that Docker is running and port 52851 is available.",
                    timeout.as_secs()
                );
            }

            tokio::time::sleep(poll_interval).await;
        }
    }
}
