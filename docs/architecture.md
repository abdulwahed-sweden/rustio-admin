# Architecture

Module map — read this before contributing or filing a structural bug. The full design rationale lives in [`STRATEGIC_RESET_PLAN.md`](./archive/STRATEGIC_RESET_PLAN.md); security-sensitive subsystems each have a contract under [`design/`](./design/).

## Workspace

```
rustio-admin/
├── crates/
│   ├── rustio-admin/          ← the library (everything project code consumes)
│   ├── rustio-admin-macros/   ← #[derive(RustioAdmin)] proc-macro
│   ├── rustio-admin-cli/      ← `rustio-admin` CLI binary (scaffolding, migrations,
│   │                            users, `theme`, `ai` assistant permissions, builder)
│   └── rio-theme/             ← build-time theme engine (brand colors → tokens.css)
└── examples/
    └── clinic/                ← canonical end-to-end consumer of the library
```

The `ai` surface (`crates/rustio-admin-cli/src/ai.rs`) is a permissions / approval / audit layer over external AI coding assistants — see [`design/DESIGN_AI_ASSISTANT.md`](./design/DESIGN_AI_ASSISTANT.md). It is offline by default; its `--as <email>` path authenticates an approver and mirrors decisions into `rustio_admin_actions` via three typed `AuditEvent` variants (`ai_proposal_approved` / `_rejected` / `_applied`).

**Project memory (CLOUD.md)** — the non-authoritative *why-layer* an external assistant reads for decision / rejected-idea / business-intent history — is governed by [`design/DESIGN_CLOUD.md`](./design/DESIGN_CLOUD.md). It is **subordinate to code forever, append-only, and human-ratified** (§3 invariants), and composes the `ai` lifecycle above: memory-write is a `needs_approval` capability moving through the same Suggested → Reviewed → Approved → Applied states. **Design-stage — the contract is approved; no runtime exists yet** (format, command surface, and retrieval are deferred to a later implementation design).

`rio-theme` runs only at build/CLI time (`rustio-admin theme generate`); the runtime library never links it — see [`design/DESIGN_THEME.md`](./design/DESIGN_THEME.md).

Workspace deps (`tokio`, `hyper`, `sqlx`, `minijinja`, `chrono`, `argon2`, …) are pinned at the workspace level for one-place version control. Project-side code never reaches for them transitively — the framework re-exports the surface it cares about (`Db`, `Router`, `Server`, `Templates`, etc.).

## Module map (`rustio-admin`)

Read top-to-bottom — the crate layers cleanly from the HTTP core up to the admin panel.

| Path | Responsibility |
|---|---|
| `error.rs`        | `Error` / `Result` + `From<sqlx::Error>` mapping (constraint violations → 409 Conflict). |
| `http.rs`         | `Request`, `Response`, `FormData`, typed per-request `Context`. |
| `router.rs`       | Tiny middleware-aware router; `:param` captures; 404 vs 405 distinction; `Next`. |
| `server.rs`       | hyper-based `Server` with graceful Ctrl-C shutdown + static serving. |
| `orm.rs`          | `Db`, `DbOptions`, `Model`, `Value`, `Row`, generic CRUD helpers. Postgres-only — **not** an ORM. |
| `migrations.rs`   | Walks numerically prefixed `*.sql` files; transactional apply with a tracking table. |
| `multipart.rs`    | `multipart/form-data` parser for file-upload widgets and CSV import (16 MB total / 8 MB per-file caps). |
| `templates.rs`    | minijinja loader; admin templates baked via `include_str!`; disk override via `RUSTIO_TEMPLATE_DIR`. |
| `background.rs`   | Tiny periodic task runner (currently the session sweeper). |
| `meta.rs`         | `rustio_admin_meta` key/value table — backs the startup fast-path: `init_tables` / `seed_permissions` stamp a version/fingerprint and skip redundant boot work. |
| `email/`          | Framework-emitted email (SMTP transport, project-branded HTML) used by the recovery flows. See `design/DESIGN_EMAIL.md`. |
| `middleware/`     | `logger`, `security_headers`, `rate_limit` (DashMap token bucket), `compression`, `csrf` (double-submit cookie), `correlation_id` (UUID v7, threaded into audit rows), `locale` (`Accept-Language` → `Locale`). |
| `auth/`           | `Identity`, `Role` ladder, `sessions`, `users` (Argon2id), `permissions` (60-s cache), `recovery` (self-service), `recovery_admin` (admin-driven), `mfa` (TOTP + backup codes), `emergency` (CLI), `guards`. **`sessions::invalidate_sessions` is the single writer for `revoked_at` (Doctrine 22).** |
| `admin/`          | The whole admin panel — see below. |

