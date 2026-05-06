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

### Known gaps (next phases)

- `rustio-admin-cli` is a stub — `startproject` / `startapp` /
  `migrate apply` / `user create` land before the v0.1.0 tag.
- No live browser-walk acceptance test recorded yet (waits on the
  CLI's `user create` so a clean Postgres can boot a populated
  admin).
- Crates.io publish blocked on `rustio-admin-macros` going up first
  (path-only deps can't be published).
