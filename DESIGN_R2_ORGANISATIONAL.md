# R2 — Organisational Recovery

Companion to `DESIGN_SYSTEM.md`, `DESIGN_SESSIONS.md`,
`DESIGN_AUDIT.md`, and `DESIGN_RECOVERY.md`. Codifies the R2
contract — admin-driven recovery flows, login-throttle, re-auth
wall, forced password rotation, and the multi-tenant readiness
hook — that layers on top of R1's self-recovery primitives.

> R1 (0.5.0) shipped self-service: the user is the actor and the
> target. R2 (target 0.6.0) ships organisational recovery: an
> admin acts upon another user. The substrate is unchanged —
> `auth::invalidate_sessions` remains the single writer of
> `revoked_at` (Doctrine 22), the audit chain still ties through
> `correlation_id`, the `PasswordPolicy` is still the single
> source of truth for length / complexity. R2 adds the actor /
> target separation, an interstitial for forced rotation, and
> the throttling / re-auth machinery that the rest of the
> doctrine has been waiting for.

This is a contract. PR review compares against it.

---

## 0. Status

**Phase:** R2 — organisational recovery.
**Branch:** `feat/organisational-recovery` (cut from `main` at
`d4f5182`, the 0.5.0 release tip).
**Target:** `rustio-admin@0.6.0`.
**Blocking:** R3 (TOTP MFA + backup codes), R4 (CLI emergency
recovery).

R1 (0.5.0) shipped the foundations this phase depends on:

- Centralised `auth::invalidate_sessions` with
  `SessionInvalidationReason::PasswordResetByOther` /
  `AdministrativeRevoke` already declared (the latter unused
  until R2 wires it).
- Typed `AuditEvent` enum is `pub` with `#[non_exhaustive]`;
  variants `PasswordResetByOther`, `AccountLocked`,
  `AccountUnlocked`, `SessionsRevokedByOther` already declared
  (strings frozen, emission lights up here).
- `LogEntry::with_event(AuditEvent)` is the typed-event boundary;
  `record_session_revocations(... via: &'static str)` already
  carries a `via` parameter R2 extends.
- `rustio_users.must_change_password` column added in R1 commit
  #1; R2 reads + clears it.
- `rustio_sessions.elevated_until` column added back in R0; R2
  is the first phase to read + write it.
- `Admin::active_password_policy()` + `active_recovery_policy()`
  are stable; R2 reads both.

R2 adds the layer above all of that.

---

## 1. Doctrine recap (LOCKED — confirmed at R2 kickoff)

The three R1-locked decisions carry through unchanged. Confirmed
at kickoff per the roadmap's checklist:

| Decision | Status |
|----------|--------|
| Reset token TTL: **1 hour** | Unchanged. Admin-issued reset links use the same TTL — admin shares the link out-of-band; the target has the same 1-hour window to consume. |
| Mailer failure: **log + uniform user response** | Unchanged. When an admin initiates a reset, a mailer failure still produces the same user-facing copy; `metadata.email_send_status = "failed"` lands on the audit row and the admin can retry. |
| Default `MfaPolicy::Optional` | Unchanged. R3 owns MFA; R2 doesn't promote the floor. |

R2's locked additions:

