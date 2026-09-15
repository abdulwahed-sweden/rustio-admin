//! **Contract: the authority guards, at the public boundary.**
//!
//! `auth::guards` is the framework's last line of defence before an
//! authority mutation reaches the database. These four rules are what stop
//! an operator locking the project out of its own admin:
//!
//! - an actor may not demote or deactivate **themselves**;
//! - an actor may not touch a peer at or above their own rank;
//! - an actor may not grant a role above their own ceiling;
//! - the role ladder itself is ordered and transitive.
//!
//! The guards are pure functions over `Identity`, so this suite needs no
//! database and runs in the default `cargo test --workspace` gate. The
//! database-backed sibling (`enforce_no_orphan_role`, the last-administrator
//! rule) lives in `contract_authority_recovery.rs` behind `integration-test`.
//!
//! These are deliberately *boundary* assertions. The exhaustive 25-case role
//! matrix stays where it belongs, in `auth::role`'s own unit tests.

use rustio_admin::auth::{Identity, Role, SessionTrust};

/// Build an actor. Only the four fields the guards read carry meaning.
fn actor(user_id: i64, role: Role) -> Identity {
    Identity {
        user_id,
        email: format!("user{user_id}@local"),
        role,
        is_active: true,
        is_demo: false,
        demo_label: None,
        must_change_password: false,
        mfa_enabled: false,
        trust_level: SessionTrust::Authenticated,
    }
}

// ---- the role ladder ------------------------------------------------------

#[test]
fn role_ladder_is_ordered_and_transitive() {
    // Authorization everywhere else is `Role::includes(min)`. If the ladder
    // inverts, every permission check inverts with it.
    assert!(Role::Developer.includes(Role::Administrator));
    assert!(Role::Administrator.includes(Role::Staff));
    assert!(Role::Developer.includes(Role::Staff));
    assert!(!Role::Staff.includes(Role::Administrator));
    assert!(!Role::User.includes(Role::Staff));
}

#[test]
fn panel_access_starts_at_staff() {
    assert!(Role::Developer.can_access_panel());
    assert!(Role::Administrator.can_access_panel());
    assert!(Role::Staff.can_access_panel());
    assert!(!Role::User.can_access_panel());
}

// ---- self-protection (Phase 5 item 8) -------------------------------------

#[test]
fn an_actor_may_not_deactivate_themselves() {
    let me = actor(1, Role::Administrator);
    let err = rustio_admin::auth::guards::enforce_self_demote_safe(
        &me,
        me.user_id,
        Role::Administrator,
        false, // new_active
    )
    .expect_err("self-deactivation must be refused");
    assert!(
        matches!(err, rustio_admin::Error::Forbidden(_)),
        "expected Forbidden, got {err:?}"
    );
}

#[test]
fn an_actor_may_not_demote_themselves() {
    let me = actor(1, Role::Administrator);
    rustio_admin::auth::guards::enforce_self_demote_safe(&me, me.user_id, Role::Staff, true)
        .expect_err("self-demotion must be refused");
}

#[test]
fn self_edits_that_keep_rank_and_activity_are_allowed() {
    let me = actor(1, Role::Administrator);
    rustio_admin::auth::guards::enforce_self_demote_safe(
        &me,
        me.user_id,
        Role::Administrator,
        true,
    )
    .expect("editing yourself without demoting must be allowed");
}

#[test]
fn the_self_guard_does_not_fire_for_other_users() {
    let me = actor(1, Role::Administrator);
    rustio_admin::auth::guards::enforce_self_demote_safe(&me, 2, Role::Staff, false)
        .expect("the self guard is scoped to the actor's own row");
}

// ---- peer protection (Phase 5 item 9) -------------------------------------

#[test]
fn an_actor_may_not_touch_a_peer_at_or_above_their_rank() {
    let admin = actor(1, Role::Administrator);
    rustio_admin::auth::guards::enforce_cross_rank_safe(&admin, 2, Role::Administrator)
        .expect_err("equal rank must be refused");
    rustio_admin::auth::guards::enforce_cross_rank_safe(&admin, 3, Role::Developer)
        .expect_err("higher rank must be refused");
}

#[test]
fn an_actor_may_touch_someone_below_their_rank() {
    let admin = actor(1, Role::Administrator);
    rustio_admin::auth::guards::enforce_cross_rank_safe(&admin, 2, Role::Staff)
        .expect("lower rank is in scope");
}

// ---- privilege ceiling ----------------------------------------------------

#[test]
fn an_actor_may_not_grant_a_role_above_their_own() {
    let admin = actor(1, Role::Administrator);
    rustio_admin::auth::guards::enforce_role_ceiling(&admin, Role::Developer)
        .expect_err("escalation above the actor's own rank must be refused");
}

#[test]
fn an_actor_may_grant_their_own_rank_or_lower() {
    let admin = actor(1, Role::Administrator);
    rustio_admin::auth::guards::enforce_role_ceiling(&admin, Role::Administrator)
        .expect("granting your own rank is allowed");
    rustio_admin::auth::guards::enforce_role_ceiling(&admin, Role::Staff)
        .expect("granting below your rank is allowed");
}
