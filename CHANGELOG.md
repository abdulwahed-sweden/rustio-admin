# Changelog

All notable changes to `rustio-admin` are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project
adheres to [SemVer](https://semver.org/) once it leaves the alpha track.

## [0.2.0] — 2026-05-07

Premium-chrome release. The list view, form view, and Auth pages all
share one design language; dark mode is now a calm graphite workspace
rather than an OLED-black hacker terminal. New list-view toolbar
(filters / sort / per-page / active-filter pills / numbered
pagination / search glyph) ships with full URL state preservation, and
projects can register custom bulk actions alongside the built-in
delete.

> **Breaking change:** `AdminTheme` is now an override-patch type
> with `Option<String>` fields instead of `String` snapshots; default
> is *all `None`* (no inline `<style>` emitted at all).
> `Admin::accent()` returns `Option<&str>`. See **Migrating from
> 0.1.x** below.

### Added — list view toolbar

- **Filters dropdown panel.** The old stacked `<aside class="rio-filters">`
  card is replaced by a single-row toolbar dropdown anchored next to
  the search bar. Active filter count surfaces as an accent badge on
  the toggle. Each chip is still an `<a href>` so navigation remains
  the source of truth — no client state, no Apply step.
- **Sort dropdown** with field-type-aware copy (`title (A → Z)`,
  `created_at (newest first)`, `published (off → on)`). Toggle reads
  `Sort: <current>`; menu lists every visible field × {asc, desc}
  plus a leading **Default order** reset.
- **Per-page picker dropdown** (`25 / 50 / 100 / 200 per page`). The
  handler's allow-list changed from `[10, 25, 50, 100]` to
  `[25, 50, 100, 200]` — the design mockup's set, more useful for
  real workflows.
- **Active-filter pills strip** below the toolbar (`Published: Yes
  ×`). Each `×` drops only that filter while keeping query / sort /
  other filters; a `Clear all` link resets every filter without
  losing search / sort. Hidden when no filters are set, so the strip
  has zero default-state cost.
- **Numbered pagination** (`← Previous   1 [2] 3 … 9   Next →`).
  Compresses past 7 pages to first / current ± 1 / last with `…`
  markers. Active page is accent-filled.
- **Search input glyph.** A magnifying-glass icon sits inside the
  search field on the left.
- **URL state preservation.** Every interactive widget — filter
  chip, sort option, header sort, pagination, per-page picker —
  composes its `href` through the new `build_list_url` helper so
  clicking one never silently drops the others. The search form
  carries hidden inputs for active filters / sort / per-page so
  submitting a query keeps the rest of the state. URL-encoded
  values; defaults skipped from the URL so `/admin/posts` stays
  clean.
- **Generic dropdown primitive** — `[data-rio-dropdown]` wrapper +
  `.rio-dropdown-toggle` + `.rio-dropdown-panel`, plus two layout
  variants (chip-row + vertical menu) — so future widgets reuse the
  same machinery without new CSS.

### Added — bulk select + bulk actions

- **Per-row + master-row checkboxes** on the list view. Master shows
  indeterminate state when partially selected.
- **Sticky bulk action bar** (`N selected · Delete selected · Clear
  selection`) that appears above the table when ≥ 1 row is checked.
  Without JS the bar stays hidden — per-row Delete is the fallback,
  no functional regression.
- **Built-in bulk delete** — `POST /admin/:model/bulk_delete` with
  the same two-step (confirm page → commit on `_confirmed=1`)
  semantics as the per-row delete. Each row is deleted individually
  so per-row hooks and audit entries fire as expected.
- **Project-defined bulk actions.** New
  `ModelAdmin::bulk_actions() -> &'static [BulkAction]` registers
  buttons in the bulk bar; each routes to
  `POST /admin/:model/bulk/:name`. Actions can be destructive (red
  styling) and/or require confirmation. The runtime dispatcher is
  `AdminOps::execute_bulk_action`, with a default `Err` that
  surfaces a clear "no project handler" message — projects override
  it to apply the work.

### Added — form view + Auth refresh

- **Editorial form shell.** Edit / new / confirm-delete /
  password-change pages cap content at 880px and centre-align so
  labels and inputs read at a comfortable line length. Single-column
  field flow (the previous 2-col grid left gaps with sparse models).
- **Action bar grouping.** New `.rio-form-actions-spacer` flex
  pusher: primary save buttons sit left, secondary (History) and
  destructive (Delete) + Cancel push right. Top divider grounds the
  row.
- **Auth pages on parity** — `users_list`, `groups_list` get the
  proper `rio-th` / `rio-td--{number,text,datetime,actions}` classes
  + compact `--sm` row buttons with icons. `user_edit`, `user_new`,
  `group_edit`, `group_new`, `user_view`, `user_confirm_delete`,
  `group_confirm_delete` all wrapped in the form shell.
- **Sticky sidebar** at tablet+ widths (`position: sticky; top:
  topbar-h; align-self: flex-start; height: calc(100vh -
  topbar-h)`). Scrolls independently when long.

### Added — design tokens

- **Surface ladder** for layered depth without dramatic shadow:
  `--rio-bg → --rio-surface → --rio-surface-2 → --rio-surface-3`.
- **`--rio-text-strong`** for page titles + KPI numbers (white in
  dark mode for clear hierarchy above the body).
- **`--rio-border-soft`** for in-card row dividers, **`--rio-accent-hover`**
  for primary-button hover state.
- **Soft shadows** at `0.04–0.10` alpha — depth comes from layering,
  not drop shadow.

### Added — code-level hooks

- New `BulkAction` struct (re-exported at `rustio_admin::admin::BulkAction`).
- New `AdminOps::execute_bulk_action(db, name, ids)` trait method
  with default `Err`.
- New `build_list_url` URL composer in `render.rs`; new types
  `SortOptionCtx`, `PerPageOptionCtx`, `ActiveFilterPillCtx`,
  `PageItem`, `BulkActionBtnCtx`, `BulkConfirmDeleteCtx`,
  `BulkConfirmActionCtx`, `BulkDeleteItem`.

### Changed — theme architecture

- **`AdminTheme` is now an override-patch type.** All fields are
  `Option<String>`, defaulting to `None`. The framework stylesheet
  (`admin.css`) is the single source of truth for every design
  token; `AdminTheme` only injects overrides for fields a project
  explicitly sets. Out of the box, **no inline `<style>` block is
  emitted at all** — admin.css alone resolves the look.
- **`_theme.html` placement flipped.** It now loads *after*
  `<link rel="stylesheet" href="/static/admin.css">` in `_base.html`,
  so a project override wins the cascade on source order without
  needing `!important`. Selector list `html, html[data-rio-theme=
  "light"], html[data-rio-theme="dark"]` makes overrides apply
  across all theme states.
- **`Admin::accent()` returns `Option<&str>`** (was `&str`). `None`
  means *"no override — admin.css owns it"*.
- **Inline theme bootstrap script** in `<head>` — reads
  `localStorage["rio-theme"]` and sets `data-rio-theme` *before*
  CSS loads, so the chosen mode lands on the first paint with no
  flash-of-light-on-dark. The previous hardcoded `data-rio-theme=
  "light"` defeated the "System" toggle option on every page reload.

### Changed — dark mode

- **Soft graphite, not near-black.** Page bg is now `#2B313C` (was
  `#0b1120`); surfaces lift in 4–5% steps to `#444D5E`. Text scale
  is warm slate (`#F3F4F6 / #D2D6DC / #B2B8C2`).
- **Accent lifted** in dark from `#A0341A` (deep crimson) to
  `#C84934` so primary buttons pop on graphite (~3.1:1 vs ~1.5:1)
  while keeping the same warm-crimson hue family.
- **Danger lifted** in dark from `#F87171` (pastel pink) to
  `#DC4444` — a saturated red that communicates destructive intent
  rather than candy.
- **`text-strong`** in dark is now pure white for clear hierarchy
  above the `#F3F4F6` body.

### Changed — list view rendering

- **`ModelAdmin::list_display()` now actually filters columns.** The
  renderer was iterating over every model field unconditionally,
  contradicting the doc comment on `AdminEntry::list_display`.
  Models that declared `list_display = ["title", "published",
  "created_at"]` were still rendering `body` and `id` and any other
  field — now they render exactly what was declared.
- **Datetime cells** render in monospace tabular nums with `nowrap`
  so ISO timestamps don't break at the `T`.
- **Text cells** clip to a single line with `min(20rem, 28vw)`
  ellipsis; the template adds a `title=` attribute so hovering
  reveals the full clipped value natively.

### Changed — layout

- **`.rio-layout` is flex, not grid.** The previous 2-column grid
  reserved a 240px sidebar slot on every page, which crammed the
  login card into a 240px-wide column when no sidebar was rendered
  (visible disaster on `/admin/login`).
- **`.rio-form` is a flex column with `gap`** so consecutive
  `.rio-field` siblings have automatic spacing; `.rio-form-actions`
  no longer needs its own `margin-top`.
- **Main content cap** at 1280px on desktop so wide monitors don't
  sprawl table rows across the full viewport.

### Migrating from 0.1.x

If you set theme tokens via struct literal:

```rust
// 0.1.x
.theme(AdminTheme {
    accent:     "#2563EB".into(),
    bg:         "#F4F6FB".into(),
    surface:    "#FFFFFF".into(),
    text:       "#111827".into(),
    text_muted: "#4B5563".into(),
    border:     "#D1D5DB".into(),
})
```

Either wrap each value in `Some(...)` (struct literal still works),
or use the new fluent builder (recommended):

```rust
// 0.2.x — fluent
.theme(
    AdminTheme::new()
        .accent("#2563EB")
        .bg("#F4F6FB")
        .surface("#FFFFFF")
        .text("#111827")
        .text_muted("#4B5563")
        .border("#D1D5DB"),
)
```

Setting fewer fields is now valid and recommended:

```rust
// 0.2.x — only override what you care about
.accent_color("#FF8800")
```

Skipping fields is the new default — admin.css's tokens fill in
everywhere you don't override, including dark-mode variants.

If you read `Admin::accent()` returning `&str`, switch to
`accent().unwrap_or(default)` or match on `Option`.

If your `ModelAdmin` set `list_per_page = 10` expecting it to
allow `?per_page=10`, switch to `25` or `?per_page=25`. The
allow-list now starts at 25.

## [0.1.1] — 2026-05-07

Design-system release. No public API surface changes — the typography,
font, and brand-color work all happens behind the existing `Admin`,
`AdminTheme`, and template-override surfaces. Drop-in upgrade from 0.1.0.

### Added

- **Self-hosted fonts (SIL OFL-1.1)** baked into the binary, served
  at `/static/fonts/*.woff2` with year-long immutable cache:
  - **Geist Variable** + **Geist Mono Variable** — Latin UI + code
    (single woff2 each covers the full `wght` 100..900 axis).
  - **Tajawal** 400 / 500 / 700 — Arabic UI surfaces (buttons,
    sidebar, tables, badges).
  - **Noto Naskh Arabic Variable** — Arabic body / paragraph copy.
  - All filtered by `unicode-range` so Latin-only pages pay zero
    Arabic download cost. Total embedded: ~270 KB.
- **Complete typography token system** in `admin.css` — three family
  tokens (`--rio-font-sans`, `--rio-font-arabic`, `--rio-font-arabic-body`,
  `--rio-font-mono`), a 9-step size scale (`--rio-fs-xs` 13px through
  `--rio-fs-display` 40px), four line-height tokens including a
  dedicated `--rio-lh-arabic: 1.9`, four weight tokens, and Latin-only
  tracking tokens that auto-reset for Arabic / RTL contexts.
- **`:lang(ar)` / `[dir="rtl"]` resolution rules** — Arabic text
  automatically picks up Tajawal (UI) or Noto Naskh (when inside a
  `.rio-prose` or a `<p lang="ar">`); Geist's stylistic alternates
  (ss01/ss03/cv11) are stripped so joining-script shaping stays
  intact. Mixed Latin/Arabic strings shape correctly out of the box.
- **`--rio-surface-2`** + **`--rio-border-strong`** + **`--rio-text-subtle`**
  tokens for secondary surfaces, heavy outlines, and tertiary text.

### Changed

- **Default brand accent** is now `#A0341A` (Andalusian crimson),
  replacing the previous cobalt `#2563EB`. The new value applies in
  three places, all of which agree:
  - `AdminTheme::default()` — what `rustio startproject` scaffolds.
  - `admin.css :root --rio-accent` — what unstyled chrome resolves to.
  - `render::hex_to_rgb_triplet` fallback — what bad config falls back
    to so the admin chrome never breaks over a user typo.
  Projects override via `Admin::theme(...)` or `Admin::accent_color(...)`
  exactly as before.
- **Tightened light palette** for stronger reading contrast: `--rio-bg`
  `#f4f6fb` → `#ebeef4` (deeper page bg so white surfaces visibly pop),
  `--rio-text-muted` `#4b5563` → `#3d4452` (AAA against surface),
  `--rio-border` `#d1d5db` → `#cdd3df`. Dark-mode tokens similarly
  bumped — `--rio-text` `#e5e7eb` → `#f1f5f9`, `--rio-text-muted`
  `#94a3b8` → `#c0c8d6`. Every text/surface pair clears WCAG AAA in
  both themes.
- **Body font-size** raised from 14px to **16px**; minimum helper-text
  size enforced at **13px**; table cells at **15px**; headings rescaled
  (h1 34px, h2 26px, h3 22px). Mobile bumps `html` to 16.5px below
  600px so dense forms stay comfortable.
- **Sidebar + topbar typography** polished — sidebar links now 15px
  medium with crimson hover on text + icon, topbar identity 15px
  regular, theme toggle 13px medium with accent border on hover.
- **`/static/admin.css` and `/static/admin.js`** Cache-Control flipped
  from `public, max-age=3600` to `no-cache, must-revalidate` so
  theme + design tweaks roll out the moment the binary restarts.
  Fonts keep their year-long immutable cache (their bytes never
  change per release).

### Removed

- IBM Plex Sans Arabic dropped from the bundled fonts — Tajawal +
  Noto Naskh Arabic cover the UI/body split more elegantly. Anyone
  who customised templates to reference `"IBM Plex Sans Arabic"` by
  name will need to switch to `"Tajawal"` / `"Noto Naskh Arabic"`;
  the `--rio-font-arabic` and `--rio-font-arabic-body` tokens
  resolve both automatically.

## [0.1.0] — 2026-05-07

First public release. Strategic-reset rollout of phases 1–15 plus
the live browser walk and the operator CLI is feature-complete.

### Added

- **Django-style `ModelAdmin` trait** with seven hooks: `list_display`,
  `list_filter`, `search_fields`, `ordering`, `list_per_page`,
  `readonly_fields`, `fieldsets`. Every method has a default body —
  projects write `impl ModelAdmin for X {}` (empty body) to opt in,
  override individual methods to customise.
- **Generic admin runtime**: `Admin::new().model::<M>()` registers a
  Postgres-backed CRUD page; `register_admin_routes` mounts every URL
  the admin needs onto a `Router`. List / create / edit / delete /
  per-object history all wired.
- **Built-in user / group pages** at `/admin/users/*` and
  `/admin/groups/*`, plus `/admin/password_change`, `/admin/history`.
- **Server-side filters + ILIKE search + pagination** pushed into a
  single SQL query with column-name validation against `M::COLUMNS`.
  No more in-memory `retain` for ten-thousand-row tables.
- **Sortable list-page columns** via `?sort=col&dir=desc`, falling
  back to `ModelAdmin::ordering()`.
- **Hand-written CSS theme**: six CSS custom properties driven by
  `Admin::theme(...)`. Mobile-first responsive (3 breakpoints), dark
  mode via `prefers-color-scheme` plus a manual toggle persisted to
  `localStorage`.
- **Auth & RBAC**: 5-tier role ladder (User → Developer), Argon2
  password hashing, DB-backed sessions, per-model permissions with a
  60-second cache, last-developer orphan guard on user delete.
- **Audit log**: every create/update/delete writes to
  `rustio_admin_actions`; surfaced in the dashboard's "Recent
  actions" widget, the global `/admin/history` page, and per-object
  `/admin/<model>/<id>/history`.
- **Middleware bundle**: rate limit, CSRF (double-submit cookie),
  security headers, gzip, request logger.
- **Migrations runner** that walks numerically prefixed `*.sql` files
  in a directory and applies them transactionally with a tracking
  table.

### Architecture

- **Tier 1, single-binary, Postgres-only.** Schema contracts, drift
  validation, AI planners, multi-database backends, search backends —
  all explicitly out of scope for this crate (see the [strategic
  reset plan](./rustio-admin-strategic-reset-plan.md) §1, §3, §8).
- **No Tailwind, no PostCSS, no build step.** The CI pipeline
  enforces the no-Tier-2-symbols invariant with a `git grep` guard
  on every PR.

### `rustio` CLI

The binary that ships from `rustio-admin-cli` now covers the
operationally critical surface for v0.1.0:

- `rustio migrate apply` / `status` — drive the framework's
  numerically prefixed `migrations/*.sql` runner.
- `rustio user create` / `list` / `role` / `delete` — auth-table
  CRUD with Argon2 hashing and a confirm-twice password prompt.
  Honours the developer-orphan guard.
- `rustio group create` / `list` / `add-user` / `remove-user` —
  group CRUD + membership.
- `rustio perm grant-user` / `grant-group` / `list` — permission
  grants on top of `auth::permissions`.
- `rustio doctor` — read-only health check (DB reachable? auth
  tables present? at least one administrator?). Exits non-zero on
  any blocker so a CI step can gate on it.
- `rustio startproject <name>` — scaffold a fresh project at
  `./<name>/` with a working `Cargo.toml` (git dep against
  rustio-admin's main), a demo `Post` model with a populated
  `ModelAdmin` impl, starter migration, `.env.example`, and a
  `README.md`. Templates are baked into the CLI binary via
  `include_str!`; the project name is validated and the directory
  is refused if it already exists.
- `rustio startapp <name>` — add a model + migration to an
  existing project. Generates `src/<name>.rs` (full `Model` +
  empty `ModelAdmin` impl) and `migrations/<NNNN>_create_<name>s.sql`
  with an auto-incremented number. Pluralisation is naive (`s`
  suffix) — fine for the starter table; adjust for irregular
  plurals. Refuses to mutate `src/main.rs` automatically; prints
  the exact `mod` / `use` / `.model::<>()` lines instead.

Browser-walked end-to-end against a local Postgres: a single
`rustio startproject blog` + two `rustio startapp` invocations
generated 3 models which all rendered their list pages on
`/admin/posts`, `/admin/comments`, and `/admin/book_reviews` after
a copy-paste edit to `src/main.rs`.

### Released to crates.io

All three workspace crates are on crates.io as of 2026-05-07:

- [`rustio-admin@0.1.0`](https://crates.io/crates/rustio-admin)
- [`rustio-admin-macros@0.1.0`](https://crates.io/crates/rustio-admin-macros)
- [`rustio-admin-cli@0.1.0`](https://crates.io/crates/rustio-admin-cli)

Project consumers add `rustio-admin = "0.1"` to their `Cargo.toml`;
operators install the CLI with `cargo install rustio-admin-cli`.
