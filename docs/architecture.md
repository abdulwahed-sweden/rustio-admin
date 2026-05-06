# Architecture

Brief module map — read this before contributing or filing a structural bug. The full design rationale lives in [`rustio-admin-strategic-reset-plan.md`](../rustio-admin-strategic-reset-plan.md).

## Workspace

```
rustio-admin/
├── crates/
│   ├── rustio-admin/          ← the library (everything project code consumes)
│   ├── rustio-admin-macros/   ← #[derive(RustioAdmin)] proc-macro
│   └── rustio-admin-cli/      ← `rustio` CLI binary (work in progress)
└── examples/
    └── minimal/               ← canonical project skeleton
```

Workspace deps (`tokio`, `hyper`, `sqlx`, `minijinja`, `chrono`, `argon2`, …) are pinned at the workspace level for one-place version control. Project-side code never reaches for them transitively — the framework re-exports the surface it cares about (`Db`, `Router`, `Server`, `Templates`, etc.).

## Module map (`rustio-admin`)

| Path | Responsibility |
|---|---|
| `error.rs`        | `Error` / `Result` + `From<sqlx::Error>` mapping (constraint violations → 409 Conflict). |
| `http.rs`         | `Request`, `Response`, `FormData`, typed `Context` for per-request middleware data. |
| `router.rs`       | Tiny middleware-aware router; `:param` captures; 404 vs 405 distinction. |
| `server.rs`       | hyper-based `Server` with graceful Ctrl-C shutdown + `serve_static`. |
| `orm.rs`          | `Db`, `DbOptions`, `Model`, `Value`, `Row`, generic CRUD helpers. Postgres-only. |
| `migrations.rs`   | Walks numerically prefixed `*.sql` files; transactional apply with a tracking table. |
| `templates.rs`    | minijinja loader with disk-override path; embedded admin templates baked via `include_str!`. |
| `background.rs`   | Tiny periodic task runner (currently only the session sweeper). |
| `middleware/`     | `logger`, `security_headers`, `rate_limit` (DashMap token bucket), `gzip`, `csrf_protect` (double-submit cookie). |
| `auth/`           | `Identity`, `Role` ladder, sessions, users (Argon2), per-model permissions with a 60-s cache. |
| `admin/`          | The whole admin panel — see below. |

### `admin/` submodules

| Path | Responsibility |
|---|---|
| `types.rs`        | `Admin` builder, `AdminEntry`, `AdminField`, `AdminModel` trait, `AdminTheme`, `SiteBranding`, `ListOpts`/`ListPage`, `AdminOps` trait + `CoreUserOps` stub for the synthetic User entry. |
| `modeladmin.rs`   | `ModelAdmin` trait + `Fieldset` + `SortDir` + `parse_order_spec`. |
| `ops.rs`          | `ConcreteOps<M>` — the single live `AdminOps` impl. Builds the SQL list-page query (WHERE / ORDER BY / LIMIT / OFFSET) with `M::COLUMNS` validation. |
| `routes.rs`       | `register_admin_routes`. Mounts every admin URL with permission/role guards. |
| `handlers.rs`     | `dashboard`, `list_model`, `show_*_form`, `do_create/update/delete`, `show_log_entries`, `show_password_change`, … one handler per URL. |
| `render.rs`       | Template context builders (`BaseContext`, `ListCtx`, `FormCtx`, …). All HTML stays in templates; this file produces only `serde::Serialize` data. |
| `builtin.rs`      | Bespoke pages for `/admin/users/*` and `/admin/groups/*` (richer than the generic CRUD — last-developer guards, perm-grid editor, profile-page tabs). |
| `audit.rs`        | `rustio_admin_actions` schema + `record` / `recent` / `for_object` queries. |
| `relations.rs`    | `RelationRegistry` built from `&[AdminEntry]` — drives the delete-confirm cascade list and FK widgets. |
| `filters.rs`      | `classify_field`, `field_ui_metadata`, `infer_filters` — pure helpers that turn schema metadata into UI hints. |
| `icons.rs`        | Inline SVG catalogue baked at compile time; exposed via the `icon()` minijinja function. |

