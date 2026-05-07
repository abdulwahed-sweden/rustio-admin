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
├── _base.html                  ← shell with named blocks; inline theme bootstrap script
├── _topbar.html                ← partial: brand + identity + theme toggle
├── _sidebar.html               ← partial: nav with model entries + auth links
├── _theme.html                 ← partial: <style> tag injecting AdminTheme overrides
├── includes/
│   ├── _form_field.html        ← single render path for every widget
│   └── _field_errors.html
├── login.html
├── index.html                  ← dashboard
├── list.html                   ← changelist (toolbar + filters + sort + per-page + pills + numbered pagination + bulk select)
├── form.html                   ← generic create/edit form
├── confirm_delete.html
├── bulk_confirm_delete.html    ← built-in cascade-aware bulk delete confirm
├── bulk_confirm_action.html    ← generic confirm page for project-defined bulk actions
├── error.html / forbidden.html
├── object_history.html / log_entries.html
├── password_change.html
├── users_list.html / user_new.html / user_edit.html / user_view.html / user_confirm_delete.html
└── groups_list.html / group_new.html / group_edit.html / group_confirm_delete.html
```

## CSS + JS

Located under `crates/rustio-admin/assets/static/`. Single hand-written stylesheet (~1.9k LOC) and a minimal JS file (~210 LOC) for theme toggle + sidebar drawer + dropdown wiring + bulk select. **No build step.**

The stylesheet is the single source of truth for every design token — light defaults at `:root`, dark variants under both `@media (prefers-color-scheme: dark)` and `html[data-rio-theme="dark"]`, and a symmetric `[data-rio-theme="light"]` block so an explicit user toggle while the OS is dark renders correctly. The token surface is wider than 0.1.x:

- **Six override-point tokens** that `AdminTheme` can patch: `--rio-accent`, `--rio-bg`, `--rio-surface`, `--rio-text`, `--rio-text-muted`, `--rio-border`.
- **Surface ladder** (`--rio-surface`, `--rio-surface-2`, `--rio-surface-3`) for layered depth.
- **Text scale** (`--rio-text-strong`, `--rio-text`, `--rio-text-muted`, `--rio-text-subtle`) for clear hierarchy.
- **Border scale** (`--rio-border-soft`, `--rio-border`, `--rio-border-strong`) for hover / dividers / focus rings.
- **Typography scale** (sizes, line-heights, weights, family fallback chains).
- **Spacing scale** (`--rio-s1` through `--rio-s7`).
- **Three soft shadows** (xs / regular / lg) at `0.04–0.10` alpha.
- **Brand accent semantics** plus **success / warning / danger / info** with per-theme tuned variants.

`_theme.html` is **purely an override patch** — when `AdminTheme` has no fields set, the partial emits no markup at all and `admin.css` is the only style source. When fields are set, the partial emits a `<style>` block *after* `<link rel="stylesheet" href="/static/admin.css">` in `_base.html` with selector list `html, html[data-rio-theme="light"], html[data-rio-theme="dark"]` so an override wins cascade ties on source order without needing `!important`. Project re-skins are one `Admin::accent_color("…")` call away — the framework's own dark-mode resolution still applies for tokens you didn't override.

Three responsive breakpoints (mobile-first):

| Breakpoint | Layout |
|---|---|
| `< 768px`  | Single column. Sidebar off-canvas behind a hamburger. Tables horizontally scrollable. |
| `≥ 768px`  | Two-column flex row. Sidebar `position: sticky` below the topbar with its own internal scroll. |
| `≥ 1280px` | Wider sidebar, more padding. Main content capped at 1280px so wide monitors don't sprawl table rows. |

Dark mode flips on either `prefers-color-scheme: dark` (OS-level) or `<html data-rio-theme="dark">` (manual toggle, persisted to `localStorage`). The mode is resolved by an inline bootstrap script in `<head>` *before* CSS loads, so the chosen mode lands on the first paint with no flash-of-light-on-dark.

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
    BulkAction, Fieldset, ModelAdmin,
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
