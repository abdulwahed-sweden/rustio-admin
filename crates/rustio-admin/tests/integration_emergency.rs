//! R4 emergency-recovery integration tests
//! (`DESIGN_R4_EMERGENCY.md` §9.2).
//!
//! Boots an ephemeral Postgres container per test via
//! `testcontainers`, runs `auth::init_tables` to apply the
//! framework migrations (R0 + R1 + R2 + R3 schema in one shot),
//! then exercises each `auth::emergency::*` runtime fn end-to-end
//! against the live DB.
//!
//! ## Scope of this suite
//!
//! These tests cover the **framework-side** R4 surface — the
//! atomic DB-mutation functions in `auth::emergency`. The CLI
//! wiring (banner rendering, reason validation, audit-row
//! emission, the user-facing summary) is exercised by the
//! per-command live-DB smoke runs documented in commits #5-#8 of
//! the R4 chain. Splitting the test surfaces this way matches the
//! splitting of the implementation: framework owns the mutation;
//! CLI owns the audit emission (D12).
//!
//! ## Scenarios
//!
//! - `reset_password_updates_password_and_invalidates_sessions`
//! - `unlock_clears_lock_state_and_does_not_touch_sessions`
//! - `disable_mfa_clears_columns_and_backup_codes`
//! - `promote_succeeds_for_non_sole_admin`
//! - `promote_refuses_sole_admin_demote`
//! - `promote_returns_no_change_when_role_already_matches`
//! - `emergency_access_token_hash_matches_r1_consume_format` —
//!   the load-bearing test that would have caught commit #8's
//!   hex-vs-base64 drift before publish.
//! - `emergency_access_refuses_inactive_target`
//!
//! ## What this suite is NOT
//!
//! - Not a HTTP-level smoke. The end-to-end consume path
//!   (operator hands URL → user opens form → user POSTs new
//!   password → user logs in) lives in the lursystem flagship
//!   smoke documented in commit #8.

#![cfg(feature = "integration-test")]

use sqlx::Row as _;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

use rustio_admin::__integration::hash_token_for_storage;
use rustio_admin::auth::emergency::{
    disable_mfa, emergency_access, promote, reset_password, unlock, DisableMfaOutcome,
    EmergencyAccessOutcome, PromoteOutcome, ResetOutcome, UnlockOutcome,
};
use rustio_admin::auth::{self, Role};
use rustio_admin::orm::Db;

// ---- Test harness --------------------------------------------------------

struct TestEnv {
    db: Db,
    _container: testcontainers::ContainerAsync<Postgres>,
}

async fn boot() -> TestEnv {
    let container = Postgres::default()
        .start()
        .await
        .expect("postgres container starts");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("port mapping");
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let db = Db::connect(&url).await.expect("Db::connect");
    auth::init_tables(&db).await.expect("auth::init_tables");
    TestEnv {
        db,
        _container: container,
    }
}

async fn create_user(db: &Db, email: &str, password: &str, role: Role) -> i64 {
    auth::create_user(db, email, password, role)
        .await
        .expect("create_user")
}

async fn mint_session_token(db: &Db, user_id: i64) -> String {
    auth::create_session(db, user_id)
        .await
        .expect("create_session")
}

