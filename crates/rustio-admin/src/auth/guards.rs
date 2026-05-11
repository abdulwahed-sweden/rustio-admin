//! Authority guards — server-side enforcement of the rank model.
//!
//! Every authority mutation (user create / update / delete, future
//! group / permission / recovery flows) routes through one of these
//! pure verdict functions before touching the DB. UI hiding is a
//! courtesy, not a security boundary; the framework refuses unsafe
//! state changes here regardless of what the form said.
//!
//! The guards encode five invariants:
//!
//! 1. **Self-demote / self-deactivate are blocked.** A signed-in user
//!    cannot lower their own role nor flip themselves to inactive
//!    (matches the existing self-delete rule). Self-keep-rank is fine.
//!
//! 2. **Cross-rank protection.** A user cannot edit another user
//!    whose role rank is at-or-above their own. Editing one's own
//!    record is not blocked by this guard (the self-* guards cover
//!    that), and editing a strictly lower-rank target is allowed.
//!
//! 3. **Role ceiling.** A user cannot grant a role with a rank
//!    strictly above their own — even to themselves. Equal is
//!    allowed (an Admin keeping their own Admin role on save).
//!
//! 4. **Protected-role orphan prevention.** Lives in
//!    [`super::would_orphan_protected`]; the guard wrapper here
//!    converts the resolved orphan-role into a clear human message.
//!
//! 5. _(deferred to a later phase)_ Permission ceiling — a user
//!    cannot grant permissions they themselves don't hold. Today the
//!    group routes are gated by `Role::Administrator`, who bypasses
//!    group checks, so this guard is unreachable; reinstate when
//!    delegated group management lands.
//!
//! All five return [`Error::Forbidden`] on rejection so the HTTP
//! layer renders a 403 with the supplied reason.

use crate::error::{Error, Result};
use crate::orm::Db;

use super::role::Role;
use super::users::{would_orphan_protected, Identity};

/// Forbid a user from saving an edit to their own record that drops
/// their role below its current rank or flips `is_active` to false.
/// Self-keep-rank is allowed; raising one's own rank is blocked
/// separately by [`enforce_role_ceiling`].
pub fn enforce_self_demote_safe(
    actor: &Identity,
    target_id: i64,
    new_role: Role,
    new_active: bool,
) -> Result<()> {
    if actor.user_id != target_id {
        return Ok(());
    }
    if !new_active {
        return Err(Error::Forbidden(
            "You cannot deactivate yourself.".to_string(),
        ));
    }
    if new_role.rank() < actor.role.rank() {
        return Err(Error::Forbidden(
            "You cannot demote yourself below your current authority level.".to_string(),
        ));
    }
    Ok(())
}

/// Forbid a user from modifying another user whose role is at-or-above
/// their own. Editing one's own record is allowed; the self-* guards
/// catch the dangerous cases.
pub fn enforce_cross_rank_safe(actor: &Identity, target_id: i64, target_role: Role) -> Result<()> {
    if actor.user_id == target_id {
        return Ok(());
    }
    if target_role.rank() >= actor.role.rank() {
        return Err(Error::Forbidden(
            "You cannot modify users at or above your authority level.".to_string(),
        ));
    }
    Ok(())
}

/// Forbid a user from assigning a role with rank strictly greater
/// than their own. Same-rank is allowed so an Admin can re-save
/// another Admin's record (already-existing target's rank ladder is
/// covered by [`enforce_cross_rank_safe`]).
pub fn enforce_role_ceiling(actor: &Identity, requested_role: Role) -> Result<()> {
    if requested_role.rank() > actor.role.rank() {
        return Err(Error::Forbidden(
            "You cannot assign a role higher than your own authority.".to_string(),
        ));
    }
    Ok(())
}