| # | Doctrine | Implication for R2 |
|---|----------|---------------------|
| **D1** | Actor / target separation | Every R2 audit row carries `metadata.actor_user_id` (the admin) DISTINCT from `object_id` (the affected user). The legacy `LogEntry::user_id` field continues to carry the actor for backwards compat with /admin/history. |
| **D2** | Reason-required for organisational actions | Admin-driven reset / lock / unlock / revoke require a non-empty `reason` (≥ 8 chars) text field on the form. The reason persists into `metadata.reason`. Defense against impulsive ops + supports compliance audit. |
| **D3** | Re-auth before sensitive actions | Admin-driven recovery routes refuse without a session whose `elevated_until > NOW()`. Re-auth flow (`/admin/reauth`) prompts password (and R3+ TOTP) and stamps the column. Window: 15 minutes. |
| **D4** | Auto-throttle is a soft lock | Locked-until accounts can be unlocked by an admin via the same `/admin/users/:id/unlock` route as manual locks; auto-throttle does NOT permanently disable an account. |
| **D5** | Forced rotation cannot be skipped | Once `must_change_password = TRUE`, the user MUST complete the must-change interstitial before any /admin/* route renders for them. The check sits in `login_guard` BEFORE `role_guard`. |
| **D6** | Self-revoke on consume of admin-issued reset | When the target consumes an admin-issued reset link, every session is still revoked — same as self-reset. The admin's intent ("I'm forcing rotation") is honoured at the consume point, not at the issue point. |

Doctrine 22 (centralised invalidation) is unchanged — R2 calls
`invalidate_sessions(SessionTarget::User, …PasswordResetByOther)`
or `…AdministrativeRevoke`; the framework's only `revoked_at`
writer remains `auth::sessions::invalidate_sessions`.

---

## 2. Threat model

### 2.1 Adversaries

| Adversary | Capability | Mitigation |
|-----------|------------|------------|
| **Anonymous attacker, off-network** | Brute-force the login endpoint to enumerate accounts or find weak passwords | Login throttle: 5 fails / 10min / IP+email tuple → `locked_until = NOW() + 15min`. Uniform 401 response masks "wrong password" vs "account locked" until the user signs in successfully. |
| **Anonymous attacker, off-network** | Brute-force the admin-reset link the same way as R1's self-reset | Inherits R1's atomic single-use consume + 1h TTL + per-IP rate-limit on `/admin/reset-password/<token>`. R2 adds nothing new here. |
| **Compromised admin session** (cookie stolen, admin logged in elsewhere) | Issue resets / revoke sessions / lock accounts on other users | Re-auth wall (`elevated_until`). Admin must re-enter password (and R3+ MFA) before any admin-driven recovery route renders its form. Stolen cookie alone gets the attacker access to read pages but not to mutate authority. |
| **Lower-rank admin** (`Supervisor`) | Reset another user's password to take over their session | Existing R0 authority guards (`enforce_cross_rank_safe`) prevent editing users at-or-above your own rank. R2's admin-recovery routes route through the same guard. A Supervisor cannot reset an Administrator. |
| **Lower-rank admin** (Staff with `change_user` permission) | Same | Same. R2 reuses `enforce_cross_rank_safe`. |
| **Locked-out user** (`locked_until > NOW()`) | Attempt login during the lock window | Login flow refuses with the same uniform "account is currently disabled — contact your administrator" page used for `is_active = false`. The two states are indistinguishable from the user's side. |
| **Auto-throttled user with valid password** | Submit correct password during the lock window | Same as above. The lock fires BEFORE password verification; correct credentials don't unlock. Manual unlock by admin is required. |
| **Admin trying to bypass forced rotation** | Skip the must-change-password interstitial after admin reset | Cannot. The check sits in `login_guard` before any other admin route renders; only `/admin/must-change-password`, `/admin/logout`, and `/admin/account/sessions` (so the user can sign out / verify the lock) are reachable while the flag is set. |

### 2.2 Disclosure rules (LOCKED)

R2 inherits R1's disclosure rules (§2.3) for self-recovery
flows. Admin-driven flows have a different disclosure stance:

- The **admin actor** sees real errors (`mailer failed`, `target
  not found`, `cannot reset administrators`, etc.) — they're
  trusted operators acting on behalf of the org.
- The **target user** still sees uniform copy — "your password
  was changed; sign in again" is identical whether the change
  was self-initiated or admin-initiated.
- The /admin/users/:id form pages distinguish `locked_until` /
  `must_change_password` state explicitly to the admin (read +
  write); they do NOT render to the target user (the
  `/admin/account/sessions` page does NOT show the `locked_until`
  status — that's admin-only).

### 2.3 Out-of-scope (deferred)

- **CAPTCHA on login.** Throttle alone should suffice; CAPTCHA
  is a project-supplied middleware if needed.
- **Email notification to target on admin reset** ("Your admin
  reset your password"). Considered, deferred — operators may
  prefer not to surface admin actions to users by email. Future
  optional `RecoveryPolicy::notify_target_on_admin_action(bool)`
  hook.
- **Audit-log retention sweeper.** Operator-owned per
  DESIGN_AUDIT.md §9.
- **Permission-based admin recovery delegation** ("Staff with
  `reset_password` permission can issue resets but not locks").
  Future fine-grained-permissions phase. R2 routes are gated by
  `Role::Administrator` baseline + cross-rank guard.

---

## 3. R2 state machines

### 3.1 Admin-driven password reset

```
                                ┌────────────┐
   admin GET                    │            │
   /users/:id/reset-password    │            │
   ────────────────────────►    │   Form     │
                                │            │
                                └─────┬──────┘
                                      │ admin POST
                                      │ (reason, mode=email|temp_pw)
                                      ▼
                            ┌─────────────────┐
                            │  Authority      │
                            │  guards         │
                            │  (cross-rank,   │
                            │   re-auth wall) │
                            └────────┬────────┘
                                     │ pass
                          ┌──────────┴──────────┐
                          ▼                     ▼
                    mode=email            mode=temp_pw
                          │                     │
                  issue_admin_reset      generate temp pw
                  _token (1h TTL)        + set_password
                          │                     │
                  Mail::framework_       set must_change_password=TRUE
                  envelope               │
                  to target              invalidate_sessions(
                          │                User, PasswordResetByOther)
                  invalidate_sessions    │
                  on consume only        emit AuditEvent::PasswordResetByOther
                  (single-use atomic)    │
                          │              redirect to /admin/users/:id
                  emit AuditEvent::      with one-time temp pw shown
                  PasswordResetByOther   to admin (NOT logged)
                  with metadata.         (admin shares out-of-band)
                  email_send_status
                          │
                  redirect to
                  /admin/users/:id
                  with "reset email
                  sent" flash
```

**Mode choice — email vs temporary password:**

R2 ships BOTH paths. Operators choose per situation:

- **email mode:** generates an admin-issued reset token (same
  shape as R1's self-reset token but with `metadata.actor_user_id`
  on the audit row). Target receives an email with the reset
  link. Same 1h TTL; target consumes; sessions revoke.
- **temp_pw mode:** generates a 16-char random temp password,
  calls `set_password(target, temp)` directly, sets
  `must_change_password = TRUE`. The temp password is rendered
  ONCE on the admin's success page (NEVER logged, NEVER stored
  beyond the hash). Admin shares the temp password out-of-band
  (in person, secure chat, etc.). Target signs in with the temp
  password → forced rotation interstitial → sets a real
  password.

The temp-pw mode exists because not every operator deployment
runs a working mailer. The framework's strict-mailer guard
forces the operator to make the mailer choice; the temp-pw mode
gives them a working path even when the mailer isn't wired.

### 3.2 Account lock / unlock (manual)

```
   admin POST /users/:id/lock              admin POST /users/:id/unlock
   (reason)                                (reason)
        │                                          │
        ▼                                          ▼
   ┌────────────────────┐                  ┌────────────────────┐
   │ Authority guards   │                  │ Authority guards   │
   │ + re-auth wall     │                  │ + re-auth wall     │
   └────────┬───────────┘                  └────────┬───────────┘
            │                                       │
   UPDATE rustio_users                     UPDATE rustio_users
   SET locked_until =                      SET locked_until = NULL,
       NOW() + INTERVAL '<duration>'           failed_login_count = 0
                                                  (operator's reset)
            │                                       │
   invalidate_sessions(                    (no session touch — locked
   User, AdministrativeRevoke)             user already had sessions
            │                              revoked at lock time)
   emit AuditEvent::AccountLocked                  │
   metadata: { actor_user_id,             emit AuditEvent::AccountUnlocked
              reason, until, via:         metadata: { actor_user_id,
              "manual" }                             reason, via: "manual" }
            │                                       │
            ▼                                       ▼
   target's next login: refused          target's next login: allowed
   with uniform "account is              (subject to other guards)
   currently disabled" page
```

**Lock duration:** the admin's form offers presets (15min, 1h,
24h, 7d, "indefinite"); the column holds the absolute timestamp.
"Indefinite" is encoded as a far-future timestamp (year 9999) —
the column is never NULL while locked. NULL = unlocked.

### 3.3 Auto-throttle on failed login

```
                ┌──────────────────┐
   POST         │                  │
   /admin/login │   Login flow     │
   ────────────►│                  │
                └──────┬───────────┘
                       │
              ┌────────┴───────────────┐
              │                        │
              ▼ fail                   ▼ pass
   UPDATE rustio_users               (existing R0 path:
   SET failed_login_count =          create_session, set cookie,
       failed_login_count + 1,       redirect to /admin)
       last_failed_login_at = NOW()       │
              │                            │
              ▼                       UPDATE rustio_users
   IF failed_login_count >= 5        SET failed_login_count = 0
      AND last_failed_login_at >=    (success resets the counter)
      NOW() - INTERVAL '10 minutes'
   THEN
       UPDATE rustio_users
       SET locked_until = NOW() +
           INTERVAL '15 minutes'
       emit AuditEvent::AccountLocked
       metadata: { via: "auto_throttle",
                  failed_count: 5,
                  unlock_at: ... }
              │
              ▼
   user sees uniform "invalid email
   or password" — same response
   regardless of whether the account
   just got locked
```

**Throttle parameters** (`RecoveryPolicy::login_throttle()` —
default `LoginThrottle { max_attempts: 5, window_minutes: 10,
lock_minutes: 15 }`):

- 5 failures within 10 minutes → 15-minute lock.
- The 10-minute window is sliding (compares
  `last_failed_login_at` to NOW()).
- Successful login zeroes the counter regardless of prior
  failures.
- Per-account, NOT per-IP: an attacker can't lock another user
  out by guessing their password 5 times from a different IP.
  The IP-based rate limit on /admin/login (R0 already covers
  this for sessions/rate-limit middleware) is the
  attacker-side defence; this column is the user-account-side
  defence.

**Locked accounts auto-unlock** when `locked_until < NOW()`. The
login flow checks this BEFORE password verification; an
auto-throttled user with valid credentials simply waits 15
minutes and signs in normally. Admin can manually unlock
earlier via `/admin/users/:id/unlock`.

### 3.4 Forced password rotation (`must_change_password`)

```
   user POST /admin/login (success)
        │
        ▼
   login_guard reads identity
   AND identity.must_change_password
        │
        ├── FALSE ──► normal /admin flow
        │
        └── TRUE ──► redirect 303 → /admin/must-change-password
                          │
                          ▼
                  GET /admin/must-change-password
                  (form: new_password1, new_password2)
                          │
                          ▼ POST
                  validate via PasswordPolicy
                          │
                          ├── policy fail ──► re-render with field error
                          │
                          ▼ pass
                  set_password(user_id, new)
                  UPDATE rustio_users
                  SET must_change_password = FALSE
                          │
                  invalidate_sessions(
                  UserExceptCurrent,
                  UserRequested)
                          │
                  emit AuditEvent::ForcedPasswordChangeCompleted
                          │
                          ▼
                  redirect 303 → /admin
                  (now reachable since flag is FALSE)
```

While `must_change_password = TRUE`, the user can ONLY reach:
- `/admin/must-change-password` (the interstitial)
- `/admin/logout`
- `/admin/account/sessions` (so they can verify which devices
  are signed in — useful when a security incident triggered the
  forced rotation)

Every other `/admin/*` route redirects to
`/admin/must-change-password`. The check sits inside
`login_guard` BEFORE `role_guard` — even Administrators with
the flag set are forced through.

### 3.5 Re-auth wall (`elevated_until`)

```
   admin clicks /admin/users/:id/reset-password
                          │
                          ▼
                  re-auth check:
                  session.elevated_until > NOW()
                          │
                          ├── YES ──► render the action's form
                          │
                          └── NO ───► redirect 303 →
                                      /admin/reauth?return_to=<encoded>
                                              │
                                              ▼
                                      GET /admin/reauth
                                      (form: password input)
                                              │
                                              ▼ POST
                                      verify_password(actor)
                                              │
                                              ├── fail ──► 401, re-render
                                              │
                                              ▼ pass
                                      UPDATE rustio_sessions
                                      SET elevated_until =
                                          NOW() + INTERVAL '15 minutes'
                                      WHERE session_id = current
                                              │
                                              ▼
                                      redirect 303 →
                                      <return_to URL>
                                      (now passes the re-auth check)
```

**Routes that demand re-auth** (LOCKED):

- `POST /admin/users/:id/reset-password`
- `POST /admin/users/:id/lock`
- `POST /admin/users/:id/unlock`
- `POST /admin/users/:id/revoke-sessions`

Plus every R3 / R4 sensitive route when those land. Re-auth is
the framework-level lock for "this is a destructive admin
action; verify the actor's identity in this session right now."

The 15-minute window means an admin doing a chain of mutations
re-auths once and proceeds; an admin returning to their desk an
hour later re-auths again. Trade-off chosen for ergonomics:
shorter windows produce friction and lead to "just turn it
off" workarounds; longer windows soften the protection.

`return_to` URL is validated at /admin/reauth — must be a
relative path starting with `/admin/`. Defends against
open-redirect through the `?return_to=` parameter.

---

## 4. Schema deltas

R2 adds columns on `rustio_users` for login throttle + manual
lock state. No new tables. The `must_change_password` and
`elevated_until` columns already exist (R1 commit #1 + R0
respectively); R2 reads + writes them.

```sql
ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS
    failed_login_count    INT NOT NULL DEFAULT 0;
ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS
    last_failed_login_at  TIMESTAMPTZ;
ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS
    locked_until          TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS rustio_users_locked_until_idx
    ON rustio_users (locked_until)
    WHERE locked_until IS NOT NULL;
```

The partial index supports the "list all currently-locked
accounts" admin view (the operator's incident-triage surface).
Storage cost is small at admin-tier scale.

**Backfill:** existing rows default to `failed_login_count = 0`,
`last_failed_login_at = NULL`, `locked_until = NULL` — clean
unlocked state. No pre-existing user is auto-locked by the
upgrade.

**Migration function shape** (extends R1's
`auth::recovery::migrate_user_recovery_schema`):

```rust
pub(crate) async fn migrate_user_lockout_schema(db: &Db) -> Result<()> {
    // ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS …
    // CREATE INDEX IF NOT EXISTS …
}
```

Called from `auth::init_tables` after the existing R1 migration.
Idempotent. Rolling back to R1 is data-safe (the new columns
become unreferenced; nothing hard-fails).

---

## 5. Audit event plan

R2 emits **4 already-declared** AuditEvent variants (their
strings frozen in 0.5.0) and adds **1 new variant**:

| Variant | `as_str()` | Status | Wired in R2 commit |
|---------|-----------|--------|---------------------|
| `PasswordResetByOther` | `"password_reset_by_other"` | declared 0.5.0 | admin-reset handler |
| `AccountLocked` | `"account_locked"` | declared 0.5.0 | manual lock + auto-throttle |
| `AccountUnlocked` | `"account_unlocked"` | declared 0.5.0 | manual unlock |
| `SessionsRevokedByOther` | `"sessions_revoked_by_other"` | declared 0.5.0 | admin-revoke + per-revoked-id rows from admin-reset |
| `ForcedPasswordChangeCompleted` | `"forced_password_change_completed"` | **NEW in 0.6.0** | must-change-password POST |

The new variant is added with the same `#[non_exhaustive]`
ergonomics — additive, no breaking match-arms for external
consumers.

### 5.1 `metadata` shapes

For `PasswordResetByOther`:
```json
{
  "actor_user_id": 7,
  "actor_email_hash": "<8-char-fingerprint>",
  "reason": "user requested via support ticket #1284",
  "mode": "email" | "temp_pw",
  "email_send_status": "sent" | "failed" | null,
  "must_change_password_set": true,
  "token_fingerprint": "<token:…XXXXXXXX>"  // mode=email only
}
```

For `AccountLocked` (manual):
```json
{
  "actor_user_id": 7,
  "reason": "suspected credential leak; locking pending investigation",
  "until": "2026-05-09T14:00:00Z",
  "via": "manual"
}
```

For `AccountLocked` (auto-throttle):
```json
{
  "actor_user_id": null,
  "reason": "auto-throttle: 5 failed logins within 10 minutes",
  "until": "2026-05-09T14:15:00Z",
  "via": "auto_throttle",
  "failed_count": 5
}
```

For `AccountUnlocked`:
```json
{
  "actor_user_id": 7,
  "reason": "false positive; user verified identity",
  "via": "manual"
}
```

For `SessionsRevokedByOther` (one row per revoked session):
```json
{
  "actor_user_id": 7,
  "session_id": <revoked_id>,
  "reason": "preemptive on credential leak",
  "via": "manual" | "admin_password_reset"
}
```

For `ForcedPasswordChangeCompleted`:
```json
{
  "triggered_by_audit_id": 4231,  // the prior PasswordResetByOther row
  "invalidated_session_count": 3
}
```

### 5.2 Actor / target separation

Doctrine D1: `metadata.actor_user_id` is the admin who initiated
the action; `object_id` is the affected user. The legacy
`LogEntry::user_id` field continues to carry the actor for
backwards compat with `/admin/history`'s "who did what" view.

For auto-throttle (no actor): `metadata.actor_user_id = null`;
`LogEntry::user_id = the affected user` (closest reasonable
choice — the row is "about" the affected user).

Pinned by a new test
`audit_event_admin_actions_carry_actor_metadata` on the R2
emission paths.

### 5.3 `correlation_id` chains

R2 chains:

- **admin reset (email mode)**:
  `PasswordResetByOther` (request) → target consumes link →
  `PasswordResetSelfConsume` (R1 emission, same
  `correlation_id`? — NO; the consume is the target's request,
  with its own `correlation_id`. The link is via
  `metadata.token_fingerprint` which appears in both rows,
  letting an investigator pivot.)
- **admin reset (temp_pw mode)**:
  `PasswordResetByOther` (issue + immediate set_password) →
  N × `SessionsRevokedByOther` (one per revoked session) — all
  share one `correlation_id` (the admin's POST).
- **manual lock**:
  `AccountLocked` (manual) → N × `SessionsRevokedByOther` —
  same `correlation_id`.
- **auto-throttle lock**:
  `AccountLocked` (auto_throttle) — single row, no session
  revocation (locking doesn't revoke; the user's existing
  session remains until they next request, at which point the
  `locked_until` check refuses).
- **forced rotation**:
  `ForcedPasswordChangeCompleted` (target's POST) → N ×
  `SessionsRevokedSelf` — same `correlation_id`. The
  `metadata.triggered_by_audit_id` links back to the prior
  `PasswordResetByOther` row that originally set the flag.

---

## 6. Module + types layout

### 6.1 New submodule

`crates/rustio-admin/src/auth/recovery_admin.rs` — sibling to
the R1 `recovery.rs`. Holds:

```rust
// schema migration
pub(crate) async fn migrate_user_lockout_schema(db: &Db) -> Result<()>;

// admin-driven flows
pub(crate) async fn issue_admin_reset_token(...) -> Result<AdminIssueOutcome>;
pub(crate) async fn admin_set_temp_password(...) -> Result<AdminTempPwOutcome>;
pub(crate) async fn lock_user_account(...) -> Result<LockOutcome>;
pub(crate) async fn unlock_user_account(...) -> Result<UnlockOutcome>;
pub(crate) async fn admin_revoke_sessions(...) -> Result<RevokeOutcome>;

// login-flow integration
pub(crate) async fn check_account_lockout(db: &Db, user_id: i64) -> Result<LockState>;
pub(crate) async fn record_failed_login(db: &Db, user_id: i64, throttle: LoginThrottle) -> Result<ThrottleOutcome>;
pub(crate) async fn record_successful_login(db: &Db, user_id: i64) -> Result<()>;

// re-auth wall
pub(crate) async fn promote_session_elevated(db: &Db, session_id: i64, ttl: Duration) -> Result<()>;
pub(crate) async fn check_session_elevated(db: &Db, session_id: i64) -> Result<bool>;
```

Outcome enums follow R1's pattern (Issued / RateLimited /
Invalid). Each variant carries enough typed information for the
handler to decide page rendering without embedding HTTP
concerns in the runtime layer.

### 6.2 New handler module

`crates/rustio-admin/src/admin/admin_recovery_handlers.rs` —
wires the runtime functions to HTTP. 8-9 new handlers (see
§7).

### 6.3 New `RecoveryPolicy` methods

```rust
pub trait RecoveryPolicy: Send + Sync {
    // existing R1 methods …

    /// Login-throttle parameters. Default 5 failures / 10min →
    /// 15min lock.
    fn login_throttle(&self) -> LoginThrottle {
        LoginThrottle::default()
    }

    /// Re-auth wall window after `/admin/reauth` succeeds.
    /// Default 15 minutes.
    fn reauth_window(&self) -> Duration {
        Duration::minutes(15)
    }

    /// Multi-tenant readiness hook. Default impl returns
    /// `Arc::clone(&self_as_arc)` — single-tenant deployments
    /// see no change. Multi-tenant projects override to scope
    /// rate-limits / TTLs / lockout windows per tenant.
    fn scope_for(&self, _identity: &Identity) -> SharedRecoveryPolicy {
        // default: return self unchanged
        // (impl detail: requires policy to expose Arc<Self>;
        // see commit plan for the indirection trick)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LoginThrottle {
    pub max_attempts: u32,        // default 5
    pub window_minutes: i64,      // default 10
    pub lock_minutes: i64,        // default 15
}
impl Default for LoginThrottle { /* … */ }
```

### 6.4 New `LogEntry` field (additive)

```rust
pub struct LogEntry<'a> {
    // existing R1 fields …

    /// The acting principal when the row records an admin acting
    /// on another user. R2 emissions set this; R0/R1 emissions
    /// leave it `None`. Persisted under `metadata.actor_user_id`
    /// (the column itself doesn't change).
    pub actor_user_id: Option<i64>,
}
```

Backwards-compat: existing struct-literal call sites add
`actor_user_id: None` (one line). The serialization to
`metadata.actor_user_id` happens inside `audit::record` —
LogEntry stays the typed boundary, JSON metadata stays the
free-form persistence layer. Same pattern as R1's
`LogEntry::with_event(...)` boundary (Option A).

---

## 7. Routes table

| Method | Path | Handler | Guard | Re-auth required |
|--------|------|---------|-------|------------------|
| `GET` | `/admin/users/:id/reset-password` | `show_admin_reset_password` | `Role::Administrator` + cross-rank | yes |
| `POST` | `/admin/users/:id/reset-password` | `do_admin_reset_password` | same | yes |
| `POST` | `/admin/users/:id/lock` | `do_lock_user` | same | yes |
| `POST` | `/admin/users/:id/unlock` | `do_unlock_user` | same | yes |
| `POST` | `/admin/users/:id/revoke-sessions` | `do_admin_revoke_sessions` | same | yes |
| `GET` | `/admin/reauth` | `show_reauth` | `Role::User` | n/a |
| `POST` | `/admin/reauth` | `do_reauth` | same | n/a |
| `GET` | `/admin/must-change-password` | `show_must_change` | login_guard only | bypasses must-change check |
| `POST` | `/admin/must-change-password` | `do_must_change` | same | same |

All POST routes inherit the project's `csrf_protect` middleware
(unchanged from R0 / R1).

### 7.1 Modifications to existing routes

- **`POST /admin/login`** (existing R0 handler): integrates
  `check_account_lockout` BEFORE password verification; on
  failure, calls `record_failed_login` (which auto-throttles
  if needed); on success, calls `record_successful_login` to
  reset the counter.
- **`role_guard` / `login_guard`**: after R2 lands, every
  authenticated route runs `check_must_change_password`
  immediately after `login_guard`. If the flag is set and the
  current path is NOT in the must-change-password whitelist
  (`/admin/must-change-password`, `/admin/logout`,
  `/admin/account/sessions`), redirect.

### 7.2 Registration order

Recovery routes (R1) and admin-recovery routes (R2) must both
be registered BEFORE the `/admin/:admin_name` model wildcards.
The commit-#9-equivalent for R2 adds the new routes between
the existing `/admin/users/*` builtin routes and the model
wildcards (where the `/admin/users/:id/edit` etc. already sit
— putting the new ones nearby keeps the user-related cluster
contiguous).

---

## 8. Trait extensions

### 8.1 `RecoveryPolicy` adds 3 methods

All have provided defaults so existing impls don't break.

- `fn login_throttle(&self) -> LoginThrottle` — see §3.3.
- `fn reauth_window(&self) -> Duration` — default 15 minutes.
- `fn scope_for(&self, identity: &Identity) -> SharedRecoveryPolicy`
  — default returns `self` (no per-tenant scoping). The
  signature requires `Arc<Self>` indirection; details land in
  commit #5-equivalent.

### 8.2 New `LoginThrottle` struct

```rust
#[derive(Debug, Clone, Copy)]
pub struct LoginThrottle {
    pub max_attempts: u32,    // 5
    pub window_minutes: i64,  // 10
    pub lock_minutes: i64,    // 15
}

impl LoginThrottle {
    pub const DEFAULT: Self = Self {
        max_attempts: 5,
        window_minutes: 10,
        lock_minutes: 15,
    };
}

impl Default for LoginThrottle {
    fn default() -> Self { Self::DEFAULT }
}
```

`pub` on the framework's `auth` re-export surface.

### 8.3 No new traits

All R2 organisational behaviour fits in `RecoveryPolicy` +
existing `PasswordPolicy`. No `LockoutPolicy` or
`AdminRecoveryPolicy` trait needed — operators with custom
behaviour subclass `RecoveryPolicy`.

---

## 9. Existing-handler integration deltas

### 9.1 Login flow (`do_login` in `admin/handlers.rs`)

Pre-R2:

```rust
match auth::login(&ctx.db, email, password).await {
    Ok(token) => { /* set cookie, redirect */ }
    Err(_)    => { /* render login form with error */ }
}
```

Post-R2:

```rust
// 1. Look up the user (don't verify password yet).
let user = match auth::find_user_by_email(&ctx.db, email).await? {
    Some(u) => u,
    None => return /* uniform 401 */,
};