async fn read_password_hash(db: &Db, user_id: i64) -> String {
    sqlx::query("SELECT password_hash FROM rustio_users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db.pool())
        .await
        .expect("read password_hash")
        .try_get::<String, _>("password_hash")
        .unwrap()
}

async fn read_must_change(db: &Db, user_id: i64) -> bool {
    sqlx::query("SELECT must_change_password FROM rustio_users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db.pool())
        .await
        .expect("read must_change_password")
        .try_get::<bool, _>("must_change_password")
        .unwrap()
}

async fn count_active_sessions(db: &Db, user_id: i64) -> i64 {
    sqlx::query(
        "SELECT COUNT(*)::bigint AS n FROM rustio_sessions \
          WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(db.pool())
    .await
    .expect("count active sessions")
    .try_get::<i64, _>("n")
    .unwrap()
}

async fn set_locked(db: &Db, user_id: i64) {
    sqlx::query(
        "UPDATE rustio_users \
         SET locked_until = NOW() + INTERVAL '15 minutes', failed_login_count = 5 \
         WHERE id = $1",
    )
    .bind(user_id)
    .execute(db.pool())
    .await
    .expect("set locked state");
}

async fn read_lock_state(db: &Db, user_id: i64) -> (bool, i32) {
    let row = sqlx::query(
        "SELECT (locked_until IS NOT NULL AND locked_until > NOW()) AS still_locked, \
                failed_login_count \
         FROM rustio_users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(db.pool())
    .await
    .expect("read lock state");
    (
        row.try_get::<bool, _>("still_locked").unwrap(),
        row.try_get::<i32, _>("failed_login_count").unwrap(),
    )
}

async fn fake_enable_mfa(db: &Db, user_id: i64) {
    sqlx::query(
        "UPDATE rustio_users SET \
            mfa_enabled = TRUE, \
            mfa_secret_ciphertext = decode('00112233445566778899aabbccddeeff', 'hex'), \
            mfa_secret_key_id = 1, \
            mfa_last_used_step = 12345 \
         WHERE id = $1",
    )
    .bind(user_id)
    .execute(db.pool())
    .await
    .expect("fake-enable mfa");
    for h in ["fakehash_a", "fakehash_b", "fakehash_c"] {
        sqlx::query("INSERT INTO rustio_mfa_backup_codes (user_id, code_hash) VALUES ($1, $2)")
            .bind(user_id)
            .bind(h)
            .execute(db.pool())
            .await
            .expect("insert backup code");
    }
}

async fn read_mfa_state(db: &Db, user_id: i64) -> (bool, bool, i64) {
    let user_row = sqlx::query(
        "SELECT mfa_enabled, mfa_secret_ciphertext IS NULL AS secret_cleared \
         FROM rustio_users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(db.pool())
    .await
    .expect("read mfa state");
    let codes: i64 =
        sqlx::query("SELECT COUNT(*)::bigint AS n FROM rustio_mfa_backup_codes WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(db.pool())
            .await
            .expect("count backup codes")
            .try_get("n")
            .unwrap();
    (
        user_row.try_get::<bool, _>("mfa_enabled").unwrap(),
        user_row.try_get::<bool, _>("secret_cleared").unwrap(),
        codes,
    )
}

async fn set_inactive(db: &Db, user_id: i64) {
    sqlx::query("UPDATE rustio_users SET is_active = FALSE WHERE id = $1")
        .bind(user_id)
        .execute(db.pool())
        .await
        .expect("set inactive");
}

// ---- reset_password ------------------------------------------------------

#[tokio::test]
async fn reset_password_updates_password_and_invalidates_sessions() {
    let env = boot().await;
    let uid = create_user(&env.db, "alice@example.com", "OldPassword1!", Role::Staff).await;
    let _t1 = mint_session_token(&env.db, uid).await;
    let _t2 = mint_session_token(&env.db, uid).await;
    assert_eq!(count_active_sessions(&env.db, uid).await, 2);

    let old_hash = read_password_hash(&env.db, uid).await;
    assert!(!read_must_change(&env.db, uid).await);

    let outcome = reset_password(&env.db, uid, "BrandNewPassword2!")
        .await
        .unwrap();
    match outcome {
        ResetOutcome::Ok {
            revoked_session_count,
        } => assert_eq!(revoked_session_count, 2, "both sessions revoked"),
        other => panic!("expected Ok, got {other:?}"),
    }

    let new_hash = read_password_hash(&env.db, uid).await;
    assert_ne!(old_hash, new_hash, "password_hash rotated");
    assert!(
        read_must_change(&env.db, uid).await,
        "must_change_password flipped to TRUE"
    );
    assert_eq!(
        count_active_sessions(&env.db, uid).await,
        0,
        "every session revoked"
    );
}

#[tokio::test]
async fn reset_password_unknown_target_short_circuits() {
    let env = boot().await;
    let outcome = reset_password(&env.db, 999_999_999, "Anything10!")
        .await
        .unwrap();
    assert!(matches!(outcome, ResetOutcome::UnknownTarget));
}

// ---- unlock --------------------------------------------------------------

#[tokio::test]
async fn unlock_clears_lock_state_and_does_not_touch_sessions() {
    let env = boot().await;
    let uid = create_user(&env.db, "alice@example.com", "OldPassword1!", Role::Staff).await;
    set_locked(&env.db, uid).await;
    let _session = mint_session_token(&env.db, uid).await;
    assert_eq!(count_active_sessions(&env.db, uid).await, 1);

    let (was_locked, _) = read_lock_state(&env.db, uid).await;
    assert!(was_locked, "lock pre-state");

    let outcome = unlock(&env.db, uid).await.unwrap();
    match outcome {
        UnlockOutcome::Ok { previously_locked } => assert!(previously_locked),
        other => panic!("expected Ok, got {other:?}"),
    }

    let (still_locked, failed_count) = read_lock_state(&env.db, uid).await;
    assert!(!still_locked, "locked_until cleared");
    assert_eq!(failed_count, 0, "failed_login_count zeroed");
    assert_eq!(
        count_active_sessions(&env.db, uid).await,
        1,
        "unlock must NOT revoke sessions"
    );
}

#[tokio::test]
async fn unlock_already_clear_returns_previously_locked_false() {
    let env = boot().await;
    let uid = create_user(&env.db, "alice@example.com", "OldPassword1!", Role::Staff).await;
    // Don't set_locked — user starts clear.

    let outcome = unlock(&env.db, uid).await.unwrap();
    match outcome {
        UnlockOutcome::Ok { previously_locked } => assert!(!previously_locked),
        other => panic!("expected Ok, got {other:?}"),
    }
}

// ---- disable_mfa ---------------------------------------------------------

#[tokio::test]
async fn disable_mfa_clears_columns_and_backup_codes() {
    let env = boot().await;
    let uid = create_user(&env.db, "alice@example.com", "OldPassword1!", Role::Staff).await;
    fake_enable_mfa(&env.db, uid).await;

    let (enabled, cleared, codes) = read_mfa_state(&env.db, uid).await;
    assert!(enabled);
    assert!(!cleared, "secret should be populated pre-state");
    assert_eq!(codes, 3, "3 backup codes inserted");

    let outcome = disable_mfa(&env.db, uid).await.unwrap();
    match outcome {
        DisableMfaOutcome::Ok {
            was_enabled,
            deleted_backup_codes,
            ..
        } => {
            assert!(was_enabled);
            assert_eq!(deleted_backup_codes, 3);
        }
        other => panic!("expected Ok, got {other:?}"),
    }

    let (enabled, cleared, codes) = read_mfa_state(&env.db, uid).await;
    assert!(!enabled, "mfa_enabled flipped to false");
    assert!(cleared, "secret ciphertext cleared");
    assert_eq!(codes, 0, "every backup code deleted");
}

#[tokio::test]
async fn disable_mfa_already_disabled_writes_no_op_outcome() {
    let env = boot().await;
    let uid = create_user(&env.db, "alice@example.com", "OldPassword1!", Role::Staff).await;
    // Don't fake_enable_mfa — user has no MFA yet.

    let outcome = disable_mfa(&env.db, uid).await.unwrap();
    match outcome {
        DisableMfaOutcome::Ok {
            was_enabled,
            deleted_backup_codes,
            ..
        } => {
            assert!(!was_enabled);
            assert_eq!(deleted_backup_codes, 0);
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

// ---- promote -------------------------------------------------------------

#[tokio::test]
async fn promote_succeeds_for_non_sole_admin_and_revokes_sessions() {
    let env = boot().await;
    let admin = create_user(
        &env.db,
        "lead@example.com",
        "OldPassword1!",
        Role::Administrator,
    )
    .await;
    let other_admin = create_user(
        &env.db,
        "lead2@example.com",
        "OldPassword1!",
        Role::Administrator,
    )
    .await;
    let _ = (admin, other_admin); // both exist, so promote demoting `admin` is safe

    let uid = create_user(&env.db, "alice@example.com", "OldPassword1!", Role::Staff).await;
    let _session = mint_session_token(&env.db, uid).await;

    let outcome = promote(&env.db, uid, Role::Supervisor).await.unwrap();
    match outcome {
        PromoteOutcome::Ok {
            previous_role,
            new_role,
            revoked_session_count,
        } => {
            assert_eq!(previous_role, Role::Staff);
            assert_eq!(new_role, Role::Supervisor);
            assert_eq!(revoked_session_count, 1, "session revoked on promote");
        }
        other => panic!("expected Ok, got {other:?}"),
    }
    assert_eq!(count_active_sessions(&env.db, uid).await, 0);
}

#[tokio::test]
async fn promote_refuses_sole_admin_demote() {
    let env = boot().await;
    let sole_admin = create_user(
        &env.db,
        "lead@example.com",
        "OldPassword1!",
        Role::Administrator,
    )
    .await;
    let _other = create_user(&env.db, "alice@example.com", "OldPassword1!", Role::Staff).await;
    let _session = mint_session_token(&env.db, sole_admin).await;

    let outcome = promote(&env.db, sole_admin, Role::Staff).await.unwrap();
    assert!(matches!(
        outcome,
        PromoteOutcome::SoleAdministratorDemoteRefused
    ));
    // Refused → no role change, no session revoke.
    let actual_role: String = sqlx::query("SELECT role FROM rustio_users WHERE id = $1")
        .bind(sole_admin)
        .fetch_one(env.db.pool())
        .await
        .unwrap()
        .try_get("role")
        .unwrap();
    assert_eq!(actual_role, "administrator", "role unchanged after refusal");
    assert_eq!(
        count_active_sessions(&env.db, sole_admin).await,
        1,
        "sessions untouched after refusal"
    );
}

#[tokio::test]
async fn promote_returns_no_change_when_role_already_matches() {
    let env = boot().await;
    let uid = create_user(
        &env.db,
        "alice@example.com",
        "OldPassword1!",
        Role::Supervisor,
    )
    .await;
    let _session = mint_session_token(&env.db, uid).await;

    let outcome = promote(&env.db, uid, Role::Supervisor).await.unwrap();
    match outcome {
        PromoteOutcome::NoChange { current_role } => assert_eq!(current_role, Role::Supervisor),
        other => panic!("expected NoChange, got {other:?}"),
    }
    assert_eq!(
        count_active_sessions(&env.db, uid).await,
        1,
        "NoChange must not revoke sessions"
    );
}

// ---- emergency_access ---------------------------------------------------

/// The load-bearing test. Commit #8 surfaced a bug where the
/// CLI-side token-hash format (lowercase hex) drifted from R1's
/// canonical helper (URL-safe-base64). The URL printed, but the
/// consume path's `hash_token_for_storage(token)` lookup returned
/// "no such row", and the user saw "this link is no longer
/// valid". A unit test can't catch this — the hash function on
/// the issue side and the lookup side both produce *some* string,
/// they just don't match. The real cross-check is the DB row.
///
/// This test asserts the structural invariant:
///   hash_token_for_storage(<token from URL>) == <token_hash stored on the row>
///
/// If commit #8's drift is ever re-introduced, this fails.
#[tokio::test]
async fn emergency_access_token_hash_matches_r1_consume_format() {
    let env = boot().await;
    let uid = create_user(&env.db, "alice@example.com", "OldPassword1!", Role::Staff).await;

    let outcome = emergency_access(&env.db, uid, 30).await.unwrap();
    let (token_id, url_path, _expires_at) = match outcome {
        EmergencyAccessOutcome::Ok {
            token_id,
            url_path,
            expires_at,
        } => (token_id, url_path, expires_at),
        other => panic!("expected Ok, got {other:?}"),
    };

    // Extract the plaintext token from the URL path.
    let plaintext_token = url_path
        .strip_prefix("/admin/reset-password/")
        .expect("url_path has the locked prefix");
    assert!(
        !plaintext_token.is_empty(),
        "url path carries a non-empty token"
    );

    // Hash it the way R1's consume_reset_token will.
    let expected_hash = hash_token_for_storage(plaintext_token);

    // Read the row the function inserted.
    let stored: String =
        sqlx::query("SELECT token_hash FROM rustio_password_reset_tokens WHERE id = $1")
            .bind(token_id)
            .fetch_one(env.db.pool())
            .await
            .expect("read token row")
            .try_get("token_hash")
            .unwrap();

    assert_eq!(
        stored, expected_hash,
        "stored token_hash must match R1's hash_token_for_storage format; \
         any drift here would make CLI-issued URLs fail with \
         'this link is no longer valid' at consume time"
    );
}

#[tokio::test]
async fn emergency_access_clamps_ttl_to_bounds() {
    let env = boot().await;
    let uid = create_user(&env.db, "alice@example.com", "OldPassword1!", Role::Staff).await;

    // Below the floor — clamped to 1 minute.
    let now = chrono::Utc::now();
    let outcome = emergency_access(&env.db, uid, -5).await.unwrap();
    match outcome {
        EmergencyAccessOutcome::Ok { expires_at, .. } => {
            let secs = (expires_at - now).num_seconds();
            assert!(
                (50..=80).contains(&secs),
                "TTL clamped to 1 minute (got {secs}s)"
            );
        }
        other => panic!("expected Ok, got {other:?}"),
    }

    // Above the ceiling — clamped to 60 minutes.
    let now = chrono::Utc::now();
    let outcome = emergency_access(&env.db, uid, 600).await.unwrap();
    match outcome {
        EmergencyAccessOutcome::Ok { expires_at, .. } => {
            let secs = (expires_at - now).num_seconds();
            // 60 min = 3600 s, allow ±30 s for test latency.
            assert!(
                (3570..=3630).contains(&secs),
                "TTL clamped to 60 minutes (got {secs}s)"
            );
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[tokio::test]
async fn emergency_access_refuses_inactive_target() {
    let env = boot().await;
    let uid = create_user(&env.db, "alice@example.com", "OldPassword1!", Role::Staff).await;
    set_inactive(&env.db, uid).await;

    let outcome = emergency_access(&env.db, uid, 15).await.unwrap();
    assert!(matches!(outcome, EmergencyAccessOutcome::InactiveTarget));

    // No row should have been inserted.
    let count: i64 = sqlx::query("SELECT COUNT(*)::bigint AS n FROM rustio_password_reset_tokens")
        .fetch_one(env.db.pool())
        .await
        .unwrap()
        .try_get("n")
        .unwrap();
    assert_eq!(count, 0, "refused issuance must not insert a token row");
}

#[tokio::test]
async fn emergency_access_unknown_target_short_circuits() {
    let env = boot().await;
    let outcome = emergency_access(&env.db, 999_999_999, 15).await.unwrap();
    assert!(matches!(outcome, EmergencyAccessOutcome::UnknownTarget));
}
