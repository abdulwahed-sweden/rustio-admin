//! Configuration, loaded from the environment (and `.env` if present).

use rustio_admin::{Error, Result};

/// Everything the server needs to start. Add fields here as the
/// project grows (SMTP, secret key, feature toggles, …).
pub struct Config {
    /// Postgres connection string, e.g. `postgres://user:pass@host/db`.
    pub database_url: String,
}

/// Read configuration from the environment. `DATABASE_URL` is required.
///
/// `.env` is loaded first as a convenience for local development;
/// real environment variables always win.
pub fn load() -> Result<Config> {
    let _ = dotenvy::dotenv();
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        Error::Internal(
            "DATABASE_URL is not set. Copy .env.example to .env and edit it before running.".into(),
        )
    })?;
    Ok(Config { database_url })
}
