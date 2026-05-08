//! Authentication & authorization.
//!
//! Three pieces:
//! - `users.rs`       — user records, password hashing, login
//! - `sessions.rs`    — DB-backed sessions with expiry cleanup
//! - `permissions.rs` — granular permissions + groups
//!
//! A user belongs to zero or more groups. Permissions come from two
//! sources: (a) direct assignments on the user, (b) inherited from
//! the user's groups. The permission string is
//! `<app>.<action>_<model>` — e.g. `posts.change_post`.

pub mod guards;
mod permissions;
mod role;
mod sessions;
mod users;

pub(crate) use permissions::invalidate_user_cache;
pub use permissions::{
    add_user_to_group, check_permission, create_group, grant_to_group, grant_to_user,
    init_permission_tables, permissions_for_user, register_model_permissions,
    remove_user_from_group, Permission, PermissionError, Superuser,
};
pub use role::{protected_roles, Role};
pub use sessions::{
    create_session, current_session_id, delete_session, identity_from_session,
    init_session_tables, invalidate_sessions, list_active_for_user, logout_session,
    purge_expired_sessions, session_token_from_cookie, InvalidationOutcome, Session,
    SessionInvalidationReason, SessionTarget, SessionTrust, SESSION_COOKIE,
};
#[allow(deprecated)]
pub use users::would_orphan_developers;
pub use users::{
    create_user, find_user_by_email, hash_password, init_user_tables, load_user_profile, login,
    migrate_user_schema, set_password, update_user_role, verdict_for_orphan_role, verify_password,
    would_orphan_protected, would_orphan_role, Identity, StoredUser, UserProfile,
};

use crate::error::Result;
use crate::orm::Db;

/// Initialise every auth-related table. Safe to call on every boot.
pub async fn init_tables(db: &Db) -> Result<()> {
    init_user_tables(db).await?;
    migrate_user_schema(db).await?;
    init_session_tables(db).await?;
    sessions::migrate_session_schema(db).await?;
    sessions::migrate_session_lifecycle(db).await?;
    init_permission_tables(db).await?;
    Ok(())
}