// 2. Lockout check FIRST (before password verify). Same uniform 401
//    response whether locked or wrong password — no enumeration leak.
match auth::recovery_admin::check_account_lockout(&ctx.db, user.id).await? {
    LockState::Locked { .. } => return /* uniform 401 */,
    LockState::Unlocked => {} // proceed
}

// 3. Verify password.
if !auth::verify_password(password, &user.password_hash) {
    auth::recovery_admin::record_failed_login(
        &ctx.db, user.id, ctx.admin.active_recovery_policy().login_throttle()
    ).await?;
    return /* uniform 401 */;
}

// 4. Success — reset throttle counter, mint session.
auth::recovery_admin::record_successful_login(&ctx.db, user.id).await?;
let token = auth::create_session(&ctx.db, user.id).await?;
// ... existing cookie + redirect logic
```

Failure path emits `AccountLocked` (auto-throttle) audit when
the threshold trips.

### 9.2 `login_guard` adds must-change-password check

```rust
async fn login_guard(ctx: &AdminCtx, req: &Request) -> Result<Guard> {
    // ... existing identity resolution ...

    // R2 — forced rotation gate. Check BEFORE any role gate so
    // even Administrators with the flag set are forced through.
    if ident.must_change_password
        && !is_must_change_whitelisted_path(req.path())
    {
        return Ok(Guard::Redirect(
            Response::redirect("/admin/must-change-password")
        ));
    }

    Ok(Guard::Allow(ident))
}

