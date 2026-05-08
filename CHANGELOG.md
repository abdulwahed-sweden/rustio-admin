# Changelog

All notable changes to `rustio-admin` are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project
adheres to [SemVer](https://semver.org/) once it leaves the alpha track.

## [0.4.0] — 2026-05-09

Session lifecycle + recovery foundations release. R0 of the universal
account-recovery architecture documented in `DESIGN_SESSIONS.md` and
`DESIGN_AUDIT.md`. **No password-reset flow yet** — that ships in R1
(0.5.0). This release lays the safe foundation: hashed-at-rest session
tokens, centralised invalidation, typed lifecycle vocabulary, audit
forensic chain, and an email abstraction.

> **Migrating from 0.3.x:** bump `rustio-admin = "0.4"` in your
> project's `Cargo.toml`, then **add `middleware::correlation_id`
> BEFORE `middleware::csrf_protect`** in your router so audit rows
> get a populated `correlation_id`. Run `cargo update -p rustio-admin`.
> The schema migration is additive; existing sessions continue to
> authenticate through a 14-day plaintext-fallback window.

### Added

- **Hashed-at-rest session tokens.** `rustio_sessions.token_hash`
  stores `sha256(cookie-token)`; the cookie keeps the plaintext.
  Lookup: hash-first, with a plaintext fallback for the 14-day
  transition window since release. After 14 days every legacy
  session has expired and a 0.5.x patch can drop the fallback +
  the plaintext column.
- **Centralised session invalidation** (`auth::invalidate_sessions`).
  The single legitimate writer of `rustio_sessions.revoked_at`. A
  `grep -rE "revoked_at\s*="` returns only this function. Called by
  logout, password reset (R1), MFA disable (R3), administrative
  revoke (R2), and trust-escalation rotation. The companion
  `auth::logout_session` is a thin wrapper for the existing logout
  handler.
- **Typed lifecycle vocabulary**: `SessionTrust`
  (`Authenticated < Elevated < MfaVerified` with `satisfies()`),
  `SessionInvalidationReason` (12 variants), `SessionTarget`
  (`User` / `UserExceptCurrent` / `Single`), `Session` (read-only
  view), `InvalidationOutcome`.
- **`auth::list_active_for_user(db, user_id)`** + **`auth::current_session_id(db, token)`**
  feed the new active-sessions UI.
- **Read-only `/admin/account/sessions`** page. Lists every active
  session for the signed-in user with: trust label, IP, short
  user-agent summary (e.g. `macOS · Safari`), created_at, last-seen
  relative time, expires relative time. Current device gets an
  accent rail. Revoke buttons land in 0.5.x once the password-reset
  flow exercises the invalidation engine end-to-end.
- **`email::Mailer` trait + `LogMailer` default + `Mail::framework_envelope`.**
  Defines the recovery-flow email primitive without locking the
  framework into SMTP. The envelope appends a fixed security footer
  (system name, timestamp, source IP, device summary, "if this was
  not you" guidance) to every framework-emitted message.
  `MailerError::ConfigurationMissing` is the hard-boot-failure
  signal R1 will check.
- **`audit::redact` helpers** (`redact_password`, `redact_token`,
  `redact_mfa_secret`, `redact_backup_code`). `redact_token` returns
  a non-reversible `<token:…XXXXXXXX>` 8-char SHA-256 fingerprint —
  enough for log correlation without leaking the plaintext.
- **`middleware::correlation_id`** stamps a UUID v7 on every
  request, surfaces it in the `x-correlation-id` response header,
  and stashes it in the request context for the audit pipeline.
  Honours an inbound `x-correlation-id` header when shape-safe;
  replaces adversarial inputs with a fresh v7. Designed to be
  installed **before** `csrf_protect` so 403/429 rejections still
  trace.
- **Audit row gains structure**: new `metadata JSONB`,
  `correlation_id TEXT`, `session_id BIGINT` columns + partial
  indexes on the latter two. Existing handlers populate
  `correlation_id` from the new middleware; `session_id` and
  `metadata` are R1+ population.
- **Internal `audit::AuditEvent` enum** (pub(crate) in 0.4.0) with
  18 variants covering R0-R4 actions. Drift tests assert as_str()
  uniqueness + snake_case shape so a copy-paste error fails CI.
  Public typed surface lands in 0.5.x.
- **`DESIGN_SESSIONS.md`** — canonical lifecycle reference: state
  machine with invariants, token storage shape, trust-escalation
  rotation rules, single-writer guarantee on `revoked_at`,
  `SessionInvalidationReason` ↔ behaviour table, expiration / sweeper
  paths, active-sessions UI contract, forensic correlation_id
  story, versioning policy.
- **`DESIGN_AUDIT.md`** — companion. Documents row shape, typed
  evolution path, redaction helpers, forensic chain queries,
  required middleware ordering, reserved metadata JSONB keys.

### Changed

- **Logout is now a soft revoke.** `do_logout` calls
  `auth::logout_session` which routes through `invalidate_sessions`
  with `reason = Logout`, setting `revoked_at = NOW()` and
  `revoked_reason = 'logout'`. The row is preserved for audit;
  `purge_expired_sessions` deletes it once `expires_at` passes.
- **Session lookup excludes revoked rows.** `revoked_at IS NULL` is
  part of every active-session query — a logged-out cookie cannot
  re-authenticate regardless of which lookup path matched.
- **`templates::render` logs render failures at error level.**
  Previously the minijinja error was wrapped in `Error::Internal`
  and never surfaced in any log target. The fix surfaced during R0
  validation when a missing `EMBEDDED_TEMPLATES` registration
  silently produced 500s. Diagnostic value is high; attack surface
  is none — the message contains the template name and the
  structured minijinja error, neither of which is user-supplied or
  secret.
- **Required middleware ordering.** Projects must install
  `middleware::correlation_id` before `csrf_protect` in their
  `Router::new()` chain. Without it, audit rows land with NULL
  `correlation_id` (the framework does not fabricate ids).

### Schema

Additive, idempotent, runs on every boot:

- `rustio_sessions`: `session_id BIGINT` (sequence-backed),
  `token_hash TEXT`, `device_id TEXT` (reserved),
  `trust_level TEXT` CHECK (`authenticated`/`elevated`/`mfa_verified`),
  `elevated_until TIMESTAMPTZ`, `parent_session_id BIGINT`,
  `revoked_at TIMESTAMPTZ`, `revoked_reason TEXT`. New unique
  partial indexes on `session_id`, `token_hash`, plus `(user_id) WHERE revoked_at IS NULL`
  and `parent_session_id` partial.
- `rustio_admin_actions`: `metadata JSONB`, `correlation_id TEXT`,
  `session_id BIGINT` + partial indexes.

### Dependencies

- `sha2 = "0.10"` (new) — session-token hashing at rest.
- `uuid` gains the `v7` feature for time-sortable correlation ids.

[0.4.0]: https://github.com/abdulwahed-sweden/rustio-admin/releases/tag/v0.4.0

## [0.3.0] — 2026-05-08

Authority + design-system stabilization release. Server-side guards
enforce the rank model on every authority mutation; group permissions
render as a model × action matrix instead of a flat alphabetical list;
foreign-key columns on list pages resolve to display labels with
click-throughs; the framework's canonical accent moves to
teal-emerald with the previous terracotta retired. New
`DESIGN_SYSTEM.md` codifies the authority + visual vocabulary, and a
PR template gates token changes behind a visual regression checklist.

> **Migrating from 0.2.x:** bump `rustio-admin = "0.3"` in your
> project's `Cargo.toml`, run `cargo update -p rustio-admin`, then
> hard-refresh the admin in your browser so the new `admin.css` is
> fetched. If your project redefined `--rio-accent` in its own CSS
> to swap to teal, that block is now redundant and can be deleted —
> the framework default already serves the same value. Any other
> `--rio-*` token redefinitions in project CSS are now considered
> framework forks; see `DESIGN_SYSTEM.md §2` for the supported
> override paths.

[0.3.0]: https://github.com/abdulwahed-sweden/rustio-admin/releases/tag/v0.3.0


### Added

- **Authority guards (`auth::guards`).** Five composable, server-side
  guards enforce the rank model on every authority mutation:
  `enforce_self_demote_safe`, `enforce_cross_rank_safe`,
  `enforce_role_ceiling`, `enforce_no_orphan_role`. Guards return
  `Error::Forbidden` with a clear human-readable reason; the HTTP
  layer renders that reason on the standard 403 page. UI hiding is
  treated as a courtesy, not security — every guard runs on POST
  regardless of what the form said.
- **Generalised orphan-prevention.** `auth::would_orphan_role(role)`
  and `auth::would_orphan_protected()` cover every entry in
  `auth::protected_roles()` (currently `[Administrator, Developer]`)
  instead of only Developer. The pure verdict
  `auth::verdict_for_orphan_role(...)` is exposed for unit testing
  without a `Db`.
- **Audit logging on authority mutations.** `do_user_edit`,
  `do_new_user`, `do_user_delete`, `do_new_group`, `do_group_edit`,
  and `do_group_delete` now write `rustio_admin_actions` rows with
  actor, target, IP (read from `x-forwarded-for` / `x-real-ip`), and
  a diff summary (role before / after, group / permission add and
  remove sets, password-reset flag).
- **Role dropdown ceiling.** `role_select_options(editor_rank)`
  filters out roles strictly above the editor's own rank in both
  user-new and user-edit forms. Server-side `enforce_role_ceiling`
  catches forged POSTs as defense-in-depth.
- **Group permissions matrix.** The Group edit page now lays out
  permissions as a model × action grid (View / Add / Change / Delete
  columns, one row per model) instead of a flat 60+ row alphabetical
  checkbox list. Permissions whose codename doesn't fit the
  `<table>.<action>_<singular>` pattern fall through to a collapsed
  "Other permissions" group below the matrix so nothing is silently
  dropped. Per-row "All" button toggles every permission for that
  model in one click; degrades to plain multi-checkbox UX without JS.
- **Foreign-key list-cell hydration.** List pages now resolve every
  `belongs_to` column on the current page from the raw id to the
  target row's display field, and wrap the cell in an
  `<a href="/admin/{admin_name}/{id}/edit">…</a>` so foreign-key
  columns become click-throughs to the related row. The hydration is
  N+1-safe by construction: at most one batched
  `SELECT id, <display> FROM <target> WHERE id = ANY($1)` per FK
  column on the entry, regardless of page size. Stale or
  display-field-less FKs leave the raw id in place (no 500). New
  `CellLink` type and a parallel `cell_links` vector on `ListRow`
  carry the link metadata.
- **`ListRowCtx.links: HashMap<String, String>`** exposes per-column
  FK click-through URLs to the list template.
- **`admin.css` token sectioning.** Banner comments
  (`/* === Tokens — typography ===`, `colors`, `spacing`,
  `components`) make canonical token blocks explicit so future
  feature branches can't silently override the design system without
  a visible diff in those sections.

### Changed

- **Canonical accent moved from terracotta to teal-green.** Framework
  default `--rio-accent` is now `#0F8C7E` (light) / `#3FAA9D` (dark).
  Replaces the previous `#A0341A` / `#C84934`. Same value the
  Bosphorus & Sham downstream had been overriding to in
  `dashboard.css`; promoting it to the framework default removes the
  duplicate-token-system risk and makes one accent the single source
  of truth across every admin page (login, list, edit, group
  permissions, dashboard).
- **`Role::rank()` widened to `u32`** with spaced numeric values
  (`User=100 / Staff=300 / Supervisor=600 / Administrator=900 /
  Developer=1000`) so projects extending the rank ladder via group
  labels have headroom between framework tiers. Compare relatively;
  never match literally.
- **`admin/list.html`** wraps a cell in `<a class="rio-fk-link">` when
  `row.links[<field>]` is set. Cells without a registered relation
  render unchanged.

### Deprecated

- **`auth::would_orphan_developers`** — kept as a thin wrapper around
  `auth::would_orphan_role(_, _, Role::Developer, _, _)` so external
  callers keep compiling, but new code should use
  `would_orphan_protected` to cover Administrator orphan-prevention
  too.

### Documentation

- **`DESIGN_SYSTEM.md`** at the repo root: canonical doctrine for
  the framework's authority + visual vocabulary. Three principles
  stated explicitly (UI hiding is reflection, not security · Rank
  controls WHO; permissions control WHAT · Groups are permission
  bundles, not authority roots), token ownership rules, Arabic
  typography rules, branch + merge expectations, and a
  versioning-of-the-design-system policy.
- **`.github/pull_request_template.md`**: any PR touching
  `admin.css`, token definitions, font-family declarations, or
  `:root` blocks must complete a Token disclosure section (tokens
  changed / migration impact / regression risk) and walk an 8-item
  visual regression checklist (login / dashboard / tables / forms /
  dark mode / Arabic rendering / mobile width / permission matrix)
  before merging.

## [0.2.1] — 2026-05-07

CLI-only patch. `rustio-admin` and `rustio-admin-macros` stay at 0.2.0.

### Fixed

- **`rustio startproject` scaffold template** pinned new projects to
  `rustio-admin = "0.1"`. Cargo's `"0.1"` constraint resolves to
  `>=0.1.0, <0.2.0`, so anyone who ran `cargo install rustio-admin-cli`
  immediately after the 0.2.0 release would get a project locked to
  the previous framework line and miss every 0.2.0 feature. The
  scaffold now pins to `rustio-admin = "0.2"`.

  Re-install with `cargo install rustio-admin-cli --force` to pick
  up the corrected template; existing projects are unaffected (they
  set their own pin in their own `Cargo.toml`).

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
