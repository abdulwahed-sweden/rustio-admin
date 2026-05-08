# Session Architecture

Companion to `DESIGN_SYSTEM.md`. This document is the canonical
reference for how rustio-admin issues, rotates, elevates, revokes,
expires, and forensically tracks user sessions. It is *not* a tutorial
— it is the contract. Pull request review against this doc.

> **Doctrine 3** in `DESIGN_SYSTEM.md` makes sessions first-class
> security infrastructure. Doctrines 16–22 add the device-scoping,
> trust-escalation rotation, centralised invalidation, and rate-limit
> dimensions encoded below.

---

## 1. Session is a device-context, not a "login"

A session is **one device/browser context** with **one current trust
level** and **one issuance lineage**. A user can have many concurrent
sessions; each is independent for audit, revocation, and elevation
purposes.

The `rustio_sessions` row carries:

| Column                 | Meaning                                                 |
|------------------------|---------------------------------------------------------|
| `session_id`           | Stable BIGINT identifier; survives token rotation       |
| `user_id`              | Owner                                                   |
| `token` (legacy)       | Pre-0.4.0 cookie value; transition only                 |
| `token_hash`           | SHA-256 of the cookie token; primary lookup key         |
| `device_id`            | Reserved for future device-recognition; nullable in R0  |
| `trust_level`          | `authenticated` / `elevated` / `mfa_verified`           |
| `elevated_until`       | Re-auth wall expiry (R3+)                               |
| `parent_session_id`    | Lineage anchor — set when this row was minted by escalation |
| `created_at`           | Issuance time                                           |
| `last_seen`            | Touched on every authenticated request                  |
| `expires_at`           | Hard TTL                                                |
| `revoked_at`           | Soft-delete marker; non-NULL → invisible to auth        |
| `revoked_reason`       | Stable string from `SessionInvalidationReason::as_str`  |
| `ip` / `user_agent`    | Issuance-time metadata                                  |

---

## 2. Cookie ↔ database

The cookie carries the **plaintext** token. The database stores
**SHA-256(token)** in `token_hash`. A cookie leak that doesn't ship
the database remains exploitable; a database leak that doesn't ship
the cookie is **not** — the operator's threat surface narrows.

```text
Cookie:    rustio_session=Mt7…aBc                       (256-bit URL-safe-b64)
DB row:    token_hash = sha256(Mt7…aBc) → URL-safe-b64  (43 chars)
```

Lookup is constant-time at the index level; `token_hash` carries a
unique partial index `(WHERE revoked_at IS NULL AND token_hash IS NOT NULL)`
so an active session resolves in one index seek.

The legacy plaintext `token` column persists for the 14-day
transition since 0.4.0 ship — pre-0.4.0 sessions had no hash. After
14 days every legacy session has expired (`SESSION_LENGTH_DAYS`); a
0.5.x patch can drop the column.

---

## 3. Lifecycle — explicit state machine

A session is in exactly one of these states at any time:

```
                    ┌──────────────┐
       login        │              │  expires
   ────────────────►│    Active    ├───────────────►  Expired
                    │              │
                    │ trust_level: │  revoked
                    │ Authenticated│  (logout / admin / reset / etc.)
                    └──┬───────────┘                       │
                       │                                   ▼
        re-auth        │                            ┌──────────┐
   ────────────────────┤                            │ Revoked  │
                       ▼                            └──────────┘
                    ┌──────────────┐
                    │              │  trust expires
                    │   Elevated   ├───────────────►  Active
                    │              │  (`elevated_until` past NOW)
                    └──┬───────────┘
                       │
        TOTP step      │
   ────────────────────┤
                       ▼
                    ┌──────────────┐
                    │              │  trust expires / re-auth needed
                    │ MFA-verified ├───────────────►  Active / Elevated
                    │              │
                    └──────────────┘
```

**Invariants** (enforced by code, asserted by test where possible):

- A session in `Revoked` never re-authenticates regardless of cookie
  validity — every active-session query carries `revoked_at IS NULL`.
- A session in `Expired` is treated identically to `Revoked` for
  authentication decisions; the row may still exist for audit.
- `trust_level = elevated` requires `elevated_until > NOW()` to be
  honored; otherwise the row decays to `authenticated`.
- `trust_level = mfa_verified` is granted only by a successful TOTP
  step **on this session row** (R3+). It does not transfer to
  sibling sessions.
- `parent_session_id` always points at a row whose `revoked_at IS NOT NULL`
  with `revoked_reason = 'trust_escalation'`. Lineage is one-way; no
  cycles.

---

## 4. Trust escalation rotates the session token

Doctrine 17. When a session transitions
`Authenticated → Elevated → MfaVerified`:

1. Mint a fresh row (`session_id_new`, fresh `token_hash`).
2. Set the new row's `parent_session_id = session_id_old`.
3. Revoke the old row via `invalidate_sessions(target=Single,
   reason=TrustEscalation)`.
4. Replace the user's cookie with the new plaintext token.

Rationale: a token captured before the escalation cannot ride the
elevation. The lineage chain (`parent_session_id`) is queryable for
audit ("which session triggered this MFA verification?").

---

## 5. Invalidation is centralised — one writer

Doctrine 22. Exactly one function writes `revoked_at`:
`auth::sessions::invalidate_sessions`. Every other code path requesting
a revoke calls into it with a typed `SessionTarget` and
`SessionInvalidationReason`. PR review enforces:

```sh
grep -rnE "revoked_at\s*=" --include="*.rs" crates/
# Must return only matches inside auth/sessions.rs::invalidate_sessions.
```