const MUST_CHANGE_WHITELIST: &[&str] = &[
    "/admin/must-change-password",
    "/admin/logout",
    "/admin/account/sessions",
];
```

The `Identity` struct needs a new field
`must_change_password: bool` — populated by `find_user_by_email`
+ `identity_from_session`. Additive change, defaults to false
for pre-R2 sessions issued before the feature lands.

### 9.3 Admin-driven recovery handlers go through `enforce_cross_rank_safe`

R0's authority guards (`enforce_cross_rank_safe`,
`enforce_role_ceiling`, `enforce_no_orphan_role`) apply to R2's
admin-driven routes verbatim. A Supervisor cannot reset an
Administrator's password; an Administrator cannot lock the last
Developer's account. Reuse, no new guard logic.

---

## 10. Test plan

### 10.1 Unit (pure)

- `LoginThrottle::DEFAULT` — locked 5 / 10 / 15.
- `is_must_change_whitelisted_path("/admin")` — false; for the
  three whitelisted paths — true.
- `parse_lock_duration("15m") / "1h" / "indefinite"` — pure
  duration parse; covers the form's preset + freeform combo.
- `redact_actor_email(email) -> 8-char hash` — fingerprint
  shape for the `metadata.actor_email_hash` field; same
  property test as R1's `redact_token`.

### 10.2 Schema migration

- Boot fresh DB → migrations apply, columns + index appear.
- Boot 0.5.0 DB → idempotent re-application, no errors.
- Re-boot → no errors.

### 10.3 DB integration (NEW — testcontainers)

R1's downstream-validation pass surfaced one bug
(`check_reset_token_valid` SQL type mismatch) that no unit test
caught because no test hit a real Postgres. R2 ships with a
local-Postgres integration suite using `testcontainers` —
boots an ephemeral Postgres in CI, applies migrations, exercises
each runtime function end-to-end:

- `issue_admin_reset_token` writes a row, mailer fires, audit
  row carries actor_user_id, target session unaffected (only
  affected on consume).
- `admin_set_temp_password` writes hash, sets
  `must_change_password = TRUE`, revokes sessions,
  audit-emits `PasswordResetByOther`.
- `lock_user_account` sets `locked_until`, revokes sessions,
  audit-emits `AccountLocked`.
- `unlock_user_account` clears `locked_until`, resets
  `failed_login_count`, audit-emits `AccountUnlocked`.
- `record_failed_login` increments + auto-locks at threshold,
  emits `AccountLocked(via=auto_throttle)`.
- `check_account_lockout` honours `locked_until`.
- `promote_session_elevated` / `check_session_elevated` round-
  trip.

Located under `crates/rustio-admin/tests/integration/`. Gated
behind a `--features integration-test` flag so CI runs them but
`cargo test --workspace` (no flag) skips them — keeps the unit-
test gate fast.

### 10.4 End-to-end (downstream validation pass)

Stockholm POS smoke test against the live DB before publish:

- Admin resets target via email mode → target receives
  `LogMailer` log line (URL redacted) → admin shares link →
  target consumes → forced rotation interstitial → new
  password works.
- Admin resets target via temp-pw mode → admin sees temp pw
  ONCE → target signs in with temp pw → forced rotation
  interstitial → new password works.
- Admin locks target with reason "test" for 15 min → target
  cannot sign in → wait → target signs in normally.
- Admin manually unlocks target → target signs in immediately.
- Auto-throttle: 5 failed logins → 6th attempt gets locked
  response → wait 15 min → succeed.
- Re-auth wall: admin without elevated_until tries to reset
  → redirect to /admin/reauth → enter password → return to
  reset form.

---

## 11. Implementation phases & atomic commit plan

R1's discipline (small commits, one concern per commit, gates
after each risky one) carries through. Estimated 17 commits.

| # | Concern | Files |
|---|---------|-------|
| 1 | Schema: `failed_login_count`, `last_failed_login_at`, `locked_until` + partial index | `auth/recovery_admin.rs` (new), `auth/mod.rs` (init_tables wiring) |
| 2 | Policy unification: CLI uses `DefaultPasswordPolicy` directly | `crates/rustio-admin-cli/src/user.rs` |
| 3 | Policy unification: admin-create-user form's hint reads `min_length` from policy | `admin/render.rs::user_new_form_sections`, `admin/builtin.rs::do_new_user` (validate via policy) |
| 4 | Policy unification: remove admin-edit form's password field per `DESIGN_RECOVERY.md` §14.4 | `admin/builtin.rs`, `admin/render.rs`, `assets/templates/admin/user_edit.html` |
| 5 | `LoginThrottle` struct + `RecoveryPolicy::login_throttle` provided default; `RecoveryPolicy::reauth_window` provided default; `scope_for` extension hook | `auth/recovery.rs` |
| 6 | New `AuditEvent::ForcedPasswordChangeCompleted` variant + drift-test update | `admin/audit.rs` |
| 7 | `LogEntry::actor_user_id` field + `record()` writes it under `metadata.actor_user_id` | `admin/audit.rs` |
| 8 | `Identity::must_change_password` field; `find_user_by_email` + `identity_from_session` populate it | `auth/users.rs`, `auth/sessions.rs` |
| 9 | Login throttle runtime: `check_account_lockout`, `record_failed_login`, `record_successful_login`. Plus login flow integration in `admin/handlers.rs::do_login` | `auth/recovery_admin.rs`, `admin/handlers.rs` |
| 10 | Re-auth wall runtime: `promote_session_elevated`, `check_session_elevated` | `auth/recovery_admin.rs`, `auth/sessions.rs` (the column already exists from R0) |
| 11 | Re-auth handler + template: `GET/POST /admin/reauth`, return_to validation, locked copy | `admin/admin_recovery_handlers.rs` (new), `assets/templates/admin/reauth.html` (new) |
| 12 | Forced-rotation handler + interstitial template: `GET/POST /admin/must-change-password` | `admin/admin_recovery_handlers.rs`, `assets/templates/admin/must_change_password.html` (new) |
| 13 | `login_guard` integrates must-change-password redirect; whitelist constant | `admin/routes.rs` |
| 14 | Admin reset runtime: `issue_admin_reset_token` (email mode), `admin_set_temp_password` (temp_pw mode) | `auth/recovery_admin.rs` |
| 15 | Admin reset handler + form template (mode selector + reason) | `admin/admin_recovery_handlers.rs`, `assets/templates/admin/admin_reset_password.html` (new) |
| 16 | Lock / unlock / revoke handlers + form templates | `admin/admin_recovery_handlers.rs`, three new templates |
| 17 | Route registration (5 admin-recovery + 2 reauth + 2 must-change = 9 new routes), middleware ordering re-confirmed | `admin/routes.rs` |
| (docs) | CHANGELOG entry under `[Unreleased]`, README pointer | `CHANGELOG.md`, `README.md` |
| (chore) | Prepare 0.6.0 — version bump | `Cargo.toml`, `crates/rustio-admin-cli/Cargo.toml`, `templates/project/Cargo.toml.tmpl`, `README.md` install snippet |

`cargo test --workspace` runs after #1 (schema), #5 (policy
extensions), #8 (Identity field), #9 (login throttle integration),
#11 (re-auth handler), #13 (login_guard wiring), #15 (admin
reset), #17 (route registration). `cargo clippy --workspace
--all-targets -- -D warnings` runs at the same gates.

After commit 17 the pre-publish gate runs in full per
`working_style.md`:

```
cargo fmt --all
cargo test --workspace
cargo test --workspace --features integration-test  # NEW: testcontainers suite
cargo clippy --workspace --all-targets -- -D warnings
cargo publish --dry-run -p rustio-admin-macros
cargo publish --dry-run -p rustio-admin
```

Then a downstream validation pass against the live Stockholm
POS DB. Then — and only then — wait for explicit "publish 0.6.0"
before `cargo publish`.

---

## 12. Locked decisions

| Decision | Value | Override |
|----------|-------|----------|
| Login throttle: max attempts | **5** | `LoginThrottle::max_attempts` |
| Login throttle: window | **10 min** | `LoginThrottle::window_minutes` |
| Login throttle: lock duration | **15 min** | `LoginThrottle::lock_minutes` |
| Re-auth wall window | **15 min** | `RecoveryPolicy::reauth_window` |
| Admin reset token TTL | **1 hour** (same as R1 self-reset) | `RecoveryPolicy::reset_token_ttl` (already from R1) |
| Temp password length | **16 chars URL-safe-base64** | none (not project-tunable; doctrine — see below) |
| Temp password rendering | **shown ONCE on the admin's success page; never logged, never persisted in plaintext** | none (doctrine D2) |
| Reason field minimum | **8 chars** (matches R4 CLI `--reason`) | none |
| `must_change_password` whitelisted paths | `/admin/must-change-password`, `/admin/logout`, `/admin/account/sessions` | none |
| Auto-throttle revokes sessions | **NO** — locking only prevents future logins; existing sessions stay valid until `locked_until` is checked at the next request | doctrine D4 + §3.3 |
| Manual lock revokes sessions | **YES** — admin's intent is "kick this user out NOW"; `invalidate_sessions(User, AdministrativeRevoke)` runs synchronously | doctrine D4 |
| Default `MfaPolicy::Optional` | inherited from R1 | R3 |

---

## 13. Open questions for kickoff (re-confirm before commit #1)

The three R1-locked decisions were re-confirmed at kickoff
(carry through unchanged). New R2-specific questions to confirm
before commit #1 lands:

1. **Re-auth window = 15 minutes?** Recommendation: yes. Long
   enough for a chain of admin actions; short enough that a
   walked-away workstation isn't permanent.
2. **Auto-throttle threshold = 5 / 10min?** Recommendation:
   yes. Industry-standard. Operators with stricter compliance
   (banking, healthcare) override via
   `LoginThrottle::with_max_attempts(3)`.
3. **Auto-throttle does NOT revoke sessions?** Recommendation:
   yes. The lock is "block future logins"; the existing
   sessions stay live. Manual lock DOES revoke sessions
   (admin's stronger intent).
4. **Temp-password mode renders once on the admin's page?**
   Recommendation: yes. NOT logged, NOT in audit metadata
   (only fingerprint persists). Admin shares out-of-band.
5. **`metadata.actor_email_hash` instead of `actor_email`?**
   Recommendation: yes (8-char SHA-256 fingerprint per the R1
   `redact_token` pattern). Auditing can pivot via the hash;
   PII stays out of the audit table. Actor identity is
   discoverable via `metadata.actor_user_id` joining on
   `rustio_users.id`.
6. **`/admin/users/:id/lock` form duration presets** —
   `15min / 1h / 24h / 7d / indefinite`. Recommendation: yes.
   All five plus a freeform "until <timestamp>" input.
7. **Forced-rotation whitelist scope** — only the three paths
   listed in §3.4? Recommendation: yes. Adding more paths
   (e.g. `/admin/account/profile`) feels like scope creep;
   forced rotation should be brief and uncontentious.

---

## 14. What R2 does NOT do (deferred)

- **TOTP MFA + backup codes** → R3.
- **CLI emergency recovery** (`rustio user reset-password / unlock /
  …`) → R4. R2's web-side admin recovery covers the operator's
  steady state; R4 covers the "framework is down or the only
  Administrator's password is lost" emergency path.
- **Email notification to target** when an admin resets their
  password. Considered, deferred (see §2.3).
- **Reset-link IP / device pinning** — same residual leak as
  R1; out of scope.
- **CAPTCHA on login** — not in scope; project middleware can
  layer.
- **Multi-tenant schema migration** — R2 ships the
  `RecoveryPolicy::scope_for` hook; the actual multi-tenant
  schema work is a separate phase.
- **Audit-log retention sweeper** — operator-owned per
  `DESIGN_AUDIT.md` §9.
- **Permission-based admin recovery delegation** ("Staff with
  `reset_password` permission can issue resets"). R2 routes
  gate on `Role::Administrator` baseline; finer-grained perms
  is a future phase.

---

## 15. Versioning

R2 ships as `rustio-admin@0.6.0`. Patch releases on the 0.6.x
line are reserved for fixes that don't change semantics.

- New table: minor.
- New `AuditEvent` variant (`ForcedPasswordChangeCompleted`):
  minor (additive; `#[non_exhaustive]` covers).