## Templates

Located under `crates/rustio-admin/assets/templates/admin/`. Baked into the binary via `include_str!` in `templates.rs::EMBEDDED_TEMPLATES`. Disk overrides win at render time when a project sets `RUSTIO_TEMPLATE_DIR` (or passes a path to `Templates::new`).

```
assets/templates/admin/
├── _base.html             ← shell with named blocks
├── _topbar.html           ← partial: brand + identity + theme toggle
├── _sidebar.html          ← partial: nav with model entries + auth links
├── _theme.html            ← partial: <style> tag injecting CSS custom properties
├── includes/
│   ├── _form_field.html   ← single render path for every widget
│   └── _field_errors.html
├── login.html
├── index.html             ← dashboard
├── list.html              ← changelist with sortable headers + filter chips + pagination
├── form.html              ← generic create/edit form
├── confirm_delete.html
├── error.html / forbidden.html
├── object_history.html / log_entries.html
├── password_change.html
├── users_list.html / user_new.html / user_edit.html / user_view.html / user_confirm_delete.html
└── groups_list.html / group_new.html / group_edit.html / group_confirm_delete.html
```

## CSS + JS

Located under `crates/rustio-admin/assets/static/`. Single hand-written stylesheet (~715 LOC) and a minimal JS file (~91 LOC) for theme toggle + sidebar drawer. **No build step.** The CSS uses six CSS custom properties (`--rio-accent`, `--rio-bg`, `--rio-surface`, `--rio-text`, `--rio-text-muted`, `--rio-border`) which the `_theme.html` partial injects from the active `AdminTheme`; project re-skins are one `Admin::theme(...)` call away.

Three responsive breakpoints (mobile-first):

| Breakpoint | Layout |
|---|---|
| `< 768px`         | Single column. Sidebar collapsed off-canvas behind a hamburger. Tables horizontally scrollable. |
| `≥ 768px`         | Two-column. Sidebar pinned. |
| `≥ 1280px`        | Wider sidebar, more padding. |

Dark mode flips on either `prefers-color-scheme: dark` (OS-level) or `<html data-rio-theme="dark">` (manual toggle, persisted to `localStorage`).

## Public API surface

Stable export points (in import order of how a project usually meets them):

```rust
use rustio_admin::{
    // Core types
    Db, DbOptions, Model, Row, Value,
    Router, Server, Response, Request, FormData,
    Error, Result,

    // Auth
    Identity, Role,
    auth, background, middleware, migrations,

    // Admin
    Admin, AdminField, AdminModel, FieldType,
    Fieldset, ModelAdmin,
    register_admin_routes,

    // Macros
    RustioAdmin,
};

// Sub-namespaces a project may reach into:
use rustio_admin::admin::{AdminTheme, SiteBranding, UserProfileRow, UserProfileSection};
use rustio_admin::templates::Templates;
```

Everything else (`ConcreteOps`, `AdminOps`, `ListOpts`, `BaseContext`, …) is `pub(crate)` or `pub` within `admin::*` for testing only — projects never touch it directly.

## Hard architectural rules

The full list lives in [§8 of the strategic reset plan](../rustio-admin-strategic-reset-plan.md#8-strict-architectural-rules). The CI pipeline enforces **rule #1** (no Tier 2 features) with a `git grep` guard on every PR:

```yaml
forbidden='HasSchema|ModelSchema|RustType|SchemaOps|from_schema|contract_validator|contract_doctor|RustioModel'
```

If you need any of those for a project, the right layer is a future `rustio-pro` crate — not this one.

Other rules summarised:

- **Postgres only.** No SQLite branches, no MySQL branches, no abstraction over databases.
- **No second runtime.** `ConcreteOps<M>` is the runtime. No bridges, no schema-driven sibling.
- **No magic.** Macros emit obvious code; `cargo expand` should always show the full picture.
- **Templates and CSS are hand-written.** No Tailwind, no PostCSS, no Sass, no build step.
- **Boring software.** Prefer obvious code over clever code. Prefer explicit over inferred. Prefer compile-time errors over runtime errors.
- **Every public type and method has a doc comment.** No silent contracts.
