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

// public:
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
    /// Mirrors the `rustio_users.must_change_password` column added in
    /// R1's recovery migration. When `TRUE`, R2's `login_guard`
    /// (commit #13) redirects every authenticated request to
    /// `/admin/must-change-password` until the user completes the
    /// forced rotation, except for a small whitelist
    /// (`/admin/must-change-password`, `/admin/logout`,
    /// `/admin/account/sessions`). R1 emissions don't read this
    /// field; this commit only loads it from the SQL paths so commits
    /// #9 / #13 can act on it.
    pub must_change_password: bool,
    /// Mirrors the `rustio_users.mfa_enabled` column added in R3's
    /// MFA migration (commit #1). When `TRUE`, the login flow
    /// (commit #16) redirects to `/admin/mfa/verify` after
    /// successful password verification, and R3's `login_guard`
    /// extension (commit #18) restricts non-MFA-verified
    /// sessions to a small whitelist
    /// (`/admin/mfa/verify`, `/admin/logout`,
    /// `/admin/account/sessions`). Pre-R3 sessions and users
    /// who have not enrolled get `FALSE` and bypass the
    /// challenge entirely — pre-R3 framework behaviour.
    pub mfa_enabled: bool,
    /// The active session's trust level
    /// (`authenticated` / `elevated` / `mfa_verified`). R3's
    /// `login_guard` (commit #18) reads this together with
    /// `mfa_enabled` to gate the pending-MFA state: an
    /// MFA-enrolled user whose current session has
    /// `trust_level != mfa_verified` (i.e. just signed in,
    /// hasn't yet completed `/admin/mfa/verify`) is restricted
    /// to a tiny whitelist until they finish the second-factor
    /// challenge. Pre-R3 sessions default to
    /// `SessionTrust::Authenticated` from the schema's column
    /// default — no migration data change.
    pub trust_level: crate::auth::SessionTrust,
}

impl Identity {
    // public:
    /// Administrator-or-higher (Administrator, Developer).
    pub fn is_admin(&self) -> bool {
        self.is_active && self.role.includes(Role::Administrator)
    }

    // public:
    /// Anyone allowed into the admin panel (Staff and above).
    pub fn can_access_admin(&self) -> bool {
        self.is_active && self.role.can_access_panel()
    }
}

// public:
pub struct StoredUser {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub role: Role,
    pub is_active: bool,
    pub is_demo: bool,
    pub demo_label: Option<String>,
    /// Mirrors `rustio_users.must_change_password`. The R2 login flow
    /// (commit #9) reads this through `find_user_by_email` to decide
    /// whether to set `must_change_password` on the freshly-issued
    /// `Identity`; when the flag is set the user is redirected
    /// immediately to `/admin/must-change-password` after sign-in.
    pub must_change_password: bool,
    /// Mirrors `rustio_users.mfa_enabled`. R3's `do_login`
    /// (commit #16) reads this to decide whether to redirect
    /// the freshly-authenticated user to `/admin/mfa/verify`
    /// before allowing access to `/admin`.
    pub mfa_enabled: bool,
    /// Profile identity columns. All nullable in the DB; consumers
    /// pick the first non-empty entry following the
    /// `display_name → first_name → email-local-part → "there"`
    /// fallback chain when rendering greetings. Surface examples:
    /// recovery-email greeting, signature block, future
    /// security-alert emails, operator-facing chrome.
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub job_title: Option<String>,
}

