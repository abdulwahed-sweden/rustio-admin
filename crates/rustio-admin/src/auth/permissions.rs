//! Granular permissions with groups.
//!
//! Data model:
//!   rustio_permissions         (id, name, description)
//!   rustio_groups              (id, name, description)
//!   rustio_group_permissions   (group_id, permission_id)
//!   rustio_user_groups         (user_id, group_id)
//!   rustio_user_permissions    (user_id, permission_id)    -- direct grants
//!
//! Permission naming convention: `<app>.<action>_<model>`, e.g.
//! `posts.add_post`, `posts.change_post`, `posts.delete_post`,
//! `posts.view_post`.
//!
//! An Administrator-or-higher role automatically has every permission
//! (see `Role::bypasses_group_checks`). Lower tiers are checked against
//! the tables above.
//!
//! Permissions for a user are cached in a `DashMap<user_id, …>` with a
//! 60-second TTL so hot paths don't hit the DB. A write to the
//! permission tables calls `invalidate_user_cache(user_id)`.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use once_cell::sync::Lazy;
use sqlx::Row as SqlxRow;

use crate::error::{Error, Result};
use crate::orm::Db;

use super::users::Identity;

#[cfg(test)]
use super::role::Role;

// public:
/// Marker type used by the admin's authorize macro for fast-paths on admins.
pub struct Superuser;

// public:
#[derive(Debug, Clone)]
pub struct Permission {
    pub id: i64,
    pub name: String,
    pub description: String,
}

// public:
#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    #[error("permission `{0}` not found")]
    Missing(String),
    #[error("user not found")]
    NoSuchUser,
    #[error("group not found")]
    NoSuchGroup,
}

// --- schema ---------------------------------------------------------------

// public:
pub async fn init_permission_tables(db: &Db) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_permissions (
            id          BIGSERIAL PRIMARY KEY,
            name        TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_groups (
            id          BIGSERIAL PRIMARY KEY,
            name        TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_group_permissions (
            group_id      BIGINT NOT NULL REFERENCES rustio_groups(id)      ON DELETE CASCADE,
            permission_id BIGINT NOT NULL REFERENCES rustio_permissions(id) ON DELETE CASCADE,
            PRIMARY KEY (group_id, permission_id)
        )",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_user_groups (
            user_id  BIGINT NOT NULL REFERENCES rustio_users(id)  ON DELETE CASCADE,
            group_id BIGINT NOT NULL REFERENCES rustio_groups(id) ON DELETE CASCADE,
            PRIMARY KEY (user_id, group_id)
        )",
    )
    .execute(db.pool())
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_user_permissions (
            user_id       BIGINT NOT NULL REFERENCES rustio_users(id)       ON DELETE CASCADE,
            permission_id BIGINT NOT NULL REFERENCES rustio_permissions(id) ON DELETE CASCADE,
            PRIMARY KEY (user_id, permission_id)
        )",
    )
    .execute(db.pool())
    .await?;

    Ok(())
}

// --- cache ----------------------------------------------------------------

struct CacheEntry {
    perms: Arc<HashSet<String>>,
    expires: Instant,
}

static PERM_CACHE: Lazy<DashMap<i64, CacheEntry>> = Lazy::new(DashMap::new);

const PERM_CACHE_TTL: Duration = Duration::from_secs(60);

pub(crate) fn invalidate_user_cache(user_id: i64) {
    PERM_CACHE.remove(&user_id);
}

fn invalidate_group_cache(db: &Db, group_id: i64) {
    // Users in this group need their cached permission sets evicted.
    // Fire-and-forget — the TTL will catch anything we miss.
    let db = db.clone();
    tokio::spawn(async move {
        let rows = sqlx::query("SELECT user_id FROM rustio_user_groups WHERE group_id = $1")
            .bind(group_id)
            .fetch_all(db.pool())
            .await
            .unwrap_or_default();
        for r in rows {
            if let Ok(uid) = r.try_get::<i64, _>("user_id") {
                invalidate_user_cache(uid);
            }
        }
    });
}

// --- reads ----------------------------------------------------------------

// public:
/// All permission names belonging to the given user — direct + via
/// groups — unioned into one set. Cached for 60s.
pub async fn permissions_for_user(db: &Db, user_id: i64) -> Result<Arc<HashSet<String>>> {
    if let Some(e) = PERM_CACHE.get(&user_id) {
        if e.expires > Instant::now() {
            return Ok(e.perms.clone());
        }
    }

    let rows = sqlx::query(
        "SELECT DISTINCT p.name
           FROM rustio_permissions p
           LEFT JOIN rustio_user_permissions up ON up.permission_id = p.id
           LEFT JOIN rustio_group_permissions gp ON gp.permission_id = p.id
           LEFT JOIN rustio_user_groups ug ON ug.group_id = gp.group_id
          WHERE up.user_id = $1 OR ug.user_id = $1",
    )
    .bind(user_id)
    .fetch_all(db.pool())
    .await?;

    let mut set = HashSet::with_capacity(rows.len());
    for r in rows {
        if let Ok(name) = r.try_get::<String, _>("name") {
            set.insert(name);
        }
    }
    let arc = Arc::new(set);
    PERM_CACHE.insert(
        user_id,
        CacheEntry {
            perms: arc.clone(),
            expires: Instant::now() + PERM_CACHE_TTL,
        },
    );
    Ok(arc)
}

