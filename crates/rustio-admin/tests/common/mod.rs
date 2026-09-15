//! Shared harness for the database-backed contract suites.
//!
//! Every `contract_*.rs` file that needs a live Postgres boots it through
//! here. Before the lean-core reset each suite carried its own byte-identical
//! copy of [`boot`]; this module is the single owner.
//!
//! ## Why one container per test
//!
//! Container startup is ~3–5 s on a warm Docker daemon. Sharing one container
//! across tests would buy a few seconds but introduces test-order coupling and
//! forces every test to reason about leftover state. A container per test is
//! the right trade-off for a CI-only suite.
//!
//! The whole module is gated on the `integration-test` feature, so a plain
//! `cargo test --workspace` never compiles `testcontainers`.

#![cfg(feature = "integration-test")]
// Each contract suite is its own test binary and uses a subset of this module;
// the unused remainder would otherwise warn per-binary.
#![allow(dead_code)]

use rustio_admin::auth::{self, Role};
use rustio_admin::orm::Db;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

/// A running Postgres container alongside its connected [`Db`].
///
/// The container is shut down when the value drops, so a test only has to
/// keep the `TestEnv` alive for as long as it needs the database.
pub struct TestEnv {
    pub db: Db,
    _container: testcontainers::ContainerAsync<Postgres>,
}

/// Boot an ephemeral Postgres and connect to it. No schema is applied.
///
/// Use this when the test owns its own DDL; use [`boot`] when the test needs
/// the framework's auth/admin tables.
pub async fn boot_bare() -> TestEnv {
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
    TestEnv {
        db,
        _container: container,
    }
}

/// Boot an ephemeral Postgres with the framework schema applied.
///
/// `auth::init_tables` lays down the R0 + R1 + R2 + R3 + R4 schema in one
/// shot, which is what every authority suite needs before it can exercise a
/// runtime fn.
pub async fn boot() -> TestEnv {
    let env = boot_bare().await;
    auth::init_tables(&env.db).await.expect("auth::init_tables");
    env
}

/// Create a user and return its id. Panics on failure — a harness fault is
/// not a test outcome worth asserting on.
pub async fn create_user(db: &Db, email: &str, password: &str, role: Role) -> i64 {
    auth::create_user(db, email, password, role)
        .await
        .expect("create_user")
}
