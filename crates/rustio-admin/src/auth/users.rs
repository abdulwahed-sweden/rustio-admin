//! User records, password hashing, and the login flow.

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use chrono::{DateTime, Utc};
use sqlx::Row as SqlxRow;

use crate::error::{Error, Result};
use crate::orm::{Db, Row};

use super::role::Role;
use super::sessions::create_session;

/// The identity attached to a request by the auth middleware. Kept
/// cheap to clone because we pass it into handler bodies.
#[derive(Debug, Clone)]
pub struct Identity {
    pub user_id: i64,
    pub email: String,
    pub role: Role,
    pub is_active: bool,
    /// Whether this user was seeded by a demo-fixture flow. Drives the
    /// red banner in the admin UI; remains FALSE for users created via
    /// the normal `create_user` path.
    pub is_demo: bool,
    pub demo_label: Option<String>,
}

impl Identity {
    /// Administrator-or-higher (Administrator, Developer).
    pub fn is_admin(&self) -> bool {
        self.is_active && self.role.includes(Role::Administrator)
    }

    /// Anyone allowed into the admin panel (Staff and above).
    pub fn can_access_admin(&self) -> bool {
        self.is_active && self.role.can_access_panel()
    }
}

pub struct StoredUser {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub role: Role,
    pub is_active: bool,
    pub is_demo: bool,
    pub demo_label: Option<String>,
}

/// Read-only view of a user, used by the built-in admin profile page.
/// Excludes `password_hash` deliberately. Construct via
/// [`load_user_profile`].
#[derive(Debug, Clone)]
pub struct UserProfile {
    pub id: i64,
    pub email: String,
    pub role: Role,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub full_name: Option<String>,
    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub is_demo: bool,
    pub demo_label: Option<String>,
}

pub async fn init_user_tables(db: &Db) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rustio_users (
            id            BIGSERIAL PRIMARY KEY,
            email         TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role          TEXT NOT NULL DEFAULT 'user',
            is_active     BOOLEAN NOT NULL DEFAULT TRUE,
            created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(db.pool())
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS rustio_users_email_idx ON rustio_users (email)")
        .execute(db.pool())
        .await?;

    Ok(())
}

/// Idempotent schema upgrade for the 5-tier role hierarchy + demo + profile
/// columns. Safe to call repeatedly; safe on a fresh DB and on a legacy
/// `'admin'`-roled DB.
///
/// Order is load-bearing:
/// 1. Rename existing `'admin'` rows to `'administrator'` BEFORE the CHECK
///    constraint is added, otherwise the constraint would reject the row.
/// 2. Add the demo columns idempotently.
/// 3. Add the CHECK constraint conditionally (PG has no `IF NOT EXISTS`
///    for CHECK constraints, so we guard via `pg_constraint`).
/// 4. Add the indexes.
/// 5. Add the profile-display columns.
pub async fn migrate_user_schema(db: &Db) -> Result<()> {
    sqlx::query("UPDATE rustio_users SET role = 'administrator' WHERE role = 'admin'")
        .execute(db.pool())
        .await?;

    sqlx::query(
        "ALTER TABLE rustio_users \
         ADD COLUMN IF NOT EXISTS is_demo BOOLEAN NOT NULL DEFAULT FALSE",
    )
    .execute(db.pool())
    .await?;
    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS demo_label TEXT")
        .execute(db.pool())
        .await?;

    sqlx::query(
        "DO $$
         BEGIN
            IF NOT EXISTS (
                SELECT 1 FROM pg_constraint WHERE conname = 'rustio_users_role_check'
            ) THEN
                ALTER TABLE rustio_users
                ADD CONSTRAINT rustio_users_role_check
                CHECK (role IN ('user','staff','supervisor','administrator','developer'));
            END IF;
         END $$",
    )
    .execute(db.pool())
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS rustio_users_role_idx ON rustio_users(role)")
        .execute(db.pool())
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS rustio_users_is_demo_idx \
         ON rustio_users(is_demo) WHERE is_demo = TRUE",
    )
    .execute(db.pool())
    .await?;

    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS full_name TEXT")
        .execute(db.pool())
        .await?;
    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS locale TEXT")
        .execute(db.pool())
        .await?;
    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS timezone TEXT")
        .execute(db.pool())
        .await?;

    Ok(())
}

pub fn hash_password(plain: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| Error::Internal(format!("password hashing: {e}")))
}

