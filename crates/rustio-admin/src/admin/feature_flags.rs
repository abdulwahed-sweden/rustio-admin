//! `rustio_feature_flags` — a tiny project-side feature-flag
//! surface. Projects flip a named flag from the admin UI; project
//! code reads it via [`feature_enabled`] without paying for a
//! Postgres round-trip on every call.
//!
//! Schema:
//!
//! ```sql
//! CREATE TABLE rustio_feature_flags (
//!     key         TEXT        PRIMARY KEY,
//!     enabled     BOOLEAN     NOT NULL DEFAULT FALSE,
//!     description TEXT        NOT NULL DEFAULT '',
//!     created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
//!     updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
//! );
//! ```
//!
//! Reads go through a process-local 60-second cache (mirrors the
//! permissions cache pattern). The cache is per-key — a single
//! flag flip refreshes only that key's entry.
//!
//! The admin UI at `/admin/feature_flags` lets administrators
//! create / toggle / describe flags. Project code calls
//! [`feature_enabled`] from request handlers, background jobs,
//! migrations — anywhere a `Db` is in scope.

use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use sqlx::Row as _;

use crate::error::Result;
use crate::orm::Db;

/// Cache TTL for `feature_enabled` lookups. Matches the
/// permissions cache — short enough that a flag flip propagates
/// across a fleet within a minute, long enough that hot-path
/// reads stay process-local.
const FLAG_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy)]
struct CacheEntry {
    enabled: bool,
    expires: Instant,
}

static FLAG_CACHE: Lazy<DashMap<String, CacheEntry>> = Lazy::new(DashMap::new);

pub(crate) const CREATE_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS rustio_feature_flags (
    key         TEXT        PRIMARY KEY,
    enabled     BOOLEAN     NOT NULL DEFAULT FALSE,
    description TEXT        NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
)";

// public:
/// Ensure the `rustio_feature_flags` table exists. Idempotent.
pub async fn ensure_table(db: &Db) -> Result<()> {
    sqlx::query(CREATE_TABLE_SQL).execute(db.pool()).await?;
    Ok(())
}

// public:
/// One feature flag's stored state. Surfaced by [`list_flags`]
/// for the admin UI; project code reads booleans via
/// [`feature_enabled`].
#[derive(Debug, Clone)]
pub struct FeatureFlag {
    pub key: String,
    pub enabled: bool,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// public:
/// `true` when the named flag is set and `enabled = TRUE`.
/// Missing keys read as `false`. Reads go through a 60-second
/// per-key cache so hot paths stay cheap.
///
/// **Stability:** the key is the public contract between project
/// code and the operator. Treat key strings as snake_case
/// identifiers (e.g. `"new_signup_flow"`) — the table accepts
/// any TEXT, but a stable convention makes the admin UI legible.
pub async fn feature_enabled(db: &Db, key: &str) -> bool {
    if let Some(entry) = FLAG_CACHE.get(key) {
        if entry.expires > Instant::now() {
            return entry.enabled;
        }
    }
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM rustio_feature_flags WHERE key = $1")
            .bind(key)
            .fetch_optional(db.pool())
            .await
            .ok()
            .flatten();
    let enabled = enabled.unwrap_or(false);
    FLAG_CACHE.insert(
        key.to_string(),
        CacheEntry {
            enabled,
            expires: Instant::now() + FLAG_CACHE_TTL,
        },
    );
    enabled
}

// public:
/// Drop every cached entry. Called by [`set_flag`] /
/// [`create_flag`] so a fresh write is observable on the next
/// read without waiting for the TTL.
pub fn invalidate_cache() {
    FLAG_CACHE.clear();
}

/// List every flag, newest-created first. Powers the admin UI.
pub(crate) async fn list_flags(db: &Db) -> Result<Vec<FeatureFlag>> {
    ensure_table(db).await?;
    let rows = sqlx::query(
        "SELECT key, enabled, description, created_at, updated_at \
         FROM rustio_feature_flags ORDER BY created_at DESC",
    )
    .fetch_all(db.pool())
    .await?;
    let out = rows
        .iter()
        .map(|r| FeatureFlag {
            key: r.try_get("key").unwrap_or_default(),
            enabled: r.try_get("enabled").unwrap_or(false),
            description: r.try_get("description").unwrap_or_default(),
            created_at: r.try_get("created_at").unwrap_or_else(|_| Utc::now()),
            updated_at: r.try_get("updated_at").unwrap_or_else(|_| Utc::now()),
        })
        .collect();
    Ok(out)
}

/// Create a new flag with the given description. Initial state
/// is `enabled = FALSE`. Idempotent on the key (no-op when the
/// key already exists).
pub(crate) async fn create_flag(db: &Db, key: &str, description: &str) -> Result<()> {
    ensure_table(db).await?;
    sqlx::query(
        "INSERT INTO rustio_feature_flags (key, enabled, description) \
         VALUES ($1, FALSE, $2) ON CONFLICT (key) DO NOTHING",
    )
    .bind(key)
    .bind(description)
    .execute(db.pool())
    .await?;
    invalidate_cache();
    Ok(())
}

/// Flip one flag's `enabled` state to the supplied value. Bumps
/// `updated_at`. Missing keys silently no-op (the admin UI only
/// surfaces existing rows).
pub(crate) async fn set_flag(db: &Db, key: &str, enabled: bool) -> Result<()> {
    ensure_table(db).await?;
    sqlx::query(
        "UPDATE rustio_feature_flags \
         SET enabled = $1, updated_at = NOW() \
         WHERE key = $2",
    )
    .bind(enabled)
    .bind(key)
    .execute(db.pool())
    .await?;
    invalidate_cache();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_cache_ttl_is_60_seconds() {
        // Locked-in constant — the admin UI's "may take up to a
        // minute to propagate" copy depends on it. Bumping the
        // TTL is fine, but update the help text too.
        assert_eq!(FLAG_CACHE_TTL, Duration::from_secs(60));
    }

    #[test]
    fn invalidate_cache_clears_every_entry() {
        FLAG_CACHE.insert(
            "key_a".into(),
            CacheEntry {
                enabled: true,
                expires: Instant::now() + Duration::from_secs(60),
            },
        );
        FLAG_CACHE.insert(
            "key_b".into(),
            CacheEntry {
                enabled: false,
                expires: Instant::now() + Duration::from_secs(60),
            },
        );
        invalidate_cache();
        assert!(FLAG_CACHE.get("key_a").is_none());
        assert!(FLAG_CACHE.get("key_b").is_none());
    }
}
