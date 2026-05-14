# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

`rustio-admin` is a Postgres-first administrative framework for Rust ("Django Admin, but for Rust"). It is a Cargo workspace shipping three crates: the library, a proc-macro crate, and a `rustio` CLI binary. Authentication, sessions, recovery, and audit are designed as one system — not assembled from separate parts.

Project README: `README.md`. Architecture map: `docs/architecture.md`. Design contracts (the source of truth for security-sensitive subsystems): `docs/design/DESIGN_*.md`.

## Common commands

CI runs with `RUSTFLAGS="-D warnings"` — clippy/build/test all fail on any warning. Match that locally.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build  --workspace --all-targets
cargo test   --workspace --all-targets                   # unit + doc tests only
cargo test   --workspace --all-targets --features integration-test  # adds testcontainers Postgres suite
```

Running one test:

```sh
cargo test -p rustio-admin <name_substring>
cargo test -p rustio-admin --features integration-test --test integration_recovery <name>
```

Integration suites under `crates/rustio-admin/tests/integration_*.rs` are gated by the `integration-test` feature (see the `[features]` block in `crates/rustio-admin/Cargo.toml`). They each boot an ephemeral Postgres container per test via `testcontainers`, so Docker must be running. The `cascade_lockstep.rs` test is *not* gated and runs in the default suite.

CI's Tier-2-symbol guard (`.github/workflows/ci.yml`) — also run this locally before pushing if you've touched core or examples:

```sh
git grep -nE 'HasSchema|ModelSchema|RustType|SchemaOps|from_schema|contract_validator|contract_doctor|RustioModel' -- crates/ examples/ Cargo.toml
```

Any match is a CI failure. These symbols belong to a future `rustio-pro` layer and must never appear in this repo.

The `rustio` CLI (built from `crates/rustio-admin-cli`) reads `DATABASE_URL` from `.env`:

```sh
cargo run -p rustio-admin-cli -- migrate apply
cargo run -p rustio-admin-cli -- user create --email admin@local --role administrator
cargo run -p rustio-admin-cli -- doctor                # or `doctor email --to ...` for SMTP
```

## Architecture — the big picture

**One library crate, one runtime, no second runtime.** `crates/rustio-admin/src/` is the entire framework surface. The split into `rustio-admin-macros` and `rustio-admin-cli` exists only to keep proc-macro and CLI compile time off the hot path.

The library layers cleanly — read in this order:

- `error.rs`, `http.rs`, `router.rs`, `server.rs` — small hyper-based HTTP core. The `Request`/`Response`/`FormData` and the `:param`-capturing router are the framework's foundation. `Error` maps `sqlx` constraint violations to 409.
- `orm.rs` — Postgres-only sqlx shim: `Db`, `Model`, `Value`, `Row`. **Not** an ORM. `migrations.rs` walks numerically prefixed `*.sql` files transactionally.
- `templates.rs` — minijinja loader; admin templates baked via `include_str!` into `EMBEDDED_TEMPLATES`. Set `RUSTIO_TEMPLATE_DIR` for disk override.
- `middleware/` — `logger`, `security_headers`, `rate_limit` (DashMap token bucket), `compression`, `csrf` (double-submit cookie), `correlation_id` (UUID v7 threaded into audit rows).
- `auth/` — `Identity`, `Role` (5-tier ladder), `sessions` (Argon2id passwords, SHA-256 hashed-at-rest tokens), `permissions` (60-s cache), plus `recovery`, `recovery_admin`, `mfa`, `emergency`. **`auth::sessions::invalidate_sessions` is the single writer for `revoked_at` — Doctrine 22.**
- `admin/` — the whole admin panel. The most important seam in the codebase, see below.
- `background.rs`, `email/` — periodic task runner (session sweeper), framework-emitted email.

### Inside `admin/`

`admin::types` defines `Admin` (the builder), `AdminEntry`, `AdminModel`, `AdminTheme`, `SiteBranding`, plus the `AdminOps` trait. **`admin::ops::ConcreteOps<M>` is the only live `AdminOps` impl** — there is no schema-driven sibling, and adding one would violate the "no second runtime" rule. `ConcreteOps` builds list-page SQL (WHERE/ORDER BY/LIMIT/OFFSET) validating column names against `M::COLUMNS`.

`admin::routes::register_admin_routes` mounts every admin URL with permission and role guards. One handler per URL lives in `admin::handlers` (generic CRUD) or `admin::builtin` (the bespoke `/admin/users/*` and `/admin/groups/*` pages with last-developer guards and perm-grid editor). `admin::render` builds `serde::Serialize` template contexts — **all HTML stays in templates**; render.rs never emits markup.

`admin::audit` writes the `rustio_admin_actions` table; `admin::relations::RelationRegistry` drives delete-confirm cascade lists; `admin::filters` turns schema metadata into UI filter hints; `admin::icons` is the inline-SVG catalogue exposed via the `icon()` minijinja function.

### Templates and CSS

- Templates live under `crates/rustio-admin/assets/templates/admin/`, baked at compile time. A disk-side `templates/admin/<page>.html` wins over the embedded copy if `RUSTIO_TEMPLATE_DIR` is set.
- CSS lives under `crates/rustio-admin/assets/static/admin/`, organized as a Primer/Carbon-style multi-file architecture: `tokens/` → `themes/` → `base/` → `layout/` → `components/` → `pages/` → `print/`. The runtime concatenates fragments and serves one bundle at `/static/admin.css`. **The `@import` list in `admin/admin.css` and the `ADMIN_CSS` `concat!(include_str!(…), …)` block in `src/admin/routes.rs` must stay in lock-step** — order matters, and `responsive.css` is intentionally loaded last to override desktop layout.
- The full token philosophy and dark-mode contract is `docs/DESIGN_DOCTRINE.md` (visual identity) and `docs/design/DESIGN_SYSTEM.md` (token ownership). Read both before changing CSS.

## Hard rules this codebase refuses to break

These are enforced by some combination of CI, code review, and doctrine docs. Internalize them before editing:

- **Postgres only.** No SQLite/MySQL branches, no abstraction over databases. `orm.rs` assumes Postgres semantics.
- **No second runtime.** `ConcreteOps<M>` is *the* runtime. No bridges, no schema-driven sibling. Tier-2 symbols (`HasSchema`, `ModelSchema`, `RustType`, `SchemaOps`, …) are forbidden by `ci.yml`.
- **No build step.** Hand-written CSS and JS — no Tailwind, no PostCSS, no Sass, no bundler. Templates and stylesheets are baked via `include_str!`.
- **No magic.** Macros emit obvious code; `cargo expand` should always show the full picture.
- **Session invalidation has a single writer** (`auth::sessions::invalidate_sessions`) — Doctrine 22 in `DESIGN_SESSIONS.md`.
- **Audit-by-default.** Every authority mutation emits a typed `AuditEvent` with correlation ID. See `DESIGN_AUDIT.md` for required middleware ordering.
- **No plaintext at rest.** Argon2id for passwords. SHA-256 for session and reset tokens.
- **Every public type and method has a doc comment.** No silent contracts.
- **Uniform outward responses** on login and recovery surfaces — every failure mode collapses to a single response shape (see `DESIGN_RECOVERY.md`).

The narrow surface is the point. If a feature feels like it wants schema-driven metadata, multi-DB support, or a frontend build step, the right layer is a future `rustio-pro` crate — not this one. `README.md` "Non-goals" enumerates what is intentionally out of scope.

## Where to look first

- Touching authority/sessions/recovery/MFA/emergency → read the matching `docs/design/DESIGN_*.md` *before* the code. Pull requests are reviewed against the doctrine, not only the diff.
- Touching CSS, tokens, or templates → `docs/DESIGN_DOCTRINE.md` § 1 (tokens), § 7 (source layout), § 9 (adding a fragment). The PR template requires a token disclosure and a visual regression checklist (`.github/pull_request_template.md`).
- Changing what's public → `docs/public-api.md` is generated/descriptive; the canonical `pub use` surface lives in `crates/rustio-admin/src/lib.rs`. Anything not re-exported there is `pub(crate)` or `pub` inside `admin::*` for testing only.
- Understanding scope and history → `ROADMAP.md`, `CHANGELOG.md`, and `docs/archive/rustio-admin-strategic-reset-plan.md` § 8 (strict architectural rules).
- The canonical end-to-end consumer of the library lives at `examples/library-circulation/`.

## Workflow conventions

- `CHANGELOG.md` has an `[Unreleased]` section. Behaviour changes and any new `--rio-*` token need an entry; pure refactors / docs / test-only changes do not.
- Migrations are **append-only by contract**. Never edit an applied migration; write a new numerically prefixed `*.sql` that moves the schema forward.
- Internal-visibility markers: `// internal:` comments mark items that are intentionally `pub(crate)` rather than `pub`. The recent privatisation commit (`2cb38b3`) is the reference for the convention.
