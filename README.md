<div align="center">
  <img src="docs/assets/rustio-logo.png" alt="RustIO" width="200" />

  <h1>rustio-admin</h1>

  <p><strong>A Rust-first business-system engine — the operational foundation for serious software.</strong></p>

  <p>
    <a href="https://crates.io/crates/rustio-admin"><img alt="rustio-admin on crates.io" src="https://img.shields.io/crates/v/rustio-admin.svg?label=rustio-admin"></a>
    <a href="https://crates.io/crates/rustio-admin-cli"><img alt="rustio-admin-cli on crates.io" src="https://img.shields.io/crates/v/rustio-admin-cli.svg?label=rustio-admin-cli"></a>
    <a href="https://docs.rs/rustio-admin"><img alt="docs.rs" src="https://img.shields.io/docsrs/rustio-admin"></a>
    <a href="https://github.com/abdulwahed-sweden/rustio-admin/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/abdulwahed-sweden/rustio-admin/actions/workflows/ci.yml/badge.svg"></a>
    <a href="./LICENSE"><img alt="License" src="https://img.shields.io/crates/l/rustio-admin.svg"></a>
  </p>
</div>

---

RustIO is the layer that sits **underneath** your internal tools — not one more CRUD screen, but the operational core they stand on: admin panels, dispatch and booking backends, workflows, and audit trails. It is engineered to stay understandable as a company scales. In Django terms, it is the Rust answer to Django Admin — built for operations that have to last.

> [!NOTE]
> **Postgres only. No build step. Single binary deployment.**

## Contents