- New `LogEntry::actor_user_id` field: minor (additive; legacy
  struct-literal callers add `None` in one line).
- New `Identity::must_change_password` field: minor.
- New `RecoveryPolicy` methods (`login_throttle`,
  `reauth_window`, `scope_for`): minor (provided defaults
  preserve existing impls).
- `do_login` flow change (lockout check + auto-throttle): minor
  (behaviour change visible to end-users — `Behaviour change`
  CHANGELOG section, same as R1's revoke-other-devices
  treatment).
- New routes (9): minor.

---

## 16. PR review checklist (R2-specific additions)

Walked alongside the existing 8-item visual regression checklist
+ token-disclosure section in `.github/pull_request_template.md`:

- [ ] Grep proof: `revoked_at\s*=` returns only
      `auth/sessions.rs::invalidate_sessions`.
- [ ] Grep proof: no plaintext temp password in any template,
      log statement, or audit summary.
- [ ] Grep proof: no plaintext actor email in audit metadata
      (only `actor_user_id` + `actor_email_hash`).
- [ ] Manual: admin reset (email mode) → target consumes →
      forced rotation interstitial → new password works.
- [ ] Manual: admin reset (temp_pw mode) → temp pw rendered
      once → target signs in → forced rotation → new password
      works.
- [ ] Manual: lock account → target login refused with uniform
      "currently disabled" copy.
