//! The database connection — one pool, shared by the whole server.

use crate::config::Config;
use rustio_admin::{Db, Result};

/// Open a Postgres connection pool from the loaded [`Config`].
pub async fn pool(cfg: &Config) -> Result<Db> {
    Db::connect(&cfg.database_url).await
}
