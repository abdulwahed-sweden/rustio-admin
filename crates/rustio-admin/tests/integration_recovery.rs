//! R2 organisational-recovery integration tests
//! (`DESIGN_R2_ORGANISATIONAL.md` §10.3).
//!
//! Boots an ephemeral Postgres container per test via
//! `testcontainers`, runs `auth::init_tables` to apply the
//! framework migrations (R0 + R1 + R2 schema in one shot), then
//! exercises each R2 runtime fn end-to-end against the live DB.
//!
//! ## Why one container per test
//!
//! Container startup is ~3–5 s on a warm Docker daemon; running
//! seven tests sequentially is ~30–60 s of suite time. The
//! alternative — share one container across tests with manual
//! schema cleanup — buys a few seconds but introduces test-order
//! coupling and forces every test to be careful about leftover
//! state. The simple per-test container is the right trade-off
//! for a CI-only suite.
//!
//! ## What this suite is NOT
//!
//! - Not a HTTP-level smoke test. Routes / handlers go through the
//!   downstream-validation pass on the live Stockholm POS DB
//!   (§10.4). This suite covers the SQL + audit + invalidation
//!   contracts at the runtime-fn boundary.
//! - Not a property-based fuzz suite. Each test asserts a single
//!   §3 / §5.1 invariant against a single happy-path setup.

#![cfg(feature = "integration-test")]

use chrono::Duration as ChronoDuration;
use sqlx::Row as _;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

use rustio_admin::__integration::{
    admin_revoke_sessions, admin_set_temp_password, check_account_lockout, check_session_elevated,
    fake_request, lock_user_account, promote_session_elevated, record_failed_login,
    record_successful_login, unlock_user_account, AdminActor, AdminRevokeOutcome,
    AdminTempPwOutcome, LockDuration, LockOutcome, LockState, ThrottleOutcome, UnlockOutcome,
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

/// Issue a session and return the resolved `session_id`. Used by
/// every test that needs a target session to exist for revocation /
/// elevation assertions.
async fn create_session(db: &Db, user_id: i64) -> i64 {
    let token = auth::create_session(db, user_id)
        .await
        .expect("create_session");
    auth::current_session_id(db, &token)
        .await
        .expect("current_session_id")
        .expect("current_session_id is Some")
}

/// Count rows in `rustio_admin_actions` for a given typed event.
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
    row.try_get("n").expect("count column")
}