- [ ] Manual: 5 wrong logins → uniform 401 → 6th attempt =
      same uniform 401 (no enumeration leak that auto-lock
      fired).
- [ ] Manual: re-auth wall: admin without elevated_until tries
      sensitive admin route → redirect to /admin/reauth →
      success → return_to original page.
- [ ] Manual: forced rotation cannot be skipped — direct GET
      /admin/users/1 with the flag set redirects to
      /admin/must-change-password.
- [ ] `cargo test --workspace` passes at every commit.
- [ ] `cargo test --workspace --features integration-test`
      passes (NEW — testcontainers Postgres suite).
- [ ] CHANGELOG entry placed under `[Unreleased]`,
      sectioned by `Recovery / Sessions / Audit / Security /
      Behaviour change / Documentation / Internal`.
- [ ] `DESIGN_R2_ORGANISATIONAL.md` entries updated if any
      locked decision was amended.

---

## 17. What this document does NOT cover

- TOTP MFA (R3) — separate doc on R3 kickoff.
- CLI emergency recovery (R4) — separate doc on R4 kickoff.
- Multi-tenant scoping schema — separate phase.
- API tokens / service accounts — re-uses the same invalidation
  engine; recovery vocabulary may extend with a
  `ServiceAccountKeyRotated` reason in a future phase.
