//! Nullable non-`bigint` columns must not make a row uneditable.
//!
//! `orm::create` and `orm::update` build their SQL from `Model::INSERT_COLUMNS`
//! and bind `insert_values()` positionally. A `Value::Null` carries no type —
//! the blanket `From<Option<T>>` erases it — so binding it as a typed parameter
//! means guessing. The guess used to be `None::<i64>`, and Postgres rejected
//! every statement whose target column was not a `bigint`:
//!
//! ```text
//! column "seen_at" is of type timestamp with time zone
//! but expression is of type bigint
//! ```
//!
//! The consequence was not a corner case. Any row that had never had its
//! optional timestamp set — which is to say the ones an operator most often
//! needs to act on — could not be edited at all, and the admin returned 500.
//!
//! These tests pin the behaviour against a live Postgres: a row holding NULL in
//! a `TIMESTAMPTZ` and a `TEXT` column can be created, read, and updated, the
//! NULLs stay NULL, and the columns that were not touched keep their values.

#![cfg(feature = "integration-test")]

use rustio_admin::orm::{self, Db};
use rustio_admin::{Result, RustioAdmin};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

/// Holds the running Postgres container alongside its connected `Db`. The
/// container shuts down when the env drops.
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

    // One table carrying both shapes the old binding broke on: a nullable
    // timestamp and a nullable text column, alongside a nullable bigint that
    // always worked and is here to prove the fix did not regress it.
    sqlx::query(sqlx::AssertSqlSafe(
        "CREATE TABLE widgets (
             id         BIGSERIAL PRIMARY KEY,
             label      TEXT        NOT NULL,
             seen_at    TIMESTAMPTZ,
             note       TEXT,
             owner_id   BIGINT
         )"
        .to_string(),
    ))
    .execute(db.pool())
    .await
    .expect("create table");

    TestEnv {
        db,
        _container: container,
    }
}

/// A row with one nullable column of each shape the binding has to get right.
#[derive(RustioAdmin)]
struct Widget {
    id: i64,
    label: String,
    seen_at: Option<chrono::DateTime<chrono::Utc>>,
    note: Option<String>,
    owner_id: Option<i64>,
}

impl rustio_admin::admin::ModelAdmin for Widget {}

async fn load(db: &Db, id: i64) -> Result<Option<Widget>> {
    orm::find::<Widget>(db, id).await
}

/// The defect, end to end: a row whose optional timestamp and text are NULL is
/// created, then edited, and the edit lands.
#[tokio::test]
async fn a_row_with_null_timestamps_can_be_updated() {
    let env = boot().await;

    let widget = Widget {
        id: 0,
        label: "before".to_string(),
        seen_at: None,
        note: None,
        owner_id: None,
    };
    let id = orm::create(&env.db, &widget)
        .await
        .expect("create with NULLs");

    let stored = load(&env.db, id).await.expect("get").expect("row exists");
    assert!(
        stored.seen_at.is_none(),
        "the timestamp was written as NULL"
    );
    assert!(stored.note.is_none(), "the text was written as NULL");

    let edited = Widget {
        id,
        label: "after".to_string(),
        seen_at: None,
        note: None,
        owner_id: None,
    };
    orm::update(&env.db, id, &edited)
        .await
        .expect("a NULL timestamp must not make a row uneditable");

    let after = load(&env.db, id).await.expect("get").expect("row exists");
    assert_eq!(after.label, "after", "the edit landed");
    assert!(after.seen_at.is_none(), "NULL stayed NULL");
    assert!(after.note.is_none(), "NULL stayed NULL");
    assert!(after.owner_id.is_none());
}

/// The other half: values that are present must still travel as bound
/// parameters, and an update that changes one column must leave the rest alone.
#[tokio::test]
async fn present_values_survive_an_update_that_only_changes_one_column() {
    let env = boot().await;

    let when = chrono::Utc::now();
    let widget = Widget {
        id: 0,
        label: "kept".to_string(),
        seen_at: Some(when),
        note: Some("kept note".to_string()),
        owner_id: Some(7),
    };
    let id = orm::create(&env.db, &widget).await.expect("create");

    let edited = Widget {
        id,
        label: "changed".to_string(),
        seen_at: Some(when),
        note: Some("kept note".to_string()),
        owner_id: Some(7),
    };
    orm::update(&env.db, id, &edited).await.expect("update");

    let after = load(&env.db, id).await.expect("get").expect("row exists");
    assert_eq!(after.label, "changed");
    assert_eq!(after.note.as_deref(), Some("kept note"));
    assert_eq!(after.owner_id, Some(7));
    assert_eq!(
        after.seen_at.map(|t| t.timestamp_millis()),
        Some(when.timestamp_millis()),
        "an untouched timestamp must come back unchanged"
    );
}

/// A row may go from holding values to holding NULLs and back. Clearing a
/// column is an ordinary edit, and it used to be the one that failed.
#[tokio::test]
async fn a_column_can_be_cleared_to_null_and_set_again() {
    let env = boot().await;

    let when = chrono::Utc::now();
    let id = orm::create(
        &env.db,
        &Widget {
            id: 0,
            label: "w".to_string(),
            seen_at: Some(when),
            note: Some("first".to_string()),
            owner_id: Some(1),
        },
    )
    .await
    .expect("create");

    orm::update(
        &env.db,
        id,
        &Widget {
            id,
            label: "w".to_string(),
            seen_at: None,
            note: None,
            owner_id: None,
        },
    )
    .await
    .expect("clearing every nullable column must be allowed");

    let cleared = load(&env.db, id).await.expect("get").expect("row");
    assert!(cleared.seen_at.is_none() && cleared.note.is_none() && cleared.owner_id.is_none());

    orm::update(
        &env.db,
        id,
        &Widget {
            id,
            label: "w".to_string(),
            seen_at: Some(when),
            note: Some("second".to_string()),
            owner_id: Some(2),
        },
    )
    .await
    .expect("setting them again must be allowed");

    let refilled = load(&env.db, id).await.expect("get").expect("row");
    assert_eq!(refilled.note.as_deref(), Some("second"));
    assert_eq!(refilled.owner_id, Some(2));
    assert!(refilled.seen_at.is_some());
}
