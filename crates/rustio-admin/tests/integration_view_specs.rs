//! View-designer persistence round-trip against a real Postgres.
//!
//! Boots an ephemeral Postgres container per test via `testcontainers` and
//! exercises the `admin::view_specs` store (lazy `ensure_table` → `save` →
//! `load` → `saved_models`) through the `__integration` re-exports. Gated by
//! the `integration-test` feature, so a plain `cargo test` skips it.

#![cfg(feature = "integration-test")]

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

use rustio_admin::__integration::{view_specs_load, view_specs_save, view_specs_saved_models};
use rustio_admin::orm::Db;
use rustio_admin::view_layer::{infer_view_spec, FieldKind, FieldMeta, ViewMode, ViewSpec};

async fn boot() -> (Db, testcontainers::ContainerAsync<Postgres>) {
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
    (db, container)
}

fn sample_spec(model: &str) -> ViewSpec {
    infer_view_spec(
        model,
        &[
            FieldMeta {
                name: "id".into(),
                kind: FieldKind::Integer,
                nullable: false,
            },
            FieldMeta {
                name: "full_name".into(),
                kind: FieldKind::Text,
                nullable: false,
            },
            FieldMeta {
                name: "status".into(),
                kind: FieldKind::Enum,
                nullable: false,
            },
            FieldMeta {
                name: "password_hash".into(),
                kind: FieldKind::Text,
                nullable: false,
            },
        ],
    )
}

#[tokio::test]
async fn save_then_load_roundtrips_exactly() {
    let (db, _c) = boot().await;

    // No spec yet — load is None (the store creates its table on first hit).
    assert!(view_specs_load(&db, "customer").await.unwrap().is_none());

    let spec = sample_spec("customer");
    view_specs_save(&db, "customer", &spec).await.unwrap();

    let loaded = view_specs_load(&db, "customer")
        .await
        .unwrap()
        .expect("a saved spec");
    assert_eq!(loaded, spec, "round-tripped spec must equal the saved one");

    let models = view_specs_saved_models(&db).await.unwrap();
    assert!(models.contains(&"customer".to_string()));
}

#[tokio::test]
async fn save_upserts_and_keeps_one_row_per_model() {
    let (db, _c) = boot().await;

    let mut spec = sample_spec("order");
    view_specs_save(&db, "order", &spec).await.unwrap();

    // Re-save with a changed default mode — load reflects the latest write.
    spec.default_mode = ViewMode::Cards;
    view_specs_save(&db, "order", &spec).await.unwrap();

    let loaded = view_specs_load(&db, "order").await.unwrap().unwrap();
    assert_eq!(loaded.default_mode, ViewMode::Cards);

    // Still exactly one row for "order" (upsert, not insert).
    let models = view_specs_saved_models(&db).await.unwrap();
    assert_eq!(models.iter().filter(|m| m.as_str() == "order").count(), 1);
}