### `admin/` submodules

**Core seam** — the generic engine every model flows through:

| Path | Responsibility |
|---|---|
| `types.rs`        | `Admin` builder, `AdminEntry`, `AdminField`, `AdminModel` trait, `AdminTheme`, `SiteBranding`, `ListOpts`/`ListPage`/`ListRow`/`EditRow`/`CellLink`, the `AdminOps` trait. |
| `modeladmin.rs`   | `ModelAdmin` trait + `Fieldset`, `Inline`, `BulkAction`, `FieldValidationError`, `SortDir`, `parse_order_spec`. |
| `ops.rs`          | `ConcreteOps<M>` — **the only live `AdminOps` impl**. Builds list-page SQL (WHERE / ORDER BY / LIMIT / OFFSET) validating column names against `M::COLUMNS`. |
| `routes.rs`       | `register_admin_routes` mounts every admin URL with permission/role guards. Also owns the `ADMIN_CSS` concat + `/static/admin.css` serving (incl. the `RUSTIO_TOKENS_CSS` override). |
| `handlers.rs`     | Generic CRUD: `dashboard`, `list_model`, `show_*_form`, `do_create/update/delete`, history, password change — one handler per URL. |
| `render.rs`       | `serde::Serialize` context builders (`BaseContext`, `ListCtx`, `FormCtx`, …). **All HTML stays in templates; render.rs emits no markup.** |
| `builtin.rs`      | Bespoke `/admin/users/*` and `/admin/groups/*` pages (last-developer guards, perm-grid editor, profile tabs). |
| `audit.rs`        | `rustio_admin_actions` schema + `record` / `recent` / `for_object`; `AuditEvent` / `ActionType` / `LogEntry`. |
| `redact.rs`       | Type-level redaction helpers (`redact_password` / `_token` / `_mfa_secret` / `_backup_code`). |
| `relations.rs`    | `RelationRegistry` from `&[AdminEntry]` — delete-confirm cascade list + FK widgets. |
| `filters.rs`      | `classify_field`, `infer_filters`, field-UI metadata, `mask_pii` — pure schema-metadata → UI hints. |
| `icons.rs`        | Inline-SVG catalogue baked at compile time; exposed via the `icon()` minijinja function. |

**Feature surfaces** — each owns one operator-facing capability and its routes:

| Path | Surface |
|---|---|
| `bulk.rs`                  | Cascade-aware bulk delete + project `ModelAdmin::bulk_actions()` dispatch. |
| `csv_export.rs` / `csv_import.rs` | `/admin/:model/export.csv` (10k-row cap) and `…/import.csv`. |
| `saved_filters.rs`         | Per-operator saved-filter bookmarks. |
| `json_api.rs`              | `?format=json` / `Accept: application/json` negotiation on read + write paths. |
| `openapi.rs` / `sdk_gen.rs`| `/admin/apis` — OpenAPI 3.0 doc, HTML companion + playground, TypeScript SDK skeleton. |
| `db_browser.rs`            | `/admin/db` — read-only schema explorer (Developer-gated). |
| `feature_flags.rs`         | `rustio_feature_flags` + `feature_enabled(db, key)` + `/admin/feature_flags`. |
| `notifications.rs`         | `rustio_notifications` + `send_notification(...)` + topbar bell + `/admin/notifications`. |
| `health_dashboard.rs` / `healthz.rs` | `/admin/health` (web doctor) and public `/admin/healthz` liveness probe. |
| `docs.rs`                  | `/admin/docs` — renders the bundled `architecture.md` / `modeladmin.md` / `public-api.md`. |
| `recovery_handlers.rs` / `admin_recovery_handlers.rs` / `mfa_handlers.rs` | Self-service recovery, admin-driven recovery, and TOTP/backup-code HTTP handlers. |

