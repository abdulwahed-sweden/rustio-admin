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

The framework currently sits at **v0.15.1** (released 2026-05-16). See [`CHANGELOG.md`](./CHANGELOG.md) for the release-by-release record. The shape:

- Core admin runtime with list / create / edit / delete pages per registered Postgres model
- Built-in user / group / permission management at `/admin/users/*` and `/admin/groups/*`
- 5-tier role ladder (`User → Staff → Editor → Administrator → Developer`)
- Full list-page widget set: search + DateRange + multi-select + FK autocomplete + status dropdowns + sortable columns + per-page picker + numbered pagination, all composed via `build_list_url` so every interaction preserves the rest of the URL state. `<mark>` highlight on `?q=` matches; saved-filter bookmarks per operator
- Inline forms on edit pages (parent-with-children), `readonly_fields()` + `fieldsets()` honoured on every form
- Cascade-aware bulk delete + project-defined `ModelAdmin::bulk_actions()`
- `?format=json` / `Accept: application/json` content negotiation on list + detail endpoints with JSON error envelopes; CSV export on the list endpoint (`/admin/:model/export.csv`, capped at 10k rows)
- `GET /admin/_search` global ⌘K palette (per-model `view`-permission gated), `GET /admin/_lookup/:admin_name` FK typeahead, `GET /admin/healthz` public liveness probe
- Audit log table (`rustio_admin_actions`) surfaced in the dashboard, the global history page, and per-object history
- Admin-driven password recovery + self-service reset + TOTP + backup codes + emergency-access CLI; framework-emitted email with project branding
- Read-only admin mode via `Admin::read_only(true)`, enforced by `read_only_guard` middleware
- Hand-written multi-file stylesheet under `assets/static/admin/` with a six-token override surface and a chrome-scope cascade. Light-only — the parallel dark palette was retired in `[Unreleased]`
- Self-hosted fonts (Geist + Geist Mono + Tajawal + Noto Naskh Arabic + Inter + Thai + Devanagari, locale-gated) with `unicode-range` filtering
- Migrations runner that walks numerically prefixed `*.sql` files transactionally
- Operator CLI (`rustio`) with `startproject`, `startapp`, `override`, `theme`, `migrate`, `user` (incl. `perms --email`), `group` (incl. `show --name`), `perm`, `audit tail [--since]`, `doctor` (incl. `doctor email`). Builder verbs (`new` / `add model` / `add field` / `plan` / `commit`) live in the same binary as a pre-MVP build-time layer — see `docs/design/DESIGN_BUILDER.md`
- Three crates published on crates.io: [`rustio-admin`](https://crates.io/crates/rustio-admin), [`rustio-admin-macros`](https://crates.io/crates/rustio-admin-macros), [`rustio-admin-cli`](https://crates.io/crates/rustio-admin-cli)

For module-level architecture, see [`docs/architecture.md`](./docs/architecture.md). For the `ModelAdmin` customisation surface, see [`docs/modeladmin.md`](./docs/modeladmin.md).

---

## Recent Work

See [`CHANGELOG.md`](./CHANGELOG.md) for the canonical per-release notes. This file is no longer a rolling summary of shipped work — that path produced drift. Sections below describe **direction**, not history.

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
- ✅ **DateRange filter widget** — `?<col>__gte=YYYY-MM-DD&__lte=YYYY-MM-DD`, two `<input type="date">` controls, combined "from → to" pill
- ✅ **Multi-select filter widget** — checkboxes in the dropdown, repeated `?<col>=v` per option, value domain closed against the field's declared `choices`
- ✅ **FK autocomplete filter** — `/admin/_lookup/:admin_name` endpoint, typeahead input, pill carries hydrated row label with `#id` fallback
- ✅ **Search highlight** — matched substring wrapped in `<mark>` for `?q=…` results
- ✅ **Saved filters** — per-operator bookmarks, dropdown in the toolbar, `POST /admin/:model/saved_filters` + `…/saved_filters/:id/delete`
- ✅ **CSV export** — `/admin/:model/export.csv` with the current filter query, 10k-row cap, RFC 4180 quoting
- ✅ **Distinct-text dropdown widget** — `Status`-typed columns now populate the filter dropdown from `SELECT DISTINCT col::text FROM <table> WHERE col IS NOT NULL ORDER BY col::text LIMIT 50`. Column name validated against `entry.fields` before interpolation; clicking a chip composes a filter URL via the existing `build_list_url` machinery. CSV-export filter parser mirrors the same arm.

### Form view

- ✅ Generic create / edit form with field grouping
- ✅ Editorial form shell (880 px width cap, grouped action bar)
- ✅ `RustioAdmin` derive auto-generates field metadata
- ✅ **`readonly_fields()`** honoured — disabled input + reload-from-DB on update so the persisted value can't be smuggled past via a hand-crafted POST
- ✅ **`fieldsets()`** honoured — `group_fields_by_fieldsets` in `render.rs`; falls back to name-heuristic grouping only when the model declares none
- ✅ **Inline forms** — parent edit pages render child rows via `fetch_inline_sections`; click-through navigation today, in-page row editing still pending
- ⚪ **Inline form in-page editing.** Add / remove / reorder child rows on the parent page without a round-trip. Requires a per-relation form context.
- ⚪ **Field-level validation hooks.** Project closures that return validation errors before the row hits the database, surfaced in the same UI as the existing constraint-violation flash.
- ⚪ **Rich-text widget.** Optional Tiptap-style editor for `String` fields tagged with a `widget = "richtext"` attribute.
- ✅ **File / image upload widget** — `#[rustio(file)]` columns (`FieldType::FilePath` / `OptionalFilePath`) render the form input as `<input type="file">`, persist relative paths under `Admin::uploads_dir`, and serve the bytes back via `GET /admin/uploads/<rel>`. Multipart parser caps at 16 MB total / 8 MB per file; canonicalised storage root refuses path traversal.

### Bulk actions

- ✅ Per-row + master checkboxes; sticky bulk action bar
- ✅ Built-in cascade-aware bulk delete
- ✅ Project-defined bulk actions via `ModelAdmin::bulk_actions()` + `AdminOps::execute_bulk_action`
- ⚪ **Bulk progress UI.** For long-running actions (>~500 rows), show progress server-rendered via a dedicated `/bulk_action_progress` page.
- ✅ **Per-action permission gate** — `BulkAction.permission: Option<&'static str>`. When set, the handler enforces `<admin_name>.<permission>_<singular>` on top of the route's `change` gate. Mirrors `perm_guard` semantics: developers/administrators bypass via role; everyone else needs the scoped grant. `None` inherits — `change` is the only check.

### Dashboard

- ✅ Per-app model index with quick "Add" / "View" links
- ✅ "Recent actions" widget (last 10 audit entries)
- 🟡 **Per-model KPIs.** Total row counts (from `pg_class.reltuples`) and "new this week" (exact count of rows with `created_at` within the last 7 days, when the model declares that column) ship on the dashboard. "Pending" counts and other domain-specific KPIs are still project-side concerns — a generic `AdminOps::stats()` extension is pending.
- 🟡 **Charts.** A 7-day audit-activity sparkline ships on the dashboard — inline-SVG bars rendered from a single `SELECT DATE(timestamp), COUNT(*) … GROUP BY day` query, padded to seven entries. Category breakdowns and per-model time-series are still planned.
- ⚪ **Pinnable widgets.** User-customisable layout — pin / unpin / reorder.

### Audit log + activity feed

- ✅ `rustio_admin_actions` table populated on every create / update / delete
- ✅ Per-object history page at `/admin/<model>/<id>/history`
- ✅ Global history page at `/admin/history`
- ✅ **Activity feed** — per-actor filtering (`?user_id=N`, click-through on `By` column, banner + clear-link) and date grouping (rows grouped under a `YYYY-MM-DD` divider when the day changes). Same machinery powers both `/admin/history` and per-object history pages.
- ⚪ **Diff view.** For update events, show the before / after of changed columns inline on the history entry.

### Read-only admin mode

- ✅ **Whole-admin read-only mode** — `Admin::read_only(true)` builder, enforced by the `read_only_guard` middleware (every mutating route returns 403; auth-flow routes still pass through for password rotations). Templates branch on `ctx.read_only`
- ✅ **Per-model read-only toggle** — `Admin::read_only_model(slug)` builder freezes one model. The `read_only_guard` middleware extracts the `:admin_name` segment via `extract_admin_name` and returns 403 with a `Model X is frozen (read-only)` message on mutating verbs. Coexists with the whole-admin flag — a frozen slug stays frozen even when the rest of the admin is writable

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
- ✅ **Email-based password reset flow** — admin-driven + self-service variants, single-use signed token, real SMTP transport, project-branded HTML email
- ✅ **Two-factor authentication** — TOTP + single-use backup codes
- ✅ **Emergency-access CLI** — `rustio user reset-password / unlock / disable-mfa / promote / emergency-access`
- ✅ **Session management UI** — `user_view.html` sessions tab lists active sessions with a per-row Revoke button (hidden on the actor's own session). `POST /admin/users/:id/sessions/:session_id/revoke` enforces cross-rank, runs through `auth::invalidate_sessions` (Doctrine 22 single writer), emits `SessionsRevokedByOther`. Self-service variants at `/admin/account/sessions/:id/revoke`, `…/revoke-others`, `…/revoke-all`
- ✅ **Audit on auth events** — `LoginSucceeded`, `LoginFailed` (with `reason = wrong_password | inactive | locked` in metadata), `PasswordChangedSelf`, `PasswordResetByOther`, `SessionsRevokedSelf` / `…ByOther`, `SessionLogout`, `MfaEnabled` / `MfaDisabled` / `MfaCodeConsumed`, `BackupCodesRegenerated`, `AccountLocked` / `…Unlocked`, `ForcedPasswordChangeCompleted`, `EmergencyRecovery` (CLI-only) — 58 emission sites across the framework. SIEM-friendly stable strings locked by `audit_event_existing_variants_have_stable_strings`.
- 🔬 **WebAuthn / passkeys.** Strictly research at this point; the trade-off between framework surface area and operator value is unclear.

---

## APIs & Documentation

JSON content negotiation ships today on list + detail endpoints; CSV export ships on the list endpoint. Write paths are still HTML-only.

- ✅ **JSON content negotiation on read paths** — `?format=json` or `Accept: application/json` on `/admin/:model` and `/admin/:model/:id`. Same permission gates. JSON shape derived from the model. JSON error envelopes on the detail endpoint
- ✅ **CSV export** — `/admin/:model/export.csv` (also reachable via `Accept: text/csv`), reuses the current filter query, capped at 10k rows
- ✅ **Liveness probe** — `GET /admin/healthz` (public, no auth, DB ping)
- ✅ **JSON on write paths** — `do_create` / `do_update` / `do_delete` honour the same `wants_json` negotiation the read path already used. Success returns `{"ok": true, "admin_name", "id"}` (201 for create, 200 for update/delete); validation errors return `{"errors": [...], "status": 400}`; framework errors return the existing `json_error` envelope. Request body still parsed as form-encoded / multipart — only the response shape switches per `Accept` / `?format=json`. JSON-body parsing for writes is a future extension.
- ⚪ **Built-in docs pages.** Render the `docs/*.md` files inside the admin chrome at `/admin/docs` so operators can read framework docs without leaving the panel.
- ✅ **Auto-generated OpenAPI surface** — `GET /admin/apis/openapi.json` serves an OpenAPI 3.0 document with per-model component schemas and full path coverage (list / create / detail / update / delete). `GET /admin/apis` is the HTML companion: one section per model with an endpoint table, a field table (name / type label / nullable), and a one-click download link to the JSON spec. Footer carries a permanent "API surface" link next to "Audit log".
- ⚪ **Interactive API playground.** Embedded request-builder per endpoint — choose a method, fill in JSON, see the response. Implemented as a server-rendered page, no third-party JS dependency.
- ⚪ **SDK generation.** `rustio sdk-gen rust|typescript|python` builds a typed client from the OpenAPI spec. Out-of-tree initially; a CLI subcommand once the OpenAPI spec stabilises.
- 🔬 **GraphQL surface.** Open question — the framework's CRUD shape is regular enough that a typed GraphQL endpoint is feasible, but it's a sizeable maintenance commitment for unclear demand.

---

## Templates & Overrides

- ✅ Hand-written `admin.css` with a six-token override surface
- ✅ Disk override path — set `RUSTIO_TEMPLATE_DIR=…` and any embedded template can be overridden file-for-file
- ✅ Single-source-of-truth theme architecture (`AdminTheme` is an override patch, not a snapshot — light-only)
- ✅ Self-hosted fonts (Geist, Tajawal, Noto Naskh Arabic, Inter, Thai, Devanagari) with `unicode-range` filtering
- ✅ **Per-model template override** — `templates/admin/<model>/list.html` wins over the framework default for that model only; covered by `Templates::render_for_model` tests
- ✅ **`rustio override <template>`** — CLI verb that copies a named embedded template into the project's `templates/` dir (refuses to clobber without `--force`); pair with `RUSTIO_TEMPLATE_DIR=./templates` at runtime
- ✅ **`rustio theme` presets** — curated `AdminTheme` palettes; subcommand prints a Rust snippet for the operator to paste into their `Admin::new()` chain

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
- ✅ `rustio doctor` — read-only health check (DB reachable, auth tables present, ≥1 administrator); `doctor email` for SMTP self-validation (incl. `--html-preview`)
- ✅ `rustio audit tail [--since <duration>]` — read-only audit-trail viewer
- ✅ `rustio override <template>` — copy an embedded template into the project's `templates/` dir
- ✅ `rustio theme` — curated `AdminTheme` preset snippets
- ✅ Builder verbs (`rustio new / add model / add field / plan / commit`) — pre-MVP build-time scaffolder; see `docs/design/DESIGN_BUILDER.md`
- 🟡 **Zero-config bootstrap.** `rustio startproject` requires a project name argument and assumes a Postgres at `localhost:5432`. A truly zero-config path (auto-detect a Postgres, prompt only for a project name) is not built. *Note: SQLite as a fallback collides with the "Postgres only" non-goal — the more likely path is to ship a `docker compose up` snippet rather than abstract over backends.*
- ⚪ **Project presets / starters.** `rustio startproject <name> --preset blog` / `e-commerce` / `crm` produces a richer skeleton with multiple models, sample data, and a tuned `AdminTheme`. Today only the minimal preset exists.
- ⚪ **`rustio sdk-gen <lang>`.** Pairs with the OpenAPI spec.
- ✅ **Initial test generation** — `rustio test-init` writes a stdlib-only `tests/smoke.rs` integration test that spawns `cargo run`, probes the bound HTTP port, sends a raw GET to `/admin/`, and asserts a 302/303 redirect to `/admin/login` (`Location` header inspection). `--force` to clobber; `--out <dir>` to override the destination root.
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

*Last revision: 2026-05-21 — drift sync against v0.15.1. This document is rewritten as the framework evolves; the [git history](https://github.com/abdulwahed-sweden/rustio-admin/commits/main/ROADMAP.md) is the authoritative timeline of how the plan changed.*