pub fn verify_password(plain: &str, stored_hash: &str) -> bool {
    match PasswordHash::new(stored_hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(plain.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

pub async fn create_user(db: &Db, email: &str, password: &str, role: Role) -> Result<i64> {
    let hash = hash_password(password)?;
    let row = sqlx::query(
        "INSERT INTO rustio_users (email, password_hash, role)
         VALUES ($1, $2, $3)
         RETURNING id",
    )
    .bind(email)
    .bind(&hash)
    .bind(role.as_str())
    .fetch_one(db.pool())
    .await
    .map_err(|e| {
        // Keep Postgres internals out of the client response. The full
        // error stays in the operator's log; the user sees a clean,
        // generic message — except the unique-email collision, which
        // is worth surfacing because it's actionable.
        log::warn!("create_user failed for {email}: {e}");
        let detail = e.to_string();
        if detail.contains("rustio_users_email_key") {
            Error::BadRequest("An account with this email already exists.".into())
        } else {
            Error::BadRequest("Could not create user. Please check your input.".into())
        }
    })?;
    let id: i64 = row
        .try_get("id")
        .map_err(|e| Error::Internal(format!("returning id: {e}")))?;
    Ok(id)
}

pub async fn find_user_by_email(db: &Db, email: &str) -> Result<Option<StoredUser>> {
    let row = sqlx::query(
        "SELECT id, email, password_hash, role, is_active, is_demo, demo_label
           FROM rustio_users
          WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(db.pool())
    .await?;
    match row {
        Some(r) => {
            let r = Row::from_pg(&r);
            Ok(Some(StoredUser {
                id: r.get_i64("id")?,
                email: r.get_string("email")?,
                password_hash: r.get_string("password_hash")?,
                role: Role::parse(&r.get_string("role")?)?,
                is_active: r.get_bool("is_active")?,
                is_demo: r.get_bool("is_demo")?,
                demo_label: r.get_optional_string("demo_label")?,
            }))
        }
        None => Ok(None),
    }
}

/// Load a user by id for display purposes. Returns `Ok(None)` for a
/// missing id (callers map to 404). Returns `Err` only on a real DB
/// failure or a corrupted role string. Never reads `password_hash`.
pub async fn load_user_profile(db: &Db, user_id: i64) -> Result<Option<UserProfile>> {
    let row = sqlx::query(
        "SELECT id, email, role, is_active, created_at,
                full_name, locale, timezone, is_demo, demo_label
           FROM rustio_users
          WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(db.pool())
    .await?;
    match row {
        Some(r) => {
            let r = Row::from_pg(&r);
            Ok(Some(UserProfile {
                id: r.get_i64("id")?,
                email: r.get_string("email")?,
                role: Role::parse(&r.get_string("role")?)?,
                is_active: r.get_bool("is_active")?,
                created_at: r.get_datetime("created_at")?,
                full_name: r.get_optional_string("full_name")?,
                locale: r.get_optional_string("locale")?,
                timezone: r.get_optional_string("timezone")?,
                is_demo: r.get_bool("is_demo")?,
                demo_label: r.get_optional_string("demo_label")?,
            }))
        }
        None => Ok(None),
    }
}

pub async fn set_password(db: &Db, user_id: i64, new_password: &str) -> Result<()> {
    let hash = hash_password(new_password)?;
    sqlx::query("UPDATE rustio_users SET password_hash = $1, updated_at = $2 WHERE id = $3")
        .bind(&hash)
        .bind(Utc::now())
        .bind(user_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn update_user_role(db: &Db, user_id: i64, role: Role) -> Result<()> {
    sqlx::query("UPDATE rustio_users SET role = $1, updated_at = $2 WHERE id = $3")
        .bind(role.as_str())
        .bind(Utc::now())
        .bind(user_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

/// Would the proposed change leave the system with zero active Developers?
///
/// `new_role`:
/// - `None` → user is being deleted entirely.
/// - `Some(role)` → user's role is being changed to `role`.
///
/// Returns `true` only when:
/// - exactly one active Developer exists, AND
/// - the target user IS that Developer, AND
/// - the action would remove their Developer status.
///
/// Used as a server-side guard in user-edit / user-delete handlers,
/// and as a CLI warning before destructive role changes.
pub async fn would_orphan_developers(
    db: &Db,
    user_id: i64,
    new_role: Option<Role>,
) -> Result<bool> {
    if matches!(new_role, Some(Role::Developer)) {
        return Ok(false);
    }

    let active_dev_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rustio_users \
         WHERE role = 'developer' AND is_active = TRUE",
    )
    .fetch_one(db.pool())
    .await?;

    if active_dev_count == 0 {
        return Ok(false);
    }
    if active_dev_count > 1 {
        return Ok(false);
    }

    let target_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM rustio_users WHERE id = $1 AND is_active = TRUE")
            .bind(user_id)
            .fetch_optional(db.pool())
            .await?;
    Ok(target_role.as_deref() == Some("developer"))
}

/// Verify credentials and create a session. Returns the session token
/// to set in the cookie. A deliberately vague error on failure — we
/// don't want to leak whether the email was valid.
pub async fn login(db: &Db, email: &str, password: &str) -> Result<String> {
    let user = find_user_by_email(db, email)
        .await?
        .ok_or_else(|| Error::Unauthorized("invalid email or password".into()))?;
    if !user.is_active {
        return Err(Error::Forbidden("account disabled".into()));
    }
    if !verify_password(password, &user.password_hash) {
        return Err(Error::Unauthorized("invalid email or password".into()));
    }
    create_session(db, user.id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_profile_derives_debug_and_clone() {
        fn assert_traits<T: std::fmt::Debug + Clone>() {}
        assert_traits::<UserProfile>();
    }

    #[test]
    fn password_round_trip() {
        let h = hash_password("secret").unwrap();
        assert!(verify_password("secret", &h));
        assert!(!verify_password("wrong", &h));
    }
}
