# Recovery Architecture

Companion to `DESIGN_SYSTEM.md`, `DESIGN_SESSIONS.md`, and
`DESIGN_AUDIT.md`. Codifies the contract for **R1 — self password
recovery** (target 0.5.0) and lays the structural foundation that
R2–R4 plug into without re-litigation.

> **Doctrine 9** in `DESIGN_SYSTEM.md` makes recovery part of the
> authority system, not an exception. **Doctrine 13** locks the
> visual register: calm, GitHub-settings-like, no marketing
> gradients, no auth illustrations. This document operationalises
> both for the unauthenticated self-reset flow.

This is a contract. PR review is against this doc.

---

## 0. Status

**Phase:** R1 — self password recovery.
**Branch:** `feat/self-password-recovery` (cut from `main` at
`67f28bc`).
**Target:** `rustio-admin@0.5.0`.
**Blocking:** R2 (organisational recovery), R3 (TOTP MFA), R4 (CLI
emergency recovery).

R0 (0.4.0) shipped the foundations this phase depends on:

- Hashed-at-rest session tokens (`token_hash`) — ✅ in
  `auth/sessions.rs`.
- Centralised `invalidate_sessions(SessionTarget,
  SessionInvalidationReason)` — ✅; the `PasswordReset` reason variant
  already exists.
- Typed `AuditEvent` enum with `PasswordResetSelfRequest` /
  `PasswordResetSelfConsume` already enumerated (pub(crate)) — ✅.
- `email::Mailer` trait + `LogMailer` default + `Mail::framework_envelope`
  — ✅.
- `audit::redact_password()` / `redact_token()` — ✅.
- `correlation_id` middleware before `csrf_protect` — ✅.
- Audit row carries `correlation_id` / `session_id` / `metadata` JSONB
  — ✅.
- Read-only `/admin/account/sessions` page — ✅.

R1 is what lights this engine up.

---

## 1. Doctrine recap (LOCKED — do not re-litigate)

| # | Doctrine | Implication for R1 |
|---|----------|---------------------|
| 1 | Three-axis ownership (identity / authority / infrastructure) | The reset flow operates on the **identity** axis. It cannot grant or change authority; the resulting session is `Authenticated` only, never `Elevated` or `MfaVerified`. |
| 6 | Email is operational infrastructure | `Mailer` is project-supplied. Framework refuses to lock into SMTP. R1 ships no transport. |
| 7 | Active-sessions page is the core security surface | R1 wires its revoke buttons; the page becomes the user's first stop after a "wasn't me" reset email. |
| 8 | Audit logs must be forensically useful | Every reset action (request, success, rejection) writes a row tied to a `correlation_id` so the chain "request → token issued → consumed → sessions revoked" is queryable. |
| 9 | Email is convenience, **not root of trust** | A reset never bypasses MFA (R3+); a reset never lifts trust above `Authenticated`; admin-driven recovery (R2) and CLI (R4) outrank email-driven reset. |
| 11 | Never log secrets | No plaintext token in any log line, audit summary, or error response. The `metadata` JSONB stores `token_fingerprint` (8-char redacted) for cross-row pivot, never the token itself. |
| 13 | Recovery pages match visual doctrine | Calm cards. Tajawal/Geist stack. `--rio-accent` teal-emerald. No marketing gradients, no illustrations, no "✨ Reset your password ✨" copy. |
| 15 | Recovery never bypasses rank doctrine | Self-reset cannot promote a user. The only mutation is `password_hash`. Every other column on `rustio_users` (`role`, `is_active`, `is_demo`, `must_change_password`, …) is untouched by the reset path. |
| 18 | Audit events evolve toward typed | R1 promotes `AuditEvent` from `pub(crate)` to `pub` on the same release that introduces the new variants. Lower bound to the public typed surface lands as part of 0.5.0. |
| 22 | Centralised session invalidation | A grep for `revoked_at\s*=` across `crates/` must continue to return only `auth::sessions::invalidate_sessions`. R1 calls `invalidate_sessions(SessionTarget::User, SessionInvalidationReason::PasswordReset)`. |

---

## 2. Threat model

### 2.1 Adversaries

