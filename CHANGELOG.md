# Changelog

All notable changes to `rustio-admin` are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project
adheres to [SemVer](https://semver.org/) once it leaves the alpha track.

## [Unreleased]

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

### Known gaps (next phases)

- Crates.io publish blocked on `rustio-admin-macros` going up first
  (path-only deps can't be published).