impl StoredUser {
    // public:
    /// Resolve the greeting label per the documented fallback:
    /// `display_name → first_name → email-local-part → "there"`.
    /// Always returns a non-empty string suitable for direct
    /// interpolation into "Hello {x},".
    pub fn greeting_name(&self) -> String {
        if let Some(d) = self.display_name.as_deref() {
            let t = d.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
        if let Some(f) = self.first_name.as_deref() {
            let t = f.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
        if let Some((local, _)) = self.email.split_once('@') {
            let t = local.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
        "there".to_string()
    }

    // public:
    /// "Best display name + role" pair used by the recovery
    /// email's signature block. Falls back gracefully when the
    /// profile fields are unset.
    pub fn signature_lines(&self) -> (String, Option<String>) {
        // Line 1: full name preferred — `first last`, otherwise
        // display_name, otherwise email-local-part.
        let primary = match (
            self.first_name.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            self.last_name.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        ) {
            (Some(f), Some(l)) => format!("{f} {l}"),
            (Some(f), None) => f.to_string(),
            (None, Some(l)) => l.to_string(),
            (None, None) => {
                if let Some(d) = self.display_name.as_deref() {
                    let t = d.trim();
                    if !t.is_empty() {
                        return (t.to_string(), self.job_title.clone());
                    }
                }
                self.email
                    .split('@')
                    .next()
                    .unwrap_or(self.email.as_str())
                    .to_string()
            }
        };
        let secondary = self.job_title.clone().filter(|s| !s.trim().is_empty());
        (primary, secondary)
    }
}

// public:
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

// public:
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
// public:
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

    // Profile identity columns surfaced by recovery emails + chrome.
    // All nullable — legacy installs continue to work without them.
    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS first_name TEXT")
        .execute(db.pool())
        .await?;
    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS last_name TEXT")
        .execute(db.pool())
        .await?;
    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS display_name TEXT")
        .execute(db.pool())
        .await?;
    sqlx::query("ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS job_title TEXT")
        .execute(db.pool())
        .await?;

    Ok(())
}

// public:
pub fn hash_password(plain: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| Error::Internal(format!("password hashing: {e}")))
}

// public:
pub fn verify_password(plain: &str, stored_hash: &str) -> bool {
    match PasswordHash::new(stored_hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(plain.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

// public:
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

// public:
pub async fn find_user_by_email(db: &Db, email: &str) -> Result<Option<StoredUser>> {
    let row = sqlx::query(
        "SELECT id, email, password_hash, role, is_active, is_demo, demo_label, \
                must_change_password, mfa_enabled, \
                first_name, last_name, display_name, job_title \
           FROM rustio_users \
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
                must_change_password: r.get_bool("must_change_password")?,
                mfa_enabled: r.get_bool("mfa_enabled")?,
                first_name: r.get_optional_string("first_name")?,
                last_name: r.get_optional_string("last_name")?,
                display_name: r.get_optional_string("display_name")?,
                job_title: r.get_optional_string("job_title")?,
            }))
        }
        None => Ok(None),
    }
}

// public:
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

/// Re-hash and write a new password for `user_id`. Stamps both
/// `password_changed_at` and `updated_at` to the same `NOW()` —
/// `password_changed_at` is the doctrine-7 surface ("Password last
/// changed: 2 days ago") that the active-sessions UI reads; the
/// existing `updated_at` continues to track row-level edits.
///
/// R1 (DESIGN_RECOVERY.md §14.1) introduced the
/// `password_changed_at` write here so every code path that mutates a
/// password — self-change, self-reset, R2 admin-driven reset, R4 CLI
/// emergency reset — stamps the column without each caller having to
/// remember. Pre-0.5.0 rows have `NULL` for the column; it populates
/// on the next change.
///
/// Callers that need to invalidate sessions (per doctrine 22) do so
/// separately by calling `auth::invalidate_sessions(...)` after this
/// returns. This function deliberately does NOT call into the session
/// engine: a CLI flow may want to keep sessions live, and the auth-
/// driven self-change wants to keep the current device alive
/// (`SessionTarget::UserExceptCurrent`). Wiring it in here would
/// remove that flexibility.
// public:
pub async fn set_password(db: &Db, user_id: i64, new_password: &str) -> Result<()> {
    let hash = hash_password(new_password)?;
    sqlx::query(
        "UPDATE rustio_users \
            SET password_hash = $1, password_changed_at = $2, updated_at = $2 \
          WHERE id = $3",
    )
    .bind(&hash)
    .bind(Utc::now())
    .bind(user_id)
    .execute(db.pool())
    .await?;
    Ok(())
}

