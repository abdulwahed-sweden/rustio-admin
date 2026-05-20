# RustIO Roadmap

This document tracks the long-term direction of the RustIO ecosystem. It is written for contributors, project consumers, and anyone trying to understand what is real today versus what is on the planning surface.

The roadmap is **organised by topic, not by date.** RustIO does not promise calendar deadlines, and items are not labelled with version targets unless they are already in flight. Reorganisation is expected as the framework evolves.

For the canonical record of what shipped in each release, read [`CHANGELOG.md`](./CHANGELOG.md). For the project's foundational design rules, read [`docs/archive/STRATEGIC_RESET_PLAN.md`](./docs/archive/STRATEGIC_RESET_PLAN.md).

---

## Status Legend

| Symbol | Meaning |
|---|---|
| ✅ | **Implemented.** Live on the current release; covered by the test suite or browser-walked end-to-end. |
| 🟡 | **Partial.** Some surface ships today; clear gaps remain. The note describes what's there and what isn't. |
| ⚪ | **Planned.** Decided, scoped at the design level, awaiting an implementation slot. |
| 🔬 | **Experimental / Research.** Open question. May land in RustIO, may live in `rustio-pro-*`, may be rejected. |

---

## Philosophy

RustIO is a **small, focused admin framework for Postgres-backed Rust apps.** The design is intentionally narrow:

- **One database backend (Postgres), one runtime, one stylesheet.** No abstraction over databases, no second runtime that races the first, no Tailwind / PostCSS / Sass.
- **Boring software.** Prefer obvious code over clever code. Prefer compile-time errors over runtime errors. Macros emit code that `cargo expand` should make legible at a glance.
- **Single binary deploy.** Templates, CSS, JS, fonts, and SVG icons are baked in via `include_str!` / `include_bytes!`. No separate static-file server, no runtime asset compilation.
- **Calm, comfortable UI.** The chrome is designed for ten-hour sessions, not five-minute demos. Dark mode is graphite, not OLED black. Typography breathes.
- **Honest defaults.** Every public type has a doc comment. Every default is a deliberate choice, not an accident of the language.
- **Project users own their templates.** The framework ships defaults; projects override per file via the disk-loader path. There is no template-builder DSL, no shadow-DOM trick.

**Hard non-goals** (locked in at the strategic-reset level): schema contracts, drift validation, AI planners, multi-database backends, search backends. If those return, they ship as a separate `rustio-pro-*` family of crates, never inside `rustio-admin` itself.

---

## Current Foundation

The framework currently sits at **v0.2.1** (released 2026-05-07). The shape:

