//! Tiny key/value metadata table (`rustio_admin_meta`) used to fast-path
//! idempotent startup work.
//!
//! Several boot steps — auth-schema creation/migration ([`crate::auth::init_tables`])
//! and per-model RBAC seeding ([`crate::admin::Admin::seed_permissions`]) — are
//! idempotent, but re-issuing their DDL / `INSERT`s on every boot costs
//! tens of milliseconds of Postgres round-trips. Each step records a
//! sentinel here (typically the running crate version, optionally mixed
//! with a model fingerprint) and skips itself when the stored sentinel
//! already matches. A fresh database (no row), a framework upgrade (crate
//! version changed), or a changed model set (fingerprint changed) all
//! produce a mismatch, so the work still self-heals exactly when it must.
//!
//! To force a full re-run, `TRUNCATE rustio_admin_meta` (or drop the
//! relevant key); the next boot will rebuild and re-stamp.

use crate::error::Result;
use crate::orm::Db;

/// Create the metadata table if it does not exist. Idempotent; one cheap
/// `CREATE TABLE IF NOT EXISTS` round-trip.
pub(crate) async fn ensure_table(db: &Db) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_admin_meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
    )
    .execute(db.pool())
    .await?;
    Ok(())
}

/// Read a metadata value, or `None` when the key is absent.
pub(crate) async fn get(db: &Db, key: &str) -> Result<Option<String>> {
    let value = sqlx::query_scalar::<_, String>("SELECT value FROM rustio_admin_meta WHERE key = $1")
        .bind(key)
        .fetch_optional(db.pool())
        .await?;
    Ok(value)
}

/// Upsert a metadata value.
pub(crate) async fn set(db: &Db, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO rustio_admin_meta (key, value) VALUES ($1, $2)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(key)
    .bind(value)
    .execute(db.pool())
    .await?;
    Ok(())
}
