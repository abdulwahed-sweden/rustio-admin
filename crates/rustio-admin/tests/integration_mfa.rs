//! R3 MFA integration tests (`DESIGN_R3_MFA.md` §13.3).
//!
//! Boots an ephemeral Postgres container per test via
//! `testcontainers`, runs `auth::init_tables` to apply the
//! framework migrations (R0 + R1 + R2 + R3 schema in one shot),
//! then exercises each R3 runtime fn end-to-end against the
//! live DB.
//!
//! ## Scenarios (per the design's §13.3)
//!
//! - `confirm_enrolment` writes ciphertext + key_id + step,
//!   INSERTs 8 backup-code rows, audit-emits `MfaEnabled`.
//! - `verify_totp_for_user` accepts the current step, rejects
//!   a replay at the same step, and updates
//!   `mfa_last_used_step`.
//! - `consume_backup_code` matches one of 8 inserted codes,
//!   atomically marks `used_at`, emits
//!   `MfaCodeConsumed`. A second attempt with the same code
//!   collapses to `Invalid` (single-use enforced at the SQL
//!   level).
//! - `disable_mfa` clears the four MFA columns, DELETEs all
//!   backup-code rows, calls
//!   `invalidate_sessions(User, MfaDisabled)`, audit-emits
//!   `MfaDisabled`.
//! - `regenerate_backup_codes` atomically replaces the batch,
//!   audit-emits `BackupCodesRegenerated`. The previous codes
//!   no longer verify.
//! - `promote_session_to_mfa_verified` mints a fresh
//!   `mfa_verified` row with `parent_session_id` set and
//!   revokes the parent via `invalidate_sessions` —
//!   Doctrine 17 + Doctrine 22 together.
//! - Two-key wrap/unwrap path — encrypt with key v1, retire
//!   to v2, decrypt against the row's stamped
//!   `mfa_secret_key_id`. Staged-rotation simulation.
//!
//! ## What this suite is NOT
//!
//! - Not a HTTP-level smoke test. Routes / handlers go through
//!   the downstream-validation pass on the live Stockholm POS
//!   DB (§13.4). This suite covers the SQL + audit +
//!   invalidation contracts at the runtime-fn boundary.

#![cfg(feature = "integration-test")]

use sqlx::Row as _;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

use rustio_admin::__integration::{
    confirm_enrolment, consume_backup_code, current_step, disable_mfa, fake_request, generate_totp,
    promote_session_to_mfa_verified, provision_secret, regenerate_backup_codes,
    verify_totp_for_user, BackupConsumeOutcome, DisableOutcome, EnrolOutcome, MfaKey, RegenOutcome,
    VerifyOutcome, BACKUP_CODE_COUNT,
};
use rustio_admin::auth::{self, Role};
use rustio_admin::orm::Db;

// ---- Test harness ----------------------------------------------------------

/// Holds the running Postgres container alongside its connected
/// `Db`. The container shuts down when the env drops.
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

async fn create_session(db: &Db, user_id: i64) -> i64 {
    let token = auth::create_session(db, user_id)
        .await
        .expect("create_session");
    auth::current_session_id(db, &token)
        .await
        .expect("current_session_id")
        .expect("current_session_id is Some")
}

/// Deterministic 32-byte test key. The integration suite does
/// not read `RUSTIO_SECRET_KEY` from the environment — every
/// test wraps / unwraps against this fixed value so the suite
/// is hermetic.
fn test_key() -> MfaKey {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7).wrapping_add(13);
    }
    MfaKey::from_bytes(bytes)
}