- Core admin runtime with list / create / edit / delete pages per registered Postgres model
- Built-in user / group / permission management at `/admin/users/*` and `/admin/groups/*`
- 5-tier role ladder (`User → Staff → Editor → Administrator → Developer`)
- Server-side filters + ILIKE search + sortable columns + per-page picker + numbered pagination, all in a single SQL query with column-name validation
- Audit log table (`rustio_admin_actions`) surfaced in the dashboard, the global history page, and per-object history
- Hand-written single stylesheet (~1.9 k LOC) with a six-token override surface, three responsive breakpoints, and a deep-slate chrome on a light page canvas
- Self-hosted fonts (Geist + Geist Mono + Tajawal + Noto Naskh Arabic, ~270 KB embedded, SIL OFL-1.1)
- Migrations runner that walks numerically prefixed `*.sql` files transactionally
- Operator CLI (`rustio`) with `startproject`, `startapp`, `migrate`, `user`, `group`, `perm`, `doctor`
- Three crates published on crates.io: [`rustio-admin`](https://crates.io/crates/rustio-admin), [`rustio-admin-macros`](https://crates.io/crates/rustio-admin-macros), [`rustio-admin-cli`](https://crates.io/crates/rustio-admin-cli)

For module-level architecture, see [`docs/architecture.md`](./docs/architecture.md). For the `ModelAdmin` customisation surface, see [`docs/modeladmin.md`](./docs/modeladmin.md).

---

## Recent Work — 0.2 Admin Refresh

Summary of the design-and-feature work that landed in v0.2.0 / v0.2.1. Each item is also recorded in `CHANGELOG.md` with full detail.

- **Graphite design language.** Dark mode rebuilt on a soft graphite ladder (`#2B313C → #444D5E`); accent lifted in dark for legibility while preserving the warm crimson hue family; text scale rebalanced for clear hierarchy. The whole admin reads as a calm professional workspace, not a hacker terminal.
- **List view architecture.** Toolbar with search (input glyph), filters dropdown, sort dropdown (field-type-aware copy), per-page picker, active-filter pills, and numbered pagination. Generic `[data-rio-dropdown]` primitive backs all four widgets so future ones reuse the same machinery.
- **URL state preservation.** Every interactive widget composes its `href` through a single `build_list_url` helper. Clicking sort never silently drops the active filter or query. The search form carries hidden inputs for filters / sort / per-page so submitting a query keeps the rest of the state.
- **Filters / sort / pagination dispatch.** `ModelAdmin::list_display()` now actually filters columns (was being silently ignored — a real bug fix). Datetime cells render in monospace tabular nums with `nowrap`; text cells single-line ellipsize with hover-reveal `title`.
- **Bulk select + bulk delete.** Per-row checkboxes, master checkbox with indeterminate state, sticky bulk action bar, two-step confirm flow. Each row deletes individually so per-row hooks and audit entries fire. Without JS the bar stays hidden and per-row Delete remains the fallback.
- **Custom bulk actions.** New `ModelAdmin::bulk_actions()` registration surface; runtime dispatcher on `AdminOps::execute_bulk_action` with a default `Err` so a forgotten override surfaces as a clear teaching error rather than a silent no-op.
- **Form view refresh.** Editorial 880 px form shell with grouped action bar (primary save left, destructive + Cancel right). Single-column field flow. Section legends rendered as small uppercase muted labels matching the table-header treatment.
- **Auth-page parity.** `users_list`, `groups_list`, all `*_edit` / `*_new` / `*_view` / `*_confirm_delete` templates now wear the same chrome as model pages.
- **Dark mode refinement.** `text-strong` is pure white in dark for clear hierarchy above the body; danger shifted from pastel pink to a saturated red that actually communicates destructive intent; accent lifted from `#A0341A` to `#C84934` so primary buttons pop on graphite.
- **Single-source-of-truth theme architecture** (breaking change). `AdminTheme` is now an override-patch type with `Option<String>` fields. Out of the box the framework emits no inline `<style>` block at all — `admin.css` is the only style source. `_theme.html` loads after the link tag and uses a multi-state selector list so project overrides win the cascade without `!important`.
- **Release verification improvements.** The release process now includes a real CLI install smoke test (`cargo install` to a sandboxed `--root`, `rustio startproject` against the installed binary, `grep` for the framework version pin in the generated `Cargo.toml`). The 0.2.1 patch came directly from this check — the 0.2.0 CLI's scaffold template was still pinning new projects to `rustio-admin = "0.1"`. Future releases run this check before announcing.

---

## Admin System

The list view, form view, dashboard, and audit pages are the framework's largest surface. Most planned work lives here.

### List view

- ✅ Per-model list page with sortable column headers
- ✅ Server-side filters + ILIKE search + pagination in a single SQL query
- ✅ List-view toolbar (filters dropdown, sort dropdown, per-page picker, search glyph)
- ✅ Global `⌘K` search palette — cross-model topbar search (`GET /admin/_search`), per-model `view`-permission gated
- ✅ Active-filter pills strip with one-click removal
- ✅ Numbered pagination with ellipsis compression past 7 pages
- ✅ URL state preservation across every widget
- ✅ `list_display()` honoured (column filtering)
- ✅ Datetime nowrap + text truncation with hover reveal
- 🟡 **Filter widget kinds.** `BoolYesNo` is the only widget shipping today. The framework's `FilterKind` enum names other variants but only `BoolYesNo` has a render path. *Related: `crates/rustio-admin/src/admin/filters.rs`.*
- ⚪ **Date range filter widget.** `DateRange` filter + URL parser + dropdown with two date inputs.
- ⚪ **Multi-select filter widget.** Checkboxes inside the dropdown, OR-joined in SQL, comma-separated in the URL.
- ⚪ **Foreign-key autocomplete filter.** Reuses the `RelationRegistry`. Type-ahead dropdown that filters `?fk_id=` against a paginated lookup endpoint.
- ⚪ **Search highlight.** Wrap the matched substring in `<mark>` in cell content for `?q=…` results.
- ⚪ **Saved filters.** Bookmarkable named filter presets per user. Stored on `rustio_users` or a new join table.

### Form view

- ✅ Generic create / edit form with field grouping
- ✅ Editorial form shell (880 px width cap, grouped action bar)
- ✅ `RustioAdmin` derive auto-generates field metadata
- 🟡 **`readonly_fields()`** declared on `ModelAdmin` but not yet honoured by `form_ctx`. The macro's per-field `editable: false` flag still owns the strict gate.
- 🟡 **`fieldsets()`** declared but not yet honoured — falls back to the framework's name-heuristic grouping (`Default` / `System` / `Advanced`).
- ⚪ **Inline forms.** Edit related rows (e.g., comments on a post edit page) without leaving the parent page. Requires a per-relation form context and a JS-driven add/remove flow.
- ⚪ **Field-level validation hooks.** Project closures that return validation errors before the row hits the database, surfaced in the same UI as the existing constraint-violation flash.
- ⚪ **Rich-text widget.** Optional Tiptap-style editor for `String` fields tagged with a `widget = "richtext"` attribute.
- ⚪ **File / image upload widget.** Tied to the upload / media system below.

### Bulk actions

- ✅ Per-row + master checkboxes; sticky bulk action bar
- ✅ Built-in cascade-aware bulk delete
- ✅ Project-defined bulk actions via `ModelAdmin::bulk_actions()` + `AdminOps::execute_bulk_action`
- ⚪ **Bulk progress UI.** For long-running actions (>~500 rows), show progress server-rendered via a dedicated `/bulk_action_progress` page.
- ⚪ **Per-action permission gate.** Currently bulk actions inherit the model's `change` permission. A `BulkAction.permission` field would let projects gate destructive actions on a stricter permission.

### Dashboard

- ✅ Per-app model index with quick "Add" / "View" links
- ✅ "Recent actions" widget (last 10 audit entries)
- ⚪ **Per-model KPIs.** Total / new this week / pending counts surfaced on the dashboard. Computed via `AdminOps::stats()` or similar.
- ⚪ **Charts.** Time-series and category breakdowns, server-rendered SVG so no JS chart library ships in the binary.
- ⚪ **Pinnable widgets.** User-customisable layout — pin / unpin / reorder.

### Audit log + activity feed

- ✅ `rustio_admin_actions` table populated on every create / update / delete
- ✅ Per-object history page at `/admin/<model>/<id>/history`
- ✅ Global history page at `/admin/history`
- 🟡 **Activity feed.** Today's `/admin/history` is a flat reverse-chronological list. A user-centric feed ("show me what *I* did" / "show me what *my team* did") needs per-actor filtering and date grouping.
- ⚪ **Diff view.** For update events, show the before / after of changed columns inline on the history entry.

### Read-only admin mode

- ⚪ **Read-only mode.** Whole-admin or per-model toggle that disables every mutating UI (no Add / Edit / Delete buttons, all forms render as read-only) without removing the underlying permissions. Useful for incident response and demo environments.

---

## Authentication & Security

- ✅ Session-based auth with DB-backed sessions
- ✅ Argon2id password hashing
- ✅ 5-tier role ladder (`User → Developer`)
- ✅ Per-model permissions (`<model>.add_<singular>` / `change` / `delete`) with a 60-second cache
- ✅ Last-developer orphan guard on user delete
- ✅ CSRF (double-submit cookie)
- ✅ Rate limit middleware (DashMap token bucket)
- ✅ Security-headers middleware
- ⚪ **Email-based password reset flow.** Generate a single-use signed token, email it via the configured SMTP transport, accept it on a `/admin/password_reset/<token>` page. Depends on the email subsystem below.
- ⚪ **Two-factor authentication.** TOTP via authenticator apps; backup codes; per-user enforcement.
- ⚪ **Session management UI.** The `user_view.html` page already lists active sessions; add a "revoke this session" affordance.
- ⚪ **Audit on auth events.** Failed login attempts, password changes, role changes — currently the audit log is CRUD-only.
- 🔬 **WebAuthn / passkeys.** Strictly research at this point; the trade-off between framework surface area and operator value is unclear.

---

## APIs & Documentation

The framework currently has no JSON API surface — every endpoint returns HTML. The roadmap adds a parallel API path without changing the admin's HTML defaults.

- 🟡 **Built-in docs pages.** The strategic-reset planning document mentions a `/admin/docs` route; the surface itself isn't built yet. Goal: render the `docs/*.md` files inside the admin chrome so operators can read framework docs without leaving the panel.
- ⚪ **CRUD API generation.** A `?format=json` URL switch (or `Accept: application/json`) on existing list / detail / create / update / delete routes. Same permission gates, same audit, JSON request/response shape derived from the model.
- ⚪ **Auto-generated OpenAPI page.** `/admin/apis/openapi.json` + a human-readable `/admin/apis` index listing every registered model's endpoints with their schema.
- ⚪ **Interactive API playground.** Embedded request-builder per endpoint — choose a method, fill in JSON, see the response. Implemented as a server-rendered page, no third-party JS dependency.
- ⚪ **SDK generation.** `rustio sdk-gen rust|typescript|python` builds a typed client from the OpenAPI spec. Out-of-tree initially; a CLI subcommand once the OpenAPI spec stabilises.
- 🔬 **GraphQL surface.** Open question — the framework's CRUD shape is regular enough that a typed GraphQL endpoint is feasible, but it's a sizeable maintenance commitment for unclear demand.

---

## Templates & Overrides

- ✅ Hand-written `admin.css` with a six-token override surface
- ✅ Disk override path — set `RUSTIO_TEMPLATE_DIR=…` and any embedded template can be overridden file-for-file
- ✅ Single-source-of-truth theme architecture (`AdminTheme` is an override patch, not a snapshot — light-only)
- ✅ Self-hosted fonts (Geist, Tajawal, Noto Naskh Arabic) with `unicode-range` filtering
- ⚪ **Generated override scaffold.** `rustio override <template-name>` copies the named embedded template into the project's `templates/admin/` directory so the project can edit it. Currently the operator does this by hand.
- ⚪ **Per-model template override.** `templates/admin/posts/list.html` overrides only the `Post` list page; everything else falls back to the framework default. Hook lives in `Templates::render_for_model` — partially wired but not consumed by the handler yet.
- ⚪ **Theme presets.** A `rustio theme apply <preset-name>` CLI verb that writes a curated `AdminTheme` patch into the project's `main.rs`. Out-of-the-box presets: ocean / forest / sunset / monochrome.

---

## Internationalization & RTL

- ✅ Self-hosted Arabic fonts (Tajawal for UI, Noto Naskh Arabic for body) with `unicode-range`
- ✅ `:lang(ar)` + `[dir="rtl"]` automatic resolution in CSS — Arabic text never lands on a Latin face; Latin tracking + Geist stylistic alternates strip out so joining-script shaping stays intact
- ✅ Dedicated `--rio-lh-arabic: 1.9` line-height token
- 🟡 **RTL-first architecture.** Mirrored UI (sidebar on the right, etc.) is not implemented. Today's CSS uses `margin-left` / `padding-left` rather than logical properties; the framework reads correctly with mixed Arabic content but doesn't flip the chrome direction.
- ⚪ **Logical CSS properties.** Migrate `margin-left`/`padding-left` to `margin-inline-start`/`padding-inline-start` so `[dir="rtl"]` flips the layout automatically.
- ⚪ **Message catalog.** `rustio.po` files baked in or loaded from disk; framework strings (`"Save"`, `"Delete"`, `"No actions yet"`) translatable per-locale. Project models continue to use their own labels.
- ⚪ **Locale negotiation middleware.** `Accept-Language` header → user preference → fallback chain.
- 🔬 **Bidirectional embedded text.** Long-form prose with mixed RTL+LTR runs (an English brand name inside an Arabic paragraph) needs Unicode bidi marks in the right places. Today's templates rely on browser defaults; an explicit policy may surface edge cases.

---

## CLI & Project Bootstrap

The `rustio` binary handles the operationally critical surface for new and existing projects.

- ✅ `rustio startproject <name>` — scaffold a fresh project
- ✅ `rustio startapp <name>` — add a model + migration to an existing project
- ✅ `rustio migrate apply` / `status`
- ✅ `rustio user create` / `list` / `role` / `delete` (honours the developer-orphan guard)
- ✅ `rustio group create` / `list` / `add-user` / `remove-user`
- ✅ `rustio perm grant-user` / `grant-group` / `list`
- ✅ `rustio doctor` — read-only health check (DB reachable, auth tables present, ≥1 administrator)
- 🟡 **Zero-config bootstrap.** `rustio startproject` requires a project name argument and assumes a Postgres at `localhost:5432`. A truly zero-config path (auto-detect a Postgres, prompt only for a project name, default to a sandbox SQLite-via-pglite if no PG is present) is not built. *Note: SQLite as a fallback collides with the "Postgres only" non-goal — the more likely path is to ship a `docker compose up` snippet rather than abstract over backends.*
- ⚪ **Project presets / starters.** `rustio startproject <name> --preset blog` / `e-commerce` / `crm` produces a richer skeleton with multiple models, sample data, and a tuned `AdminTheme`. Today only the minimal preset exists.
- ⚪ **`rustio override <template>`.** Copy a named embedded template into the project's `templates/admin/` directory. Pairs with the per-model template override above.
- ⚪ **`rustio sdk-gen <lang>`.** Pairs with the OpenAPI spec.
- ⚪ **Initial test generation.** `rustio test-init` creates a `tests/` directory with a smoke test that boots the server, hits `/admin`, and asserts a 302 to `/admin/login`. Useful as a project's first CI check.
- ⚪ **`rustio reload`.** Dev-mode watcher: on file change, send a SIGHUP to the running server (or rebuild + restart). Today operators use `cargo watch -x run` externally.

---

## Long-Term Ideas

Each item below is real and on the planning surface, but not yet scheduled for an implementation slot. They may grow, shrink, or merge as the framework matures.

- ⚪ **Upload / media system.** A `MediaModel` trait, an admin page at `/admin/media`, and per-field upload widgets. Storage backends start with local-filesystem; S3-compatible support is a separate add-on. Disk paths are validated against a project-configured root so a bad upload can't escape the sandbox.
- ⚪ **Email subsystem.** SMTP transport, message templates baked into the binary, `rustio email send-test` CLI verb. Used by the password-reset flow and by future "notify on event" features.
- ⚪ **Email template management.** Override-friendly text + HTML templates per email type (welcome, password-reset, invitation). Same disk-override pattern as admin templates.
- ⚪ **Notifications.** Per-user notification stream surfaced in the topbar with a count badge. Backed by a new `rustio_notifications` table; operators write to it from project code or via a future `notification!` macro.
- ⚪ **Background jobs / queue.** General-purpose async job runner — today's `background.rs` carries only the session sweeper. Goal is a `Job` trait, a `rustio_jobs` table, and one runner per admin process. Multi-worker scaling is out of scope until single-process limits show up.
- ⚪ **Export / import.** CSV and JSON for any model, gated on the model's `view` permission. Streaming export so large tables don't OOM the worker. Import is opt-in per model and goes through the same validation as form-create.
- ⚪ **Feature flags.** A simple `rustio_feature_flags` table + a `feature_enabled("…")` helper for project code. Flags toggleable from the admin UI by administrators.
- ⚪ **Health dashboard (web UI).** Browser-renderable counterpart to `rustio doctor` — DB latency, audit-table size, recent error counts, session count.
- ⚪ **Search systems.** Ship a Postgres full-text option (`tsvector` columns, `phraseto_tsquery`) as the next tier above the current ILIKE fallback. Listed under "long term" because it intersects with the explicit "no search backend" non-goal — the implementation has to live entirely inside Postgres.
- ⚪ **Database browser.** A read-only schema explorer at `/admin/db` showing every table, its columns, foreign keys, and row counts. Useful during development; hidden behind the Developer role.

---

## Experimental / Future Research

These are open questions. Each may become a roadmap item, may live in a separate `rustio-pro-*` crate, or may be deliberately rejected.

- 🔬 **Plugin / extension architecture.** A way for project code to add admin pages, sidebar entries, and dashboard widgets without forking. Today the closest equivalent is `Admin::user_profile_extension(closure)`. A general extension surface is desirable but the API design is unsettled — too narrow and it doesn't help; too wide and it leaks framework internals. May be the right time to revisit once two or three concrete extension shapes are in flight.
- 🔬 **Multi-tenancy.** Per-row tenant scoping enforced at the framework level (every query gets `WHERE tenant_id = current_tenant`). Real demand exists, but the trade-offs (schema requirements, permission interactions, audit-log changes) are wide enough that this likely lives in a dedicated `rustio-pro-multitenancy` crate rather than the core.
- 🔬 **Live reload (dev-only).** Watch the project's source tree, rebuild on change, hot-swap templates without dropping sessions. Could be a thin dev-only middleware that injects a WebSocket pinger; could be a separate `rustio dev` CLI subcommand. Open question whether it belongs in `rustio-admin` proper.
- 🔬 **WebAuthn / passkeys.** See *Authentication & Security*.
- 🔬 **GraphQL surface.** See *APIs & Documentation*.
- 🔬 **Bidirectional embedded text policy.** See *Internationalization & RTL*.
- 🔬 **Project-side observability hooks.** A `tracing` integration story so project consumers can plug their own observability stack into framework-emitted spans (request, query, audit) without monkey-patching.
- ⛔ **Schema contracts / drift validation / AI planners.** Explicitly **out of scope** per the strategic-reset rules. If they return to the codebase at all, they ship as a separate `rustio-pro-*` family of crates and never inside `rustio-admin`.

---

## Notes & Architectural Direction

A few principles that should outlast any specific roadmap item.

1. **The library is the contract.** Public APIs change with semver discipline; internal modules can refactor freely. `pub` items get a doc comment; `pub(crate)` items can be terse. When in doubt, narrow visibility.
2. **`admin.css` is the single source of truth for design tokens.** Rust never duplicates a hex value. `AdminTheme` is an override patch, not a snapshot. Future tokens added to the stylesheet do not require Rust-side bookkeeping.
3. **Templates are hand-written.** No template builder, no DSL, no JSX-style component tree. Project users override per-file via the disk loader; the framework ships defaults that read top-to-bottom.
4. **No build step.** Every feature is reachable via a single `cargo build` from a fresh clone. PostCSS, Tailwind, esbuild, etc. stay outside the project.
5. **Postgres only, in the core.** Multi-database support is a non-goal. SQLite-via-pglite or DuckDB ergonomics may live in a sibling crate; never in `rustio-admin`.
6. **`#[derive(RustioAdmin)]` emits obvious code.** `cargo expand` should always show the full picture. Any macro that requires runtime inspection to understand is a design failure.
7. **Audit and security defaults are loud.** CSRF on every mutating route, audit entry on every CRUD operation, last-developer guard, permission cache with a short TTL. Adding a new mutating route without these is an oversight, not a feature.
8. **The framework is allowed to be small.** "We didn't ship X" is a valid answer. Saying no preserves the parts that already work.
9. **Releases are verified, not asserted.** Every release runs through the install smoke test (`cargo install` to a sandboxed `--root`, run `rustio startproject`, assert generated `Cargo.toml` pins the current version). The 0.2.1 patch came directly from this check; future patches should too.
10. **The roadmap is a living document.** Reorganise it. Promote items between sections. Remove things that no longer make sense. The shape of this file at any moment reflects the project's honest current direction, not a marketing snapshot.

---

*Last revision: 2026-05-08. This document is rewritten as the framework evolves; the [git history](https://github.com/abdulwahed-sweden/rustio-admin/commits/main/ROADMAP.md) is the authoritative timeline of how the plan changed.*