// public:
/// Ask "does this identity have permission X?".
///
/// Order of checks (load-bearing):
/// 1. **`is_active`** — an inactive user is denied even if their role
///    would bypass group checks.
/// 2. **`bypasses_group_checks`** — Administrator and Developer skip
///    the M2M lookup; every other tier consults the tables.
pub async fn check_permission(db: &Db, identity: &Identity, permission: &str) -> Result<bool> {
    if !identity.is_active {
        return Ok(false);
    }
    if identity.role.bypasses_group_checks() {
        return Ok(true);
    }
    let perms = permissions_for_user(db, identity.user_id).await?;
    Ok(perms.contains(permission))
}

// --- writes ---------------------------------------------------------------

async fn permission_id(db: &Db, name: &str) -> Result<i64> {
    if let Some(row) = sqlx::query("SELECT id FROM rustio_permissions WHERE name = $1")
        .bind(name)
        .fetch_optional(db.pool())
        .await?
    {
        return row
            .try_get("id")
            .map_err(|e| Error::Internal(format!("{e}")));
    }
    let row = sqlx::query(
        "INSERT INTO rustio_permissions (name, description)
         VALUES ($1, $2)
         ON CONFLICT (name) DO UPDATE SET description = rustio_permissions.description
         RETURNING id",
    )
    .bind(name)
    .bind("")
    .fetch_one(db.pool())
    .await?;
    row.try_get("id")
        .map_err(|e| Error::Internal(format!("{e}")))
}

