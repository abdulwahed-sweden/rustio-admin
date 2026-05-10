# Changelog

All notable changes to `rustio-admin` are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project
adheres to [SemVer](https://semver.org/) once it leaves the alpha track.

## [Unreleased]

R2 of the universal account-recovery architecture. Targets 0.6.0.
Where R1 covered the user-initiated path (forgot / reset / change),
R2 covers the **admin-initiated** path: an Administrator can reset
another user's password (email or temp-password mode), lock /
unlock an account, force a password rotation on next sign-in, and
revoke every active session for a target without locking. R2 also
introduces the auto-throttle on failed logins (5 failures / 10
minutes → 15-minute soft lock, no session revocation) and the
re-auth wall (15-minute elevated-session window) that gates every
destructive admin action.

> **Migrating from 0.5.x:** bump `rustio-admin = "0.6"` in your
> project's `Cargo.toml` and `cargo update -p rustio-admin`. The
> schema migration is additive (three new columns on `rustio_users`:
> `failed_login_count`, `last_failed_login_at`, `locked_until`,
> plus a partial index); existing users are unaffected because the
> new counter defaults to 0 and the lockout column defaults to NULL.
> No middleware changes required. All R2 routes register
> automatically via `register_admin_routes`. Doctrine 22 holds —
> `auth::sessions::invalidate_sessions` remains the sole writer of
> `revoked_at`; a grep across the framework still returns three SET
> arms (lines 398 / 413 / 426 in `auth/sessions.rs`).

### Recovery (admin-initiated)

- **Admin-driven password reset.** Two new routes —
  `GET /admin/users/:id/reset-password` (form) and
  `POST /admin/users/:id/reset-password` (apply). The form offers
  two modes:
  - **Email mode** issues an admin-initiated reset token (same
    table / TTL / consume path as R1's self-reset; the admin's
    audit row carries `actor_user_id` + `actor_email_hash` +
    `reason` + `mode: "email"` + `email_send_status`). The target
    receives the same `/admin/reset-password/<token>` link; their
    consume revokes every session via R1's existing path.
  - **Temp_pw mode** generates a 16-char URL-safe-base64 password,
    calls `auth::set_password` directly (Argon2id hash; the
    plaintext is the only out-of-band value), sets
    `must_change_password = TRUE`, and revokes every session via
    `SessionInvalidationReason::PasswordResetByOther`. The
    plaintext renders ONCE on the admin's success page —
    refreshing re-issues a fresh value and rotates the previous
    one out. Never logged, never persisted in any other column.
- **Manual lock / unlock.** Two new actions —
  `POST /admin/users/:id/lock` and `POST /admin/users/:id/unlock`.
  Lock takes a duration (15 min / 1 h / 24 h / 7 d / indefinite +
  freeform-minutes); the column holds the absolute timestamp.
  Indefinite encodes as a year-9999 sentinel so the partial index
  on `locked_until` continues to find it. Lock revokes every
  session via `SessionInvalidationReason::AdministrativeRevoke`;
  unlock zeroes `failed_login_count` and clears the column but
  does NOT touch sessions (the lock-time revocation already
  cleared them). Both emit typed audit rows
  (`AccountLocked` / `AccountUnlocked`) with `via: "manual"`.
- **Admin revoke-sessions.** `POST /admin/users/:id/revoke-sessions`
  — a sibling of lock that revokes every session without writing
  `locked_until`. Useful when a security incident calls for "kick
  them out everywhere" without rate-limiting future logins. Emits
  one `SessionsRevokedByOther` row per revoked session with
  `via: "manual"`.
- **`AdminActor` boundary type.** `auth::recovery_admin::AdminActor`
  bundles `user_id` + `email` for the runtime fns. The `email`
  hashes into `metadata.actor_email_hash` (8-char SHA-256
  fingerprint via `actor_email_fingerprint`); the plaintext NEVER
  lands in audit metadata.

### Auto-throttle (login-side)

- **`LoginThrottle` policy + sliding-window enforcement.**
  `RecoveryPolicy::login_throttle()` returns
  `LoginThrottle { max_attempts: 5, window_minutes: 10,
  lock_minutes: 15 }` by default. The login flow's new
  `do_login` (in `admin/handlers.rs`) calls
  `auth::recovery_admin::record_failed_login(...)` on wrong
  password and `record_successful_login(...)` on success.
  Failures are anchored on `last_failed_login_at` and reset to
  1 when the previous failure is older than `window_minutes`.
  When the threshold trips, a soft lock writes `locked_until =
  NOW() + lock_minutes` and the login flow emits an
  `AccountLocked` audit row with `via: "auto_throttle"` and
  `actor_user_id: null`. Setting `max_attempts = 0` disables
  the auto-throttle (counter still increments for visibility,
  no lock applied).
- **Uniform login response.** Every failure mode (no such user,
  inactive account, currently locked, wrong password) collapses
  to a single 401 with the same `"Invalid email or password."`
  error. No enumeration leak. The pre-R2 distinction between
  401 (wrong password) and 403 (inactive) is gone — `auth::login`
  remains as a public API for downstream projects that prefer
  the simpler shape, but the framework's own surface uses the
  uniform path.
- **Auto-throttle does NOT revoke sessions.** Locking simply
  prevents future sign-ins. Existing sessions stay valid until
  the next request, at which point the `locked_until` check at
  login refuses. Doctrine D4 + §3.3 + §13 locked-decision.

### Re-auth wall

- **15-minute elevated-session window.** Every destructive admin
  action (admin reset, lock, unlock, revoke-sessions) calls
  `auth::recovery_admin::check_session_elevated(...)` BEFORE
  rendering the form. If `elevated_until < NOW()` (or NULL),
  the handler 303-bounces to
  `/admin/reauth?return_to=<encoded path>`. The user re-enters
  their password; on success `promote_session_elevated(..., 15min)`
  writes the new `elevated_until` and `trust_level = 'elevated'`,
  and the redirect bounces back to the original URL with the
  promotion now in effect.
- **Standalone wall, validated `return_to`.** `GET/POST
  /admin/reauth` are gated `Role::User` (any authenticated user
  can re-auth their own session). The `return_to` validator
  accepts only `/admin*` paths and rejects empty / control bytes
  / protocol-relative `//` / backslash / `..`. Failed validation
  collapses to `/admin`. Failure to verify password renders a
  uniform 401 with the same error wording across every failure
  cause; no audit row, no session-state mutation.
- **No session-validity touch.** Re-auth promotion writes
  `elevated_until` + `trust_level` only. Doctrine 22 holds —
  promotion is purely additive trust-band motion, never a
  revocation.

### Forced password rotation

- **`must_change_password` interstitial.** A target whose
  `must_change_password` flag is TRUE (set by the admin reset's
  temp_pw mode) sees every authenticated `/admin/*` request
  redirected to `/admin/must-change-password` until they
  complete the rotation. The whitelist (locked per §12) carves
  out three exceptions: the rotation form itself, `/admin/logout`,
  and `/admin/account/sessions` (read-only). Sub-paths of
  `/admin/account/sessions` (the revoke buttons) are NOT
  whitelisted — a user being forced to rotate may VIEW their
  sessions but must finish the rotation before revoking
  siblings.
- **Forced-rotation gate in `login_guard`.** The check sits
  inside `login_guard` BEFORE any role gate, so even
  Administrators / Developers with the flag set are funnelled
  through. `role_guard` and `perm_guard` inherit the redirect
  automatically since they call `login_guard` first.
- **Rotation handler.** `POST /admin/must-change-password`
  validates the new password against `PasswordPolicy::validate`,
  rejects reuse of the current value (defensive — prevents the
  user "rotating" to the same temp the admin issued), calls
  `set_password`, clears the flag, revokes every other session
  via `UserExceptCurrent`, and emits one
  `AuditEvent::ForcedPasswordChangeCompleted` row with
  `triggered_by_audit_id` (best-effort lookup of the most recent
  prior `password_reset_by_other` row for the user) +
  `invalidated_session_count`. The audit chain
  `PasswordResetByOther → ForcedPasswordChangeCompleted →
  N × SessionsRevokedSelf` is now complete (§5.3).

### Audit

- **`AuditEvent::ForcedPasswordChangeCompleted` (NEW).** Locked
  string `"forced_password_change_completed"`. Stability contract:
  the string is part of the public API from 0.6.0 forward;
  renaming requires a major bump.
- **Four pre-declared events light up.** `PasswordResetByOther`,
  `AccountLocked`, `AccountUnlocked`, `SessionsRevokedByOther` —
  variants existed in the enum since 0.5.0 but had no emitting
  call sites until R2.
- **`LogEntry::actor_user_id` field (NEW; `Option<i64>`).** The
  acting principal when a row records one user performing an
  action on another. Persisted under `metadata.actor_user_id`
  (no schema change). The legacy `LogEntry::user_id` continues
  to carry the actor for backwards compat with `/admin/history`'s
  "who did what" view; `actor_user_id` is the typed mirror that
  SIEM consumers can read without heuristics. Set via
  `LogEntry::with_actor(id)` builder. The merge layer in
  `audit::record(...)` inserts the key into the metadata JSON
  object (creating one if needed); a non-object metadata is a
  programming bug and falls back with a `log::warn!`.
- **Unified §5.1 metadata schema.** Every R2 admin-action row
  carries `actor_user_id`, `actor_email_hash` (8-char SHA-256
  per §13 locked-decision), `reason`, and a `via` discriminator.
  Mode-specific keys (`mode`, `email_send_status`,
  `must_change_password_set`, `token_fingerprint`,
  `invalidated_session_count`, `until`) populate per the
  emission path. Plaintext NEVER in metadata — actor email is
  hashed; reset tokens are stored as 8-char fingerprints.

### Identity

- **`Identity::must_change_password: bool` (NEW).** Mirrors the
  `rustio_users.must_change_password` column added in R1's
  recovery migration. Loaded by `find_user_by_email` and
  `identity_from_session`; consumed by `login_guard` to drive
  the forced-rotation redirect. Pre-R2 sessions issued before
  the field was loaded resolve with `must_change_password = false`
  (the column defaults FALSE for every row).
- **`StoredUser::must_change_password: bool` (NEW).** Sibling
  on the storage layer; populated by the same SELECT.

### Re-auth runtime

- **`promote_session_elevated(db, session_id, ttl)`.** UPDATE
  on `rustio_sessions` writes
  `elevated_until = NOW() + ttl, trust_level = 'elevated'`.
  Idempotent — re-promoting extends the window. Skips revoked
  rows (`AND revoked_at IS NULL`).
- **`check_session_elevated(db, session_id) -> bool`.** True iff
  the session row exists, is not revoked, and
  `elevated_until > NOW()`. False for missing / revoked /
  never-promoted / expired sessions. Cheap — single indexed
  lookup.

### Login-throttle runtime

- **`LockState`** (`Unlocked` / `Locked { until }`) — output of
  `check_account_lockout(db, user_id)`.
- **`ThrottleOutcome`** (`Recorded { count }` /
  `JustLocked { count, until }` / `Disabled { count }`) — output
  of `record_failed_login(db, user_id, throttle)`. The variant
  tells the caller whether to emit `AccountLocked`.

### Schema

- **`rustio_users.failed_login_count INT NOT NULL DEFAULT 0`** —
  incremented by `record_failed_login`; reset by
  `record_successful_login`.
- **`rustio_users.last_failed_login_at TIMESTAMPTZ`** —
  sliding-window anchor for the auto-throttle threshold.
- **`rustio_users.locked_until TIMESTAMPTZ`** — when set and
  `> NOW()`, the login flow refuses with the uniform 401.
  Indefinite manual locks encode as a year-9999 timestamp.
- **`rustio_users_locked_until_idx`** — partial index on
  `(locked_until) WHERE locked_until IS NOT NULL`. Powers the
  "list currently-locked accounts" admin view (§9 — incident
  triage). Negligible storage at admin-tier scale.

All migrations idempotent. No data backfill required — the
counter defaults to 0 (correct neutral state because the
threshold is anchored on a sliding window, not historical
totals) and the lockout columns default to NULL (correct for
"unlocked").

### Behaviour changes

- **`do_login` rewrite.** Pre-R2 returned three response shapes
  (401 wrong creds, 403 inactive, no lockout). Post-R2 returns
  a single uniform 401 across every failure mode. After 5 wrong
  passwords within 10 minutes, a 15-minute soft lock applies.
- **CLI `user create` floor: 8 → 10 chars.** The CLI now
  delegates to `DefaultPasswordPolicy::new()` directly, the same
  floor admin-create-user and self-recovery enforce. CLI
  bootstrap flow stays uniform with the web surface.
- **Admin Add-user form respects project policy override.**
  Pre-R2 the form's hint + validation hardcoded 8 chars; post-R2
  both read from `Admin::active_password_policy()`, so a project
  with `min_length = 16` in their custom policy sees 16 in the
  hint AND has 16 enforced server-side. Closes the last
  hardcoded-floor drift in the framework.
- **Admin-edit form: password field removed.** The legacy
  `new_password` input on `/admin/users/:id/edit` is gone
  (`DESIGN_RECOVERY.md` §14.4). Admin password resets now go
  through the dedicated `/admin/users/:id/reset-password` route
  with the correct doctrine-22 semantics (typed audit, must-change
  flag, centralised invalidation). The pre-R2 path was a
  doctrine-22 spirit-violation that mutated passwords without
  invalidating sessions.

### Routes

Twelve new routes register through `register_admin_routes`:

```
GET  /admin/reauth                           → show_reauth
POST /admin/reauth                           → do_reauth
GET  /admin/must-change-password             → show_must_change_password
POST /admin/must-change-password             → do_must_change_password
GET  /admin/users/:id/reset-password         → show_admin_reset_password
POST /admin/users/:id/reset-password         → do_admin_reset_password
GET  /admin/users/:id/lock                   → show_lock_user
POST /admin/users/:id/lock                   → do_lock_user
GET  /admin/users/:id/unlock                 → show_unlock_user
POST /admin/users/:id/unlock                 → do_unlock_user
GET  /admin/users/:id/revoke-sessions        → show_admin_revoke_sessions
POST /admin/users/:id/revoke-sessions        → do_admin_revoke_sessions
```

User-targeted routes gate `Role::Administrator`; the cross-rank
guard + re-auth wall enforce inside the handlers (so a
Supervisor's probe doesn't even reach the form). Reauth and
must-change-password gate `Role::User` because any authenticated
user can re-auth their own session or be forced to rotate.

### Documentation

- **[`DESIGN_R2_ORGANISATIONAL.md`](./DESIGN_R2_ORGANISATIONAL.md)
  added.** The canonical R2 contract: threat model, five state
  machines (admin reset, manual lock, auto-throttle, forced
  rotation, re-auth wall), schema deltas, audit event plan
  including unified §5.1 metadata, module + types layout, route
  table, trait extensions, locked decisions (re-auth window,
  throttle thresholds, temp-password length, lock-duration
  presets, whitelist paths), and the 17-commit atomic
  implementation plan.
- **README architecture-doctrine table** updated with a row
  pointing to `DESIGN_R2_ORGANISATIONAL.md`. The four pre-existing
  contracts (`DESIGN_SYSTEM`, `DESIGN_SESSIONS`, `DESIGN_AUDIT`,
  `DESIGN_RECOVERY`) carry through unchanged.

### Internal

- **`auth::recovery_admin` submodule (NEW).** Sibling of R1's
  `auth::recovery`. Owns every R2 runtime fn: schema migration,
  login-throttle runtime, re-auth wall runtime, admin-reset
  runtime, lock/unlock/revoke runtime. `pub(crate)` surface
  except the `LoginThrottle` struct (re-exported from
  `auth::*` since R2 commit #5).
- **`admin::admin_recovery_handlers` submodule (NEW).** All R2
  HTTP handlers + the `validate_return_to` helper. Sibling of
  R1's `admin::recovery_handlers`. Crate-private — handlers
  reach the runtime via `crate::auth::recovery_admin::*`.
- **`Row::get_optional_datetime` helper (NEW).** Sibling of
  `get_optional_string` / `get_datetime`. Closes a gap the
  downstream POS CLAUDE.md flagged. The R2 lockout column reads
  use it; previously the framework had no idiomatic way to read
  a nullable `TIMESTAMPTZ`.
- **`admin::builtin` module promoted from `mod` to
  `pub(crate) mod`.** The fns `client_ip` and
  `correlation_id_from` were already `pub(crate)`; the module
  itself was private, blocking sibling-crate access. Promoting
  the module gives `auth::recovery_admin` access to the helpers
  without sibling-helpers in two places. No new public API.
- **`admin::handlers::record_session_revocations` promoted from
  private to `pub(super)`.** Same helper R1's
  `do_password_change` uses; the new
  `do_must_change_password` calls it with
  `via = "must_change_password"` to differentiate the metadata.
- **`build_persisted_metadata` merge helper (in
  `admin::audit`)**. Pure function the audit pipeline calls
  before binding the metadata JSONB column. Inserts
  `actor_user_id` into the metadata object when set; logs a
  warn + falls back when metadata is a non-object.

### Tests

- **+58 unit tests** across the R2 commit chain (162 → 220).
  All pure / DB-free: type-level invariants
  (`Send + Sync + Copy` bounds), pure helpers
  (`validate_return_to`, `parse_lock_duration`,
  `actor_email_fingerprint`, `random_temp_password`,
  `MUST_CHANGE_WHITELIST` membership, `LockDuration` time math),
  and the `AuditEvent` drift tests pick up the new variant
  automatically.
- **Doctrine 22 grep** is part of every R2 commit's pre-commit
  gate. Result is locked: 3 SET arms in
  `auth::sessions::invalidate_sessions` (lines 398 / 413 / 426),
  unchanged across the entire R2 chain.

### Deferred

- **Testcontainers Postgres integration suite.** Lands in a
  separate commit before the 0.6.0 publish, gated behind
  `--features integration-test`. Covers the SQL paths in
  `record_failed_login`, `check_account_lockout`,
  `promote_session_elevated`, `check_session_elevated`,
  `lock_user_account`, `admin_set_temp_password`, and the
  re-auth + forced-rotation handlers' DB-touching steps. The
  R1 hotfix `d4f5182` (INT4-vs-INT8 column-type mismatch in
  `check_reset_token_valid`) is the lesson driving this
  addition (§10.3).

## [0.5.0] — 2026-05-09

R1 of the universal account-recovery architecture. Self-service
password recovery is now end-to-end: the user clicks **Forgot your
password?** on `/admin/login`, lands on `/admin/forgot-password`,
submits an email, receives a reset link (1-hour TTL, single-use),
clicks through to `/admin/reset-password/<token>`, sets a new
password, and is signed back in via `/admin/login`. Every session
across every device is revoked at consume time. Active-session
management (revoke single / others / all) is wired on the existing
`/admin/account/sessions` page. The authenticated
`/admin/password_change` flow is brought into parity with the
recovery doctrine — successful changes now invalidate other devices
and emit typed audit events. A 7-day forensic-retention sweeper
trims old reset tokens automatically.

> **Migrating from 0.4.x:** bump `rustio-admin = "0.5"` in your
> project's `Cargo.toml` and `cargo update -p rustio-admin`. The
> schema migration is additive (new `rustio_password_reset_tokens`
> table + two new columns on `rustio_users`); existing users and
> sessions are unaffected. No middleware changes required —
> `correlation_id` BEFORE `csrf_protect` (added in 0.4.0) is still
> the only ordering constraint. Recovery routes register
> automatically via `register_admin_routes`. Production deployments
> wiring a real `Mailer` should also opt the policy into strict
> mode: `RecoveryPolicy::strict_mailer_required(true)` makes the
> framework refuse to start with the default `LogMailer`.

### Recovery

- **Self-service forgot/reset password flow.** Five new routes:
  `GET /admin/forgot-password`, `POST /admin/forgot-password`,
  `GET /admin/forgot-password/sent`, `GET /admin/reset-password/:token`,
  `POST /admin/reset-password/:token`. All sit alongside `/admin/login`
  in the public surface; no role guard, CSRF preserved through the
  existing `csrf_protect` middleware.
- **Email-link reset tokens.** 256-bit cryptographically-random
  URL-safe-base64 tokens; the plaintext leaves the framework only
  in the email body dispatched through `email::Mailer::send` and in
  the user's mailbox. The DB stores `sha256(token)` only — no
  plaintext token persistence.
- **Atomic single-use consume.** A single SQL statement
  `UPDATE rustio_password_reset_tokens SET consumed_at = NOW()
  WHERE token_hash = $1 AND consumed_at IS NULL AND expires_at > NOW()
  RETURNING user_id` flips the row exclusively. Concurrent submissions
  resolve as exactly one Consumed and one Invalid — never two of either.
- **`PasswordPolicy` trait + `DefaultPasswordPolicy`.** Length-only
  policy with `min_len: 10` baseline. Counts Unicode `char`s, not
  bytes (a 10-char password is 10 user-visible characters regardless
  of UTF-8 width). Projects override via
  `Admin::password_policy(Arc::new(...))`; the framework deliberately
  ships no complexity-class rules ("must contain a symbol") in the
  default — NIST SP 800-63B Appendix A documents that complexity
  classes push humans toward predictable patterns without raising
  entropy meaningfully.
- **`RecoveryPolicy` trait + `DefaultRecoveryPolicy`.** Tunables for
  the recovery flow: `reset_token_ttl()` (1h), `request_rate_limit()`
  (5 req / 15min / IP), `consume_rate_limit()` (10 req / 5min / IP),
  `strict_mailer_required()` (false default), `public_site_url(&Request)`
  with a provided default that honours `Forwarded` / `X-Forwarded-*`
  / `Host` headers in priority order. Projects override via
  `Admin::recovery_policy(Arc::new(...))`.
- **Strict-mailer boot guard.** When
  `RecoveryPolicy::strict_mailer_required() == true` and the project
  hasn't called `Admin::mailer(...)` to override the default
  `LogMailer`, `register_admin_routes` panics at startup with a
  clear operator-actionable error message. The check is structural:
  reads the new `Admin::has_custom_mailer()` flag (set by
  `Admin::mailer(...)`), not pointer-equality tricks against
  freshly-constructed defaults. Projects deliberately migrating can
  re-register a `LogMailer` to silence the guard during transition.
- **Reset-token sweeper integration.** `auth::recovery::purge_expired_reset_tokens`
  runs alongside the existing session sweeper on the same 10-minute
  tick. Deletes rows where `expires_at < NOW() - INTERVAL '7 days'`
  (locked retention; covers consumed AND unconsumed expired rows).
  Failure-isolated from the session sweep — a failure in either
  doesn't prevent the other.

### Sessions

- **Active-sessions revoke buttons.** The R0 read-only
  `/admin/account/sessions` page lights up. Three new POST routes:
  `POST /admin/account/sessions/:id/revoke` (rejected if the id
  matches the current device); `POST /admin/account/sessions/revoke-others`
  (`SessionTarget::UserExceptCurrent`); `POST /admin/account/sessions/revoke-all`
  (`SessionTarget::User`, additionally clears the cookie and
  redirects to `/admin/login?logout=1`).
- **Authenticated password change revokes other devices.** A
  successful `POST /admin/password_change` now goes through
  `auth::invalidate_sessions(SessionTarget::UserExceptCurrent { … },
  SessionInvalidationReason::UserRequested)`. Current device stays
  signed in; every other device is signed out and must re-authenticate
  with the new password. PRG redirect to
  `/admin/password_change?changed=1` so a browser refresh doesn't
  replay the POST.
- **Reset-password consume revokes all devices.** Successful
  `POST /admin/reset-password/<token>` calls
  `invalidate_sessions(SessionTarget::User, SessionInvalidationReason::PasswordReset)`
  before redirecting to `/admin/login?password_reset=success`.
- **Doctrine 22 preserved.** A grep across `crates/` for `revoked_at\s*=`
  continues to return only the four lines inside
  `auth::sessions::invalidate_sessions` (one docstring + three
  `SessionTarget` arms). Centralised invalidation is the single
  legitimate writer of `revoked_at` across all R1 paths.

### Audit

- **`AuditEvent` promoted to public API.** The enum is `pub` with
  `#[non_exhaustive]` from 0.5.0; its `as_str()` mapping is the
  canonical persisted-string boundary. Existing variant strings are
  locked-in by `audit_event_existing_variants_have_stable_strings`
  — renaming any of them is a breaking change requiring a major
  version bump.
- **New variant `PasswordChangedSelf`** maps to
  `"password_changed_self"`. Emitted from the corrected
  authenticated `/admin/password_change` handler.
- **`PasswordResetSelfRequest` / `PasswordResetSelfConsume` /
  `SessionsRevokedSelf`** emission paths wired. R0 declared the
  variants; R1 lights them up.
- **`LogEntry::with_event(AuditEvent)` builder** is the typed-event
  boundary. Adds an optional `event: Option<AuditEvent>` field on
  `LogEntry`; when set, the `record()` insert uses
  `event.as_str()` as the persisted `action_type`. Backwards
  compatible: existing struct-literal call sites pass `event: None`
  and continue to write the legacy `ActionType` trio
  (`create / update / delete`).
- **Correlation-id chain semantics.** Every recovery event
  (request → consume → set_password → revoke-N-sessions) shares
  the originating request's `correlation_id`. A future
  `/admin/history/<correlation_id>` page can reconstruct the full
  chain. Metadata fields:
  `token_fingerprint` (8-char SHA-256 redaction),
  `email_send_status`, `requested_ip`, `requested_user_agent`,
  `expires_at`, `invalidated_session_count`, `via`. Plaintext
  tokens never appear in any field.

### Security

- **Token hashes only.** `rustio_password_reset_tokens.token_hash`
  stores `sha256(token)` URL-safe-base64. Plaintext lives in the
  email body and the user's mailbox; never in the DB, never in any
  log line, never in audit metadata. Tests pin the property:
  `IssueOutcome` and `ConsumeOutcome` Debug formats are
  structurally token-free.
- **Uniform outward responses on the recovery flow.** The
  `do_forgot_password` handler always 303-redirects to
  `/admin/forgot-password/sent` regardless of whether the email
  matched, the user was active, or the IP was rate-limited. The
  `do_reset_password` handler renders the same "this link is no
  longer valid" page for unknown / expired / consumed / rate-limited
  tokens. Variant-level distinctions exist for audit + observability,
  never for branching the user-facing UI.
- **No direct `revoked_at` writes.** Doctrine 22 single-writer
  invariant preserved. Every revoke goes through
  `auth::invalidate_sessions(SessionTarget::*, …)`.
- **CSRF preserved across recovery flows.** The new POST routes
  inherit the project's existing `csrf_protect` middleware; recovery
  templates emit
  `<input type="hidden" name="_csrf" value="{{ csrf_token }}">`
  in every form.
- **No plaintext password leakage.** `PasswordPolicyError` variants
  carry only character counts (`TooShort { min, actual }`) or
  project-supplied messages (`Custom(String)`); never the candidate
  plaintext. Display + Debug renderings are property-tested
  plaintext-free across the runtime / handler / template surfaces.
- **Strict-mailer boot guard** (above) prevents production
  deployments from accidentally running with the dev `LogMailer`
  default — when opted in, the framework refuses to start.

### Behaviour changes

- **Authenticated password changes now sign out other devices.**
  Pre-R1 behaviour: `POST /admin/password_change` updated the hash
  and left other sessions live with the previously-issued cookies.
  This contradicted Doctrine 22's spirit and has been a known drift
  since 0.3.0. R1 closes it: the success path goes through
  `invalidate_sessions(SessionTarget::UserExceptCurrent, …)` and
  emits typed audit events. Users on multiple devices need to sign
  in again on each non-current device with the new password.
- **Default password minimum length 8 → 10.** Pre-R1 the
  authenticated change handler used an inline `MIN_PASSWORD_LEN = 8`
  constant; the policy framework didn't exist. R1 routes all
  password mutations through `Admin::active_password_policy()`
  with `DefaultPasswordPolicy::new()` returning `min_len: 10`.
  Existing users with shorter passwords are NOT forced to change
  — the policy fires only on new passwords during a change /
  reset. Production deployments are encouraged to override to 12+
  via `Admin::password_policy(Arc::new(DefaultPasswordPolicy::with_min_len(12)))`.
- **Recovery tokens retained 7 days after expiry.** Issued tokens
  expire after 1 hour; the row stays in
  `rustio_password_reset_tokens` for 7 more days as a forensic
  window (audit correlation, abuse investigation, operational
  debugging) before the periodic sweeper purges it. Pre-R1: no
  recovery table, no sweeper.
- **Strict-mailer mode can fail startup intentionally.** When
  `RecoveryPolicy::strict_mailer_required(true)` is set and the
  default `LogMailer` is still in place, `register_admin_routes`
  panics with a clear error. Default behaviour (dev / CI / testing)
  is unchanged: `strict_mailer_required = false`, framework boots,
  reset emails appear in `log::info!` output.
- **`MIN_PASSWORD_LEN` constant removed from `admin/handlers.rs`.**
  Was effectively private (a non-exported `const` in a binary-internal
  module); no external project should have depended on it. The
  constant's parallel implementation in `rustio-admin-cli`
  (`user.rs:62`, used by `rustio user create`) is unchanged at 8
  characters and remains a separate validation surface — R2 work
  will likely route CLI user creation through the same
  `PasswordPolicy` for consistency.

### Documentation

- **`DESIGN_RECOVERY.md`** added (~1100 lines) — the canonical R1
  contract. Covers: threat model, recovery state machine, token
  lifecycle, atomic-consume semantics, invalidation rules, audit
  event plan, rate-limit strategy, UX doctrine + locked page copy,
  schema/migration plan, module + types layout, route table,
  mailer integration + boot guard, `PasswordPolicy` +
  `RecoveryPolicy` trait surface, integration deltas (with a
  `§14.4` callout parking the admin-edit form's password field for
  R2 cleanup), test plan, 12-commit atomic implementation plan,
  locked decisions table, PR review checklist. Read-orientation
  for operators; PR-review surface for contributors.
- **Doctrine cross-links.** `README.md` gains an *Architecture
  doctrine* section listing the four canonical contracts —
  `DESIGN_SYSTEM.md`, `DESIGN_SESSIONS.md`, `DESIGN_AUDIT.md`,
  `DESIGN_RECOVERY.md` — with one-line summaries. Communicates
  that recovery / session / audit behaviour is doctrine-driven,
  invariants are intentional, revoke semantics are centralized,
  audit chains are correlation-aware, security-sensitive flows
  are documented before implementation.
- **`docs/getting-started.md`** clarifies that `rustio user create`
  prompts twice for the password (echo-suppressed) and that
  `--password` should only appear in CI / scripted bootstrap so the
  plaintext never lands on `argv`, in `ps` output, or in shell
  history. *(landed pre-R1 under `[Unreleased]`)*
- **Port-8000 troubleshooting** added — if the default
  `127.0.0.1:8000` is occupied, projects edit the listen address in
  the generated `src/main.rs` rather than introducing a new env var.
  *(pre-R1)*
- **Migrations clarification** — `rustio startproject` already
  creates `migrations/0001_create_posts.sql`, so the walkthrough no
  longer instructs `mkdir -p migrations`. Two valid paths are
  documented:
  - **Option A** keeps the demo `Post` as a smoke test and adds new
    tables as `0002_*.sql`, `0003_*.sql`, … via `rustio startapp`.
  - **Option B** replaces the demo with the real domain by deleting
    `migrations/0001_create_posts.sql` and `src/post.rs` **before**
    the demo migration has been applied to a real database. After
    that point, migrations are append-only and a forward
    `0002_drop_posts.sql` is the right move. *(pre-R1)*
- **"What you get after first login"** section enumerates the
  capabilities a fresh project inherits (session-backed auth,
  permission matrix, audit history, active sessions page,
  correlation IDs, FK hydration, template overrides). *(pre-R1)*
- **"Project philosophy"** section codifies the framework's design
  stance: Postgres-first, operational clarity over magic, explicit
  model registration, server-rendered admin UI, security and
  auditability built in, no AI / no cloud lock-in / no frontend
  build step. *(pre-R1)*
- **`templates/project/README.md.tmpl`** mirrors the same five
  pre-R1 clarifications so newly scaffolded projects ship with them
  in their own README. *(pre-R1)*

### Internal

- **`auth::recovery` module** — new submodule (`pub(crate)` from
  `auth::mod`) holding the schema migrations
  (`init_recovery_tables`, `migrate_user_recovery_schema`), the
  trait surface (`PasswordPolicy`, `RecoveryPolicy`, plus
  `Default*` impls and `Shared*` type aliases), the runtime
  primitives (`issue_reset_token`, `consume_reset_token`,
  `check_reset_token_valid`), and the periodic sweeper
  (`purge_expired_reset_tokens`).
- **`admin/recovery_handlers.rs`** — new submodule (`pub(crate)`)
  holding the five HTTP handlers, the `RecoveryState` carrier (two
  `Arc<RateLimiter>` buckets), and the per-page render contexts.
- **`set_password` stamps `password_changed_at`.** Single SQL
  statement updates `password_hash` + `password_changed_at` +
  `updated_at` to the same `NOW()`. Surface unchanged: callers see
  the same `(db, user_id, new_password) -> Result<()>` signature.
- **`Admin::mailer(...)` builder.** Closes the
  documented-but-unimplemented gap from 0.4.0. Builder field
  `mailer: SharedMailer`, accessor `Admin::active_mailer()`,
  override flag `Admin::has_custom_mailer()` for the strict-mailer
  guard. `Admin::new()` defaults to `Arc::new(LogMailer)`.
- **`Admin::password_policy(...)` + `Admin::recovery_policy(...)`**
  builders following the same `mailer` pattern. Both default to
  framework-supplied implementations seeded by `Admin::new()`.
- **Recovery sweeper integration** in `background::spawn_session_sweeper`.
  Same 10-minute interval as R0; recovery sweep runs after the
  session sweep with independent failure handling — neither sweep
  failure prevents the other from running on this tick or future
  ticks. New log target `rustio_admin::recovery_sweeper`.
- **`RateLimiter::allow(key)` promoted to `pub(crate)`** so the
  recovery handlers can drive their own scoped buckets without
  routing through the global middleware.
- **`auth::sessions::random_token` promoted to `pub(crate)`** so
  the recovery module reuses the same 256-bit URL-safe-base64
  token generator the session module already shipped — single
  source of truth.
- **`pub(crate) mod recovery`** in `auth/mod.rs` — the recovery
  module is reachable as `crate::auth::recovery::*` from sibling
  modules without `pub use` re-exports for every internal item.

No public-API breakage. The `Admin` struct gains four pub(crate)
fields (`mailer`, `mailer_overridden`, `password_policy`,
`recovery_policy`) and four public methods
(`mailer / has_custom_mailer / password_policy /
active_password_policy / recovery_policy / active_recovery_policy
/ active_mailer`); existing constructors and accessors are
unchanged. `LogEntry` gains one optional field (`event:
Option<AuditEvent>`) and one builder method
(`with_event(AuditEvent)`); existing struct-literal call sites add
`event: None` and continue working unchanged.

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