/// Read `(failed_login_count, last_failed_login_at, locked_until,
/// must_change_password)` for a user.
async fn read_user_lockout(
    db: &Db,
    user_id: i64,
) -> (
    i32,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
    bool,
) {
    let row = sqlx::query(
        "SELECT failed_login_count, last_failed_login_at, locked_until, must_change_password \
           FROM rustio_users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(db.pool())
    .await
    .expect("read user lockout");
    (
        row.try_get("failed_login_count").unwrap(),
        row.try_get("last_failed_login_at").unwrap(),
        row.try_get("locked_until").unwrap(),
        row.try_get("must_change_password").unwrap(),
    )
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

// ---- §10.3 scenarios -------------------------------------------------------

/// `admin_set_temp_password` writes the hash, sets
/// `must_change_password = TRUE`, revokes every session, and
/// emits the typed audit row.
#[tokio::test]
async fn admin_set_temp_password_writes_hash_flag_revokes_sessions() {
    let env = boot().await;
    let actor_id = create_user(
        &env.db,
        "admin@example.com",
        "admin-pw-1234567",
        Role::Administrator,
    )
    .await;
    let target_id = create_user(&env.db, "target@example.com", "old-pw-1234567", Role::Staff).await;
    create_session(&env.db, target_id).await;

    let actor = AdminActor {
        user_id: actor_id,
        email: "admin@example.com",
    };
    let req = fake_request();
    let outcome =
        admin_set_temp_password(&env.db, &req, target_id, actor, "support ticket #42", None)
            .await
            .expect("admin_set_temp_password");

    let temp_pw = match outcome {
        AdminTempPwOutcome::Set {
            temp_password,
            revoked_session_count,
            ..
        } => {
            assert_eq!(revoked_session_count, 1);
            temp_password
        }
        other => panic!("unexpected outcome: {other:?}"),
    };
    assert_eq!(temp_pw.len(), 16, "temp password is 16 chars per §12 lock");

    // The temp password is now the active password (Argon2 verifies).
    let user = auth::find_user_by_email(&env.db, "target@example.com")
        .await
        .unwrap()
        .unwrap();
    assert!(auth::verify_password(&temp_pw, &user.password_hash));

    // must_change_password is set.
    assert!(user.must_change_password);

    // Sessions revoked (1 was created).
    assert_eq!(count_revoked_sessions(&env.db, target_id).await, 1);

    // Audit emitted: 1 PasswordResetByOther + 1 SessionsRevokedByOther.
    assert_eq!(
        count_audit_events(&env.db, "password_reset_by_other", target_id).await,
        1
    );
    assert_eq!(
        count_audit_events(&env.db, "sessions_revoked_by_other", target_id).await,
        1
    );
}

/// `lock_user_account` writes `locked_until`, revokes sessions,
/// emits `AccountLocked`.
#[tokio::test]
async fn lock_user_account_writes_locked_until_revokes_sessions() {
    let env = boot().await;
    let actor_id = create_user(
        &env.db,
        "admin@example.com",
        "admin-pw-1234567",
        Role::Administrator,
    )
    .await;
    let target_id = create_user(
        &env.db,
        "target@example.com",
        "target-pw-1234567",
        Role::Staff,
    )
    .await;
    create_session(&env.db, target_id).await;
    create_session(&env.db, target_id).await; // two sessions

    let actor = AdminActor {
        user_id: actor_id,
        email: "admin@example.com",
    };
    let req = fake_request();
    let outcome = lock_user_account(
        &env.db,
        &req,
        target_id,
        actor,
        LockDuration::OneHour,
        "suspected credential leak",
        None,
    )
    .await
    .expect("lock_user_account");

    match outcome {
        LockOutcome::Locked {
            revoked_session_count,
            ..
        } => {
            assert_eq!(revoked_session_count, 2);
        }
        other => panic!("unexpected outcome: {other:?}"),
    }

    let (_, _, locked_until, _) = read_user_lockout(&env.db, target_id).await;
    let until = locked_until.expect("locked_until is set");
    let now = chrono::Utc::now();
    assert!(until > now);
    assert!(until <= now + ChronoDuration::hours(1) + ChronoDuration::minutes(1));

    assert_eq!(count_revoked_sessions(&env.db, target_id).await, 2);
    assert_eq!(
        count_audit_events(&env.db, "account_locked", target_id).await,
        1
    );
    assert_eq!(
        count_audit_events(&env.db, "sessions_revoked_by_other", target_id).await,
        2
    );
}

/// `unlock_user_account` clears `locked_until`, zeroes
/// `failed_login_count`, emits `AccountUnlocked`. Does NOT touch
/// sessions.
#[tokio::test]
async fn unlock_user_account_clears_lock_zeroes_counter() {
    let env = boot().await;
    let actor_id = create_user(
        &env.db,
        "admin@example.com",
        "admin-pw-1234567",
        Role::Administrator,
    )
    .await;
    let target_id = create_user(
        &env.db,
        "target@example.com",
        "target-pw-1234567",
        Role::Staff,
    )
    .await;

    // Pre-state: lock the user manually (write the column) and bump
    // the counter, simulating prior auto-throttle activity.
    sqlx::query(
        "UPDATE rustio_users SET locked_until = NOW() + INTERVAL '1 hour', \
            failed_login_count = 3, last_failed_login_at = NOW() WHERE id = $1",
    )
    .bind(target_id)
    .execute(env.db.pool())
    .await
    .unwrap();

    let actor = AdminActor {
        user_id: actor_id,
        email: "admin@example.com",
    };
    let req = fake_request();
    let outcome = unlock_user_account(
        &env.db,
        &req,
        target_id,
        actor,
        "false alarm — verified",
        None,
    )
    .await
    .expect("unlock_user_account");

    assert!(matches!(outcome, UnlockOutcome::Unlocked { .. }));

    let (counter, _last_fail, locked_until, _flag) = read_user_lockout(&env.db, target_id).await;
    assert_eq!(counter, 0);
    assert!(locked_until.is_none());

    assert_eq!(
        count_audit_events(&env.db, "account_unlocked", target_id).await,
        1
    );
    // Doctrine: unlock does NOT touch sessions.
    assert_eq!(count_revoked_sessions(&env.db, target_id).await, 0);
}

/// `admin_revoke_sessions` revokes every session; emits
/// per-session `SessionsRevokedByOther`. Does NOT write
/// `locked_until` and does NOT emit a headline event.
#[tokio::test]
async fn admin_revoke_sessions_revokes_without_locking() {
    let env = boot().await;
    let actor_id = create_user(
        &env.db,
        "admin@example.com",
        "admin-pw-1234567",
        Role::Administrator,
    )
    .await;
    let target_id = create_user(
        &env.db,
        "target@example.com",
        "target-pw-1234567",
        Role::Staff,
    )
    .await;
    create_session(&env.db, target_id).await;
    create_session(&env.db, target_id).await;
    create_session(&env.db, target_id).await; // three sessions

    let actor = AdminActor {
        user_id: actor_id,
        email: "admin@example.com",
    };
    let req = fake_request();
    let outcome = admin_revoke_sessions(&env.db, &req, target_id, actor, "incident response", None)
        .await
        .expect("admin_revoke_sessions");

    match outcome {
        AdminRevokeOutcome::Revoked {
            revoked_session_count,
            ..
        } => {
            assert_eq!(revoked_session_count, 3);
        }
        other => panic!("unexpected outcome: {other:?}"),
    }

    let (_, _, locked_until, _) = read_user_lockout(&env.db, target_id).await;
    assert!(locked_until.is_none(), "no lock written");
    assert_eq!(count_revoked_sessions(&env.db, target_id).await, 3);
    assert_eq!(
        count_audit_events(&env.db, "sessions_revoked_by_other", target_id).await,
        3
    );
    // Doctrine: no headline event for admin-revoke-sessions (only lock has one).
    assert_eq!(
        count_audit_events(&env.db, "account_locked", target_id).await,
        0
    );
}

/// `record_failed_login` increments and trips the threshold;
/// `check_account_lockout` honours `locked_until`.
#[tokio::test]
async fn record_failed_login_auto_throttles_at_threshold() {
    let env = boot().await;
    let target_id = create_user(
        &env.db,
        "target@example.com",
        "target-pw-1234567",
        Role::Staff,
    )
    .await;

    let throttle = rustio_admin::auth::LoginThrottle::DEFAULT; // 5 / 10min / 15min

    // First 4 failures: incremented but not locked.
    for expected in 1..=4 {
        let outcome = record_failed_login(&env.db, target_id, throttle)
            .await
            .unwrap();
        match outcome {
            ThrottleOutcome::Recorded { count } => assert_eq!(count, expected),
            other => panic!("attempt {expected}: unexpected outcome {other:?}"),
        }
        assert!(matches!(
            check_account_lockout(&env.db, target_id).await.unwrap(),
            LockState::Unlocked
        ));
    }

    // 5th failure trips the lock.
    let outcome = record_failed_login(&env.db, target_id, throttle)
        .await
        .unwrap();
    let until = match outcome {
        ThrottleOutcome::JustLocked { count, until } => {
            assert_eq!(count, 5);
            until
        }
        other => panic!("expected JustLocked, got {other:?}"),
    };

    // check_account_lockout reflects it.
    match check_account_lockout(&env.db, target_id).await.unwrap() {
        LockState::Locked { until: read_until } => {
            // Allow 1s drift between the two NOW() reads.
            let drift = (until - read_until).num_seconds().abs();
            assert!(drift <= 1, "drift {drift}s between writes");
        }
        LockState::Unlocked => panic!("expected locked"),
    }
}

/// `record_successful_login` zeroes the counter (locked_until
/// preserved on success path — the row is only reached when
/// `check_account_lockout` returned Unlocked, so the column is
/// already NULL or expired).
#[tokio::test]
async fn record_successful_login_zeroes_counter() {
    let env = boot().await;
    let target_id = create_user(
        &env.db,
        "target@example.com",
        "target-pw-1234567",
        Role::Staff,
    )
    .await;
    let throttle = rustio_admin::auth::LoginThrottle::DEFAULT;

    // 3 failures.
    for _ in 0..3 {
        record_failed_login(&env.db, target_id, throttle)
            .await
            .unwrap();
    }
    assert_eq!(read_user_lockout(&env.db, target_id).await.0, 3);

    // Success clears it.
    record_successful_login(&env.db, target_id).await.unwrap();
    let (counter, last_fail, _, _) = read_user_lockout(&env.db, target_id).await;
    assert_eq!(counter, 0);
    assert!(last_fail.is_none());
}

/// `promote_session_elevated` + `check_session_elevated`
/// round-trip. Initial state false; after promote true; after
/// the elevation window elapses it would flip back (we test the
/// "now" assertion, not waiting 15 min).
#[tokio::test]
async fn promote_then_check_session_elevated_round_trip() {
    let env = boot().await;
    let target_id = create_user(
        &env.db,
        "target@example.com",
        "target-pw-1234567",
        Role::Staff,
    )
    .await;
    let session_id = create_session(&env.db, target_id).await;

    // Fresh session: not elevated.
    assert!(!check_session_elevated(&env.db, session_id).await.unwrap());

    // Promote for 15 minutes.
    promote_session_elevated(&env.db, session_id, ChronoDuration::minutes(15))
        .await
        .unwrap();
    assert!(check_session_elevated(&env.db, session_id).await.unwrap());

    // Promote with a past TTL → check_session_elevated now returns
    // false. Documented escape hatch for "always require fresh
    // re-auth" via reauth_window = 0.
    promote_session_elevated(&env.db, session_id, ChronoDuration::seconds(-1))
        .await
        .unwrap();
    assert!(!check_session_elevated(&env.db, session_id).await.unwrap());
}