### `SessionTarget`

| Variant                        | Use case                              |
|--------------------------------|---------------------------------------|
| `User { user_id }`             | Password reset; "log out everywhere"  |
| `UserExceptCurrent { … }`      | "Sign out other devices"              |
| `Single { session_id }`        | Per-device revoke; logout; trust escalation |

### `SessionInvalidationReason` ↔ behaviour

| Reason                       | Cookie cleared? | Replacement minted? | Audit `action_type`        |
|------------------------------|-----------------|---------------------|-----------------------------|
| `Logout`                     | yes             | no                  | `session_logout`            |
| `Expired`                    | yes             | no                  | (none — sweeper deletes)    |
| `UserRequested`              | depends on target | only for `Single`-current device | `sessions_revoked_self` |
| `AdministrativeRevoke`       | yes             | no                  | `sessions_revoked_by_other` |
| `PasswordReset`              | yes             | no                  | `password_reset_self_consume` |
| `PasswordResetByOther`       | yes             | no                  | `password_reset_by_other`   |
| `MfaEnabled`                 | no (current device kept) | no | `mfa_enabled` |
| `MfaDisabled`                | no              | no                  | `mfa_disabled`              |
| `MfaDisabledByOther`         | yes             | no                  | `mfa_reset_by_other`        |
| `AuthorityEscalation`        | yes             | no                  | (per-handler)               |
| `EmergencyRecovery`          | yes             | no                  | `emergency_recovery`        |
| `TrustEscalation`            | replaced inline | yes (the new row)   | (lineage trace only — no separate row) |

---

## 6. Issuance

Today: `auth::create_session(db, user_id) -> token`. After R0 the
function:

1. Generates a 256-bit random token (`rand::thread_rng()`).
2. Computes `token_hash = sha256(token)`, URL-safe-base64.
3. INSERTs both `token` (PK, legacy) and `token_hash`.
4. Sets `expires_at = NOW() + 14d`, `trust_level = 'authenticated'`,
   `revoked_at = NULL`.
5. Returns the plaintext token to the caller (which sets the cookie).

R3+ may add `device_id` to the INSERT when a project enables
device-recognition.

---

## 7. Reading a session

`auth::identity_from_session(db, cookie_token)`:

1. Compute `token_hash = sha256(cookie_token)`.
2. SELECT WHERE `token_hash = $1 AND revoked_at IS NULL`. If hit →
   the row.
3. **Fallback**, only for the 0.4.0 transition window:
   SELECT WHERE `token = $1 AND token_hash IS NULL AND revoked_at IS NULL`.
4. If `expires_at < NOW()` → delete the row (legacy purge), return
   `None`.
5. Touch `last_seen = NOW()` async (fire-and-forget).
6. Return `Identity` (the user's row joined into the session).

The `revoked_at IS NULL` filter is the single most important rule
— it makes invalidation trustworthy regardless of which lookup path
matched.

---

## 8. Expiration

- Hard TTL: 14 days from issuance (`SESSION_LENGTH_DAYS`).
- Read path: an `expires_at < NOW()` row is treated as Expired and
  deleted lazily.
- Background: `auth::purge_expired_sessions` runs from
  `background::spawn_session_sweeper` and DELETEs rows where
  `expires_at < NOW()`. It does NOT touch revoked-but-not-expired
  rows — those persist until expiry for audit.

---

## 9. The active-sessions UI

`/admin/account/sessions` is doctrine 7's "core security surface".
Every authenticated user can browse their own sessions. R0 ships
read-only; R1 wires:

- `POST /admin/account/sessions/<id>/revoke` (cannot revoke current)
- `POST /admin/account/sessions/revoke-others` → `UserExceptCurrent`
- `POST /admin/account/sessions/revoke-all` → `User` + cookie clear

Each calls into `invalidate_sessions` with
`reason = UserRequested` and writes one audit row per revoked
session.

---

## 10. Forensic chain — `correlation_id`

Doctrine 8. Every HTTP request gets a UUID v7 stamped by the
`correlation_id` middleware (positioned **before** CSRF so even
rejected requests trace). Audit rows written under that request
share the id. Future `/admin/history/<correlation_id>` will surface
the chain.

The middleware honours an inbound `x-correlation-id` header when
its shape looks safe (16–64 chars, ASCII alphanumeric + `-_`);
adversarial inputs are replaced. Echoed back in the response header
so external tooling can pivot.

---

## 11. Versioning

- Token-storage shape change → minor (e.g. moving from SHA-256 to a
  KDF-based marker would be a 0.x → 0.(x+1)).
- Trust-level enum addition → minor.
- Trust-level removal or semantics change → major.
- `SessionInvalidationReason` variant addition → minor; semantics
  change → major.
- Schema column rename / removal → major.

CHANGELOG calls every session-impacting change out under `Sessions`.
A consumer pinning `rustio-admin = "0.4"` can rely on the rotation
behaviour, the centralised-invalidation contract, and the
`token_hash` column staying in place across patch releases.

---

## 12. What this document does NOT cover

- Password reset flow → ships in R1; will land in `DESIGN_RECOVERY.md`.
- TOTP / backup codes → ships in R3.
- API tokens / service accounts → out of scope; will reuse the same
  invalidation engine when they land.
- WebAuthn / passkeys → out of scope.
- Multi-tenant scoping → schema fields are placeholder-ready
  (`device_id`, `metadata` JSONB on audit rows); semantics arrive
  with the multi-tenancy phase.
