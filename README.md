<p align="center">
  <img src="docs/assets/rustio-logo.png" alt="RustIO" width="250">
</p>

<p align="center">
  <strong>rustio-admin</strong>
</p>

<p align="center">
  Postgres-first administrative framework for Rust applications.
</p>

<p align="center">
  <a href="https://crates.io/crates/rustio-admin"><img alt="rustio-admin on crates.io" src="https://img.shields.io/crates/v/rustio-admin.svg?label=rustio-admin"></a>
  <a href="https://crates.io/crates/rustio-admin-cli"><img alt="rustio-admin-cli on crates.io" src="https://img.shields.io/crates/v/rustio-admin-cli.svg?label=rustio-admin-cli"></a>
  <a href="https://docs.rs/rustio-admin"><img alt="docs.rs" src="https://img.shields.io/docsrs/rustio-admin"></a>
  <a href="https://github.com/abdulwahed-sweden/rustio-admin/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/abdulwahed-sweden/rustio-admin/actions/workflows/ci.yml/badge.svg"></a>
  <a href="./LICENSE"><img alt="License" src="https://img.shields.io/crates/l/rustio-admin.svg"></a>
</p>

---

Authentication, sessions, audit trails, and recovery are designed
as one system, not assembled as separate parts.

Postgres only. No build step. Single binary deployment.


## 30-second example

An admin surface is one derive, one impl, one register call.

```rust
#[derive(RustioAdmin)]
pub struct Post { pub id: i64, pub title: String, /* … */ }

impl Model      for Post { /* TABLE, COLUMNS, from_row, insert_values */ }
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


## Why RustIO exists

Three observations shape the framework.

### CRUD is the surface

The weight of production administrative work sits underneath —
authority transitions, session lifecycle, recovery, audit chains.

RustIO treats these as first-class concerns.

### Doctrine before implementation

Authentication, recovery, session invalidation, and audit
behaviour are governed by checked-in contract documents.

Pull requests are reviewed against the doctrine, not only
against the implementation diff.

### Operational clarity over flexibility

The framework targets one database, ships one stylesheet, and
requires no build step.

Authority, sessions, recovery, and audit receive the
engineering attention. The rest stays intentionally simple.

---

The framework surface stays intentionally narrow so the
security-sensitive paths remain reviewable.


## Core principles

The invariants the framework refuses to break.

> **Doctrine 22**
> Session invalidation has a single writer.

> **Uniform outward responses**
> Recovery and login surfaces collapse every failure mode
> into a single response shape.

> **Audit-by-default**
> Every authority mutation emits a typed `AuditEvent`.

> **No plaintext at rest**
> Argon2id for passwords. SHA-256 for session and reset tokens.


## What's in the box

The surface is grouped into these concerns.

### Admin surface

- `#[derive(RustioAdmin)]` on a struct generates list, create,
  edit, and delete pages.
- `impl ModelAdmin` overrides defaults — `list_display`,
  `list_filter`, `search_fields`, `ordering`, `list_per_page`.
- Per-model RBAC over a five-tier role hierarchy.

### Identity and sessions

- DB-backed sessions with Argon2id passwords.
- Hashed-at-rest session tokens.
- Centralised invalidation through `auth::sessions::invalidate_sessions`.

### Recovery

- Self-service forgot and reset.
- Admin-driven password reset, lock, unlock, and revoke.
- Auto-throttle on failed logins.
- Re-auth wall for destructive admin actions.

### Audit and observability

- Typed `AuditEvent` with stable string identifiers.
- Per-request correlation IDs.
- Redaction helpers for tokens and passwords.

### AI assistant permissions

- `rustio ai` governs what an external AI coding assistant (Claude
  Code, Copilot, Cursor, …) may do in the project — a permissions /
  approval / audit layer, **not** an embedded model.
- A version-controlled `.rustio/ai.toml` policy sorts every capability
  into Allowed / Needs approval / Blocked. Changes are proposed,
  reviewed, approved (distinct approvers enforced), and applied as an
  explicit step; Blocked is refused outright.
- Offline by default. `--as <email>` authenticates the approver against
  the users table and mirrors the decision into `rustio_admin_actions`.
- Contract: [`DESIGN_AI_ASSISTANT.md`](./docs/design/DESIGN_AI_ASSISTANT.md).

### Operational

- Postgres-only. Hyper, sqlx, and minijinja under the hood.
- Single binary deploy. No build step. One stylesheet.
- Project templates and CSS embedded at compile time.

### Visual design

- Hand-written CSS driven by `--rio-*` custom-property tokens.
- `rio-theme` (build-time companion crate) generates a safe
  `tokens.css` from one or more brand colors — WCAG-checked,
  vivid colors auto-tamed, brand-vs-semantic-state collisions
  resolved.
- v0.19 "Quiet Expert" design language: unified `.rio-section`,
  `.rio-empty-state`, `.rio-confirm`, kebab row actions,
  area-chart sparklines. See [`CHANGELOG.md`](./CHANGELOG.md)
  `[0.19.0]` for the per-page record.

---

Most projects use a subset. The framework does not require
adopting all of it.


## Install

The library and the CLI ship as separate crates.

```toml
[dependencies]
rustio-admin = "0.25.0"
tokio  = { version = "1", features = ["macros", "rt-multi-thread"] }
chrono = { version = "0.4", features = ["serde"] }
```

```sh
cargo install rustio-admin-cli      # provides the `rustio-admin` binary
```


## Documentation

