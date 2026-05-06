# rustio-admin

> **Django Admin, but for Rust.** A small, focused admin framework for Postgres-backed Rust apps. From a `struct` to a working CRUD admin in under 50 lines of project code.

[![crates.io](https://img.shields.io/crates/v/rustio-admin.svg?label=rustio-admin)](https://crates.io/crates/rustio-admin)
[![crates.io](https://img.shields.io/crates/v/rustio-admin-cli.svg?label=rustio-admin-cli)](https://crates.io/crates/rustio-admin-cli)
[![docs.rs](https://img.shields.io/docsrs/rustio-admin)](https://docs.rs/rustio-admin)
[![CI](https://github.com/abdulwahed-sweden/rustio-admin/actions/workflows/ci.yml/badge.svg)](https://github.com/abdulwahed-sweden/rustio-admin/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/rustio-admin.svg)](./LICENSE)

> 🚀 **v0.1.0 released on 2026-05-07** — first public release on crates.io. Three crates landed together: [`rustio-admin`](https://crates.io/crates/rustio-admin) (the library), [`rustio-admin-macros`](https://crates.io/crates/rustio-admin-macros) (the `RustioAdmin` derive), and [`rustio-admin-cli`](https://crates.io/crates/rustio-admin-cli) (the `rustio` binary: `startproject`, `startapp`, `migrate`, `user`, `group`, `perm`, `doctor`). See the [v0.1.0 changelog](./CHANGELOG.md#010--2026-05-07) for the full feature inventory and the [release tag](https://github.com/abdulwahed-sweden/rustio-admin/releases/tag/v0.1.0) for an immutable reference.

## What you get

- `#[derive(RustioAdmin)]` on a plain struct → list / create / edit / delete pages.
- `impl ModelAdmin` for Django-style customisation (`list_display`, `list_filter`, `search_fields`, `ordering`, `list_per_page`).
- DB-backed sessions, Argon2 passwords, 5-tier role hierarchy, per-model RBAC.
- Hyper + sqlx + minijinja under the hood. **Postgres only.** No Tailwind, no PostCSS, no build step — one hand-written stylesheet.
- Single binary deploy. Project templates and CSS are baked in via `include_str!`; project overrides drop into `templates/admin/`.

## Install

```toml
[dependencies]
rustio-admin = "0.1"
tokio  = { version = "1", features = ["macros", "rt-multi-thread"] }
chrono = { version = "0.4", features = ["serde"] }
```

The CLI ships separately:

```sh
cargo install rustio-admin-cli      # provides the `rustio` binary
```

## The 3-line idea

```rust
#[derive(RustioAdmin)]
pub struct Post { pub id: i64, pub title: String, /* … */ }

impl Model     for Post { /* TABLE, COLUMNS, from_row, insert_values */ }
impl ModelAdmin for Post {}                            // accept every default

// In your `main`:
let admin = Admin::new().model::<Post>();
let router = register_admin_routes(Router::new(), admin, db, templates);
Server::new(router, addr).run().await?;
```

A model that wants more control fills in the `ModelAdmin` body:

```rust
impl ModelAdmin for Post {
    fn list_display()  -> &'static [&'static str] { &["title", "published", "created_at"] }
    fn list_filter()   -> &'static [&'static str] { &["published"] }
    fn search_fields() -> &'static [&'static str] { &["title", "body"] }
    fn ordering()      -> &'static [&'static str] { &["-created_at"] }
}
```

For a complete project skeleton see [`examples/minimal/`](./examples/minimal/) and the [`docs/getting-started.md`](./docs/getting-started.md) walkthrough.

## Documentation

| | |
|---|---|
| [Getting started](./docs/getting-started.md)        | `cargo new` to a running admin in 10 minutes. |
| [`ModelAdmin` reference](./docs/modeladmin.md)     | Every hook, every default, when to override. |
| [Architecture](./docs/architecture.md)              | Module map, runtime, public API surface. |
| [Strategic reset plan](./rustio-admin-strategic-reset-plan.md) | Why the framework exists, what's in/out of scope. |
| [Changelog](./CHANGELOG.md)                         | Per-release summary. |

## Workspace layout

| Crate | Purpose |
|---|---|
| `rustio-admin`        | The library. Re-exports the macros. |
| `rustio-admin-macros` | Proc-macros (re-exported from `rustio-admin`). |
| `rustio-admin-cli`    | The `rustio` binary — `startproject`, `startapp`, `migrate`, `user`, `group`, `perm`, `doctor`. |

```sh
cargo build --workspace
cargo test  --workspace
```

## Status

**v0.1.0 — released 2026-05-07.** Browser-walked end-to-end against a local Postgres against both the [`examples/minimal`](./examples/minimal/) skeleton and a fresh `rustio startproject` + `rustio startapp` flow; the [`classrooms`](https://github.com/abdulwahed-sweden/classrooms) project consumes the published crate. See the [strategic reset plan](./rustio-admin-strategic-reset-plan.md) for the design and the [changelog](./CHANGELOG.md) for the feature inventory.

Future work tracks under separate milestones — v0.2 candidates include richer filter widgets (date range, multi-select), bulk actions on the list page, inline forms, and dashboard widgets. The schema-contract / drift-validator / AI-planner work that originally lived in this repo's predecessor stays explicitly out of scope and will, if it returns at all, ship as a separate `rustio-pro-*` family of crates.

## Non-goals

`rustio-admin` is the admin layer and the auth/permissions/templates that surround it — nothing more. **Not** a full web framework. **Not** an ORM (the `Model` trait is a thin sqlx shim). **Not** a content management system. **Not** AI-augmented. **Not** multi-database (Postgres only, on purpose). **Not** schema-contract-driven (the schema-contract / drift-validator / AI-planner work belongs in a future Tier-2 crate, kept entirely out of this repo).

## License

MIT — see [`LICENSE`](./LICENSE).
