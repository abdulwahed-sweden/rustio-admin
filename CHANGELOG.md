# Changelog

All notable changes to `rustio-admin` are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project
adheres to [SemVer](https://semver.org/) once it leaves the alpha track.

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