// public:
pub async fn grant_to_user(db: &Db, user_id: i64, permission: &str) -> Result<()> {
    let pid = permission_id(db, permission).await?;
    sqlx::query(
        "INSERT INTO rustio_user_permissions (user_id, permission_id)
         VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(pid)
    .execute(db.pool())
    .await?;
    invalidate_user_cache(user_id);
    Ok(())
}

// public:
pub async fn grant_to_group(db: &Db, group_id: i64, permission: &str) -> Result<()> {
    let pid = permission_id(db, permission).await?;
    sqlx::query(
        "INSERT INTO rustio_group_permissions (group_id, permission_id)
         VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
    )
    .bind(group_id)
    .bind(pid)
    .execute(db.pool())
    .await?;
    invalidate_group_cache(db, group_id);
    Ok(())
}

// public:
/// Idempotent. A second call with the same `name` returns the
/// existing group's id; the stored `description` is preserved
/// (first-write-wins). Mirrors the `permission_id` upsert idiom
/// in this module.
pub async fn create_group(db: &Db, name: &str, description: &str) -> Result<i64> {
    let row = sqlx::query(
        "INSERT INTO rustio_groups (name, description)
         VALUES ($1, $2)
         ON CONFLICT (name) DO UPDATE SET description = rustio_groups.description
         RETURNING id",
    )
    .bind(name)
    .bind(description)
    .fetch_one(db.pool())
    .await?;
    row.try_get("id")
        .map_err(|e| Error::Internal(format!("{e}")))
}

// public:
pub async fn add_user_to_group(db: &Db, user_id: i64, group_id: i64) -> Result<()> {
    sqlx::query(
        "INSERT INTO rustio_user_groups (user_id, group_id)
         VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(group_id)
    .execute(db.pool())
    .await?;
    invalidate_user_cache(user_id);
    Ok(())
}

// public:
pub async fn remove_user_from_group(db: &Db, user_id: i64, group_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM rustio_user_groups WHERE user_id = $1 AND group_id = $2")
        .bind(user_id)
        .bind(group_id)
        .execute(db.pool())
        .await?;
    invalidate_user_cache(user_id);
    Ok(())
}

// public:
/// For an admin model named `posts`, register the canonical four
/// permissions: `add_post`, `change_post`, `delete_post`, `view_post`.
/// Idempotent.
pub async fn register_model_permissions(db: &Db, app: &str, singular: &str) -> Result<()> {
    let actions = ["add", "change", "delete", "view"];
    for action in actions {
        let name = format!("{app}.{action}_{singular}");
        let _ = permission_id(db, &name).await?;
    }
    Ok(())
}

// public:
/// The three structural permission groups every fresh database is
/// seeded with (PR 2.2 / `DESIGN_PERMISSIONS.md`).
///
/// - `administrator` — full system access.
/// - `editor` — create / read / update on content models only.
/// - `viewer` — read-only on content models.
///
/// **These are structural defaults, not demo data.** Group names
/// MUST exactly match the `--role` values accepted by `rustio-admin user
/// create` so a developer's role choice and the group their account
/// gets dropped into are the same string. The CLI's lockstep test
/// (`crates/rustio-admin-cli/src/user.rs`) fails CI if either side
/// drifts.
pub const DEFAULT_GROUP_NAMES: [&str; 3] = ["administrator", "editor", "viewer"];

// public:
/// Seed the three structural permission groups on a fresh database.
///
/// Idempotent: calls `create_group` (ON CONFLICT (name) DO UPDATE
/// description) for each name. Safe to invoke on every boot.
///
/// **Guard (PR 2.2 doctrine):** the seed is SKIPPED when the
/// `rustio_groups` table already contains any group name NOT in
/// [`DEFAULT_GROUP_NAMES`]. An existing project that has built its
/// own group structure on 0.20.x is never silently re-shaped by
/// upgrading to 0.21.0; only databases that are either fresh or
/// already match the default set get the seed applied.
pub async fn seed_default_groups(db: &Db) -> Result<()> {
    let foreign_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rustio_groups
         WHERE name NOT IN ('administrator', 'editor', 'viewer')",
    )
    .fetch_one(db.pool())
    .await
    .map_err(|e| Error::Internal(format!("seed_default_groups guard: {e}")))?;
    if foreign_count > 0 {
        // Project has user-defined groups; respect that and skip.
        return Ok(());
    }
    create_group(db, "administrator", "Full system access.").await?;
    create_group(
        db,
        "editor",
        "Create / read / update on content models only. No user, group, settings, or framework-admin actions.",
    )
    .await?;
    create_group(db, "viewer", "Read-only access to content models.").await?;
    Ok(())
}

// public:
/// Per-model permission grants for the seeded default groups
/// (PR 2.2 / `DESIGN_PERMISSIONS.md`). Called by
/// [`crate::admin::Admin::seed_permissions`] after the four CRUD
/// permissions are registered for `<app>.<singular>`. Each grant
/// is idempotent (`grant_to_group` uses ON CONFLICT DO NOTHING);
/// missing groups (because [`seed_default_groups`] was skipped by
/// the user-defined-groups guard) cause silent no-ops, not errors.
///
/// Grant matrix:
///
/// |              | `add` | `change` | `delete` | `view` |
/// |--------------|-------|----------|----------|--------|
/// | administrator | ✓     | ✓        | ✓        | ✓      |
/// | editor        | ✓     | ✓        |          | ✓      |
/// | viewer        |       |          |          | ✓      |
///
/// `editor` deliberately lacks `delete` — destructive operations
/// belong to administrators by default. Projects that want
/// editor-level delete access either grant `<app>.delete_<model>`
/// to the `editor` group explicitly via the admin permission-matrix
/// UI, or move those users to `administrator`.
pub async fn grant_model_to_default_groups(db: &Db, app: &str, singular: &str) -> Result<()> {
    // Look up the three group IDs. Missing => skip (user-defined
    // groups guard fired; the default set isn't installed on this
    // database, so per-model grants would have nowhere to land).
    let admin_id = group_id_by_name(db, "administrator").await?;
    let editor_id = group_id_by_name(db, "editor").await?;
    let viewer_id = group_id_by_name(db, "viewer").await?;

    let add = format!("{app}.add_{singular}");
    let change = format!("{app}.change_{singular}");
    let delete = format!("{app}.delete_{singular}");
    let view = format!("{app}.view_{singular}");

    if let Some(id) = admin_id {
        grant_to_group(db, id, &add).await?;
        grant_to_group(db, id, &change).await?;
        grant_to_group(db, id, &delete).await?;
        grant_to_group(db, id, &view).await?;
    }
    if let Some(id) = editor_id {
        grant_to_group(db, id, &add).await?;
        grant_to_group(db, id, &change).await?;
        grant_to_group(db, id, &view).await?;
        // No delete — see grant matrix above.
    }
    if let Some(id) = viewer_id {
        grant_to_group(db, id, &view).await?;
    }
    Ok(())
}

/// Look up a group ID by name. Returns Ok(None) when the group
/// doesn't exist (intentional: callers want graceful no-op on
/// missing-default-groups, not error propagation).
async fn group_id_by_name(db: &Db, name: &str) -> Result<Option<i64>> {
    let id: Option<i64> = sqlx::query_scalar("SELECT id FROM rustio_groups WHERE name = $1")
        .bind(name)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| Error::Internal(format!("group_id_by_name({name}): {e}")))?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn administrator_and_developer_bypass_group_checks() {
        // The two top tiers skip the M2M lookup. Lower tiers don't.
        for &(role, expected) in &[
            (Role::User, false),
            (Role::Staff, false),
            (Role::Supervisor, false),
            (Role::Administrator, true),
            (Role::Developer, true),
        ] {
            let id = Identity {
                user_id: 1,
                email: "a@b.com".into(),
                role,
                is_active: true,
                is_demo: false,
                demo_label: None,
                must_change_password: false,
                mfa_enabled: false,
                trust_level: crate::auth::SessionTrust::Authenticated,
            };
            assert_eq!(
                id.role.bypasses_group_checks(),
                expected,
                "{role:?} should be {expected}"
            );
        }
    }

    #[test]
    fn cache_ttl_is_one_minute() {
        assert_eq!(PERM_CACHE_TTL.as_secs(), 60);
    }
}