// public:
pub async fn update_user_role(db: &Db, user_id: i64, role: Role) -> Result<()> {
    sqlx::query("UPDATE rustio_users SET role = $1, updated_at = $2 WHERE id = $3")
        .bind(role.as_str())
        .bind(Utc::now())
        .bind(user_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

/// Pure verdict for the orphan check, factored out so it can be
/// unit-tested without a `Db`. The async wrapper [`would_orphan_role`]
/// supplies `active_count` and `target_is_protected` from SQL.
///
/// Returns `true` only when removing this user from the protected
/// pool would empty it (count == 1 and the target user IS in that
/// pool, AND the proposed new state would no longer satisfy
/// membership).
// public:
pub fn verdict_for_orphan_role(
    active_count_in_protected: i64,
    target_is_in_protected: bool,
    new_role_is_protected: bool,
    new_active: bool,
) -> bool {
    if !target_is_in_protected {
        return false;
    }
    if active_count_in_protected != 1 {
        return false;
    }
    // The target IS the only active member. Block unless the proposed
    // state keeps them in the same protected role and active.
    !(new_active && new_role_is_protected)
}

/// Would the proposed change leave the system with zero active members
/// of `protected_role`?
///
/// `new_role` / `new_active` describe the target row's proposed state:
/// - delete: pass `new_active = false` (the row goes away).
/// - role change: pass the new role.
/// - deactivate: pass `new_active = false`.
///
/// Returns `true` only when:
/// - exactly one active member of `protected_role` exists, AND
/// - the target user IS that member, AND
/// - the proposed state would remove them from the protected pool.
// public:
pub async fn would_orphan_role(
    db: &Db,
    user_id: i64,
    protected_role: Role,
    new_role: Role,
    new_active: bool,
) -> Result<bool> {
    let active_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rustio_users WHERE role = $1 AND is_active = TRUE",
    )
    .bind(protected_role.as_str())
    .fetch_one(db.pool())
    .await?;

    let target_role_str: Option<String> =
        sqlx::query_scalar("SELECT role FROM rustio_users WHERE id = $1 AND is_active = TRUE")
            .bind(user_id)
            .fetch_optional(db.pool())
            .await?;
    let target_is_in_protected = target_role_str.as_deref() == Some(protected_role.as_str());

    Ok(verdict_for_orphan_role(
        active_count,
        target_is_in_protected,
        new_role == protected_role,
        new_active,
    ))
}

/// Walk every entry in [`super::role::protected_roles`] and return
/// the first protected role whose membership would be orphaned by
/// the proposed change. `None` means the change is safe.
// public:
pub async fn would_orphan_protected(
    db: &Db,
    user_id: i64,
    new_role: Role,
    new_active: bool,
) -> Result<Option<Role>> {
    for &role in super::role::protected_roles() {
        if would_orphan_role(db, user_id, role, new_role, new_active).await? {
            return Ok(Some(role));
        }
    }
    Ok(None)
}

/// Legacy alias preserved so external callers keep compiling. Prefer
/// [`would_orphan_protected`] which generalises across every role in
/// [`super::role::protected_roles`].
// public:
#[deprecated(
    since = "0.3.0",
    note = "use `would_orphan_protected` to cover every protected role, not just Developer"
)]
pub async fn would_orphan_developers(
    db: &Db,
    user_id: i64,
    new_role: Option<Role>,
) -> Result<bool> {
    let (role, active) = match new_role {
        Some(r) => (r, true),
        None => (Role::User, false),
    };
    would_orphan_role(db, user_id, Role::Developer, role, active).await
}

// public:
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

    // ---- verdict_for_orphan_role ----

    #[test]
    fn verdict_safe_when_target_not_in_protected_pool() {
        // Target is Staff; we're checking Administrator orphan-ness.
        // Even if active_count == 0 the change is irrelevant to that pool.
        assert!(!verdict_for_orphan_role(0, false, false, true));
        assert!(!verdict_for_orphan_role(1, false, false, false));
        assert!(!verdict_for_orphan_role(5, false, true, true));
    }

    #[test]
    fn verdict_safe_when_more_than_one_member() {
        // Target IS the protected role, but there's a second active
        // member — losing this one keeps the floor satisfied.
        assert!(!verdict_for_orphan_role(2, true, false, true));
        assert!(!verdict_for_orphan_role(5, true, false, false));
    }

    #[test]
    fn verdict_blocks_when_last_member_demoting() {
        // active_count = 1, target IS that member, new state drops
        // them out of the pool → block.
        assert!(verdict_for_orphan_role(1, true, false, true));
    }

    #[test]
    fn verdict_blocks_when_last_member_deactivating() {
        // Same shape but new_active = false; new_role doesn't matter.
        assert!(verdict_for_orphan_role(1, true, true, false));
    }

    #[test]
    fn verdict_blocks_when_last_member_deleting() {
        // Delete is modelled as new_active = false in the wrapper.
        assert!(verdict_for_orphan_role(1, true, false, false));
    }

    #[test]
    fn verdict_safe_when_last_member_keeps_role() {
        // No-op save: still in pool, still active → safe.
        assert!(!verdict_for_orphan_role(1, true, true, true));
    }
}