Full documentation index: [`docs/README.md`](./docs/README.md).

| Path                                       | Contents                                                       |
|--------------------------------------------|----------------------------------------------------------------|
| [`docs/`](./docs/)                         | Guides, doctrine, architecture overview.                       |
| [`docs/design/`](./docs/design/)           | Long-form design contracts — one file per security-sensitive subsystem. |
| [`docs/public-api.md`](./docs/public-api.md) | Enumerated public API surface (generated; descriptive, not normative). |
| [`docs/archive/`](./docs/archive/)         | Historical and superseded planning material.                   |


## Reading paths

Where to start depends on the work.

**New to the framework**
→ 30-second example above
→ [`docs/getting-started.md`](./docs/getting-started.md)
→ [`DESIGN_SYSTEM.md`](./docs/design/DESIGN_SYSTEM.md)

**Working on authentication or recovery**
→ [`DESIGN_RECOVERY.md`](./docs/design/DESIGN_RECOVERY.md)
→ [`DESIGN_R2_ORGANISATIONAL.md`](./docs/design/DESIGN_R2_ORGANISATIONAL.md)

**Auditing authority boundaries**
→ [`DESIGN_AUDIT.md`](./docs/design/DESIGN_AUDIT.md)
→ [`DESIGN_SESSIONS.md`](./docs/design/DESIGN_SESSIONS.md)

**Building on the published crate**
→ Install above
→ [`docs/modeladmin.md`](./docs/modeladmin.md)
→ [`examples/clinic/`](./examples/clinic/)

**Understanding scope and design history**
→ [`docs/architecture.md`](./docs/architecture.md)
→ [`STRATEGIC_RESET_PLAN.md`](./docs/archive/STRATEGIC_RESET_PLAN.md)
→ [`CHANGELOG.md`](./CHANGELOG.md)


## Doctrine documents

Security-sensitive behaviour is governed by explicit contract
documents reviewed alongside the code.

### [`DESIGN_SYSTEM.md`](./docs/design/DESIGN_SYSTEM.md)

Visual, token, and branding contract.

### [`DESIGN_SESSIONS.md`](./docs/design/DESIGN_SESSIONS.md)

Session lifecycle, trust escalation, and Doctrine 22 — the
single-writer invariant on `revoked_at`.

### [`DESIGN_AUDIT.md`](./docs/design/DESIGN_AUDIT.md)

Typed audit events, correlation-id chains, and the required
middleware ordering.

### [`DESIGN_RECOVERY.md`](./docs/design/DESIGN_RECOVERY.md)

Self-service password recovery (R1, ships in 0.5.0).

### [`DESIGN_R2_ORGANISATIONAL.md`](./docs/design/DESIGN_R2_ORGANISATIONAL.md)

Admin-driven recovery, auto-throttle, re-auth wall (R2,
ships in 0.6.0).

### [`DESIGN_AI_ASSISTANT.md`](./docs/design/DESIGN_AI_ASSISTANT.md)

Permissions, approval, and audit layer over external AI coding
assistants — the `rustio ai` policy, proposal lifecycle, and opt-in
audit mirroring (ships in 0.23.0).

---

Each document is the source of truth for its surface.


## Workspace layout

Four crates ship together. The split keeps proc-macros, the
theme engine, and CLI compilation off the project's hot path.

| Crate | Purpose |
|---|---|
| `rustio-admin`        | The library. Re-exports the macros. |
| `rustio-admin-macros` | Proc-macros (re-exported from `rustio-admin`). |
| `rustio-admin-cli`    | The `rustio-admin` binary — `new` (friendly alias for `startproject`), `startproject`, `startapp`, `migrate`, `user`, `group`, `perm`, `theme`, `ai` (AI assistant permissions), `doctor`, `docs`, `builder new`. |
| `rio-theme`           | Build-time theme engine. Turns raw brand colors into a WCAG-safe `tokens.css`. Not depended on by `rustio-admin` at runtime. |

```sh
cargo build --workspace
cargo test  --workspace
```


## Non-goals

RustIO is intentionally narrow in scope.

- Not a general-purpose web framework.
- Not an ORM. The `Model` trait is a thin sqlx shim.
- Not a content management system.
- Not AI-augmented — the framework embeds no model or planner. (`rustio ai`
  *governs* an external AI assistant via a permissions / approval / audit
  layer; it does not call out to one.)
- Not multi-database. Postgres only, by design.
- Not schema-contract-driven.


## CLI

See [`docs/cli.md`](docs/cli.md) for the full command reference.


## Naming — what about `rustio`?

There's a separate project at **[`rustio`](https://github.com/abdulwahed-sweden/rustio)** — a strict-by-construction system builder with SQLite, a guided schema-evolution wizard (`rustio evolve`), and an admin UI. Through `rustio-admin`'s v0.21.x line, this project also shipped a CLI binary called `rustio`, which meant `cargo install rustio-admin-cli` and `cargo install rustio-cli` silently overwrote each other in `~/.cargo/bin`.

As of v0.22.0 the binary published by this project is named **`rustio-admin`**, so the two no longer collide. You can install both on the same machine; `rustio-admin` always means this project, `rustio` always means the other.

The two are different in scope — this project (`rustio-admin`) targets Postgres-only admin panels with a narrow, doctrine-governed surface; the sibling project layers an admin UI, an ORM, and a guided schema-evolution wizard over a strict typed core with SQLite. Same name prefix, different goals.


## License

MIT — see [`LICENSE`](./LICENSE).