| Adversary | Capability | Mitigation |
|-----------|------------|------------|
| **Anonymous attacker, off-network** | Probes `/admin/forgot-password` to enumerate registered emails, or to spam the mail backend | Uniform user response (§3.5). Tight per-IP rate-limit (§7). No audit row written for unknown emails (avoids enumeration via row count). |
| **Anonymous attacker, off-network** | Submits crafted reset tokens to `/admin/reset-password/<x>` to brute-force a valid one | Tokens are 256-bit random; consume path is constant-time-ish at the index level (`token_hash` lookup). 1h TTL bounds the window. Per-IP rate-limit on the consume path too. |
| **Email recipient** | Receives a leaked reset link (forwarded mail, shoulder surfing, public Wi-Fi) | Tokens are single-use (atomic `UPDATE … SET consumed_at = NOW() WHERE token_hash = $1 AND consumed_at IS NULL RETURNING …`). 1h TTL. Successful consume revokes ALL sessions, so even if the legit user lost the link, the attacker's window closes when the legit user logs in and notices. |
| **Operator with read access to the DB** (`SELECT *` on `rustio_password_reset_tokens`) | Tries to use a stored token to reset someone | Tokens stored as SHA-256 only — `token_hash` column. Plaintext exists only in the email body (delivered to the user's mailbox) and the email message in transit. The DB row alone cannot reset. |
| **Operator who can read mail backend logs** | Tries to harvest reset URLs from log streams | `LogMailer::redact_likely_tokens` strips any 32+ char alnum segment before logging; production mailers are project-owned and out of scope, but `Mail.text_body` is the only place the token ever appears. The framework never logs the rendered email. |
| **A user who's lost their password but has access to the email** | Legitimate use case | The flow itself. |
| **A user whose email account is compromised** | Attacker can request resets at will | This is the doctrine-9 floor: email is convenience, NOT root of trust. R3 MFA breaks this — a reset email cannot complete sign-in if the policy requires MFA. R1 page copy explicitly tells the user: "If your email account may be compromised, contact your administrator instead — they can lock the account (R2)." |

### 2.2 Out-of-scope (deferred)

- **Account lockout after repeated reset requests** — R2 (login-throttle generalises). R1 relies on rate-limit alone.
- **Email send-failure notification to the operator** — R1 writes `metadata.email_send_status = "failed"` and surfaces it on `/admin/history`. A push-notification or webhook is project-owned.
- **Reset-link IP / device pinning** — out of scope. Many users request reset on phone, click on laptop. We refuse to break that.
- **CAPTCHA on the request page** — out of scope. Rate-limit + uniform response should suffice. If a project needs CAPTCHA they can wrap the route in a project-supplied middleware.
- **Reset-token rotation** — single-use only. No "request a fresh link from this page" once consumed; user re-runs the request flow.

### 2.3 Disclosure rules (LOCKED)

The flow MUST NOT reveal:

- Whether an email is registered.
- Whether a previous reset email was already sent (no "we already sent one — check your inbox" copy).
- Whether the request was rate-limited (rate-limited responses must be visually identical to "successful" submissions; the only difference is HTTP 429 vs 200, and even that is pulled into a uniform "thanks, check your inbox" page in §3.5).
- The shape of the token (no length, no character class) anywhere except the URL itself.

The flow MAY reveal:

- That a token at `/admin/reset-password/<token>` is invalid / expired / already consumed — a user clicking a stale link deserves a clear error. The token is in the URL; the user already has it. Disclosure here cannot leak anything they didn't already have.

---

## 3. Recovery state machine

```
                                 ┌──────────────────────────────────┐
                                 │                                  │
            email submitted      ▼                                  │
   ┌──────────────────────► IssueRequested ─── unknown email ──► (no-op, log only)
   │                              │
   │                              │ user matched + rate-limit ok
   │                              ▼
   │                        TokenIssued ─── mailer error ──► TokenIssuedMailFailed
   │                              │                              │
   │                              ▼                              ▼
   │                        EmailDispatched           (audit row + uniform user response)
   │                              │
   │       click in <1h           │           click after 1h
   │   ┌──────────────────────────┴─────────────────────┐
   │   ▼                                                ▼
   │ TokenConsumePending                          TokenExpired
   │   │                                                │
   │   │ password validates against PasswordPolicy      │
   │   ▼                                                ▼
   │ TokenConsumed ───► InvalidateSessions ──► UI(reset complete)
   │   │
   │   │
   │   └──────► (token row's consumed_at = NOW(); no second use)
   │
   └────────────────────────────────────────────────────── (uniform response)
```

### 3.1 States

| State | Persisted shape | Visible to user |
|-------|-----------------|------------------|
| `IssueRequested` | (no row yet) | "If that email is registered, a link has been sent." |
| `TokenIssued` | `rustio_password_reset_tokens` row with `consumed_at IS NULL`, `expires_at = NOW() + 1h`, `mail_status = 'pending'` | Same uniform copy. |
| `EmailDispatched` | Same row, `mail_status = 'sent'` | Same uniform copy. |
| `TokenIssuedMailFailed` | Same row, `mail_status = 'failed'` (transient or permanent) | Same uniform copy. Audit row carries `metadata.email_send_status = 'failed'`. |
| `TokenConsumePending` | User landed on `GET /admin/reset-password/<t>` with valid + unconsumed + unexpired token | Form: "Set a new password" + 2 password inputs + CSRF. |
| `TokenConsumed` | `consumed_at = NOW()`. Atomic per §4.3. | Redirect to `/admin/login` with flash "Password updated. Please sign in." |
| `TokenExpired` | `expires_at < NOW()` OR `consumed_at IS NOT NULL` | Error page with "This link has expired or already been used." + link to request a new one. |

### 3.2 Invariants

- A user can have **multiple unconsumed tokens** at any time (resilience: phone/laptop/work email all in flight). The newest one wins implicitly because each is independently valid until its TTL or its single consume; we do NOT auto-consume older tokens when a newer one issues. (Rationale: a legitimate user may request twice, click the first email; that's fine.)
- A consumed token NEVER reverts to unconsumed.
- A successful consume MUST call `invalidate_sessions(SessionTarget::User { user_id }, SessionInvalidationReason::PasswordReset)` BEFORE returning to the user.
- No password reset bumps a session's `trust_level`. Period.
- The handler MUST NOT auto-create a session for the user after a successful reset. The user goes back through `/admin/login` so MFA (R3+) gets exercised.

### 3.3 Sequencing under concurrency

The consume path uses `RETURNING` to atomically commit the consume + read the user_id in one statement (§4.3). Two concurrent consumes against the same token resolve as: one wins and proceeds, the other sees `RETURNING` returning zero rows and renders the "expired or already used" page.

### 3.4 What R1 does NOT do

- Does not auto-redirect a successfully-consumed user into a logged-in state.
- Does not cancel pending tokens for the same user when one consumes (other tabs / browsers might have legitimate clicks queued; their consume will fail with the standard "expired or already used" page).
- Does not send a "your password was changed" notification email. (R2 adds this for admin-driven resets where the user didn't initiate; for self-reset the user knows what they did.)
- Does not surface a "see all active reset tokens" page. Operators query `rustio_password_reset_tokens` directly when investigating; the user-facing surface is `/admin/account/sessions`.

### 3.5 Uniform response copy

The single canonical line, returned by `POST /admin/forgot-password` regardless of branch (success / unknown email / mailer failure):

> **If that email address has an account, we just sent a sign-in link to it.**
>
> **The link expires in 1 hour and can only be used once.** If you don't see the email within a few minutes, check your spam folder, then try again.
>
> If your email account may be compromised, contact your administrator instead.

Rendered as a plain card on `/admin/forgot-password/sent` (a dedicated landing route) so a refresh doesn't resubmit the form.

---

## 4. Token lifecycle

### 4.1 Generation

```rust
let mut bytes = [0u8; 32];
rand::thread_rng().fill_bytes(&mut bytes);
let token = URL_SAFE_NO_PAD.encode(bytes);   // ~43 chars
let token_hash = URL_SAFE_NO_PAD.encode(sha2::Sha256::digest(token.as_bytes()));
```

Rationale: identical shape to session-cookie token. `SHA-256` is the right choice (not Argon2): the input is a 256-bit random token, brute force is infeasible regardless of work factor; SHA-256 is fast enough that the consume-path index seek stays sub-millisecond. Argon2 would add latency without security benefit. Same reasoning as `auth::sessions::hash_token_for_storage`; we re-use the same helper or expose a small `recovery::hash_token` shim that calls into it.

### 4.2 Issuance (write path)

`POST /admin/forgot-password`:

1. Read `email` from form. Trim. Lowercase.
2. Apply per-IP rate-limit (§7).
3. `find_user_by_email(db, email)`.
   - **Unknown** → `log::info!("password_reset_self_request: unknown email user_agent={} ip={}", ua_summary, ip)`. Return 200 + uniform response. **No DB row, no audit row.** This is the enumeration-resistance lever.
   - **Inactive** (`!user.is_active`) → treat as unknown. Same path. (Rationale: revealing the existence of a deactivated account is itself enumeration.)
   - **Known + active** → continue.
4. Generate `token`, `token_hash`. Insert `rustio_password_reset_tokens` row with `expires_at = NOW() + recovery_policy.reset_token_ttl()`.
5. Compose `Mail::framework_envelope(...)` with the reset URL (`{site_url}/admin/reset-password/{token}`).
6. Dispatch via `Admin::mailer().send(mail).await`.
   - On `Ok(())` → `mail_status = 'sent'`.
   - On `MailerError::ConfigurationMissing` at runtime → already caught at boot (§12); cannot reach this path in production. If somehow reached: log error, set `mail_status = 'failed'`, but still return uniform response.
   - On `MailerError::Transient` / `Permanent` → log error, set `mail_status = 'failed'`, write `metadata.email_send_status = "failed"` on the audit row.
7. Write audit row: `AuditEvent::PasswordResetSelfRequest`, `model_name = "user"`, `object_id = user.id`, `correlation_id` from request context, `session_id = None` (this path is unauthenticated), `metadata = { token_fingerprint: redact_token(&token), email_send_status: "sent" | "failed", expires_at }`.
8. Return 303 → `/admin/forgot-password/sent`.

### 4.3 Consume (read path)

`POST /admin/reset-password/<token>`:

1. Apply per-IP rate-limit (tighter than the issuance bucket; §7).
2. Hash the URL token: `token_hash = sha256(token)`.
3. Validate form: `password1`, `password2`. Run `password_policy.validate(password1)` (§13). If invalid: re-render the form with field errors.
4. **Atomic consume**:
   ```sql
   UPDATE rustio_password_reset_tokens
      SET consumed_at = NOW()
    WHERE token_hash = $1
      AND consumed_at IS NULL
      AND expires_at > NOW()
   RETURNING user_id
   ```
   If `RETURNING` is empty → render the "expired or already used" error page. Audit row: `AuditEvent::PasswordResetSelfRequest` (NOT a new "rejected" variant — see §6.2 for why) is NOT written here; the consume path that fails is logged (`log::info!`) but does not write an audit row. Rationale: an attacker grinding tokens would otherwise spam the audit table.
5. **Hash + write the new password**: `auth::set_password(db, user_id, new_password)`. **Add to `set_password`**: stamp `password_changed_at = NOW()` on the same UPDATE.
6. **Invalidate every session**: `invalidate_sessions(SessionTarget::User { user_id }, SessionInvalidationReason::PasswordReset)`.
7. Write one `AuditEvent::PasswordResetSelfConsume` row with `correlation_id`, `model_name = "user"`, `object_id = user_id`, `metadata = { token_fingerprint: redact_token(&token), invalidated_session_count: outcome.revoked_session_ids.len() }`.
8. **Do NOT mint a new session.** Redirect 303 → `/admin/login?password_reset=success`. The login page reads the query param and renders a one-line confirmation banner.

### 4.4 Cleanup

A periodic sweeper deletes `rustio_password_reset_tokens` rows where `expires_at < NOW() - interval '7 days'`. The 7-day grace keeps a usable forensic trail (an investigator can see "how many resets were requested for user X in the past week"); after that, the row's audit-trail value is gone.

This piggy-backs on the existing `background::spawn_session_sweeper` — one new function `purge_expired_reset_tokens(db)` called from the same sweep tick. R1 ships the function; the sweeper integration is one of the small commits (§16).

---

## 5. Invalidation semantics (delta from DESIGN_SESSIONS.md §5)

R1 introduces no new `SessionInvalidationReason` variants. It exercises one existing variant for the first time:

| Reason | Cookie cleared? | Replacement minted? | Audit `action_type` | First wired in |
|--------|-----------------|---------------------|---------------------|----------------|
| `PasswordReset` | yes (every device — there are no current devices in the unauth flow) | no | `password_reset_self_consume` | **R1** |

The `UserRequested` reason — already wired for the `/admin/account/sessions` revoke buttons R1 ships — uses target `Single` / `UserExceptCurrent` and writes `AuditEvent::SessionsRevokedSelf`.

### 5.1 Revoke-buttons drift correction

The active-sessions page (R0) ships read-only. R1 adds three POST routes:

| Route | Target | Reason | Audit |
|-------|--------|--------|-------|
| `POST /admin/account/sessions/<id>/revoke` | `Single { session_id }` (rejected if it equals the current session id) | `UserRequested` | `SessionsRevokedSelf`, `metadata = { revoked_session_ids: [id] }` |
| `POST /admin/account/sessions/revoke-others` | `UserExceptCurrent { user_id, current_session_id }` | `UserRequested` | One row per revoked id, all sharing the same `correlation_id` |
| `POST /admin/account/sessions/revoke-all` | `User { user_id }` | `UserRequested` | One row per revoked id; **also clears the cookie** and redirects to `/admin/login`. |

All three are gated by `role_guard(min = Role::User)`, CSRF-protected.

### 5.2 Existing `do_password_change` drift correction

The existing authenticated `/admin/password_change` handler (`handlers.rs:809`) currently calls `auth::set_password(...)` and stops. R1 must update it:

1. Stamp `password_changed_at` (lands inside `set_password` itself; one site of change).
2. Call `invalidate_sessions(SessionTarget::UserExceptCurrent { user_id, current_session_id }, SessionInvalidationReason::UserRequested)` to log the user out from every other device but keep this one alive.
3. Write `AuditEvent::SessionsRevokedSelf` for each revoked id, plus a typed `AuditEvent::UserUpdated` (or a new `PasswordChangedSelf` — see §6.4) for the password change itself.

The reasoning: a user changing their password while signed in expects "log out other devices" by default. Today the framework leaves stale sessions live, which contradicts doctrine 22's spirit. R1 fixes this on the same release.

---

## 6. Audit event plan

### 6.1 Variants used by R1

All four already exist as `AuditEvent` variants from R0 (pub(crate)); none need to be added — they only need to be **emitted**:

| `AuditEvent` variant | `as_str()` | Where R1 emits it |
|----------------------|-----------|---------------------|
| `PasswordResetSelfRequest` | `"password_reset_self_request"` | After issuing a token + attempting mail dispatch (success or fail). One row per known + active user request. **Not emitted for unknown / inactive emails** (enumeration resistance). |
| `PasswordResetSelfConsume` | `"password_reset_self_consume"` | After atomic consume succeeds + sessions revoked. |
| `SessionsRevokedSelf` | `"sessions_revoked_self"` | One row per session revoked via the active-sessions revoke buttons (single, others, all). Also emitted from the corrected `/admin/password_change` for "log me out everywhere else". |
| (none for rejected consume) | — | Rejected consumes (expired, already-used, mismatched policy) emit `log::info!` only. Rationale §4.3.4. |

### 6.2 Why no `PasswordResetTokenRejected` variant

I considered adding a typed rejection variant. I'm rejecting it for R1:

- The audit table is doctrine-8's forensic trail of **authority mutations + identity changes**. A rejected consume is a non-event (state didn't change). Logging it would give attackers a way to flood the audit table with one row per brute-force attempt.
- An operator who needs to investigate a real attack reaches for the rate-limit logs first. The audit row is for "this is what happened to this account" — and what happened was nothing.
- If R2 / R3 surface a need for attempted-action visibility (e.g. "this account was hammered for 4 hours straight"), revisit by adding it as a counter on `rustio_password_reset_tokens` rather than a typed audit event.

### 6.3 `metadata` JSONB shape

For `PasswordResetSelfRequest`:

```json
{
  "token_fingerprint": "<token:…a3f9c1b2>",
  "email_send_status": "sent",
  "requested_ip": "198.51.100.42",
  "requested_user_agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) …",
  "expires_at": "2026-05-09T13:00:00Z"
}
```

For `PasswordResetSelfConsume`:

```json
{
  "token_fingerprint": "<token:…a3f9c1b2>",
  "invalidated_session_count": 3,
  "user_agent": "Mozilla/5.0 …",
  "ip": "198.51.100.42"
}
```

**`token_fingerprint` is the redacted form, not the plaintext.** The fingerprint comes from `audit::redact_token(token)` and lets an operator pivot the request → consume rows without disclosing the token to anyone reading the audit table later. Property test asserts no 4-char substring of the plaintext appears in the fingerprint output.

### 6.4 `PasswordChangedSelf` — new variant?

The currently-typed surface has no dedicated variant for "user changed their own password while authenticated" (`/admin/password_change`). Today that flow writes nothing (the existing handler doesn't call `audit::record`). R1 has a choice:

- **Option A** — add `AuditEvent::PasswordChangedSelf`. Variant name implies authoritative semantics; matches the doctrine-7 surface on `/admin/account/sessions` ("here's what happened to your account").
- **Option B** — emit `AuditEvent::PasswordResetSelfConsume` with `metadata.flow = "self_change"`. Re-uses an existing variant; loses some tokenisation precision.

**Decision: Option A.** Add `PasswordChangedSelf → "password_changed_self"`. Reason: SIEM tokenisation; doctrine-18 prefers explicit variants over polymorphic metadata; cost is one variant.

This is the **only** new `AuditEvent` variant R1 introduces.

### 6.5 `AuditEvent` visibility transition

R0 ships `pub(crate) enum AuditEvent`. R1 promotes it to `pub`:

- Public API change is additive (callers were already matching on `action_type` strings).
- 0.5.x is the right release for promotion: R1 is the first feature to *systematically* use the enum; we want external consumers to be able to type-check matches by then.
- The variants `MfaEnabled` / `MfaDisabled` etc. that don't ship until R3 stay public-but-unused — that's fine; the type surface is the public commitment, not the call-site coverage. CHANGELOG lists each variant on the release where its emission lands.

---

## 7. Rate-limit strategy

R0 ships a single global `RateLimiter` mounted as middleware. R1 will not abuse the global limiter for the recovery surface — abusing the request endpoint shouldn't lock a legitimate user out of their dashboard.

**Approach:** R1 instantiates **two named buckets**, kept in `Arc<RateLimiter>` fields on the recovery context:

| Bucket | Default | Tunable via | Applied to |
|--------|---------|-------------|------------|
| `forgot_password_request` | 5 req / 15 min / IP | `RecoveryPolicy::request_rate_limit(...)` | `POST /admin/forgot-password` only |
| `reset_password_consume` | 10 req / 5 min / IP | `RecoveryPolicy::consume_rate_limit(...)` | `POST /admin/reset-password/<token>` only |

The buckets are checked **inline** at the top of each handler via the existing `RateLimiter::allow(key)` API; no new middleware shape. On exceedance, the handler returns the standard uniform response (for the request endpoint) or "expired or already used" (for the consume endpoint) — **never** "rate limit exceeded". Disclosure rule §2.3.

The IP key derivation reuses the global limiter's logic (X-Forwarded-For first hop, falling back to "anon"). One small helper extracted from `middleware/rate_limit.rs` so the recovery surface and the global middleware stay in sync.

In-memory only; sufficient for single-node deployments. Multi-node deployments redirect to a Redis-backed limiter via a future `RecoveryPolicy::custom_limiter(...)` hook — out of scope for R1.

### 7.1 Why not "scoped middleware"?

The existing `Router::middleware(...)` API is global. Adding per-route middleware to plumb a scoped `RateLimiter` would be a framework-wide refactor for one feature. The inline check is three lines; doctrine "don't add abstractions beyond what the task requires" applies.

If R2's admin-driven reset, R3's MFA disable, and R4's CLI emergency-access endpoints all want their own buckets, that's the right time to introduce per-route middleware. R1 doesn't.

---

## 8. UX doctrine

### 8.1 Visual register

The reset surfaces extend the existing `admin.css` token system. **No new design tokens.** Both pages render inside the framework's auth-card chassis (the same shell as `/admin/login`):

- Page background: `--rio-bg`.
- Card surface: `--rio-surface`. Border `--rio-border`. Shadow per `admin.css` defaults.
- Heading: Geist 600/24px (matches login).
- Body text: Geist 400/15px.
- Single primary CTA: `--rio-accent` teal-emerald.
- No illustrations. No marketing copy. No "✨ Forgot your password? ✨". Plain, declarative.

The tone is **operational**, not consumer. Reads like the GitHub password-reset flow, not Auth0's.

### 8.2 Page inventory

| Path | Method | Purpose |
|------|--------|---------|
| `/admin/forgot-password` | GET | Single-input form: `email`. Submit button "Send sign-in link". |
| `/admin/forgot-password` | POST | Issue token, dispatch email, redirect 303 to `/admin/forgot-password/sent`. |
| `/admin/forgot-password/sent` | GET | Static "if that email has an account, we just sent a link" card. No form. |
| `/admin/reset-password/<token>` | GET | Render new-password form OR "this link has expired or already been used" card, atomically resolved on landing. |
| `/admin/reset-password/<token>` | POST | Validate + atomic consume + invalidate sessions + redirect 303 to `/admin/login?password_reset=success`. |
| `/admin/account/sessions/<id>/revoke` | POST | Single-session revoke (rejected if equal to current). |
| `/admin/account/sessions/revoke-others` | POST | "Sign out of every other device". |
| `/admin/account/sessions/revoke-all` | POST | "Sign out everywhere" — clears cookie + redirects to `/admin/login`. |

### 8.3 Templates

Two new files, embedded via `templates.rs`:

- `admin/forgot_password.html` — extends `_base.html`'s auth-card block.
- `admin/reset_password.html` — same.

Plus one shared partial extracted from `login.html` for the auth-card shell if the duplication exceeds ~30 lines (judgment call at implementation time).

The active-sessions page (`account_sessions.html`) gains three buttons but does not need a new template; it gets edited in place.

### 8.4 Login page touchups

`/admin/login` gains:

1. A `Forgot your password?` link below the password input (not above, not next to — below, in muted text).
2. When the URL carries `?password_reset=success`, render a one-line success banner above the form: "Password updated. Please sign in."

That's it. No card redesign. No animation.

### 8.5 Page copy (LOCKED)

#### `/admin/forgot-password`

> **Reset your password**
>
> Enter the email address on your account. We'll send you a sign-in link.
>
> [email input]
>
> [Send sign-in link]
>
> Already know your password? [Sign in]

#### `/admin/forgot-password/sent`

> **Check your email**
>
> If that email address has an account, we just sent a sign-in link to it.
>
> The link expires in 1 hour and can only be used once. If you don't see the email within a few minutes, check your spam folder, then try again.
>
> If your email account may be compromised, contact your administrator instead.
>
> [Back to sign in]

#### `/admin/reset-password/<token>` (valid token)

> **Set a new password**
>
> Enter a new password. After saving, you'll be signed out everywhere and need to sign in again with the new password.
>
> [new password]
> [confirm new password]
>
> Password requirements: at least *N* characters (where *N* is whatever the configured policy's `min_length()` returns; the form template renders the live value). The framework default is 10; your administrator may have configured a higher floor.
> *(R1 — the only rule is min length, set by `Admin::password_policy(...)`. R3 may add complexity.)*
>
> [Save new password]

#### `/admin/reset-password/<token>` (invalid / expired / consumed)

> **This link is no longer valid**
>
> Reset links expire 1 hour after they're sent and can only be used once.
>
> [Request a new link]

### 8.6 What the visual register MUST NOT do

- No emojis in any rendered string.
- No animated illustrations.
- No marketing-style gradients.
- No "Welcome back" copy.
- No floating "card on a wash of pastel" composition. The card is on `--rio-bg`, the same chassis as login.
- No "two-factor coming soon!" promotional banner. The login page may eventually show R3 status; the reset pages do not.

---

## 9. Schema / migration plan

R1 adds **one new table** and **one column-set delta on `rustio_users`**. Both go through the framework's idiomatic idempotent path: a function-per-migration in the responsible Rust module, called from `auth::init_tables` at boot, NOT through the file-based `migrations/000N_…sql` runner.

### 9.1 New table — `rustio_password_reset_tokens`

```sql
CREATE TABLE IF NOT EXISTS rustio_password_reset_tokens (
    id                    BIGSERIAL   PRIMARY KEY,
    user_id               BIGINT      NOT NULL REFERENCES rustio_users(id) ON DELETE CASCADE,
    token_hash            TEXT        NOT NULL,
    requested_ip          TEXT,
    requested_user_agent  TEXT,
    requested_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at            TIMESTAMPTZ NOT NULL,
    consumed_at           TIMESTAMPTZ,
    mail_status           TEXT        NOT NULL DEFAULT 'pending'
                          CHECK (mail_status IN ('pending', 'sent', 'failed')),
    correlation_id        TEXT
);

-- Active-token lookup is the hot read path; partial unique index
-- guarantees the consume statement can use it without scanning
-- consumed/expired rows.
CREATE UNIQUE INDEX IF NOT EXISTS rustio_password_reset_tokens_active_uq
    ON rustio_password_reset_tokens (token_hash)
    WHERE consumed_at IS NULL;

CREATE INDEX IF NOT EXISTS rustio_password_reset_tokens_user_idx
    ON rustio_password_reset_tokens (user_id);

CREATE INDEX IF NOT EXISTS rustio_password_reset_tokens_expires_idx
    ON rustio_password_reset_tokens (expires_at)
    WHERE consumed_at IS NULL;
```

**Notes:**

- `token_hash` is SHA-256(token) URL-safe-b64; the plaintext token never lands in this table.
- `mail_status` values are `'pending'` (just-inserted), `'sent'` (Mailer returned Ok), `'failed'` (Mailer returned Err). The state evolves in the same handler; one row per request.
- `correlation_id` mirrors the audit row's correlation_id so an operator can pivot from token row → audit chain.
- No `revoked_at` column. A token's two terminal states are "consumed" and "expired"; we don't need an admin-revoke variant for R1. (R2 may add `revoked_at` to support an admin-driven "kill any pending reset for this user" button. Schema delta is additive.)

### 9.2 `rustio_users` deltas

```sql
ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS must_change_password BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS password_changed_at  TIMESTAMPTZ;
```

- `must_change_password` is the R2 + R4 "admin-driven reset forces a change on next login" surface. R1 introduces the column because `auth::set_password` will populate the timestamp; R1 doesn't read `must_change_password` (R2 enforces it). Adding the column now keeps R2's commit set narrow.
- `password_changed_at` is updated by every `set_password` call (existing + new R1 paths). The `/admin/account/sessions` page renders "Password last changed: 2 days ago" once this column is wired.

### 9.3 Migration function shape

New file: `crates/rustio-admin/src/auth/recovery.rs` (the R1 module — see §10). Inside it:

```rust
pub(crate) async fn init_recovery_tables(db: &Db) -> Result<()> {
    // CREATE TABLE rustio_password_reset_tokens + indexes
}

pub(crate) async fn migrate_user_recovery_schema(db: &Db) -> Result<()> {
    // ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS must_change_password ...
    // ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS password_changed_at ...
}
```

`auth::init_tables` calls both in order, after the existing `init_session_tables` calls. Idempotent. Safe on every boot.

### 9.4 Backfill

- `must_change_password` defaults to `FALSE` for existing rows — no backfill needed.
- `password_changed_at` is `NULL` for pre-R1 users. The active-sessions page displays "(unknown)" or omits the row when the column is NULL. Future password changes populate it.

### 9.5 Downgrade story

If 0.5.0 ships and a project needs to roll back to 0.4.x:

- `rustio_password_reset_tokens` is unreferenced by 0.4.x code; can be dropped or left in place.
- `must_change_password` and `password_changed_at` columns are unreferenced by 0.4.x code; left in place is fine.
- No backwards-incompatible read or write; rollback is data-safe.

---

## 10. Module + types layout

New module: `auth::recovery`. Path: `crates/rustio-admin/src/auth/recovery.rs`.

### 10.1 Public surface

```rust
// re-exported from auth::mod.rs
pub use recovery::{
    init_recovery_tables,
    migrate_user_recovery_schema,
    purge_expired_reset_tokens,
    PasswordPolicy, DefaultPasswordPolicy, PasswordPolicyError,
    RecoveryPolicy, DefaultRecoveryPolicy,
};
```

### 10.2 Module shape

```rust
//! Self-service password recovery (R1).
//!
//! See `DESIGN_RECOVERY.md` for the contract this module implements.

use crate::auth::{invalidate_sessions, set_password, SessionInvalidationReason, SessionTarget};
use crate::email::{Mail, MailerError, SharedMailer};
use crate::orm::Db;
// …

pub trait PasswordPolicy: Send + Sync {
    fn validate(&self, candidate: &str) -> std::result::Result<(), PasswordPolicyError>;
    fn min_length(&self) -> usize;
}

pub struct DefaultPasswordPolicy { min_len: usize }
impl PasswordPolicy for DefaultPasswordPolicy { /* min_len check, no complexity rules */ }

pub trait RecoveryPolicy: Send + Sync {
    fn reset_token_ttl(&self) -> chrono::Duration;
    fn request_rate_limit(&self) -> (u32, std::time::Duration);
    fn consume_rate_limit(&self) -> (u32, std::time::Duration);
    fn strict_mailer_required(&self) -> bool;
    // Provided default delegates to `derive_public_site_url` (see §12.3).
    fn public_site_url(&self, req: &Request) -> Option<String> { … }
}

pub struct DefaultRecoveryPolicy {
    pub reset_token_ttl: chrono::Duration,
    pub request_rate_limit: (u32, std::time::Duration),
    pub consume_rate_limit: (u32, std::time::Duration),
    pub strict_mailer_required: bool,
}

impl DefaultRecoveryPolicy {
    pub fn new() -> Self { /* TTL 1h, request 5/15min, consume 10/5min, strict off */ }
    pub fn with_reset_token_ttl(self, ttl: chrono::Duration) -> Self;
    pub fn with_request_rate_limit(self, capacity: u32, window: std::time::Duration) -> Self;
    pub fn with_consume_rate_limit(self, capacity: u32, window: std::time::Duration) -> Self;
    pub fn with_strict_mailer_required(self, required: bool) -> Self;
}

impl RecoveryPolicy for DefaultRecoveryPolicy { /* fields → trait getters */ }
```

`SharedRecoveryPolicy = Arc<dyn RecoveryPolicy>` mirrors
`SharedPasswordPolicy` / `SharedMailer` so the field on `Admin`
clones cheaply and stays trivially Send + Sync.

// the actual flow functions
pub(crate) async fn issue_reset_token(...) -> Result<()>;
pub(crate) async fn consume_reset_token(...) -> Result<ConsumeOutcome>;
pub(crate) async fn purge_expired_reset_tokens(db: &Db) -> Result<u64>;
```

### 10.3 New `Admin` builder methods

```rust
impl Admin {
    pub fn mailer(mut self, mailer: SharedMailer) -> Self { ... }
    pub fn password_policy(mut self, policy: Arc<dyn PasswordPolicy>) -> Self { ... }
    pub fn recovery_policy(mut self, policy: Arc<dyn RecoveryPolicy>) -> Self { ... }
}
```

`Admin` gains three new fields: `mailer: Option<SharedMailer>`, `password_policy: Arc<dyn PasswordPolicy>`, `recovery_policy: Arc<dyn RecoveryPolicy>`. Defaults: `LogMailer`, `DefaultPasswordPolicy { min_len: 12 }`, `DefaultRecoveryPolicy`.

The `Admin::mailer(...)` method finally lands the API the design docs already advertised; this corrects the documented-but-unimplemented gap discovered during the audit.

### 10.4 New handlers module

`crates/rustio-admin/src/admin/recovery_handlers.rs` (new file). Mirrors the shape of `handlers.rs` but its handlers receive `(ctx, request)` only — no `Identity`, since the routes are unauthenticated.

### 10.5 Routes file

`admin/routes.rs` gains a `register_recovery_routes(...)` block, called BEFORE the `/admin/login` registration so route precedence stays explicit. The block registers:

```text
GET  /admin/forgot-password
POST /admin/forgot-password
GET  /admin/forgot-password/sent
GET  /admin/reset-password/:token
POST /admin/reset-password/:token

POST /admin/account/sessions/:id/revoke
POST /admin/account/sessions/revoke-others
POST /admin/account/sessions/revoke-all
```

The `:token` segment is treated as opaque — no length / charset validation in the router; the consume path's `token_hash` lookup resolves it.

---

## 11. Routes table (reference)

| Method | Path | Auth required | CSRF | Rate-limit | Purpose |
|--------|------|---------------|------|------------|---------|
| GET | `/admin/forgot-password` | no | (set cookie only) | (none — read) | Render request form |
| POST | `/admin/forgot-password` | no | yes | `forgot_password_request` (5/15min) | Issue token + dispatch email |
| GET | `/admin/forgot-password/sent` | no | (set cookie only) | (none — read) | Static confirmation card |
| GET | `/admin/reset-password/:token` | no | (set cookie only) | (none — read) | Render new-password form OR error card |
| POST | `/admin/reset-password/:token` | no | yes | `reset_password_consume` (10/5min) | Atomic consume + invalidate sessions |
| POST | `/admin/account/sessions/:id/revoke` | yes (User+) | yes | (global limiter) | Revoke one session |
| POST | `/admin/account/sessions/revoke-others` | yes (User+) | yes | (global limiter) | Revoke every session except current |
| POST | `/admin/account/sessions/revoke-all` | yes (User+) | yes | (global limiter) | Revoke every session including current; redirect to login |

CSRF protection works the standard way: a GET on the public form sets the `rustio_csrf` cookie; the POST validates either the `_csrf` field or the `X-CSRF-Token` header. **No exemption for the forgot-password POST** — even unauthenticated users get a CSRF cookie set on the GET. (Defends against the trivial cross-site form spammer.)

---

## 12. Mailer integration + boot guard

R1 lifts the mailer out of "documented intention" and into the actual `Admin` struct.

### 12.1 Boot-time guard

`Admin::build_for_production()` (or equivalent — see §15.2 for the question) checks: if `mailer` is `None` (still defaulted to `LogMailer`) and `recovery_policy.boot_guard_strict() == true`, refuse to boot with a clear error:

```
configuration error: production deployment must register a real mailer
via Admin::mailer(Arc::new(MyProjectMailer::new(...))). The default
LogMailer is only suitable for dev / CI / testing — recovery emails
will never reach users in production.
```

In dev / test, `LogMailer` is the silent default. The boot guard fires only when the operator opted into a strict mode. The design doc's "production" framing is simpler than reality: the framework has no concept of production vs dev. R1 introduces a `RecoveryPolicy::strict_mailer_required(bool)` flag (default `false`) that the project sets when wiring its real mailer.

### 12.2 Email body construction

The reset email is composed via `Mail::framework_envelope`:

```rust
let when = chrono::Utc::now();
let body = format!(
    "We received a request to sign you back in to {site_header}.\n\
     \n\
     Click the link below to set a new password:\n\
     \n\
     {site_url}/admin/reset-password/{token}\n\
     \n\
     The link expires {ttl_human}. If you didn't request this, you \
     can safely ignore this email.\n",
    site_header = admin.branding().site_header,
    site_url = recovery_policy.public_site_url(req).as_str(),  // §12.3
    token = token,
    ttl_human = "in 1 hour",  // generated from recovery_policy.reset_token_ttl()
);
let mail = Mail::framework_envelope(
    user.email.clone(),
    format!("{} — sign-in link", admin.branding().site_header),
    body,
    &admin.branding().site_header,
    request_ip,
    ua_summary,
    when,
);
```

The framework envelope appends the canonical "When / From IP / Device / If this was not you" footer per `email/mod.rs`.

### 12.3 Site URL derivation

`recovery_policy.public_site_url(req)` resolves the absolute base URL for the reset link. Default implementation (`auth::recovery::derive_public_site_url`): priority-ordered header scan —

1. **RFC 7239 `Forwarded`** — first comma-separated entry's `proto=` + `host=` pair.
2. **`X-Forwarded-Proto` + `X-Forwarded-Host`** — first CSV entry of each. Both required to fall through if either's missing.
3. **`Host`** — fall back with `http://`. (No HTTPS guesswork; the framework refuses to fabricate proto information.)

`proto` is whitelisted to `{http, https}` (case-insensitive); `host` rejects empty / over-long / whitespace / control-character / CRLF inputs. Malformed values fall through to the next source rather than panic.

**Trust boundary (LOCKED).** The default implementation honours these client-supplied inputs in the order above. **The operator's reverse proxy MUST strip incoming versions of these headers before adding its own.** The framework cannot know the deployment topology; if a hostile client can reach the process directly with `Forwarded: …` already set, the reset link in the dispatched email points wherever they ask.

The framework's defences are limited to:

- proto whitelist (no `javascript:` / `file:` / `data:` injection)
- host charset restriction (no `\r\n` header smuggling)
- length cap (253 chars, RFC 1035 hostname max)

**These are not a substitute for proper proxy hygiene.** Projects that can't guarantee proxy hygiene SHOULD override `RecoveryPolicy::public_site_url` to return a fixed string read from project config at startup — that bypasses the entire header chain.

**Fail-loud rule**: if neither the headers nor the override resolves, refuse to issue the token (`Error::Internal`) rather than emit a relative or broken URL. The user sees the uniform response anyway — they don't see the failure — but the audit row gets `metadata.email_send_status = "failed"` and the operator's log carries the error.

**Why no async surface**: `public_site_url` is sync. A future requirement to consult a remote service (e.g. a per-tenant config table) would push the policy trait toward an `async` shape — that's a minor breaking change we'll cross when needed; today the value is derivable from request data without I/O, and the sync trait keeps the call site straightforward.

---

## 13. PasswordPolicy trait

### 13.1 Trait

```rust
pub trait PasswordPolicy: Send + Sync {
    fn validate(&self, candidate: &str) -> std::result::Result<(), PasswordPolicyError>;
    fn min_length(&self) -> usize;
}

#[derive(Debug)]
pub enum PasswordPolicyError {
    TooShort { min: usize, actual: usize },
    Custom(String),  // for project policies that enforce complexity etc.
}

impl std::fmt::Display for PasswordPolicyError { ... }
impl std::error::Error for PasswordPolicyError {}
```

### 13.2 Default

```rust
pub struct DefaultPasswordPolicy {
    pub min_len: usize,  // 10
}
```

R1 ships with `min_len = 10`. Why 10: the secure-by-default baseline that is long enough to defeat trivial brute-force under Argon2id + per-IP rate-limiting (NIST SP 800-63B's recommended length floor is 8, with longer being preferable) without driving operators toward sticky-note workarounds. Production / regulated deployments are encouraged to override to 12+ via `Admin::password_policy(Arc::new(DefaultPasswordPolicy::with_min_len(12)))`; high-sensitivity deployments may want 16+ paired with an organisational complexity rule or a breach blocklist.

The framework deliberately ships **no complexity-class rules** ("must contain a symbol", "must include uppercase") in the default — they demonstrably push humans toward predictable patterns without improving entropy meaningfully (NIST SP 800-63B Appendix A). Projects that need them implement a custom `PasswordPolicy`.

Length is measured in Unicode `char`s (not bytes), so a 10-char password is 10 user-visible characters regardless of UTF-8 width.

The previous `MIN_PASSWORD_LEN = 12` constant in `handlers.rs:850` is removed in commit #11, where the policy becomes the single source of truth for every password write in the framework.

### 13.3 Wiring

- `auth::recovery::consume_reset_token(...)` calls `admin.password_policy().validate(candidate)` before calling `set_password(...)`.
- The corrected `do_password_change(...)` does the same.
- Project override:
  ```rust
  let admin = Admin::new()
      .password_policy(Arc::new(MyComplexityPolicy { min_len: 16, require_digit: true }))
  ```

### 13.4 What R1 deliberately does NOT do

- No password-history enforcement ("can't reuse last 5 passwords"). Adds storage + lookup; no operator has asked for it; ships in a future phase if needed.
- No common-password blocklist. Same reason.
- No keyboard-walk detector. Same reason.
- No "haveibeenpwned" lookup. Privacy + dependency footprint.

R1's policy is the floor. Projects extend.

---

## 14. Existing-handler integration deltas

R1 doesn't touch handler bodies in `admin/builtin.rs` or the per-model CRUD pipeline. It does adjust three call sites:

### 14.1 `auth::set_password`

**Today:**
```rust
pub async fn set_password(db: &Db, user_id: i64, new_password: &str) -> Result<()> {
    let hash = hash_password(new_password)?;
    sqlx::query("UPDATE rustio_users SET password_hash = $1, updated_at = $2 WHERE id = $3")
        .bind(&hash).bind(Utc::now()).bind(user_id).execute(db.pool()).await?;
    Ok(())
}
```

**R1:**
```rust
pub async fn set_password(db: &Db, user_id: i64, new_password: &str) -> Result<()> {
    let hash = hash_password(new_password)?;
    sqlx::query(
        "UPDATE rustio_users
            SET password_hash = $1, password_changed_at = $2, updated_at = $2
          WHERE id = $3"
    )
    .bind(&hash).bind(Utc::now()).bind(user_id).execute(db.pool()).await?;
    Ok(())
}
```

That's the only change to `set_password`. Single-write semantics preserved.

### 14.2 `do_password_change` (authenticated)

**Today (handlers.rs:861):**
```rust
if errors.is_empty() {
    auth::set_password(&ctx.db, user.id, new1).await?;
    // render success
}
```

**R1:**
```rust
if errors.is_empty() {
    // Apply policy (centralized) — replaces the inline MIN_PASSWORD_LEN check above.
    if let Err(e) = ctx.admin.password_policy().validate(new1) {
        // re-render with field error
    }
    auth::set_password(&ctx.db, user.id, new1).await?;

    // Doctrine 22 — log out other devices on password change.
    let cookie_token = req.header("cookie").and_then(auth::session_token_from_cookie);
    let current_session_id = match &cookie_token {
        Some(t) => auth::current_session_id(&ctx.db, t).await?,
        None => None,
    };
    let target = match current_session_id {
        Some(sid) => SessionTarget::UserExceptCurrent { user_id: user.id, current_session_id: sid },
        None      => SessionTarget::User { user_id: user.id },
    };
    let outcome = auth::invalidate_sessions(&ctx.db, target, SessionInvalidationReason::UserRequested).await?;

    // Audit: one PasswordChangedSelf row + one SessionsRevokedSelf per revoked id.
    audit::record(...);
    for sid in outcome.revoked_session_ids { audit::record(...); }

    // render success
}
```

The inline `MIN_PASSWORD_LEN` constant in `handlers.rs` is removed; the policy is the source of truth.

### 14.4 Deferred to R2 — admin-edit form's password field

The admin-facing `/admin/users/:id/edit` form (handler:
`admin/builtin.rs::do_user_edit`, call site at `builtin.rs:317`)
includes a password input. When an administrator submits the form
with a non-empty value, the handler calls `auth::set_password` for
the target user. The path is a doctrine-22 spirit-violation that
predates R0:

- The target user's other sessions are NOT invalidated. They keep
  using the previously-issued cookie until it naturally expires.
- `must_change_password` is NOT set (the column doesn't exist
  before R1 commit #1; even with the column present, the path
  doesn't write it).
- No typed `PasswordResetByOther` audit row is emitted. The
  surrounding handler writes a generic user-update row that
  includes the role / active diff but does not separately mark
  "password was reset".

R1 commit #2 made the drift *measurable* — every call to
`set_password` now stamps `password_changed_at`, so the active-
sessions UI will surface the timestamp regardless of which path
mutated the row. The behavioural drift itself is **deferred to
R2**, where the dedicated `/admin/users/:id/reset-password`
recovery route lands with the correct semantics:

- mandatory reason field;
- generates a temporary password;
- sets `must_change_password = TRUE`;
- calls `invalidate_sessions(SessionTarget::User { user_id },
  SessionInvalidationReason::PasswordResetByOther)`;
- writes one `AuditEvent::PasswordResetByOther` row plus one
  `SessionsRevokedByOther` per revoked session.

**Open R2 design question:** once the dedicated reset route exists,
either remove the password field from the generic edit form
(recommended — the dedicated route is the doctrinally-correct
surface) or route the form's password mutation through the same
recovery pipeline. Either choice is consistent with doctrine 22;
the form-field-removed option is cleaner and avoids two
overlapping admin-driven password mutation paths.

A `TODO(R2)` comment lives at the call site so the deferral
doesn't get lost. R1 does not modify the call.

### 14.3 The active-sessions page revoke buttons

`handlers.rs::show_account_sessions` already renders the page; R1 adds three new handlers next to it:

- `do_revoke_session(ctx, ident, session_id, req)` — single revoke, refusing if `session_id == current`.
- `do_revoke_other_sessions(ctx, ident, req)` — `UserExceptCurrent`.
- `do_revoke_all_sessions(ctx, ident, req)` — `User`, then clear cookie + redirect.

All three: CSRF-validated, audit-logged with `AuditEvent::SessionsRevokedSelf`, route-registered after the existing `GET /admin/account/sessions` line in `admin/routes.rs`.

---

## 15. Test plan

### 15.1 Unit (pure)

- `DefaultPasswordPolicy::validate` — short / boundary / long; non-ASCII; empty.
- `redact_token` already covered in R0; reuse.
- A pure verdict for the consume path: given `(token_status, expires_at, consumed_at, password_policy_result)` → `Outcome` enum (`Accept | Expired | AlreadyConsumed | PolicyRejected(reason))`. Exhaustive match table in tests.
- `RecoveryPolicy::reset_token_ttl` returns `Duration::hours(1)` by default.
- `redact_likely_tokens` (in `email/mod.rs`) — already tested in R0, but add a property test: any 32+ char alnum segment is redacted.
- A property test on the audit `metadata.token_fingerprint`: no 4-char substring of the plaintext token appears in the metadata JSON.
- `correlation_id` flows from request → both audit rows (issue + consume) — covered in R0.

### 15.2 Integration (DB)

- Create user → request reset → row in `rustio_password_reset_tokens` with `mail_status = 'sent'`, `consumed_at IS NULL`, `expires_at ≈ now + 1h`.
- Same email submitted twice in 2 minutes → second submission → row inserted but a `log::info!` line records "rate-limit miss; uniform response returned"; audit row count = 1 (the first); we reject the second at the bucket BEFORE writing.
  - Actually: re-read this. Per §4.2, we apply rate-limit first. If exceeded, return uniform response without inserting a token row at all. So the second submission produces NO token row and NO audit row. The bucket counter increments. Test asserts: token table count = 1, audit row count = 1.
- Submit unknown email → no token row, no audit row, uniform response, log line written.
- Submit inactive user's email → same path as unknown (no row, no audit).
- Consume valid token → password updated, sessions revoked, audit row, redirect to login. New password works.
- Consume already-consumed token → "expired or already used" page, no audit row written.
- Consume expired token (insert with `expires_at = NOW() - 1`) → same.
- Consume two-tab race: two concurrent POSTs against the same valid token → exactly one succeeds; the other gets the "already used" page.
- Multiple tokens per user: request twice in quick succession (across two IPs to bypass rate-limit), both rows exist; consume the older → newer is still valid; consume the newer → "already used" because `RETURNING` empty (sessions already revoked, but the second token row still has `consumed_at IS NULL`. Wait — actually the second token's row stays unconsumed; only `password_hash` updated. The second consume succeeds and revokes again (already-revoked sessions are excluded from the WHERE). This is acceptable behaviour. Test confirms.).
- Mailer transient error: stub a `FailingMailer`. `mail_status = 'failed'`, audit row has `metadata.email_send_status = "failed"`, user sees uniform response.

### 15.3 Schema

- Boot fresh DB → `init_tables` creates `rustio_password_reset_tokens` + adds `must_change_password` + `password_changed_at`.
- Boot against 0.4.0 DB → idempotent ALTERs apply, no errors.
- Re-boot → no errors, no row count changes.

### 15.4 End-to-end (downstream)

The Stockholm POS downstream gets a manual smoke pass before publish:

- Forgot password → request → email arrives in `LogMailer` log → click reset link → set new password → land on login with banner → sign in with new password → confirm. Audit page shows the chain.
- Active-sessions page: open in two browsers → revoke "other" → second browser bounces to login. Revoke "all" → first browser bounces too.
- Production-like config: register a real `Mailer` impl in the downstream → repeat the same flow.

### 15.5 Negative

- Missing CSRF on `POST /admin/forgot-password` → 403.
- Missing CSRF on `POST /admin/reset-password/<token>` → 403.
- Garbage `<token>` → "expired or already used" (NOT a validation error — disclosure rule).
- Token belonging to a since-deleted user → consume returns the "already used" page; `ON DELETE CASCADE` removed the row before we got there.

---

## 16. Implementation phases & atomic-commit plan

Follows the 0.4.0 cycle's discipline: small commits, one concern per commit, `cargo test --workspace` after each risky one. Estimated 12 commits across ~5 working sessions.

| # | Concern | Files |
|---|---------|-------|
| 1 | Schema: `rustio_password_reset_tokens` + `rustio_users` deltas | `auth/recovery.rs` (new), `auth/mod.rs` (init_tables wiring) |
| 2 | `auth::set_password` stamps `password_changed_at` | `auth/users.rs`, tests |
| 3 | `Admin::mailer` builder method + field; default `LogMailer` | `admin/types.rs`, tests |
| 4 | `PasswordPolicy` trait + `DefaultPasswordPolicy` + `Admin::password_policy(...)` | `auth/recovery.rs`, `admin/types.rs`, tests |
| 5 | `RecoveryPolicy` trait + `DefaultRecoveryPolicy` + `Admin::recovery_policy(...)` | `auth/recovery.rs`, `admin/types.rs`, tests |
| 6 | `AuditEvent::PasswordChangedSelf` variant + visibility promote `pub(crate) → pub` | `admin/audit.rs`, tests |
| 7 | `auth::recovery::issue_reset_token` + `consume_reset_token` (plumbing only — no routes) | `auth/recovery.rs`, tests |
| 8 | Templates + handlers for `/admin/forgot-password{,/sent}` + `/admin/reset-password/:token` | `admin/recovery_handlers.rs` (new), `assets/templates/admin/forgot_password.html` (new), `assets/templates/admin/reset_password.html` (new), `templates.rs` (registration) |
| 9 | Route registration + login-page touchups (link + success banner) | `admin/routes.rs`, `assets/templates/admin/login.html` |
| 10 | Active-sessions revoke buttons (handlers + template + routes) | `admin/handlers.rs`, `assets/templates/admin/account_sessions.html`, `admin/routes.rs` |
| 11 | `do_password_change` upgrade: policy + invalidate-others + audit | `admin/handlers.rs` |
| 12 | Reset-token sweeper integration | `auth/recovery.rs`, `background.rs` |
| (docs) | CHANGELOG entry + README pointer to DESIGN_RECOVERY | `CHANGELOG.md`, `README.md` |

Each commit message follows the existing style: `feat(...)`, `fix(...)`, `docs(...)`. `cargo test --workspace` runs after #1 (schema), #7 (plumbing), #8 (handlers), #11 (drift correction), #12 (sweeper). `cargo clippy --workspace --all-targets -- -D warnings` runs at the same gates.

After commit 12, the pre-publish gate runs in full per `working_style.md`:

```
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo publish --dry-run -p rustio-admin-macros
cargo publish --dry-run -p rustio-admin
```

Then a downstream validation pass against the live Stockholm POS DB. Then — and only then — wait for explicit "publish 0.5.0" before `cargo publish`.

---

## 17. Locked decisions (carry from the design doctrine — DO NOT re-litigate)

| Decision | Value | Override path |
|----------|-------|---------------|
| Reset token TTL | **1 hour** | `Admin::recovery_policy(Arc::new(MyPolicy))` returning a different `Duration` from `reset_token_ttl()` |
| Mailer failure user-visible behaviour | **Log + uniform user response.** Audit row carries `metadata.email_send_status = "failed"` | None — this is doctrine 9 (uniform responses defeat enumeration) |
| Default `MfaPolicy` (touches R1 page copy via §8.3 — NO promotional banner) | **Optional.** R1 page copy does not mention MFA at all. R3 wires it. | `Admin::mfa_policy(MfaPolicy::Required)` ships in R3 |
| Default `PasswordPolicy` floor | **`min_len = 10`** (production deployments encouraged to override to 12+; regulated to 16+) | `Admin::password_policy(Arc::new(DefaultPasswordPolicy::with_min_len(N)))` or a custom `PasswordPolicy` impl |
| Default per-IP rate-limit on `POST /admin/forgot-password` | **5 / 15 min** | `RecoveryPolicy::request_rate_limit` override |
| Default per-IP rate-limit on `POST /admin/reset-password/<t>` | **10 / 5 min** | `RecoveryPolicy::consume_rate_limit` override |
| `AuditEvent` visibility | **`pub(crate)` → `pub`** in 0.5.0 | None — public commitment |
| Strict-mailer boot guard | **Off by default**; opt-in via `RecoveryPolicy::strict_mailer_required(true)` | None |
| Auto-create session after successful reset | **No.** User goes through `/admin/login` so MFA (R3+) gets exercised | None |
| Email send-failure exposure | **Audit row only.** No push notification, no operator email | Project-supplied `Mailer` can wrap-and-emit on its own |

---

## 18. Open questions for kickoff (re-confirm before commit #1)

These three were already answered in `DESIGN_SESSIONS.md` and the saved memory; the kickoff confirmation is the gate before any code lands:

1. **Reset token TTL = 1 hour?** ✅ default, overridable.
2. **Mailer failure = log + uniform response?** ✅ doctrine 9.
3. **Default `MfaPolicy = Optional` (touches R1 page copy)?** ✅ R1 page copy contains no MFA mention; that lands in R3.

Plus four newer questions surfaced by the audit that need user sign-off:

4. **`AuditEvent::PasswordChangedSelf` — add new variant for the authenticated `/admin/password_change` flow?** Recommendation: **yes** (§6.4 Option A).
5. **`do_password_change` (existing authenticated handler) gains "log out other devices" by default?** Recommendation: **yes** — fixes a doctrine-22 spirit-violation that's been sitting since 0.3.0; same release as the reset flow keeps the audit story coherent (§5.2).
6. **`Admin::mailer(...)` builder method lands in R1 (currently documented but not implemented)?** Recommendation: **yes** — R1 needs it; can't ship recovery without it.
7. **Strict-mailer boot guard = off by default, opt-in via `RecoveryPolicy::strict_mailer_required(true)`?** Recommendation: **yes** — removes the framework's need to know "what is production"; project owns the call.

---

## 19. What R1 does NOT do (deferred)

- Admin-driven password reset for another user (R2).
- Login throttle / account auto-lock (R2).
- Re-auth wall for sensitive admin actions (R2 / R3).
- TOTP enrollment, backup codes (R3).
- CLI emergency recovery (R4).
- A "your password was changed" notification email — only R2 needs it (admin-driven reset where the user didn't initiate).
- Reset-link IP / device pinning. Out of scope; many users request on phone, click on laptop.
- CAPTCHA. Project-supplied middleware can wrap the route if needed.
- Reset-token rotation (request another link from a "stale link" page). User re-runs the request flow.
- Multi-tenant scoping. Schema is forward-compatible; semantics arrive with the multi-tenancy phase.
- An "all my pending reset tokens" UI. Operators investigate via direct SQL.

---

## 20. Versioning

- New table → minor.
- `AuditEvent` visibility `pub(crate) → pub` → minor (additive public surface).
- New `AuditEvent` variant `PasswordChangedSelf` → minor.
- `set_password` adding `password_changed_at` UPDATE → minor (purely additive write; existing callers see no behaviour change beyond an additional column being populated).
- `do_password_change` adding "log out other devices" → minor (behaviour change visible to end-users, but consistent with doctrine 22; CHANGELOG calls it out under `Behaviour change`).
- `Admin::mailer / password_policy / recovery_policy` builder additions → minor.
- `PasswordPolicy` / `RecoveryPolicy` trait additions → minor.

R1 ships as `0.5.0`. Patch version bumps in the 0.5.x line are reserved for fixes that don't change any of the above semantics.

---

## 21. PR review checklist (R1-specific additions)

In addition to the existing 8-item visual regression checklist + token-disclosure section in `.github/pull_request_template.md`, every R1 PR walks:

- [ ] Grep proof: `revoked_at\s*=` returns only `auth/sessions.rs::invalidate_sessions`.
- [ ] Grep proof: no plaintext token in any template, log statement, or audit summary.
- [ ] Forgot-password flow (manual): unknown email vs known email vs inactive user → uniform response in all three.
- [ ] Reset-password flow (manual): valid token → new password works, all sessions revoked.
- [ ] Reset-password flow (manual): expired / consumed token → "no longer valid" page; no audit row written.
- [ ] Active-sessions revoke flow: single / others / all → expected redirect; current session correctly identified.
- [ ] `do_password_change` (authenticated): old password validates; new password subject to `PasswordPolicy::validate`; other devices logged out.
- [ ] `cargo test --workspace` passes at every commit, not just the tip.
- [ ] CHANGELOG entry placed under `[Unreleased]`, sectioned by `Recovery / Sessions / Audit / Documentation`.
- [ ] `DESIGN_RECOVERY.md` entries updated if any locked decision was amended (decisions only get amended via explicit user signoff in this doc; no silent drift).

---

## 22. What this document does NOT cover

- Admin-driven recovery (R2) — separate doc on R2 kickoff.
- TOTP MFA (R3) — separate doc on R3 kickoff. R3 may amend §13.1 to wire `PasswordPolicy` to a multi-step "first password, then TOTP" sequence; the trait stays as-is.
- CLI emergency recovery (R4) — separate doc on R4 kickoff.
- Multi-tenant recovery scoping — out of scope until the multi-tenancy phase.
- API tokens / service accounts — re-uses the same invalidation engine; recovery vocabulary may extend with a `ServiceAccountKeyRotated` reason in a future phase.