## Templates

Located under `crates/rustio-admin/assets/templates/admin/`, baked into the binary via `include_str!` (`templates.rs`). A disk-side `templates/admin/<page>.html` wins over the embedded copy when a project sets `RUSTIO_TEMPLATE_DIR`; per-model overrides (`templates/admin/<model>/list.html`) win for that model only.

Grouped by area (not exhaustive):

- **Shell partials** — `_base.html`, `_topbar.html`, `_sidebar.html`, `_theme.html` (the `AdminTheme` override `<style>`), `_row_actions.html`, `includes/_form_field.html` (single widget render path), `includes/_field_errors.html`.
- **Core CRUD** — `index.html` (dashboard), `list.html` (toolbar + filters + sort + pagination + bulk select), `form.html`, `confirm_delete.html`, `bulk_confirm_delete.html`, `bulk_confirm_action.html`, `object_history.html`, `log_entries.html`.
- **Auth & recovery** — `login.html`, `forgot_password*.html`, `reset_password.html`, `password_change.html`, `must_change_password.html`, `reauth.html`, `account_sessions.html`, `admin_reset_password.html`, `lock_user.html`, `confirm_admin_action.html`.
- **MFA** — `mfa_enroll*.html`, `mfa_verify.html`, `mfa_disable.html`, `mfa_regenerate*.html`.
- **Users & groups** — `users_list.html`, `user_new/edit/view/confirm_delete.html`, `groups_list.html`, `group_new/edit/confirm_delete.html`.
- **Operational surfaces** — `apis_index.html`, `apis_playground.html`, `db_browser.html`, `feature_flags.html`, `notifications.html`, `health.html`, `docs_index.html`, `doc_page.html`, `csv_import_result.html`, `error.html`, `forbidden.html`.

## CSS + JS

Located under `crates/rustio-admin/assets/static/admin/`. Hand-written, **no build step** (no Tailwind / PostCSS / Sass / bundler). A minimal `admin.js` (~160 LOC) handles the sidebar drawer, dropdown wiring, bulk-select helper, and FK autocomplete.

The CSS is a **Primer/Carbon-style multi-file architecture**, concatenated into one bundle served at `/static/admin.css`:

```
tokens/ → base/ → layout/ → components/ → pages/ → print/
```

- `tokens/` — `colors.css` (itself generated by `rio-theme`), `spacing`, `radius`, `shadows`, `typography`.
- `base/`, `layout/`, `components/`, `pages/`, `print/` — reset/utilities, shell/topbar/sidebar/footer/responsive, the component library, per-page styles, and print rules.

**Lock-step invariant:** the `@import` list in `admin/admin.css` and the `ADMIN_CSS` `concat!(include_str!(…), …)` block in `src/admin/routes.rs` must stay in the same order — `layout/responsive.css` is intentionally loaded **last** so its mobile-first overrides win.

The framework ships **light and dark**, token-driven only: light in `:root`, dark re-derived in the `@media (prefers-color-scheme: dark)` and `[data-theme="dark"]` blocks of `tokens/colors.css` (no per-component dark CSS). Token **values** are owned by `design/VISUAL-CONTRACT.md` (and `design/TOKENS-EMIT-SPEC.md` for generated overrides); `design/DESIGN_DOCTRINE.md` (visual identity) and `design/DESIGN_SYSTEM.md` (token ownership) carry the principles.