/// Reject changes that would empty the active-member set for any
/// protected role. Wraps [`would_orphan_protected`] and returns a
/// human-readable error naming the role that would be orphaned.
pub async fn enforce_no_orphan_role(
    db: &Db,
    target_id: i64,
    new_role: Role,
    new_active: bool,
) -> Result<()> {
    if let Some(role) = would_orphan_protected(db, target_id, new_role, new_active).await? {
        return Err(Error::Forbidden(format!(
            "At least one active {} must remain.",
            role.label()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(role: Role, user_id: i64) -> Identity {
        Identity {
            user_id,
            email: format!("u{user_id}@test"),
            role,
            is_active: true,
            is_demo: false,
            demo_label: None,
            must_change_password: false,
            mfa_enabled: false,
            trust_level: crate::auth::SessionTrust::Authenticated,
        }
    }

    // ---- enforce_self_demote_safe ----

    #[test]
    fn self_demote_blocks_role_drop() {
        let actor = ident(Role::Administrator, 1);
        let err = enforce_self_demote_safe(&actor, 1, Role::Staff, true).unwrap_err();
        assert!(matches!(err, Error::Forbidden(_)));
    }

    #[test]
    fn self_demote_blocks_self_deactivate() {
        let actor = ident(Role::Administrator, 1);
        let err = enforce_self_demote_safe(&actor, 1, Role::Administrator, false).unwrap_err();
        assert!(matches!(err, Error::Forbidden(_)));
    }

    #[test]
    fn self_demote_allows_self_keep_rank() {
        let actor = ident(Role::Administrator, 1);
        assert!(enforce_self_demote_safe(&actor, 1, Role::Administrator, true).is_ok());
    }

    #[test]
    fn self_demote_ignores_other_targets() {
        let actor = ident(Role::Administrator, 1);
        // Lowering another user's role is fine for THIS guard
        // (cross-rank handles the inverse case).
        assert!(enforce_self_demote_safe(&actor, 2, Role::User, true).is_ok());
    }

    // ---- enforce_cross_rank_safe ----

    #[test]
    fn cross_rank_blocks_editing_higher() {
        let actor = ident(Role::Administrator, 1);
        let err = enforce_cross_rank_safe(&actor, 2, Role::Developer).unwrap_err();
        assert!(matches!(err, Error::Forbidden(_)));
    }

    #[test]
    fn cross_rank_blocks_editing_equal() {
        let actor = ident(Role::Administrator, 1);
        let err = enforce_cross_rank_safe(&actor, 2, Role::Administrator).unwrap_err();
        assert!(matches!(err, Error::Forbidden(_)));
    }

    #[test]
    fn cross_rank_allows_editing_lower() {
        let actor = ident(Role::Administrator, 1);
        assert!(enforce_cross_rank_safe(&actor, 2, Role::Staff).is_ok());
        assert!(enforce_cross_rank_safe(&actor, 2, Role::Supervisor).is_ok());
    }

    #[test]
    fn cross_rank_allows_editing_self() {
        let actor = ident(Role::Administrator, 1);
        // Self-edit isn't the concern of this guard.
        assert!(enforce_cross_rank_safe(&actor, 1, Role::Administrator).is_ok());
    }

    #[test]
    fn cross_rank_developer_can_edit_administrator() {
        let actor = ident(Role::Developer, 1);
        assert!(enforce_cross_rank_safe(&actor, 2, Role::Administrator).is_ok());
    }

    // ---- enforce_role_ceiling ----

    #[test]
    fn ceiling_blocks_promote_above_self() {
        let actor = ident(Role::Administrator, 1);
        let err = enforce_role_ceiling(&actor, Role::Developer).unwrap_err();
        assert!(matches!(err, Error::Forbidden(_)));
    }

    #[test]
    fn ceiling_allows_assigning_equal_or_below() {
        let actor = ident(Role::Administrator, 1);
        assert!(enforce_role_ceiling(&actor, Role::Administrator).is_ok());
        assert!(enforce_role_ceiling(&actor, Role::Supervisor).is_ok());
        assert!(enforce_role_ceiling(&actor, Role::Staff).is_ok());
        assert!(enforce_role_ceiling(&actor, Role::User).is_ok());
    }

    #[test]
    fn ceiling_supervisor_cannot_create_administrator() {
        let actor = ident(Role::Supervisor, 1);
        let err = enforce_role_ceiling(&actor, Role::Administrator).unwrap_err();
        assert!(matches!(err, Error::Forbidden(_)));
    }

    #[test]
    fn ceiling_developer_can_assign_anything_inclusive() {
        let actor = ident(Role::Developer, 1);
        for r in [
            Role::User,
            Role::Staff,
            Role::Supervisor,
            Role::Administrator,
            Role::Developer,
        ] {
            assert!(enforce_role_ceiling(&actor, r).is_ok());
        }
    }
}