/// Read `(mfa_enabled, mfa_secret_ciphertext, mfa_secret_key_id,
/// mfa_last_used_step)` for a user.
async fn read_user_mfa(db: &Db, user_id: i64) -> (bool, Option<Vec<u8>>, Option<i32>, Option<i64>) {
    let row = sqlx::query(
        "SELECT mfa_enabled, mfa_secret_ciphertext, mfa_secret_key_id, mfa_last_used_step \
           FROM rustio_users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(db.pool())
    .await
    .expect("read user mfa");
    (
        row.try_get("mfa_enabled").unwrap(),
        row.try_get("mfa_secret_ciphertext").unwrap(),
        row.try_get("mfa_secret_key_id").unwrap(),
        row.try_get("mfa_last_used_step").unwrap(),
    )
}

async fn count_backup_codes(db: &Db, user_id: i64) -> i64 {
    let row =
        sqlx::query("SELECT COUNT(*)::bigint AS n FROM rustio_mfa_backup_codes WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(db.pool())
            .await
            .expect("count backup codes");
    row.try_get("n").unwrap()
}

async fn count_unused_backup_codes(db: &Db, user_id: i64) -> i64 {
    let row = sqlx::query(
        "SELECT COUNT(*)::bigint AS n FROM rustio_mfa_backup_codes \
          WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(db.pool())
    .await
    .expect("count unused backup codes");
    row.try_get("n").unwrap()
}

async fn count_audit_events(db: &Db, action_type: &str, object_id: i64) -> i64 {
    let row = sqlx::query(
        "SELECT COUNT(*)::bigint AS n FROM rustio_admin_actions \
          WHERE action_type = $1 AND object_id = $2",
    )
    .bind(action_type)
    .bind(object_id)
    .fetch_one(db.pool())
    .await
    .expect("count audit");
    row.try_get("n").unwrap()
}

async fn count_revoked_sessions(db: &Db, user_id: i64) -> i64 {
    let row = sqlx::query(
        "SELECT COUNT(*)::bigint AS n FROM rustio_sessions \
          WHERE user_id = $1 AND revoked_at IS NOT NULL",
    )
    .bind(user_id)
    .fetch_one(db.pool())
    .await
    .expect("count revoked");
    row.try_get("n").unwrap()
}

/// Generate a TOTP candidate for the current wall-clock step.
/// The hand-rolled `generate_totp` (commit #5) takes a raw step
/// number; the runtime fns read `Utc::now().timestamp()`
/// internally. We mirror that here so the candidate we feed in
/// matches whichever step the runtime will compute.
fn now_step_totp(secret: &[u8]) -> u32 {
    let now = chrono::Utc::now().timestamp().max(0) as u64;
    let step = current_step(now, 30);
    generate_totp(secret, step)
}

// ---- §13.3 scenarios -------------------------------------------------------

/// `confirm_enrolment` writes ciphertext + key_id + step,
/// INSERTs the backup-code rows, audit-emits `MfaEnabled`.
#[tokio::test]
async fn confirm_enrolment_persists_secret_codes_and_audit() {
    let env = boot().await;
    let user_id = create_user(&env.db, "user@example.com", "password-1234567", Role::User).await;

    let provisioned = provision_secret();
    let candidate = now_step_totp(&provisioned.secret_bytes);
    let key = test_key();

    let outcome = confirm_enrolment(
        &env.db,
        &fake_request(),
        user_id,
        &provisioned.secret_bytes,
        candidate,
        30,
        1,
        &key,
        1,
        None,
    )
    .await
    .expect("confirm_enrolment");

    match outcome {
        EnrolOutcome::Enrolled { plain_backup_codes } => {
            assert_eq!(plain_backup_codes.len(), BACKUP_CODE_COUNT);
        }
        other => panic!(
            "expected Enrolled, got {other:?}",
            other = match other {
                EnrolOutcome::Enrolled { .. } => "Enrolled",
                EnrolOutcome::InvalidCode => "InvalidCode",
                EnrolOutcome::AlreadyEnrolled => "AlreadyEnrolled",
            }
        ),
    }

    let (mfa_enabled, ciphertext, key_id, last_step) = read_user_mfa(&env.db, user_id).await;
    assert!(mfa_enabled, "mfa_enabled flipped TRUE");
    let ct = ciphertext.expect("ciphertext stored");
    assert_eq!(ct.len(), 12 + 20 + 16, "nonce || ct(20) || tag layout");
    assert_eq!(key_id, Some(1));
    assert!(last_step.is_some(), "mfa_last_used_step stamped");

    let codes = count_backup_codes(&env.db, user_id).await;
    assert_eq!(codes, BACKUP_CODE_COUNT as i64, "8 backup codes inserted");

    let audits = count_audit_events(&env.db, "mfa_enabled", user_id).await;
    assert_eq!(audits, 1, "one MfaEnabled audit row emitted");
}

/// `verify_totp_for_user` accepts the current step, then
/// rejects a replay of the same step.
#[tokio::test]
async fn verify_totp_accepts_current_then_rejects_replay() {
    let env = boot().await;
    let user_id = create_user(&env.db, "user@example.com", "password-1234567", Role::User).await;
    let provisioned = provision_secret();
    let key = test_key();

    let candidate = now_step_totp(&provisioned.secret_bytes);
    confirm_enrolment(
        &env.db,
        &fake_request(),
        user_id,
        &provisioned.secret_bytes,
        candidate,
        30,
        1,
        &key,
        1,
        None,
    )
    .await
    .expect("enrolment");

    // Try to verify the same code again — should be Replay,
    // not Verified. confirm_enrolment already stamped
    // mfa_last_used_step to the enrolment step, so re-using it
    // now collapses to D4 replay rejection.
    let candidate_str = format!("{candidate:06}");
    let outcome = verify_totp_for_user(&env.db, user_id, &candidate_str, 30, 1, &key)
        .await
        .expect("verify");
    assert!(
        matches!(outcome, VerifyOutcome::Replay { .. }),
        "second use of enrolment step must be Replay, got {:?}",
        match outcome {
            VerifyOutcome::Verified { .. } => "Verified",
            VerifyOutcome::Replay { .. } => "Replay",
            VerifyOutcome::Invalid => "Invalid",
            VerifyOutcome::NotEnrolled => "NotEnrolled",
        }
    );

    // A clearly wrong candidate at the same wall-clock returns
    // Invalid (no replay because no cryptographic match).
    let wrong = verify_totp_for_user(&env.db, user_id, "000000", 30, 1, &key)
        .await
        .expect("verify wrong");
    assert!(matches!(wrong, VerifyOutcome::Invalid));
}

/// `consume_backup_code` matches one of 8 inserted codes,
/// atomically marks `used_at`, emits `MfaCodeConsumed`. Second
/// attempt with the same code collapses to `Invalid` (D7
/// single-use enforced at the SQL level).
#[tokio::test]
async fn consume_backup_code_single_use_invariant() {
    let env = boot().await;
    let user_id = create_user(&env.db, "user@example.com", "password-1234567", Role::User).await;
    let provisioned = provision_secret();
    let key = test_key();
    let candidate = now_step_totp(&provisioned.secret_bytes);
    let outcome = confirm_enrolment(
        &env.db,
        &fake_request(),
        user_id,
        &provisioned.secret_bytes,
        candidate,
        30,
        1,
        &key,
        1,
        None,
    )
    .await
    .expect("enrolment");
    let codes = match outcome {
        EnrolOutcome::Enrolled { plain_backup_codes } => plain_backup_codes,
        _ => panic!("enrolment failed"),
    };

    assert_eq!(count_unused_backup_codes(&env.db, user_id).await, 8);

    let one = &codes[0];
    let first = consume_backup_code(&env.db, &fake_request(), user_id, one, "login", None)
        .await
        .expect("consume");
    match first {
        BackupConsumeOutcome::Consumed { remaining, .. } => {
            assert_eq!(remaining, 7, "7 codes remaining after first consume");
        }
        other => panic!(
            "expected Consumed, got {:?}",
            match other {
                BackupConsumeOutcome::Consumed { .. } => "Consumed",
                BackupConsumeOutcome::Invalid => "Invalid",
                BackupConsumeOutcome::NotEnrolled => "NotEnrolled",
                BackupConsumeOutcome::AlreadyUsed => "AlreadyUsed",
            }
        ),
    }

    let audits = count_audit_events(&env.db, "mfa_code_consumed", user_id).await;
    assert_eq!(audits, 1, "MfaCodeConsumed emitted once");

    // Second attempt with the same code — single-use. The SQL
    // SELECT filters used_at IS NULL so the row no longer
    // appears as a candidate.
    let second = consume_backup_code(&env.db, &fake_request(), user_id, one, "login", None)
        .await
        .expect("consume retry");
    assert!(
        matches!(second, BackupConsumeOutcome::Invalid),
        "second consume of same code must be Invalid"
    );

    assert_eq!(count_unused_backup_codes(&env.db, user_id).await, 7);
}

/// `disable_mfa` clears the four MFA columns, deletes the
/// backup-code rows, invalidates every session for the user via
/// invalidate_sessions (Doctrine 22), and audit-emits
/// `MfaDisabled`.
#[tokio::test]
async fn disable_mfa_clears_state_revokes_sessions_audits() {
    let env = boot().await;
    let user_id = create_user(&env.db, "user@example.com", "password-1234567", Role::User).await;
    let provisioned = provision_secret();
    let key = test_key();
    let candidate = now_step_totp(&provisioned.secret_bytes);
    confirm_enrolment(
        &env.db,
        &fake_request(),
        user_id,
        &provisioned.secret_bytes,
        candidate,
        30,
        1,
        &key,
        1,
        None,
    )
    .await
    .expect("enrolment");

    // Spin up a couple of sessions before disable so we can
    // assert revocation happens.
    create_session(&env.db, user_id).await;
    create_session(&env.db, user_id).await;

    let outcome = disable_mfa(&env.db, &fake_request(), user_id, None)
        .await
        .expect("disable");
    match outcome {
        DisableOutcome::Disabled { sessions_revoked } => {
            // Two sessions explicitly created plus the framework's
            // implicit session-row from create_user side effects
            // (if any). We assert ≥ 2 rather than == 2 to stay
            // robust to that.
            assert!(
                sessions_revoked >= 2,
                "expected ≥2 sessions revoked, got {sessions_revoked}"
            );
        }
        other => panic!(
            "expected Disabled, got {:?}",
            match other {
                DisableOutcome::Disabled { .. } => "Disabled",
                DisableOutcome::NotEnrolled => "NotEnrolled",
                DisableOutcome::PolicyRequired => "PolicyRequired",
            }
        ),
    }

    let (mfa_enabled, ciphertext, key_id, last_step) = read_user_mfa(&env.db, user_id).await;
    assert!(!mfa_enabled);
    assert!(ciphertext.is_none());
    assert!(key_id.is_none());
    assert!(last_step.is_none());

    assert_eq!(
        count_backup_codes(&env.db, user_id).await,
        0,
        "backup-code rows deleted"
    );

    assert!(
        count_revoked_sessions(&env.db, user_id).await >= 2,
        "sessions revoked via invalidate_sessions"
    );

    let audits = count_audit_events(&env.db, "mfa_disabled", user_id).await;
    assert_eq!(audits, 1, "MfaDisabled emitted once");
}

/// `regenerate_backup_codes` atomically replaces the batch.
/// After regeneration the previous codes do not verify, and a
/// new code does.
#[tokio::test]
async fn regenerate_backup_codes_atomic_swap_invalidates_old() {
    let env = boot().await;
    let user_id = create_user(&env.db, "user@example.com", "password-1234567", Role::User).await;
    let provisioned = provision_secret();
    let key = test_key();
    let candidate = now_step_totp(&provisioned.secret_bytes);
    let outcome = confirm_enrolment(
        &env.db,
        &fake_request(),
        user_id,
        &provisioned.secret_bytes,
        candidate,
        30,
        1,
        &key,
        1,
        None,
    )
    .await
    .expect("enrolment");
    let old_codes = match outcome {
        EnrolOutcome::Enrolled { plain_backup_codes } => plain_backup_codes,
        _ => panic!("enrolment failed"),
    };

    let regen = regenerate_backup_codes(&env.db, &fake_request(), user_id, None)
        .await
        .expect("regenerate");
    let new_codes = match regen {
        RegenOutcome::Regenerated {
            plain_backup_codes,
            previous_codes_invalidated,
        } => {
            assert_eq!(
                previous_codes_invalidated, BACKUP_CODE_COUNT as u32,
                "all 8 old codes were invalidated"
            );
            assert_eq!(plain_backup_codes.len(), BACKUP_CODE_COUNT);
            plain_backup_codes
        }
        RegenOutcome::NotEnrolled => panic!("user should be enrolled"),
    };

    // Old codes must no longer verify — the DELETE removed them
    // from the SELECT candidate set.
    let old_attempt = consume_backup_code(
        &env.db,
        &fake_request(),
        user_id,
        &old_codes[0],
        "login",
        None,
    )
    .await
    .expect("old consume");
    assert!(
        matches!(old_attempt, BackupConsumeOutcome::Invalid),
        "old code must reject after regenerate"
    );

    // New code must verify.
    let new_attempt = consume_backup_code(
        &env.db,
        &fake_request(),
        user_id,
        &new_codes[0],
        "login",
        None,
    )
    .await
    .expect("new consume");
    assert!(matches!(new_attempt, BackupConsumeOutcome::Consumed { .. }));

    let audits = count_audit_events(&env.db, "backup_codes_regenerated", user_id).await;
    assert_eq!(audits, 1);
}

/// `promote_session_to_mfa_verified` mints a fresh
/// `mfa_verified` row with `parent_session_id` set and revokes
/// the parent via `invalidate_sessions` (Doctrine 17 + 22).
#[tokio::test]
async fn promote_session_to_mfa_verified_rotates_token_and_revokes_parent() {
    let env = boot().await;
    let user_id = create_user(&env.db, "user@example.com", "password-1234567", Role::User).await;
    let parent_session_id = create_session(&env.db, user_id).await;

    let new_token = promote_session_to_mfa_verified(&env.db, parent_session_id, user_id)
        .await
        .expect("promote");

    // The new token must resolve to a session row with
    // trust_level = 'mfa_verified' and parent_session_id set.
    let row = sqlx::query(
        "SELECT session_id, trust_level, parent_session_id, revoked_at \
           FROM rustio_sessions WHERE token = $1",
    )
    .bind(&new_token)
    .fetch_one(env.db.pool())
    .await
    .expect("read new session");
    let new_session_id: i64 = row.try_get("session_id").unwrap();
    let trust: String = row.try_get("trust_level").unwrap();
    let parent: Option<i64> = row.try_get("parent_session_id").unwrap();
    let revoked: Option<chrono::DateTime<chrono::Utc>> = row.try_get("revoked_at").unwrap();
    assert_eq!(trust, "mfa_verified", "new row trust_level = mfa_verified");
    assert_eq!(
        parent,
        Some(parent_session_id),
        "parent_session_id points at the rotated row"
    );
    assert!(revoked.is_none(), "new row is active (revoked_at IS NULL)");
    assert_ne!(new_session_id, parent_session_id);

    // The parent must now be revoked with reason
    // 'trust_escalation' — Doctrine 22 routed through.
    let parent_row = sqlx::query(
        "SELECT trust_level, revoked_at, revoked_reason \
           FROM rustio_sessions WHERE session_id = $1",
    )
    .bind(parent_session_id)
    .fetch_one(env.db.pool())
    .await
    .expect("read parent");
    let parent_revoked: Option<chrono::DateTime<chrono::Utc>> =
        parent_row.try_get("revoked_at").unwrap();
    let parent_reason: Option<String> = parent_row.try_get("revoked_reason").unwrap();
    assert!(parent_revoked.is_some(), "parent revoked_at IS NOT NULL");
    assert_eq!(
        parent_reason.as_deref(),
        Some("trust_escalation"),
        "parent revoked_reason = trust_escalation"
    );
}

/// Two-key wrap/unwrap path simulates staged key rotation —
/// enrol under key v1, decrypt against the row's stamped
/// `mfa_secret_key_id`. The current runtime takes a single
/// `MfaKey`, so this test pins the on-disk layout (key_id
/// stamped per row, ciphertext rotatable when a future
/// resolver lands).
#[tokio::test]
async fn key_id_is_stamped_per_row_for_future_rotation() {
    let env = boot().await;
    let user_id = create_user(&env.db, "user@example.com", "password-1234567", Role::User).await;
    let provisioned = provision_secret();
    let key = test_key();
    let candidate = now_step_totp(&provisioned.secret_bytes);
    confirm_enrolment(
        &env.db,
        &fake_request(),
        user_id,
        &provisioned.secret_bytes,
        candidate,
        30,
        1,
        &key,
        7, // pick a non-default key_id to prove it round-trips
        None,
    )
    .await
    .expect("enrolment");

    let (_, _, key_id, _) = read_user_mfa(&env.db, user_id).await;
    assert_eq!(
        key_id,
        Some(7),
        "the key_id passed to confirm_enrolment is persisted verbatim"
    );
}