Two project-side override paths, both layered *after* the baked bundle so they win the cascade without `!important`:

- **`AdminTheme`** (`_theme.html`) — a small inline `<style>` patch over six tokens (`--rio-accent`, `--rio-bg`, `--rio-surface`, `--rio-text`, `--rio-text-muted`, `--rio-border`). Emits nothing when no field is set. One `Admin::accent_color("…")` call away.
- **`RUSTIO_TOKENS_CSS=<path>`** — appends a whole generated `rio-theme` `tokens.css` to `/static/admin.css` at runtime (read once at router build; degrades to the baked bundle if unreadable). See `design/DESIGN_THEME.md` §12.

Three responsive breakpoints (mobile-first): `< 768px` single column with off-canvas sidebar; `≥ 768px` two-column with a sticky sidebar; `≥ 1280px` wider chrome with content capped so wide monitors don't sprawl rows.

## Public API surface

Stable export points (canonical list lives in `crates/rustio-admin/src/lib.rs`):

```rust
use rustio_admin::{
    // HTTP + data core
    Db, DbOptions, Model, Row, Value,
    Router, Next, Server, Request, Response, FormData,
    Error, Result,

    // Auth
    Identity, Role,

    // Admin builder + model customisation
    Admin, AdminField, AdminModel, FieldType,
    ModelAdmin, Fieldset, Inline, BulkAction, FieldValidationError,
    BulkActionContext, BulkActionResult, BulkActionFailure,
    register_admin_routes,

    // Embedded-template introspection
    embedded_template_names, embedded_template_source,

    // Derive macro
    RustioAdmin,
};

// Module namespaces a project may reach into:
use rustio_admin::{auth, background, middleware, migrations, email};
use rustio_admin::admin::{
    AdminTheme, SiteBranding, UserProfileRow, UserProfileSection,
    feature_enabled, send_notification, Notification, FeatureFlag,
    // audit + redaction (e.g. for CLI tools mirroring decisions):
    record, LogEntry, ActionType, AuditEvent,
};
use rustio_admin::templates::Templates;
```

Everything else (`ConcreteOps`, `AdminOps`, `ListOpts`, `BaseContext`, …) is `pub(crate)` or `pub` within `admin::*` for testing only — projects never touch it directly. See [`public-api.md`](./public-api.md) for the enumerated surface.

## Hard architectural rules

The full list lives in [§8 of the strategic reset plan](./archive/STRATEGIC_RESET_PLAN.md#8-strict-architectural-rules). CI enforces **rule #1** (no Tier 2 features) with a `git grep` guard on every PR (scoped to source/TOML, excluding `crates/*/assets/`):

```
forbidden='HasSchema|ModelSchema|RustType|SchemaOps|from_schema|contract_validator|contract_doctor|RustioModel'
```

If you need any of those for a project, the right layer is a future `rustio-pro` crate — not this one.

Other rules:

- **Postgres only.** No SQLite/MySQL branches, no abstraction over databases.
- **No second runtime.** `ConcreteOps<M>` is the runtime — no bridges, no schema-driven sibling.
- **No build step.** Templates and CSS/JS are hand-written and baked via `include_str!`; `rio-theme` runs at build/CLI time, never at runtime.
- **No magic.** Macros emit obvious code; `cargo expand` shows the full picture.
- **Audit by default.** Every authority mutation emits a typed `AuditEvent` with a correlation ID (`design/DESIGN_AUDIT.md`).
- **Session invalidation has a single writer** (`auth::sessions::invalidate_sessions` — Doctrine 22).
- **No plaintext at rest.** Argon2id for passwords; SHA-256 for session and reset tokens.
- **Uniform outward responses** on login and recovery surfaces — every failure mode collapses to one response shape (`design/DESIGN_RECOVERY.md`).
- **Every public type and method has a doc comment.** No silent contracts.
- **Boring software.** Prefer obvious over clever; compile-time errors over runtime errors.