- [Why it exists](#why-it-exists)
- [Quick start](#quick-start)
- [Core principles](#core-principles)
- [What's inside](#whats-inside)
- [Install](#install)
- [Workspace](#workspace)
- [Documentation](#documentation)
- [Non-goals](#non-goals)
- [License & sponsorship](#license--sponsorship)

## Why it exists

RustIO was born from a real market failure, not a wish to build another dashboard. A fast-growing housing platform in the Swedish market — serving well over a million customers — began to crack under its own success. Not for lack of engineers, but because its foundation was never meant to be a heavy-duty operational engine.

As data grows, workflows multiply, and operational pressure rises, conventional web stacks reach their limits: slower performance, climbing infrastructure cost, fragile changes, and constant firefighting. **The problem is never the team — it is a weak foundation.**

RustIO exists for exactly that moment: when a business outgrows its first system and needs a foundation that will not collapse under its own growth.

> **Idea → Draft schema → Validate → Human review → Safe foundation.**
> _AI drafts. RustIO validates. Diff protects. Human approves._

**What makes it different.** Most admin tools treat CRUD as the product and bolt on authentication, recovery, and audit afterward. RustIO inverts that order. *Authority* — who may do what, how sessions end, how access is recovered, what gets recorded — is designed as one system and governed by checked-in contract documents. The CRUD is the easy layer on top.

## Quick start

An admin surface is one derive, one impl, one register call. The
`#[derive(RustioAdmin)]` macro emits the `Model` glue (`TABLE`, `COLUMNS`,
`from_row`, `insert_values`) from the struct's fields — there is no
hand-written ORM boilerplate to keep in sync.

```rust
#[derive(RustioAdmin)]
pub struct Post { pub id: i64, pub title: String, /* … */ }

impl ModelAdmin for Post {}                  // accept every default

let admin  = Admin::new().model::<Post>();
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

## Core principles

The invariants the framework refuses to break.

> **Doctrine 22** — session invalidation has a single writer.

> **Uniform outward responses** — recovery and login surfaces collapse every failure mode into a single response shape.

> **Audit by default** — every authority mutation emits a typed `AuditEvent`.

> **No plaintext at rest** — Argon2id for passwords, SHA-256 for session and reset tokens.

## What's inside

| Concern | What you get |
|---------|--------------|
| **Admin surface** | Derive generates list, create, edit, and delete pages; per-model RBAC over a five-tier role hierarchy. |
| **Identity & sessions** | DB-backed sessions, Argon2id passwords, hashed-at-rest tokens, centralised invalidation. |
| **Recovery** | Self-service reset, admin lock / unlock / revoke, auto-throttle, and a re-auth wall for destructive actions. |
| **Audit & observability** | Typed events with stable identifiers, per-request correlation IDs, redaction for secrets. |
| **AI assistant permissions** | A permissions / approval / audit layer over an external AI coding assistant — governance, not an embedded model. |
| **Project memory** | A non-authoritative `CLOUD.md` recording the *why* behind a project. On any conflict, code wins. |
| **Operational** | Postgres-only (Hyper, sqlx, minijinja). Single binary, one stylesheet, no build step. |
| **Visual design** | Hand-written CSS on `--rio-*` tokens; a build-time engine turns brand colors into a WCAG-safe palette. |

Most projects use a subset. Nothing here is mandatory.

## Install

The library and the CLI ship as separate crates.

```toml
[dependencies]
rustio-admin = "0.30.0"
tokio  = { version = "1", features = ["macros", "rt-multi-thread"] }
chrono = { version = "0.4", features = ["serde"] }
```

```bash
cargo install rustio-admin-cli      # provides the `rustio-admin` binary
```

## Workspace

Four crates ship together — the split keeps proc-macros, the theme engine, and CLI compilation off the hot path.

| Crate | Purpose |
|-------|---------|
| `rustio-admin` | The library. Re-exports the macros. |
| `rustio-admin-macros` | Proc-macros, re-exported from `rustio-admin`. |
| `rustio-admin-cli` | The `rustio-admin` binary — `new`, `startproject`, `startapp`, `migrate`, `user`, `group`, `perm`, `theme`, `ai`, `memory`, `doctor`, `docs`. |
| `rio-theme` | Build-time theme engine; turns raw brand colors into a WCAG-safe `tokens.css`. Not a runtime dependency. |

The full command reference — including the `builder`, `override`, and `audit` verbs — lives in [`docs/cli.md`](./docs/cli.md).

```bash
cargo build --workspace
cargo test  --workspace
```

## Documentation

The full index lives in [`docs/README.md`](./docs/README.md). Security-sensitive behaviour is governed by explicit contract documents, reviewed alongside the code:

| Document | Covers |
|----------|--------|
| [`DESIGN_SYSTEM.md`](./docs/design/DESIGN_SYSTEM.md) | Visual, token, and branding contract. |
| [`DESIGN_SESSIONS.md`](./docs/design/DESIGN_SESSIONS.md) | Session lifecycle, trust escalation, Doctrine 22. |
| [`DESIGN_AUDIT.md`](./docs/design/DESIGN_AUDIT.md) | Typed audit events, correlation chains, middleware ordering. |
| [`DESIGN_RECOVERY.md`](./docs/design/DESIGN_RECOVERY.md) | Self-service password recovery. |
| [`DESIGN_AI_ASSISTANT.md`](./docs/design/DESIGN_AI_ASSISTANT.md) | The `rustio ai` policy and proposal lifecycle. |
| [`DESIGN_CLOUD.md`](./docs/design/DESIGN_CLOUD.md) | Project memory — the non-authoritative why-layer. |

## Non-goals

RustIO is intentionally narrow in scope.

- Not a general-purpose web framework.
- Not an ORM — the `Model` trait is a thin sqlx shim.
- Not a content management system.
- Not AI-augmented — it embeds no model or planner; it only *governs* an external assistant.
- Not multi-database — Postgres only, by design.

## A note on the name

> [!IMPORTANT]
> There is a separate project called [`rustio`](https://github.com/abdulwahed-sweden/rustio) — a strict system builder with SQLite and a guided schema-evolution wizard. As of **v0.22.0** the binary published here is named `rustio-admin`, so the two no longer collide.

> [!WARNING]
> Install `rustio-admin-cli` to get the `rustio-admin` binary. Do **not** run `cargo install rustio` — that is an unrelated crate.

## License & sponsorship

MIT — see [`LICENSE`](./LICENSE). Developed and maintained independently.

> Build systems quickly. Evolve them safely. Stay in control.

If RustIO is useful to you, [**sponsor it on GitHub**](https://github.com/sponsors/abdulwahed-sweden) — early backing for open Rust infrastructure that keeps the core free and inspectable.
