# Changelog

All notable changes to `rustio-admin` are recorded here. Format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
the project adheres to [SemVer](https://semver.org/) once it
leaves the alpha track.


## Releases at a glance

| Version   | Date       | Headline                                                                          |
|-----------|------------|-----------------------------------------------------------------------------------|
| **0.6.0** | 2026-05-10 | Admin-driven recovery, re-auth wall, login throttling, forced password rotation.  |
| **0.5.0** | 2026-05-09 | Self-service password recovery, active-session controls.                          |
| **0.4.0** | 2026-05-09 | Session lifecycle, centralised invalidation, audit foundations.                   |
| **0.3.0** | 2026-05-08 | Authority guards, design-system stabilisation, audit on user/group writes.        |
| **0.2.1** | 2026-05-07 | CLI scaffold-template fix.                                                        |
| **0.2.0** | 2026-05-07 | List-view toolbar, bulk actions, theme architecture, dark mode.                   |
| **0.1.1** | 2026-05-07 | Self-hosted fonts, typography token system.                                       |
| **0.1.0** | 2026-05-07 | Initial public release.                                                           |


## [Unreleased]

No changes yet.


## [0.6.0] — 2026-05-10

R2 of the universal account-recovery architecture. Covers the
admin-initiated path: password reset (email or temp-password
mode), lock and unlock, forced rotation, session revocation
without locking. Plus auto-throttle on failed logins and a
re-auth wall on every destructive admin action.

> **Migration from 0.5.x**
>
> Bump `rustio-admin = "0.6"` and run `cargo update -p rustio-admin`.
>
> Schema is additive: three new columns on `rustio_users`
> (`failed_login_count`, `last_failed_login_at`, `locked_until`)
> plus a partial index. Existing users are unaffected — the
> counter defaults to 0, lockout defaults to NULL.
>
> No middleware changes. All routes register through
> `register_admin_routes`.
>
> Doctrine 22 holds: `auth::sessions::invalidate_sessions`
> remains the sole writer of `revoked_at` — three SET arms
> (lines 398 / 413 / 426 in `auth/sessions.rs`).

### Highlights

- **Admin-driven password reset.** `POST /admin/users/:id/reset-password`
  with two modes:
  - **email** — admin-initiated reset token; target consumes via
    R1's `/admin/reset-password/<token>` flow.
  - **temp_pw** — 16-char URL-safe-base64 password rendered once
    on the admin success page; sets `must_change_password = TRUE`,
    revokes every session.
- **Manual lock and unlock.** `POST /admin/users/:id/lock`
  (duration presets: 15 min / 1 h / 24 h / 7 d / indefinite +
  freeform-minutes) and `POST /admin/users/:id/unlock`. Lock
  revokes every session; unlock zeroes the throttle counter.
- **Admin revoke-sessions.** `POST /admin/users/:id/revoke-sessions`
  — sibling of lock without the `locked_until` write.
- **Auto-throttle on failed logins.** 5 failures / 10 min →
  15-minute soft lock. Sessions are not revoked; locking refuses
  future sign-ins.
- **Re-auth wall.** 15-minute elevated-session window. Every
  destructive admin action requires a fresh password verify.
- **Forced password rotation** via `must_change_password` with a
  whitelisted interstitial path.

### Public API

- `auth::recovery_admin::AdminActor` — bundles `user_id` + `email`.
  Email hashes into `metadata.actor_email_hash` (8-char SHA-256).
- `auth::LoginThrottle { max_attempts, window_minutes, lock_minutes }`.
  Default 5 / 10 / 15 via `RecoveryPolicy::login_throttle()`.
- `RecoveryPolicy::reauth_window()` (default 15 minutes) and
  `scope_for(&Identity)` for multi-tenant overrides.
- `LogEntry::actor_user_id: Option<i64>`, persisted under
  `metadata.actor_user_id`. Builder: `LogEntry::with_actor(id)`.
- `Identity::must_change_password: bool`; parallel field on
  `StoredUser`.
- `AuditEvent::ForcedPasswordChangeCompleted` —
  `"forced_password_change_completed"`. The four pre-declared R2
  variants (`PasswordResetByOther`, `AccountLocked`,
  `AccountUnlocked`, `SessionsRevokedByOther`) light up.
- 12 new routes registered through `register_admin_routes` —
  reset, lock, unlock, revoke-sessions, reauth,
  must-change-password.

See [`DESIGN_R2_ORGANISATIONAL.md`](./DESIGN_R2_ORGANISATIONAL.md)
for the state machines, audit metadata schemas, and locked
decisions.

### Behaviour changes

- **`do_login` rewritten.** Pre-R2 returned three response shapes
  (401 wrong creds, 403 inactive, no lockout). Post-R2 returns a
  single uniform 401 across every failure mode.
- **CLI password floor 8 → 10.** `rustio user create` delegates
  to `DefaultPasswordPolicy::new()`. Same floor as
  admin-create-user and self-recovery.
- **Admin Add-user form respects policy override.** Pre-R2 the
  form hardcoded 8 chars; post-R2 reads from
  `Admin::active_password_policy()`. A project with
  `min_length = 16` sees 16 in the hint and has 16 enforced.
- **Admin-edit form: password field removed.** The legacy
  `new_password` input on `/admin/users/:id/edit` is gone
  (`DESIGN_RECOVERY.md` §14.4). Admin password resets go through
  the dedicated `/admin/users/:id/reset-password` route.
- **Auto-throttle does not revoke sessions.** Locking only
  refuses future sign-ins. Existing sessions continue until the
  next request. Doctrine D4 + §3.3.

### Schema

Additive, idempotent, runs at boot:

- `rustio_users.failed_login_count INT NOT NULL DEFAULT 0`.
- `rustio_users.last_failed_login_at TIMESTAMPTZ`.
- `rustio_users.locked_until TIMESTAMPTZ`.
- Partial index `rustio_users_locked_until_idx ON (locked_until) WHERE locked_until IS NOT NULL`.

No data backfill required. Indefinite manual locks encode as a
year-9999 timestamp so the partial index continues to find them.

### Documentation

- [`DESIGN_R2_ORGANISATIONAL.md`](./DESIGN_R2_ORGANISATIONAL.md)
  added — the canonical R2 contract.
- README architecture-doctrine table updated with a row pointing
  to `DESIGN_R2_ORGANISATIONAL.md`.

#### Internal

- New `auth::recovery_admin` submodule — every R2 runtime fn.
- New `admin::admin_recovery_handlers` submodule — every R2 HTTP
  handler.
- New `Row::get_optional_datetime` ORM helper — closes the
  nullable-`TIMESTAMPTZ` read gap.
- `admin::builtin` module visibility raised to `pub(crate)`.
- `admin::handlers::record_session_revocations` raised to
  `pub(super)`.
- `admin::audit::build_persisted_metadata` merge helper inserts
  `actor_user_id` into the metadata object before binding.

#### Tests

- +58 unit tests across the R2 commit chain (162 → 220). Pure /
  DB-free: type-level invariants, pure helpers (`validate_return_to`,
  `parse_lock_duration`, `actor_email_fingerprint`,
  `random_temp_password`, `MUST_CHANGE_WHITELIST` membership,
  `LockDuration` time math). The `AuditEvent` drift tests pick
  up the new variant automatically.
- Doctrine 22 grep is part of every R2 commit's pre-commit gate
  — three SET arms in `auth::sessions::invalidate_sessions`,
  unchanged across the entire R2 chain.

#### Deferred

- Testcontainers Postgres integration suite ships gated behind
  `--features integration-test`. Covers the SQL paths in
  `record_failed_login`, `check_account_lockout`,
  `promote_session_elevated`, `check_session_elevated`,
  `lock_user_account`, `admin_set_temp_password`, plus the
  re-auth and forced-rotation handlers' DB-touching steps.


## [0.5.0] — 2026-05-09

R1 of the universal account-recovery architecture. Self-service
password recovery is end-to-end: forgot link → email → reset
form → sign-in. Every session across every device is revoked at
consume time. Active-session controls (revoke single / others /
all) are wired on the existing `/admin/account/sessions` page.
The authenticated `/admin/password_change` flow is brought into
parity with the recovery doctrine.

> **Migration from 0.4.x**
>
> Bump `rustio-admin = "0.5"` and run `cargo update -p rustio-admin`.
>
> Schema is additive: new `rustio_password_reset_tokens` table
> plus two new columns on `rustio_users`. Existing users and
> sessions are unaffected.
>
> No middleware changes — `correlation_id` BEFORE `csrf_protect`
> (added in 0.4.0) is still the only ordering constraint.
> Recovery routes register through `register_admin_routes`.
>
> Production deployments wiring a real `Mailer` should opt the
> policy into strict mode:
> `RecoveryPolicy::strict_mailer_required(true)` makes the
> framework refuse to start with the default `LogMailer`.

### Highlights

- **Self-service forgot/reset password flow.** Five new routes —
  `GET /admin/forgot-password`, `POST /admin/forgot-password`,
  `GET /admin/forgot-password/sent`,
  `GET /admin/reset-password/:token`,
  `POST /admin/reset-password/:token`.
- **Email-link reset tokens.** 256-bit URL-safe-base64. Plaintext
  leaves the framework only in the email body. The DB stores
  `sha256(token)` only.
- **Atomic single-use consume.** Single SQL statement
  (`UPDATE … RETURNING`) flips the row exclusively. Concurrent
  submissions resolve as exactly one Consumed and one Invalid.
- **Active-sessions revoke buttons.** Three new POSTs at
  `/admin/account/sessions/...` — single revoke, others, all.
- **Authenticated password change revokes other devices.** Goes
  through `invalidate_sessions(UserExceptCurrent, UserRequested)`.
  Current device stays signed in.
- **Reset-token sweeper.** 7-day forensic-retention window;
  integrated into the existing 10-minute session-sweeper tick
  with independent failure isolation.

### Public API

- `auth::PasswordPolicy` trait + `DefaultPasswordPolicy`. Length
  floor 10, Unicode `char` count, no complexity-class rules
  (NIST SP 800-63B Appendix A). Override via
  `Admin::password_policy(Arc::new(...))`.
- `auth::RecoveryPolicy` trait + `DefaultRecoveryPolicy` —
  `reset_token_ttl()` (1h), `request_rate_limit()` (5 / 15min /
  IP), `consume_rate_limit()` (10 / 5min / IP),
  `strict_mailer_required()` (false default), `public_site_url()`
  with a header-derivation default.
- `Admin::mailer(...)` builder + `Admin::active_mailer()`
  accessor. `Admin::has_custom_mailer()` flag drives the
  strict-mailer guard.
- `AuditEvent` promoted to `pub` with `#[non_exhaustive]`. New
  variant `PasswordChangedSelf`. Variant strings locked-in by
  `audit_event_existing_variants_have_stable_strings`.
- `LogEntry::with_event(AuditEvent)` builder — typed-event
  boundary. Adds `event: Option<AuditEvent>` field; existing
  struct-literal call sites pass `event: None`.

### Behaviour changes

- **Authenticated password changes sign out other devices.**
  Pre-R1 left other sessions live. R1 closes the drift through
  `invalidate_sessions(UserExceptCurrent, UserRequested)`.
- **Default password minimum length 8 → 10.** Existing users
  with shorter passwords are not forced to change — the policy
  fires only on new passwords during a change or reset.
- **Recovery tokens retained 7 days after expiry.** Forensic
  window for audit correlation, abuse investigation, and
  operational debugging.
- **Strict-mailer mode can fail startup intentionally.** When
  `strict_mailer_required(true)` is set and the default
  `LogMailer` is still in place, `register_admin_routes` panics
  with an operator-actionable error.
- **`MIN_PASSWORD_LEN` constant removed from `admin/handlers.rs`.**
  Was effectively private. The CLI's parallel constant remains
  at 8 chars in 0.5.0; R2 unifies both surfaces.

### Security

- Token hashes only. Plaintext lives in the email body, never in
  the DB, log lines, or audit metadata. `IssueOutcome` and
  `ConsumeOutcome` Debug formats are property-tested token-free.
- Uniform outward responses on the recovery flow —
  `do_forgot_password` always 303s to
  `/admin/forgot-password/sent`; `do_reset_password` renders the
  same "no longer valid" page across unknown / expired /
  consumed / rate-limited tokens.
- No direct `revoked_at` writes. Doctrine 22 single-writer
  invariant preserved across all R1 paths — a
  `grep -rE "revoked_at\s*="` across `crates/` returns only
  `auth::sessions::invalidate_sessions`.
- CSRF preserved across recovery flows.
- `PasswordPolicyError` carries no plaintext. Display + Debug
  property-tested plaintext-free.

### Schema

Additive, idempotent:

- New table `rustio_password_reset_tokens` with `token_hash`,
  `user_id`, `requested_at`, `expires_at`, `consumed_at`,
  `mail_status`, `requested_ip`, `requested_user_agent`,
  `correlation_id`, plus a partial unique index on
  `(token_hash) WHERE consumed_at IS NULL`.
- `rustio_users.must_change_password BOOLEAN NOT NULL DEFAULT FALSE`
  (R1 declares; R2 enforces).
- `rustio_users.password_changed_at TIMESTAMPTZ`. Stamped by
  `auth::set_password` on every change.

### Documentation

- [`DESIGN_RECOVERY.md`](./DESIGN_RECOVERY.md) added — the
  canonical R1 contract.
- README *Architecture doctrine* section listing the four
  contracts (`DESIGN_SYSTEM`, `DESIGN_SESSIONS`, `DESIGN_AUDIT`,
  `DESIGN_RECOVERY`).
- `docs/getting-started.md` — clarifies CLI password prompts,
  port-8000 troubleshooting, the migrations workflow,
  what-you-get-after-first-login, and the project philosophy.
- `templates/project/README.md.tmpl` mirrors the
  pre-R1 clarifications.

#### Internal

- `auth::recovery` submodule — schema migrations, trait surface,
  runtime primitives (`issue_reset_token`, `consume_reset_token`,
  `check_reset_token_valid`), and the periodic sweeper.
- `admin::recovery_handlers` — five HTTP handlers and the
  `RecoveryState` rate-limit carrier.
- `set_password` stamps `password_changed_at` (single SQL
  statement; signature unchanged).
- `RateLimiter::allow(key)` and `auth::sessions::random_token`
  promoted to `pub(crate)` for the recovery module.
- Recovery sweeper integrated into
  `background::spawn_session_sweeper` with independent failure
  isolation.

#### Public-API summary

The `Admin` struct gains four `pub(crate)` fields and the public
methods `mailer / has_custom_mailer / password_policy /
active_password_policy / recovery_policy / active_recovery_policy
/ active_mailer`. `LogEntry` gains `event: Option<AuditEvent>`
and `with_event(...)`. Existing constructors and accessors are
unchanged; struct-literal call sites add `event: None`.


## [0.4.0] — 2026-05-09

Session lifecycle and recovery foundations. R0 of the universal
account-recovery architecture, documented in
`DESIGN_SESSIONS.md` and `DESIGN_AUDIT.md`. **No password-reset
flow yet** — that ships in R1 (0.5.0). This release lays the
safe foundation: hashed-at-rest session tokens, centralised
invalidation, typed lifecycle vocabulary, audit forensic chain,
and an email abstraction.

> **Migration from 0.3.x**
>
> Bump `rustio-admin = "0.4"` and run `cargo update -p rustio-admin`.
>
> **Add `middleware::correlation_id` BEFORE
> `middleware::csrf_protect`** in your router so audit rows get a
> populated `correlation_id`.
>
> Schema is additive; existing sessions continue to authenticate
> through a 14-day plaintext-fallback window.

### Added

- **Hashed-at-rest session tokens.** `rustio_sessions.token_hash`
  stores `sha256(cookie-token)`; the cookie keeps the plaintext.
  Lookup is hash-first with a plaintext fallback for the 14-day
  transition window.
- **Centralised session invalidation** (`auth::invalidate_sessions`)
  — the single legitimate writer of `rustio_sessions.revoked_at`.
  A `grep -rE "revoked_at\s*="` returns only this function.
  Called by logout, password reset (R1), MFA disable (R3),
  administrative revoke (R2), and trust-escalation rotation.
- **Typed lifecycle vocabulary.** `SessionTrust`
  (`Authenticated < Elevated < MfaVerified` with `satisfies()`),
  `SessionInvalidationReason` (12 variants), `SessionTarget`
  (`User` / `UserExceptCurrent` / `Single`), `Session`
  (read-only view), `InvalidationOutcome`.
- **`auth::list_active_for_user(db, user_id)`** and
  **`auth::current_session_id(db, token)`** for the active-sessions UI.
- **Read-only `/admin/account/sessions` page.** Lists every
  active session for the signed-in user with trust label, IP,
  short user-agent summary, created_at, last-seen, and
  expires-relative time. Revoke buttons land in 0.5.x.
- **`email::Mailer` trait + `LogMailer` default +
  `Mail::framework_envelope`.** Recovery-flow email primitive
  without locking the framework into SMTP. The envelope appends
  a fixed security footer (system name, timestamp, source IP,
  device summary, "if this was not you" guidance).
- **`audit::redact` helpers** — `redact_password`, `redact_token`,
  `redact_mfa_secret`, `redact_backup_code`. `redact_token`
  returns a non-reversible 8-char SHA-256 fingerprint.
- **`middleware::correlation_id`** — UUID v7 stamped on every
  request, surfaced in `x-correlation-id`, stashed in the
  request context for the audit pipeline. Honours an inbound
  `x-correlation-id` header when shape-safe; replaces
  adversarial inputs with a fresh v7. Designed to install
  before `csrf_protect`.
- **Audit row gains structure**: new `metadata JSONB`,
  `correlation_id TEXT`, `session_id BIGINT` columns + partial
  indexes on the latter two.
- **Internal `audit::AuditEvent` enum** (`pub(crate)` in 0.4.0)
  with 18 variants covering R0–R4 actions. Drift tests assert
  `as_str()` uniqueness and snake_case shape. Public typed
  surface lands in 0.5.x.
- **`DESIGN_SESSIONS.md`** — canonical lifecycle reference:
  state machine with invariants, token storage shape, trust
  escalation, single-writer guarantee on `revoked_at`,
  expiration paths, active-sessions UI contract, forensic
  correlation_id story, versioning policy.
- **`DESIGN_AUDIT.md`** — companion: row shape, typed evolution
  path, redaction helpers, forensic chain queries, required
  middleware ordering, reserved metadata JSONB keys.

### Changed

- **Logout is a soft revoke.** `do_logout` calls
  `auth::logout_session` which routes through
  `invalidate_sessions` with `reason = Logout`. The row is
  preserved for audit; `purge_expired_sessions` deletes it
  after `expires_at` passes.
- **Session lookup excludes revoked rows.** `revoked_at IS NULL`
  is part of every active-session query — a logged-out cookie
  cannot re-authenticate regardless of which lookup path
  matched.
- **Required middleware ordering.** Projects must install
  `middleware::correlation_id` before `csrf_protect`.

### Schema

Additive, idempotent, runs on every boot:

- `rustio_sessions`: `session_id BIGINT` (sequence-backed),
  `token_hash TEXT`, `device_id TEXT` (reserved), `trust_level TEXT`
  (CHECK `authenticated`/`elevated`/`mfa_verified`),
  `elevated_until TIMESTAMPTZ`, `parent_session_id BIGINT`,
  `revoked_at TIMESTAMPTZ`, `revoked_reason TEXT`. New unique
  partial indexes on `session_id`, `token_hash`, plus
  `(user_id) WHERE revoked_at IS NULL` and `parent_session_id`
  partial.
- `rustio_admin_actions`: `metadata JSONB`, `correlation_id TEXT`,
  `session_id BIGINT` + partial indexes.

### Dependencies

- `sha2 = "0.10"` (new) — session-token hashing at rest.
- `uuid` gains the `v7` feature for time-sortable correlation
  ids.

[0.4.0]: https://github.com/abdulwahed-sweden/rustio-admin/releases/tag/v0.4.0


## [0.3.0] — 2026-05-08

Authority and design-system stabilisation. Server-side guards
enforce the rank model on every authority mutation; group
permissions render as a model × action matrix; foreign-key
columns on list pages resolve to display labels with
click-throughs; the framework's canonical accent moves to
teal-emerald. New `DESIGN_SYSTEM.md` codifies the authority and
visual vocabulary.

> **Migration from 0.2.x**
>
> Bump `rustio-admin = "0.3"`, run `cargo update -p rustio-admin`,
> then hard-refresh the admin in your browser so the new
> `admin.css` is fetched.
>
> If your project redefined `--rio-accent` in its own CSS to
> swap to teal, that block is now redundant and can be deleted —
> the framework default already serves the same value. Other
> `--rio-*` token redefinitions in project CSS are now
> framework forks; see `DESIGN_SYSTEM.md` §2 for the supported
> override paths.

[0.3.0]: https://github.com/abdulwahed-sweden/rustio-admin/releases/tag/v0.3.0

### Added

- **Authority guards (`auth::guards`).** Five composable
  server-side guards: `enforce_self_demote_safe`,
  `enforce_cross_rank_safe`, `enforce_role_ceiling`,
  `enforce_no_orphan_role`. Guards return `Error::Forbidden`
  with a human-readable reason; the HTTP layer renders that
  reason on the 403 page. Every guard runs on POST regardless
  of what the form said.
- **Generalised orphan-prevention.** `auth::would_orphan_role(role)`
  and `auth::would_orphan_protected()` cover every entry in
  `auth::protected_roles()` (currently `[Administrator, Developer]`)
  instead of only Developer. The pure verdict
  `auth::verdict_for_orphan_role(...)` is exposed for unit
  testing.
- **Audit logging on authority mutations.** `do_user_edit`,
  `do_new_user`, `do_user_delete`, `do_new_group`,
  `do_group_edit`, and `do_group_delete` write
  `rustio_admin_actions` rows with actor, target, IP, and a
  diff summary.
- **Role dropdown ceiling.** `role_select_options(editor_rank)`
  filters out roles strictly above the editor's own rank in
  user-new and user-edit forms. Server-side
  `enforce_role_ceiling` catches forged POSTs as
  defence-in-depth.
- **Group permissions matrix.** The Group edit page lays out
  permissions as a model × action grid (View / Add / Change /
  Delete columns, one row per model) instead of a flat
  alphabetical checkbox list. Permissions whose codename
  doesn't fit `<table>.<action>_<singular>` fall through to a
  collapsed "Other permissions" group below the matrix. Per-row
  "All" button toggles every permission for that model;
  degrades to plain multi-checkbox UX without JS.
- **Foreign-key list-cell hydration.** List pages resolve every
  `belongs_to` column on the current page to the target row's
  display field, wrapping the cell in
  `<a href="/admin/{admin_name}/{id}/edit">…</a>`. N+1-safe by
  construction: at most one batched
  `SELECT id, <display> FROM <target> WHERE id = ANY($1)` per
  FK column. Stale or display-field-less FKs leave the raw id
  in place. New `CellLink` type and parallel `cell_links`
  vector on `ListRow`.
- **`ListRowCtx.links: HashMap<String, String>`** exposes
  per-column FK click-through URLs to the list template.
- **`admin.css` token sectioning.** Banner comments make
  canonical token blocks explicit so feature branches can't
  silently override the design system without a visible diff.

### Changed

- **Canonical accent moved from terracotta to teal-green.**
  Framework default `--rio-accent` is now `#0F8C7E` (light) /
  `#3FAA9D` (dark), replacing `#A0341A` / `#C84934`. Same value
  the Bosphorus & Sham downstream had been overriding to;
  promoting it to the framework default removes the
  duplicate-token-system risk.
- **`Role::rank()` widened to `u32`** with spaced numeric values
  (`User=100 / Staff=300 / Supervisor=600 / Administrator=900 /
  Developer=1000`) so projects extending the rank ladder via
  group labels have headroom. Compare relatively, never match
  literally.
- **`admin/list.html`** wraps a cell in `<a class="rio-fk-link">`
  when `row.links[<field>]` is set. Cells without a registered
  relation render unchanged.

### Deprecated

- **`auth::would_orphan_developers`** — kept as a thin wrapper
  around `would_orphan_role(_, _, Role::Developer, _, _)` for
  backwards compat. New code should use `would_orphan_protected`
  to cover Administrator orphan-prevention too.

### Documentation

- **`DESIGN_SYSTEM.md`** — canonical doctrine for the
  framework's authority and visual vocabulary. Three principles
  stated explicitly (UI hiding is reflection, not security;
  rank controls WHO, permissions control WHAT; groups are
  permission bundles, not authority roots), token ownership
  rules, Arabic typography rules, and a versioning policy.
- **`.github/pull_request_template.md`** — any PR touching
  `admin.css`, token definitions, font-family declarations, or
  `:root` blocks must complete a Token disclosure section
  (tokens changed / migration impact / regression risk) and
  walk an 8-item visual-regression checklist.


## [0.2.1] — 2026-05-07

CLI-only patch. `rustio-admin` and `rustio-admin-macros` stay
at 0.2.0.

### Fixed

- **`rustio startproject` scaffold template** pinned new
  projects to `rustio-admin = "0.1"`. Cargo's `"0.1"`
  constraint resolves to `>=0.1.0, <0.2.0`, so anyone who ran
  `cargo install rustio-admin-cli` immediately after the 0.2.0
  release would get a project locked to the previous framework
  line and miss every 0.2.0 feature. The scaffold now pins to
  `rustio-admin = "0.2"`.

  Re-install with `cargo install rustio-admin-cli --force` to
  pick up the corrected template; existing projects are
  unaffected.


## [0.2.0] — 2026-05-07

Design-language unification across list, form, and Auth pages.
Dark mode shifts from a high-contrast terminal aesthetic to a
calm graphite workspace. New list-view toolbar (filters / sort /
per-page / active-filter pills / numbered pagination / search
glyph) ships with full URL state preservation. Projects can
register custom bulk actions alongside the built-in delete.

> **Breaking change.** `AdminTheme` is now an override-patch
> type with `Option<String>` fields instead of `String`
> snapshots; default is *all `None`* (no inline `<style>`
> emitted at all). `Admin::accent()` returns `Option<&str>`.
> See **Migrating from 0.1.x** below.

### Added — list view toolbar

- **Filters dropdown.** Inferred from `bool` columns +
  `ModelAdmin::list_filter()`. Each filter group has an
  All / Yes / No tri-state. Active selections preserve
  across sort, search, pagination, and per-page changes.
- **Sort dropdown.** Builds from `ModelAdmin::list_display()`
  column metadata, with sortable columns flagged. Active sort
  preserved across filter / search / pagination changes.
- **Per-page picker.** 25 / 50 / 100 / 200 with the active
  value highlighted. URL state preserved on switch.
- **Active-filter pills.** A row of removable pills
  beneath the toolbar shows every active filter / sort /
  search; removing a pill drops just that constraint.
- **Numbered pagination.** Standard windowed pagination
  with "first / prev / 1…7 / next / last", smart truncation
  for large counts, and current-page accent ring.
- **Search-glyph affordance.** `ModelAdmin::search_fields()`
  surfaces a search icon in the toolbar; the search input
  expands inline.
- **URL state preservation across every widget.** Toolbar
  state lives in the query string (`?filter=foo&sort=col-desc&q=…`
  etc.) so back-button / refresh / share-link all behave.

### Added — bulk select + bulk actions

- **Per-row + master checkbox** in the list view's first
  column. Master toggles all rows on the visible page;
  per-row selections survive pagination via `?ids=...` query
  param.
- **Bulk-actions menu.** `ModelAdmin::bulk_actions()` returns
  `Vec<BulkAction>` with `(slug, label, danger)`. Built-in
  delete is registered automatically. Submitting runs the
  framework-supplied `confirm` page first (lists the affected
  ids), then the project-defined handler.
- **Built-in `delete` bulk action.** Walks the IDs through
  `Model::delete_by_id(&Db, id)`, audit-emitting one
  `ActionType::Delete` row per affected object.

### Added — form view + Auth refresh

- **Editorial form pages.** Single column at 880-px max
  width; sticky bottom action bar (Save / Cancel / Delete);
  inline help under fields; consistent typography and spacing
  with list view.
- **Auth pages.** Login / password change / forbidden / 404 /
  500 share the same chrome and tokens as the admin proper.
- **Sticky sidebar at tablet+.** Sidebar follows scroll on
  ≥ 768 px; collapses to a hamburger drawer on mobile.

### Added — design tokens

- **Calm graphite dark mode.** Page background `#0e1216`;
  surfaces `#1a1f25` / `#222932`; text `#e5e7eb` / muted
  `#94a3b8`. Accent stays consistent across themes; borders
  one shade lighter than each theme's surface.
- **Spacing scale 1..8** with `--rio-s1` 4px through
  `--rio-s8` 56px; consumed across every component for
  consistent rhythm.

### Added — code-level hooks

- **`Admin::version()`** returns the framework's published
  version string (`env!("CARGO_PKG_VERSION")`); rendered in
  the sidebar footer.
- **`Admin::has_relation(...)`** + relation registry — the
  scaffolding the 0.3.0 FK list-cell hydration sits on.

### Changed — theme architecture

- **`AdminTheme` is now an override-patch type.** Fields are
  `Option<String>`; default is all `None` (zero inline
  `<style>` emitted). `admin.css` is the single source of
  truth. Projects override per-token via `Admin::theme(...)`.
- **`Admin::accent()` returns `Option<&str>`** instead of
  `&str`. Rendering callers fall back to the CSS-side default.

### Changed — dark mode

- **Refactored from OLED-black to graphite.** New tokens above
  replace the previous near-black palette. Borders +
  muted-text + text are all bumped one notch lighter for
  AAA contrast on the new surfaces.

### Changed — list view rendering

- **Cell rendering is now type-aware.** Boolean columns render
  as Yes / No badges; date columns as relative-time chips;
  text truncates at column-width with a hover popover.
- **Empty-state treatment** ships out of the box for empty
  filters and empty result sets (distinct messaging).

### Changed — layout

- **3 breakpoints** (mobile / tablet / desktop) with a single
  shared layout grid. Sidebar collapses below 768 px.

### Migrating from 0.1.x

If your project doesn't customise the theme, no changes
required.

If your project does call `Admin::theme(AdminTheme { … })`
with `String` field values, switch to the override-patch form:

```diff
- AdminTheme {
-     accent: "#0F8C7E".to_string(),
-     surface: "#ffffff".to_string(),
-     text: "#1f2937".to_string(),
- }
+ AdminTheme {
+     accent: Some("#0F8C7E".to_string()),
+     ..Default::default()
+ }
```

`Admin::accent()` callers handle the new `Option<&str>` return
type — fall through to a CSS-side default when `None`.

The bulk-actions registry adds a `BulkAction { slug, label,
danger }` shape; if a project registered custom bulk actions
in 0.1.x via an experimental override, switch to
`ModelAdmin::bulk_actions()` returning `Vec<BulkAction>`.

The list-view URL parameters now include `filter=…`, `sort=…`,
`q=…`, `per_page=…`, `ids=…`; project-side bookmarks built on
the previous flat URL shape need updating.

The CSS allow-list (the framework's `:root --rio-*` token
namespace) is now stable; project CSS that overrode tokens
via custom selectors should switch to the override-patch
`Admin::theme(...)` API. The framework's allow-list now
starts at 25.


## [0.1.1] — 2026-05-07

Design-system pass. No public API surface changes — the
typography, font, and brand-color work all happens behind the
existing `Admin`, `AdminTheme`, and template-override surfaces.
Drop-in upgrade from 0.1.0.

### Added

- **Self-hosted fonts (SIL OFL-1.1)** baked into the binary,
  served at `/static/fonts/*.woff2` with year-long immutable
  cache:
  - Geist Variable + Geist Mono Variable — Latin UI + code
    (single woff2 each covers full `wght` 100..900).
  - Tajawal 400 / 500 / 700 — Arabic UI surfaces.
  - Noto Naskh Arabic Variable — Arabic body / paragraph copy.
  - All filtered by `unicode-range` so Latin-only pages pay
    zero Arabic download cost. Total embedded: ~270 KB.
- **Complete typography token system** in `admin.css` — three
  family tokens, a 9-step size scale (`--rio-fs-xs` 13px
  through `--rio-fs-display` 40px), four line-height tokens
  including `--rio-lh-arabic: 1.9`, four weight tokens, and
  Latin-only tracking tokens that auto-reset for Arabic / RTL.
- **`:lang(ar)` / `[dir="rtl"]` resolution rules** — Arabic
  text picks up Tajawal (UI) or Noto Naskh (body) automatically;
  Geist's stylistic alternates are stripped so joining-script
  shaping stays intact.
- **`--rio-surface-2` / `--rio-border-strong` / `--rio-text-subtle`**
  tokens for secondary surfaces, heavy outlines, and tertiary
  text.

### Changed

- **Default brand accent is now `#A0341A`** (Andalusian
  crimson), replacing the previous cobalt `#2563EB`. Applies
  in three places — `AdminTheme::default()`, `admin.css`
  `:root --rio-accent`, and `render::hex_to_rgb_triplet`
  fallback. Projects override via `Admin::theme(...)` or
  `Admin::accent_color(...)`.
- **Tightened light palette** for stronger reading contrast:
  `--rio-bg` `#f4f6fb` → `#ebeef4`, `--rio-text-muted`
  `#4b5563` → `#3d4452`, `--rio-border` `#d1d5db` → `#cdd3df`.
  Dark-mode tokens similarly bumped. Every text/surface pair
  clears WCAG AAA in both themes.
- **Body font-size** raised from 14px to **16px**; minimum
  helper-text size enforced at **13px**; table cells at
  **15px**; headings rescaled (h1 34px, h2 26px, h3 22px).
  Mobile bumps `html` to 16.5px below 600px.
- **Sidebar + topbar typography** polished — sidebar links 15px
  medium, topbar identity 15px regular, theme toggle 13px
  medium.
- **`/static/admin.css` and `/static/admin.js`** Cache-Control
  flipped from `public, max-age=3600` to
  `no-cache, must-revalidate` so theme + design tweaks roll
  out the moment the binary restarts. Fonts keep their
  year-long immutable cache.

### Removed

- **IBM Plex Sans Arabic** dropped from the bundled fonts —
  Tajawal + Noto Naskh Arabic cover the UI/body split. Anyone
  who customised templates to reference `"IBM Plex Sans Arabic"`
  by name will need to switch to `"Tajawal"` /
  `"Noto Naskh Arabic"`; the `--rio-font-arabic` and
  `--rio-font-arabic-body` tokens resolve both automatically.


## [0.1.0] — 2026-05-07

First public release. Strategic-reset rollout of phases 1–15
plus the live browser walk and the operator CLI is
feature-complete.

### Added

- **`ModelAdmin` trait** with seven hooks: `list_display`,
  `list_filter`, `search_fields`, `ordering`, `list_per_page`,
  `readonly_fields`, `fieldsets`. Every method has a default
  body — projects write `impl ModelAdmin for X {}` to opt in,
  override individual methods to customise.
- **Generic admin runtime.** `Admin::new().model::<M>()`
  registers a Postgres-backed CRUD page;
  `register_admin_routes` mounts every URL the admin needs
  onto a `Router`. List / create / edit / delete /
  per-object history all wired.
- **Built-in user / group pages** at `/admin/users/*` and
  `/admin/groups/*`, plus `/admin/password_change` and
  `/admin/history`.
- **Server-side filters + ILIKE search + pagination** pushed
  into a single SQL query with column-name validation against
  `M::COLUMNS`.
- **Sortable list-page columns** via `?sort=col&dir=desc`,
  falling back to `ModelAdmin::ordering()`.
- **Hand-written CSS theme.** Six CSS custom properties driven
  by `Admin::theme(...)`. Mobile-first responsive (3
  breakpoints), dark mode via `prefers-color-scheme` plus a
  manual toggle persisted to `localStorage`.
- **Auth and RBAC.** Five-tier role ladder (User → Developer),
  Argon2 password hashing, DB-backed sessions, per-model
  permissions with a 60-second cache, last-developer orphan
  guard on user delete.
- **Audit log.** Every create/update/delete writes to
  `rustio_admin_actions`; surfaced in the dashboard's "Recent
  actions" widget, the global `/admin/history` page, and
  per-object `/admin/<model>/<id>/history`.
- **Middleware bundle.** Rate limit, CSRF (double-submit
  cookie), security headers, gzip, request logger.
- **Migrations runner** that walks numerically prefixed
  `*.sql` files in a directory and applies them
  transactionally with a tracking table.

### Architecture

- **Tier 1, single-binary, Postgres-only.** Schema contracts,
  drift validation, AI planners, multi-database backends, and
  search backends are explicitly out of scope (see the
  [strategic reset plan](./rustio-admin-strategic-reset-plan.md)
  §1, §3, §8).
- **No Tailwind, no PostCSS, no build step.** The CI pipeline
  enforces the no-Tier-2-symbols invariant with a `git grep`
  guard on every PR.

### `rustio` CLI

The binary that ships from `rustio-admin-cli` covers the
operationally critical surface for v0.1.0:

- `rustio migrate apply` / `status` — drives the framework's
  numerically prefixed `migrations/*.sql` runner.
- `rustio user create` / `list` / `role` / `delete` —
  auth-table CRUD with Argon2 hashing and a confirm-twice
  password prompt. Honours the developer-orphan guard.
- `rustio group create` / `list` / `add-user` /
  `remove-user` — group CRUD and membership.
- `rustio perm grant-user` / `grant-group` / `list` —
  permission grants on top of `auth::permissions`.
- `rustio doctor` — read-only health check (DB reachable?
  auth tables present? at least one administrator?). Exits
  non-zero on any blocker so a CI step can gate on it.
- `rustio startproject <name>` — scaffolds a fresh project
  at `./<name>/` with a working `Cargo.toml`, a demo `Post`
  model with a populated `ModelAdmin` impl, starter
  migration, `.env.example`, and a `README.md`. Templates
  are baked into the CLI binary via `include_str!`.
- `rustio startapp <name>` — adds a model + migration to an
  existing project. Generates `src/<name>.rs` (full `Model`
  + empty `ModelAdmin` impl) and
  `migrations/<NNNN>_create_<name>s.sql` with an
  auto-incremented number. Refuses to mutate `src/main.rs`
  automatically; prints the exact `mod` / `use` /
  `.model::<>()` lines instead.

Browser-walked end-to-end against a local Postgres: a single
`rustio startproject blog` plus two `rustio startapp`
invocations generated three models which all rendered their
list pages on `/admin/posts`, `/admin/comments`, and
`/admin/book_reviews` after a copy-paste edit to
`src/main.rs`.

### Released to crates.io

All three workspace crates are on crates.io as of 2026-05-07:

- [`rustio-admin@0.1.0`](https://crates.io/crates/rustio-admin)
- [`rustio-admin-macros@0.1.0`](https://crates.io/crates/rustio-admin-macros)
- [`rustio-admin-cli@0.1.0`](https://crates.io/crates/rustio-admin-cli)

Project consumers add `rustio-admin = "0.1"` to their
`Cargo.toml`; operators install the CLI with
`cargo install rustio-admin-cli`.
