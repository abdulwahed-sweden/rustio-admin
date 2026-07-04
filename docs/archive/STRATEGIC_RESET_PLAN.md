# RustIO Admin — Strategic Reset Architecture & Extraction Plan

> **Update 2026-05-07.** Adopted in full and shipped as `v0.1.0` on
> [crates.io](https://crates.io/crates/rustio-admin) and the
> [v0.1.0 GitHub release](https://github.com/abdulwahed-sweden/rustio-admin/releases/tag/v0.1.0).
> Kept in-repo as the canonical reference for the framework's
> non-goals, the Tier-2 discard list (which the CI grep guard
> enforces on every PR), and the §8 architectural rules.

> Saved 2026-05-06. Source: strategic reset session for the RustIO ecosystem.
> This document is a design, not a coding plan. Approve, modify, or reject before any code moves.

---

## 1. New Repository Vision

### Mission

> **"Django Admin, but for Rust."**
> A small, focused, beautiful admin framework that lets a Rust developer go from a `struct` to a working CRUD admin in under 50 lines of project code.

The framework's job is **the admin panel and the auth/permissions/templates that surround it**. Not "build a backend." Not "manage your data infrastructure." Admin.

### Non-goals (explicit)

- Not a full web framework. We ship a router because the admin needs one, not because we want users routing their app through us.
- Not an ORM. The `Model` trait is a thin shim over sqlx; users keep writing SQL where it matters.
- Not multi-database. **Postgres only.** Forever, in Tier 1.
- Not a Django port. Django-inspired ergonomics; Rust-native idioms.
- Not a content management system. Static pages, page-builder UIs, plugins — out of scope.
- Not cloud-native, not multi-tenant, not service-mesh-aware.
- Not "AI-augmented" in any way.

### Architectural boundaries

```text
┌─────────────────────────────────────────────────┐
│  IN SCOPE — Tier 1 (this repo)                  │
│  ─────────────────────────────                  │
│  • #[derive(RustioAdmin)]                       │
│  • impl Model (manual, hand-written)            │
│  • Auto-generated CRUD pages                    │
│  • Auth: users, sessions, groups, permissions   │
│  • Pagination, sorting, filtering, ILIKE search │
│  • ModelAdmin trait (Django-style customisation)│
│  • Templates with project-override path         │
│  • Single hand-written CSS file                 │
│  • Migrations runner (alphabetical SQL files)   │
│  • CSRF, rate-limit, gzip, security headers     │
│  • Audit log                                    │
└─────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────┐
│  OUT OF SCOPE — Tier 2 (separate repo, later)   │
│  ─────────────────────────────                  │
│  • Schema contract types (RustType, ModelColumn)│
│  • #[derive(RustioModel)]                       │
│  • Runtime PG drift validator                   │
│  • Doctor subprocess hook                       │
│  • Schema-driven SchemaOps runtime              │
│  • Admin/search bridges                         │
│  • Validator-gated Meili wiring                 │
│  • AI planner / executor / review              │
│  • Cloud / multi-tenancy / orchestration        │
└─────────────────────────────────────────────────┘
```

### What is intentionally excluded

| Excluded | Why |
|---|---|
| `RustioModel`, `HasSchema`, `ModelSchema`, `RustType`, `SchemaFlags` | Tier 2 metadata system; admin doesn't need it |
| `contract_validator`, `contract_doctor`, `--check-schema` flag | Drift detection is a Tier 2 production concern |
| `SchemaOps`, `Admin::from_schema(s)` | Second runtime; the manual one is enough |
| `admin/from_schema.rs` (1490 LOC) | Bridge layer between two metadata systems we don't have |
| `search/from_schema.rs` (770 LOC) | Validator-gated Meili wiring |
| `ai/`, `ai_gen/` | Tier 2 / experimental |
| `cache.rs` | Performance concern, defer |
| Meili integration (Tier 1 default) | Tier 1 ships ILIKE-based search built into list pages; Meili moves to Tier 2 or behind `search-meili` feature flag |
| Demo-mode bootstrap (`bootstrap_default_groups`, demo users) | Useful but not core — moves to a CLI fixture command |
| `admin/suggestions.rs`, `admin/intelligence::classify_search`, `admin_intelligence_tests` | Heuristic engines we don't need yet |

---

## 2. Repository Naming Strategy

### Recommended

**New repository: `rustio-admin`** (a fresh repo, fresh git history). The name is descriptive — anyone reading it knows what they're getting.

**Crate layout (in the new repo):**

| Crate | Purpose |
|---|---|
| `rustio-admin` | The library. Re-exports the macros. The thing users `cargo add`. |
| `rustio-admin-macros` | Proc-macro crate. Hidden from users (re-exported from `rustio-admin`). |
| `rustio-admin-cli` | The `rustio` (or `radmin`) binary. Scaffolds projects + apps, runs migrations, manages users. |

**Future Tier 2 crates** (separate repo `rustio-pro` when/if commercial tier launches):

| Crate | Purpose |
|---|---|
| `rustio-pro-contract` | Schema contract types (port of `contract.rs`) |
| `rustio-pro-validator` | PG drift validator |
| `rustio-pro-search` | Validator-gated Meili integration |
| `rustio-pro-ai` | AI planner / migrations co-pilot |

**Old repository (`abdulwahed-sweden/rustio`):**
- **Keep on GitHub** — don't delete; the schema-contract work is genuinely valuable as Tier 2 raw material.
- **Rename to `rustio-archive` or `rustio-pro-staging`** so it stops being authoritative.
- **Mark `master` branch as `archive/v1.10`**; freeze.
- **Pin a deprecation notice** in its README pointing at `rustio-admin`.

### Package naming convention

- `rustio-admin-*` for Tier 1 crates (this repo)
- `rustio-pro-*` for Tier 2 crates (future repo)
- Binary: `rustio` (matches the brand, single short command)
- The user-facing namespace stays `rustio_admin::*` in code (`use rustio_admin::admin::Admin`)

### Why a fresh repo, not a branch

A reset on `main` of the old repo means everyone with `cargo install` or `git clone` of v1.10 gets surprised. Fresh repo = clean signal, clean version numbering (`0.1.0` from day one), clean issue tracker, clean README. The old code stays accessible for reference; the new repo signals "this is the framework."

---

## 3. Extraction Audit

Per-subsystem decision matrix. **Total LOC budget: ≤8,000 for the lib crate** (vs current 45,618).

### Admin

| Component | Decision | Notes |
|---|---|---|
| `admin/types.rs` (`Admin`, `AdminField`, `AdminEntry`, `AdminModel`, `AdminTheme`, `SiteBranding`) | **Copy → Simplify** | Drop `from_schema(s)` builders, drop `SchemaOps` integration, drop `search_hook` (moves to optional Tier 2 search). Keep `ConcreteOps` (the manual runtime). |
| `admin/types.rs::ConcreteOps` | **Copy verbatim** | The manual runtime is the runtime. |
| `admin/types.rs::SchemaOps` | **Discard** | Tier 2. |
| `admin/from_schema.rs` (1490 LOC) | **Discard** | Tier 2 bridge. |
| `admin/handlers.rs` | **Copy → Simplify** | Drop schema-runtime detection branches. Centralise audit calls. |
| `admin/render.rs` (4563 LOC) | **Copy → Heavily simplify** | Drop schema-bridge code. Target ≤1500 LOC. |
| `admin/routes.rs` | **Copy → Simplify** | Drop `--rustio-doctor-schema-check` related routing. |
| `admin/builtin.rs` | **Copy** | User/group bespoke pages. |
| `admin/audit.rs` | **Copy** | Already lean. |
| `admin/intelligence.rs` | **Copy → Simplify** | Keep `classify_field`, `field_ui_metadata`, `infer_filters`, `format_relation_cell`, `mask_pii`. **Drop** `classify_search`, `SearchIntent`. |
| `admin/relations.rs` | **Copy** | FK display registry; needed. |
| `admin/icons.rs` | **Copy** | Cheap. |
| `admin/suggestions.rs` | **Discard** | Heuristic engine; not core. |
| `admin/entry_builder.rs` | **Copy → Simplify** | Drop schema-driven branches. |
| `admin/macro_tests.rs`, `admin_intelligence_tests.rs`, etc. | **Copy selectively** | Keep tests for what we keep. |
| **NEW: `admin/modeladmin.rs`** | **Write fresh** | Django-style customisation trait (see Section 4). |

### Auth

| Component | Decision | Notes |
|---|---|---|
| `auth/users.rs` | **Copy** | Argon2 hashing, lookup, create. |
| `auth/sessions.rs` | **Copy** | DB-backed sessions. |
| `auth/groups.rs` | **Copy** | Group + membership. |
| `auth/permissions.rs` | **Copy** | Per-model `add/change/delete/view` permission seed. |
| `auth/role.rs` | **Copy** | `Role` enum. |
| Demo bootstrap (`bootstrap_default_groups`, `bootstrap_demo_users`, `lazy_attach_permissions`) | **Move to CLI fixture command** | `rustio fixtures load demo` rather than baked into core. |

### Templates

| Component | Decision | Notes |
|---|---|---|
| `templates.rs` | **Copy** | minijinja loader with project override. |
| `assets/templates/admin/base.html` (3545 LOC) | **Rewrite** | Split into `_base.html` shell (≤400 LOC), `_topbar.html`, `_sidebar.html`, `_theme.html` partials. |
| `assets/templates/admin/list.html` (525 LOC) | **Rewrite** | Aim for ≤300 LOC. Drop schema-runtime conditional branches. |
| `assets/templates/admin/{form,index,login,confirm_delete,...}.html` | **Copy → Simplify** | Light pass for consistency. |
| `assets/templates/admin/user_view.html` (629 LOC) | **Copy → Simplify** | Keep `project_user_fields` block for project extension. |
| Tailwind compilation step (`tailwind.config.js`, `package.json`, `node_modules/`) | **Discard** | Hand-written CSS instead. |

### Routing / HTTP / Server

| Component | Decision | Notes |
|---|---|---|
| `router.rs`, `server.rs`, `http.rs` | **Copy** | Minimal hyper glue; already lean. |
| `middleware/{rate_limit, logger, security_headers, compression, csrf}.rs` | **Copy** | Solid as-is. |
| `background.rs` (housekeeping session sweep) | **Copy** | Trivial, useful. |

### ORM

| Component | Decision | Notes |
|---|---|---|
| `orm.rs` (`Db`, `Model`, `Value`, `Row`) | **Copy** | The thin sqlx shim. |
| `cache.rs` (DB read cache) | **Discard** | Performance concern; defer. |

### Macros

| Component | Decision | Notes |
|---|---|---|
| `RustioAdmin` derive (lines 1–509) | **Copy → Simplify** | Drop `DateTimeAuto` complexity if not needed; otherwise keep as-is. |
| `RustioModel` derive (lines 510–1121) | **Discard** | Tier 2. |

### Search

| Component | Decision | Notes |
|---|---|---|
| `search/traits.rs` (`Searchable`) | **Move behind `meili` feature flag, or discard** | Probably discard for v0.1; ship ILIKE-only. Add feature flag later. |
| `search/client.rs`, `indexer.rs` | **Same: feature-flag or discard** | |
| `search/from_schema.rs` | **Discard** | Tier 2. |
| **NEW: ILIKE search built into list pages** | **Write fresh** | `ModelAdmin::search_fields()` → `WHERE col ILIKE '%q%' OR col2 ILIKE '%q%'`. Postgres-only, simple. |

### Permissions / RBAC

| Component | Decision | Notes |
|---|---|---|
| Per-model permission generation, group membership, `perm_guard` middleware | **Copy** | Core feature. |

### Schema runtime, contracts, validator, Meili wiring

| Component | Decision |
|---|---|
| `contract.rs`, `contract_validator.rs`, `contract_doctor.rs` | **Discard** |
| `admin/from_schema.rs`, `search/from_schema.rs` | **Discard** |
| `tests/contract_validator_pg.rs`, `tests/macro_rustio_model.rs` | **Discard** |

### CLI

| Component | Decision | Notes |
|---|---|---|
| `rustio-cli/src/main.rs` (708 LOC after session changes) | **Copy → Simplify** | Drop `--check-schema` orchestration, drop schema-runtime scaffolding. Keep `startproject`, `startapp`, `migrate`, `user`, `group`, `perm`. Target ≤500 LOC. |
| `rustio-cli/src/doctor.rs` (211 LOC) | **Heavily simplify** | "Can I connect to the DB? Are migrations applied? Is the admin user table populated?" — that's it. ≤80 LOC. |
| `rustio-cli/src/version_check.rs` | **Defer** | Not core. |

### Misc

| Component | Decision |
|---|---|
| `error.rs` | **Copy** |
| `schema.rs` (AI-layer schema export) | **Discard** |
| `ai/`, `ai_gen/` | **Discard** |
| `playground/`, `docs/phases/`, `Makefile` | **Don't bring; selectively re-add** |

### Resulting LOC estimate

| Crate | Old (relevant lines) | New target |
|---|---|---|
| `rustio-admin` (lib) | ~30,000 (after extraction) | **≤7,000** |
| `rustio-admin-macros` | 509 (RustioAdmin only) | ~500 |
| `rustio-admin-cli` | ~400 (after simplification) | **≤500** |
| Templates | ~5,900 LOC | **≤2,500** |

---

## 4. Clean Architecture Blueprint

### Directory layout

```text
rustio-admin/
├── Cargo.toml                           workspace
├── README.md
├── LICENSE
├── CHANGELOG.md
├── crates/
│   ├── rustio-admin/                    the library
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs                   re-exports + module list
│   │   │   ├── error.rs
│   │   │   ├── http.rs
│   │   │   ├── router.rs
│   │   │   ├── server.rs
│   │   │   ├── orm.rs                   Db, Model, Value, Row
│   │   │   ├── migrations.rs
│   │   │   ├── templates.rs
│   │   │   ├── background.rs            session sweep
│   │   │   ├── middleware/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── rate_limit.rs
│   │   │   │   ├── logger.rs
│   │   │   │   ├── security_headers.rs
│   │   │   │   ├── compression.rs
│   │   │   │   └── csrf.rs
│   │   │   ├── auth/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── users.rs
│   │   │   │   ├── sessions.rs
│   │   │   │   ├── groups.rs
│   │   │   │   ├── permissions.rs
│   │   │   │   └── role.rs
│   │   │   ├── admin/
│   │   │   │   ├── mod.rs               re-exports
│   │   │   │   ├── types.rs             Admin, AdminEntry, AdminField, AdminModel, AdminTheme, SiteBranding
│   │   │   │   ├── modeladmin.rs        ModelAdmin trait (NEW)
│   │   │   │   ├── ops.rs               ConcreteOps<M> (the runtime)
│   │   │   │   ├── routes.rs            URL → handler
│   │   │   │   ├── handlers.rs          list/new/create/edit/update/delete handlers
│   │   │   │   ├── render.rs            template context builders
│   │   │   │   ├── builtin.rs           user/group bespoke pages
│   │   │   │   ├── audit.rs             audit log
│   │   │   │   ├── relations.rs         FK display registry
│   │   │   │   ├── filters.rs           list_filter helpers (was intelligence::infer_filters)
│   │   │   │   ├── search.rs            ILIKE-based list search
│   │   │   │   └── icons.rs
│   │   ├── assets/
│   │   │   ├── templates/admin/
│   │   │   │   ├── _base.html           shell (≤400 LOC)
│   │   │   │   ├── _topbar.html         partial
│   │   │   │   ├── _sidebar.html        partial
│   │   │   │   ├── _theme.html          partial (CSS var override block)
│   │   │   │   ├── index.html
│   │   │   │   ├── list.html
│   │   │   │   ├── form.html
│   │   │   │   ├── login.html
│   │   │   │   ├── confirm_delete.html
│   │   │   │   ├── user_*.html
│   │   │   │   └── group_*.html
│   │   │   └── static/
│   │   │       ├── admin.css            single hand-written stylesheet (~600 LOC)
│   │   │       └── admin.js             minimal JS for sortable headers + filter chips (≤200 LOC)
│   │   └── tests/
│   ├── rustio-admin-macros/             proc-macro crate
│   │   └── src/lib.rs                   #[derive(RustioAdmin)] only
│   └── rustio-admin-cli/                CLI binary
│       └── src/
│           ├── main.rs
│           ├── scaffold.rs              startproject + startapp
│           └── doctor.rs                tiny health-check
└── examples/
    └── minimal/                         <50-line consumer demo
        └── src/main.rs
```

### Why this shape

- **Single library crate, single proc-macro crate, single CLI crate.** No "core / admin / extra" split. The framework is small enough to fit in one crate.
- **Module-per-concern under `admin/`** — every concept has one file, every file ≤500 LOC where possible.
- **No bridges, no two-runtime trait dispatch.** `ConcreteOps<M>` is **the** runtime. `AdminOps` trait might still exist (it's how `Admin::find` returns a polymorphic entry), but every implementation goes through `ConcreteOps`.
- **`ModelAdmin` is the customisation surface.** Default blanket impl means simple cases need zero customisation; advanced cases override only what they need. Mirrors Django's `ModelAdmin` class.

### The `ModelAdmin` trait (the heart of the new ergonomic)

```rust
pub trait ModelAdmin: AdminModel {
    /// Columns shown on the list page, in order. Default: every field.
    fn list_display() -> &'static [&'static str] { &[] /* meaning "all" */ }

    /// Columns offered as filter chips in the sidebar. Default: none.
    fn list_filter() -> &'static [&'static str] { &[] }

    /// Columns searched by the list-page search box (ILIKE). Default: none.
    fn search_fields() -> &'static [&'static str] { &[] }

    /// Default ordering. `-foo` for DESC, `foo` for ASC. Default: `-id`.
    fn ordering() -> &'static [&'static str] { &["-id"] }

    /// Rows per page on the list view. Default: 50.
    fn list_per_page() -> usize { 50 }

    /// Read-only fields on the change form. Default: none.
    fn readonly_fields() -> &'static [&'static str] { &[] }

    /// Field grouping on the change form. Default: one fieldset of all fields.
    fn fieldsets() -> &'static [Fieldset] { &[] }
}

/// Blanket default — every AdminModel becomes a ModelAdmin with sensible defaults.
impl<T: AdminModel> ModelAdmin for T {}
```

A user wanting custom behaviour writes:

```rust
struct CourseAdmin;
impl ModelAdmin for Course {
    fn list_display() -> &'static [&'static str] { &["code", "title", "credit_hours", "is_published"] }
    fn list_filter() -> &'static [&'static str] { &["status", "level", "is_published"] }
    fn search_fields() -> &'static [&'static str] { &["code", "title"] }
    fn ordering() -> &'static [&'static str] { &["code"] }
}
```

This is the killer ergonomic. Every Django dev recognises it instantly.

### What's deliberately not abstracted

- **The `Model` trait stays boring.** `TABLE`, `COLUMNS`, `INSERT_COLUMNS`, `from_row`, `insert_values`, `id`. Hand-written. Six fields, no magic.
- **No metadata convergence.** `AdminField` is the metadata; `Model` is the data shape. They don't merge.
- **No "smart" inference.** If you want a column shown, you say so. If you want a filter, you list it.
- **No automatic schema validation.** If your struct doesn't match your migration, you get a sqlx error at query time. That's enough for Tier 1.

---

## 5. Public API Philosophy

### Explicit (the user spells it out)

- Model registration: `.model::<Course>()`
- Migration order: numerical filename prefix
- Theme values: hex strings, no auto-generation
- `ModelAdmin` overrides: every column listed by name
- Permission grants: per-group, per-permission codename

### Convention-based (free defaults)

- Struct field name → SQL column name (snake_case identical match)
- Struct name → admin name (`Course` → `courses`)
- `*_id` columns → FK detection in `relations.rs`
- Label field heuristic: `name`, `title`, `full_name`, `label`, `email` (first match)
- `created_at`, `updated_at` → auto-managed by macro, hidden from forms

### Configurable (overridable)

- `SiteBranding` — title, header, footer, domain
- `AdminTheme` — six colour fields
- `ModelAdmin` — every list/form behaviour
- Templates — drop a file at `templates/admin/<name>.html` to override

### NOT abstracted (intentionally)

- Database choice (Postgres only)
- HTTP server choice (hyper only)
- Template engine choice (minijinja only)
- ORM (sqlx, hidden behind `Model` trait)
- Auth backend (built-in, single backend)

The framework picks one good answer for each and stops there. Pluggability is a Tier 2 concern.

---

## 6. HTML / UI Strategy

### Template strategy

Single base shell (`_base.html`, ≤400 LOC) with named blocks for every customisation point. Three partials (`_topbar.html`, `_sidebar.html`, `_theme.html`) for the cross-cutting bits. Every page-specific template extends `_base.html`.

Override path: drop `templates/admin/foo.html` in the project; it shadows the embedded version. Single-path override; no `overrides/` directory.

### CSS strategy

**One hand-written CSS file: `admin.css`, ~600 LOC**. No Tailwind. No PostCSS. No `package.json`. CSS variables for theming — `Admin::theme(...)` injects a `<style>` block in `_theme.html` that overrides the variables.

CSS structure:
- `:root { --rio-bg, --rio-surface, --rio-text, --rio-text-muted, --rio-border, --rio-accent; }` — six variables
- Layout primitives: `.rio-shell`, `.rio-topbar`, `.rio-sidebar`, `.rio-main`, `.rio-card`
- Form primitives: `.rio-field`, `.rio-input`, `.rio-textarea`, `.rio-select`, `.rio-checkbox`, `.rio-button`, `.rio-button--primary`, `.rio-button--danger`
- Table primitives: `.rio-table`, `.rio-table-row`, `.rio-table-cell`, `.rio-table--striped`
- Filter primitives: `.rio-filter-bar`, `.rio-filter-chip`, `.rio-search-box`
- Pagination: `.rio-pagination`, `.rio-pagination-link`

### Responsive strategy

Mobile-first, three breakpoints:
- **Default (mobile, <768px):** single-column, sidebar collapsed behind a hamburger menu, tables horizontally scrollable.
- **Tablet (≥768px):** two-column with collapsible sidebar.
- **Desktop (≥1280px):** full layout, sidebar pinned.

CSS Grid + Flex; **no media-query libraries**. Container queries optional if browser support is acceptable at release time.

### Dark mode strategy

`prefers-color-scheme: dark` flips the CSS variables to a dark palette. Manual toggle in the topbar stores preference in `localStorage` and applies `data-theme="dark"` on `<html>`. Two ~30-line CSS blocks; no JS framework needed.

### Mobile behavior

- Sidebar becomes a slide-in drawer behind a hamburger.
- Tables get `overflow-x: auto` with sticky first column for context.
- Forms become single-column.
- Filter sidebar becomes a top filter sheet (slide-down).
- Action menus collapse to a bottom action bar on small screens (sticky bottom).

### Admin UX priorities (in order)

1. **Speed.** Server render budget: ≤30ms. First paint: ≤100ms. No client-side hydration.
2. **Clarity.** Every action has a text label. No icon-only buttons. Empty states explain what to do.
3. **Consistency.** Every list page identical shape. Every form identical shape. Every confirm-delete page identical shape.
4. **Restraint.** Monochrome + one accent colour. Animations limited to 150ms opacity/transform fades. No carousels, no auto-playing anything.
5. **Keyboard.** Tab order makes sense. `Esc` cancels. `Enter` submits. `/` focuses the search box (if present).

### Visual language

- **Type:** Geist Sans for headings, Geist Mono for code, system stack for body. (Or fall back fully to system stack if hosting Geist is awkward.)
- **Spacing:** 4 / 8 / 12 / 16 / 24 / 32 / 48 px scale. No other values.
- **Borders:** 1px hairlines for structure. `box-shadow` reserved for dropdowns and modals only.
- **Colour:** Cobalt #2563EB default accent. Six-token palette overridable via `AdminTheme`.
- **Motion:** opacity + transform only. Never layout-shifting animations.

---

## 7. Phase 1 Roadmap (First 14 Days)

Realistic for one focused developer. Daily commits, daily browser test against `examples/minimal`.

### Day 1 — Bootstrap

- Create `rustio-admin` repo on GitHub.
- Add `Cargo.toml` workspace with three empty crates.
- Add `LICENSE` (MIT), `README.md` (one paragraph: "Django Admin, but for Rust"), `.gitignore`.
- Add CI workflow (copy from old repo): `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt --check`.
- Add `examples/minimal/` with a Hello-World binary that prints "rustio-admin alive" but doesn't yet compile against any framework code.

### Day 2 — Skeleton API

- In `rustio-admin/src/lib.rs`: define empty modules (`error`, `http`, `router`, `server`, `orm`, `migrations`, `templates`, `auth`, `admin`).
- Sketch `Admin::new()`, `.model::<T>()`, `register_admin_routes()` signatures (compile, panic at runtime is fine).
- Goal: `examples/minimal` compiles and links against the skeleton.

### Day 3 — Core extraction

- Copy `error.rs`, `http.rs`, `router.rs`, `server.rs`, `migrations.rs`, `templates.rs` from old repo.
- Strip imports that reference Tier 2 modules.
- Verify `examples/minimal` boots an HTTP server with no admin yet.

### Day 4 — ORM + middleware

- Copy `orm.rs` (Db, Model, Value, Row).
- Copy `middleware/{rate_limit, logger, security_headers, compression, csrf}.rs`.
- Copy `background.rs`.
- Wire middleware into `examples/minimal`.

### Day 5 — Auth

- Copy `auth/{users, sessions, groups, permissions, role, mod}.rs`.
- Strip `bootstrap_default_groups`, `bootstrap_demo_users`, `lazy_attach_permissions` (move to CLI fixtures later).
- `examples/minimal` now runs `auth::init_tables` at startup.

### Day 6 — Admin: types + macros

- Copy `admin/types.rs` (slim version: drop `from_schema(s)`, drop `SchemaOps`).
- Copy `rustio-admin-macros/src/lib.rs` from old `rustio-macros` (RustioAdmin half only).
- `examples/minimal` defines a `Post` struct with `#[derive(RustioAdmin)]` + manual `impl Model`. Does not register yet.

### Day 7 — Admin: runtime + handlers

- Copy `admin/ops.rs` (`ConcreteOps<M>`).
- Copy `admin/handlers.rs`, `admin/routes.rs`, `admin/render.rs` (slim).
- Copy `admin/builtin.rs`, `admin/relations.rs`, `admin/filters.rs` (renamed from `intelligence`), `admin/audit.rs`, `admin/icons.rs`.
- `examples/minimal` registers `Post`, mounts admin, browser walk: list/create/edit/delete works.

### Day 8 — Templates rewrite, part 1

- Rewrite `_base.html`: split topbar, sidebar, theme into partials.
- Browser-test `examples/minimal`: every page renders.

### Day 9 — Templates rewrite, part 2

- Rewrite `list.html`, `form.html`, `index.html`, `confirm_delete.html`, `login.html`.
- Aim for ≤300 LOC each.
- Drop schema-runtime conditional branches.

### Day 10 — CSS + JS

- Hand-write `admin.css` (~600 LOC) with the six-variable theming + responsive layout + dark mode.
- Hand-write `admin.js` (≤200 LOC) for sortable header clicks + filter chip toggle + theme toggle.
- Browser-test on desktop + mobile + dark mode.

### Day 11 — `ModelAdmin` trait + pagination + ordering

- Define `ModelAdmin` trait with blanket default impl.
- Wire `list_per_page` + `?page=N` into `ConcreteOps::list` (LIMIT/OFFSET).
- Wire `ordering()` + sortable-header URL params (`?sort=col&dir=desc`) into the list query.
- Browser-test pagination + sort headers.

### Day 12 — Filters + search

- Wire `list_filter()` + `?filter[col]=val` URL params into list query (Postgres `WHERE col = $1`).
- Wire `search_fields()` + `?q=term` into list query (`WHERE col ILIKE '%term%' OR ...`).
- Browser-test combinations.

### Day 13 — `classrooms` integration

- In the `classrooms` repo (separate, in `~/Documents/classrooms`), bump dependency from `rustio-core = "1.10"` to `rustio-admin = "0.1.0-alpha"` (path dependency for now).
- Replace imports: `rustio_core::*` → `rustio_admin::*`.
- Drop `RustioModel` derives if any leaked (none did, per the audit).
- Add `ModelAdmin` impls for the 8 models, exercising `list_display`, `list_filter`, `search_fields`, `ordering`.
- Browser walk every page.

### Day 14 — Docs + first release

- Write `README.md`: tagline, install, three-line example, link to docs.
- Write `docs/getting-started.md`: from `cargo new` to running admin in 10 minutes.
- Write `docs/modeladmin.md`: full `ModelAdmin` reference with every method documented.
- Write `docs/architecture.md`: the brief module map.
- Tag `v0.1.0`. Publish to crates.io.

### What is NOT in Phase 1 (future work)

- ILIKE search optimisation (full-text, ranking) — Phase 2.
- Bulk actions on list pages — Phase 2.
- Inline forms (Django's `inlines`) — Phase 3.
- Dashboard widgets — Phase 3.
- Multi-locale templates — Phase 4.
- Tier 2 schema-contract integration — separate project, not Phase 1.

---

## 8. Strict Architectural Rules

These are PR-blocking. Violations are reverts.

1. **No Tier 2 features in core.** Schema contract, validator, doctor, AI, multi-tenancy — none. If a feature would belong in `rustio-pro-*`, it doesn't go here.
2. **No second runtime.** `ConcreteOps<M>` is the runtime. No bridges, no schema-driven sibling.
3. **No magic.** Macros emit obvious code; you can `cargo expand` and read it. No global state, no thread-locals, no implicit registration.
4. **Postgres only.** No SQLite branches, no MySQL branches, no abstraction over databases.
5. **No new dependency without justification.** New deps must (a) solve a real problem, (b) be reasonably small (transitive footprint <20 crates), (c) be in active maintenance. Approval gate per dep.
6. **No async beyond what hyper/sqlx require.** No own executors, no `async_std`, no task-pool exotica.
7. **Total Rust LOC budget: ≤8,000 for the lib crate.** Track in CI; if a PR pushes over, justify or split.
8. **Public API additions are minor versions; renames or removals are major.** Strict semver from `0.1.0` onward.
9. **Templates and CSS are hand-written.** No Tailwind, no PostCSS, no Sass, no build step.
10. **Embedded templates are the source of truth.** Project overrides via path, never via "extends" of a different template engine.
11. **Audit + post-write hooks live in `handlers.rs`, not `ConcreteOps`.** One canonical call site per write action.
12. **No experimental abstractions.** A new abstraction requires (a) two real consumers, (b) a documented use case, (c) a benchmark. No "we might need this later" code.
13. **Boring software.** Prefer obvious code over clever code. Prefer explicit over inferred. Prefer compile-time errors over runtime errors. Prefer one good answer over five configurable answers.
14. **The framework crate must compile in <30s on a clean machine.** Track in CI.
15. **Every public type and method has a doc comment.** No silent contracts.

---

## 9. Migration Strategy from Old Repository

### How to safely reuse code

- **File-by-file copy with audit.** For every file in the extraction list, the procedure is: `cp old/path new/path` → strip imports referencing Tier 2 modules → run `cargo check` → commit. Each extraction is its own commit, mentioning the source file path in the message body for traceability.
- **Use git for blame, not for history.** Don't try to preserve the old repo's git history via subtree merges. Fresh history. Reference old commit SHAs in extraction commit bodies (`extracted from rustio@bee2fce6:rustio-core/src/admin/types.rs`).
- **No `git subtree`, no `git filter-branch`.** The amount of Tier 2 code makes filtering more work than copying.

### How to avoid carrying architectural debt

- **Discard list (Section 3) is non-negotiable.** Anything on the discard list does NOT enter the new repo, even by accident. CI grep for forbidden symbols (`HasSchema`, `ModelSchema`, `RustType`, `SchemaOps`, `from_schema`, `contract_validator`, etc.) on every PR.
- **Single dependency on the old repo: zero.** New repo's `Cargo.toml` does not reference old repo's crates.
- **The new `Admin::new().model::<T>()` builder cannot accept a schema.** No `from_schema`/`from_schemas` methods. Compile-fail by absence.
- **Templates rewritten from scratch where they exceed 500 LOC.** No 1850-line monoliths.

### How to preserve useful lessons

Document them in `docs/lessons-learned.md` (in the new repo). Specifically:
- The `intelligence::infer_filters` heuristic — port the algorithm, port the tests, simplify the implementation.
- The `relations::RelationRegistry` design — port as-is.
- The post-write hook ordering issue — solve it correctly the first time (centralised in `handlers.rs`).
- The Tailwind-vs-hand-written CSS lesson — don't repeat.
- The "two metadata systems" mistake — don't repeat.
- The `Box::leak` for `'static` trick — keep, but minimal scope.

### How to isolate advanced systems

Old repo (`rustio-archive` or similar): rename, mark deprecated, freeze. Tier 2 work, if it ever resumes, lives there or in a fresh `rustio-pro` repo. Tier 1 (`rustio-admin`) **never** depends on Tier 2 code.

If/when Tier 2 launches, integration is via:
- Tier 2 crate adds an `Admin` extension trait that registers schema-driven entries alongside manual ones.
- `rustio-admin`'s public types are usable from Tier 2 without modification.
- Tier 2's existence does not affect Tier 1 users; they don't see the additional crate.

---

## 10. Final Recommendation

### What the FIRST coding step should be

**Day 1, Step 1: create the empty repository with the workspace skeleton + a 50-line `examples/minimal` consumer that defines exactly the API we want users to write — even though it doesn't compile yet.**

```rust
// examples/minimal/src/main.rs
use rustio_admin::admin::{Admin, register_admin_routes, AdminTheme, SiteBranding};
use rustio_admin::auth;
use rustio_admin::orm::Db;
use rustio_admin::router::Router;
use rustio_admin::server::Server;
use rustio_admin::middleware;

mod post;
use post::Post;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::connect(&std::env::var("DATABASE_URL")?).await?;
    auth::init_tables(&db).await?;

    let admin = Admin::new()
        .site_branding(SiteBranding::default())
        .theme(AdminTheme::default())
        .model::<Post>();
    admin.seed_permissions(&db).await?;

    let router = Router::new()
        .middleware(middleware::logger)
        .middleware(middleware::csrf_protect);
    let router = register_admin_routes(router, admin, db, templates);

    Server::new(router, "127.0.0.1:8000".parse()?).run().await?;
    Ok(())
}
```

The point: **the 50-line example IS the API contract.** Make it compile by Day 7 and you have shipped Phase 1's core. Don't let the framework grow features that this example doesn't need.

### What should NOT be touched in Week 1

- `RustioAdmin` macro internals (copy and forget).
- Auth tables and password hashing (copy and forget).
- Middleware implementations (copy and forget).
- The migrations runner (copy and forget).
- CSS aesthetics (do template structure first; pretty later).
- `classrooms` migration (don't touch until Day 13).

### Biggest risks

1. **Scope creep during extraction.** As you copy files, you'll be tempted to "improve this one thing". Don't. Copy verbatim, simplify only by deletion, defer all rewrites.
2. **Old-repo dependencies sneaking back.** Audit imports after each commit. CI grep for forbidden symbols on every PR.
3. **Template aesthetics absorbing time.** The template rewrite (Day 8–10) is the most subjective part; bound it. If `_base.html` isn't done in 1 day, ship the old one and iterate later.
4. **Naming bikeshed.** `rustio-admin` vs `radmin` vs `rusty-admin` — pick in 30 minutes, never revisit.
5. **CLI sprawl.** Don't add CLI commands you don't immediately need. `startproject`, `startapp`, `migrate apply`, `user create` — that's it for v0.1.

### Biggest opportunities

1. **A small focused crate is dramatically easier to adopt than a 45,000-LOC framework.** Rust developers are tired of "framework in a box" pitches that turn out to need 30 minutes of reading just to do CRUD. RustIO Admin can be the "10-minute admin" framework.
2. **Django Admin is the benchmark for backend admin UX.** Beating it on developer experience in Rust — same ergonomics, native types, 100× faster — is genuinely valuable.
3. **`classrooms` is a real consumer running real code.** Daily browser tests against it during the 14-day sprint catch UX bugs the test suite never will.
4. **The Phase 14/15 work isn't lost.** It's frozen in the old repo, ready as Tier 2 raw material whenever the commercial tier needs to launch. The reset doesn't waste that work; it just relocates it to where it belongs.
5. **The two-tier story is honest and clear.** "Free admin framework for Rust apps; Pro tier with schema validation, drift detection, AI-assisted migrations" is a credible commercial structure, easy to explain, and aligned with how Django itself is positioned (free admin; commercial layer is everyone else's value-add).

### One sentence summary

**Build a small, focused, beautiful Rust admin framework that fits in 8,000 lines of code, ships in 14 days, and doesn't apologise for being only an admin framework.** Everything else is Tier 2.
