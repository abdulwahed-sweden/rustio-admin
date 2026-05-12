# Session Architecture

A session is one device-context with one trust level and one
issuance lineage.

This document is the contract for how those properties are
issued, rotated, escalated, revoked, expired, and audited.

Pull request review runs against this document, not only the
diff.

> **Doctrine inheritance**
> Doctrine 3 makes sessions first-class security infrastructure.
> Doctrines 16-22 add device-scoping, trust-escalation rotation,
> centralised invalidation, and rate-limit dimensions.

---

## 1. Purpose

### 1.1 What this governs

- Session issuance and storage shape.
- The Authenticated → Elevated → MfaVerified state machine.
- Trust-escalation token rotation.
- Revocation, including who is allowed to write `revoked_at`.
- Expiry and background sweep.
- Forensic correlation across requests and audit rows.

### 1.2 What this does not cover

- Self-service password reset → `DESIGN_RECOVERY.md`.
- Admin-driven recovery → `DESIGN_R2_ORGANISATIONAL.md`.
- TOTP / backup codes → R3.
- API tokens, WebAuthn, multi-tenant scoping → out of scope; the
  invalidation engine absorbs them when they ship.

### 1.3 Closing principle

Sessions are the core security surface. The contract is the
document; the implementation must round-trip against it.


---

## 2. Invariants

### 2.1 Doctrine inheritance

| Doctrine | Mandate                                                                    |
|----------|----------------------------------------------------------------------------|
| 3        | Sessions are first-class security infrastructure                           |
| 16       | A session is one device-context with one current trust level               |
| 17       | Trust-level escalation rotates the session token                           |
| 18       | Lineage is queryable via `parent_session_id`                               |
| 19       | Hashed-at-rest token storage; cookie carries plaintext                     |
| 20       | Revocation is soft (`revoked_at`); audit precedes cookie clear             |
| 21       | Expiry is hard (14 days from issuance)                                     |
| 22       | `auth::sessions::invalidate_sessions` is the sole writer of `revoked_at`   |

### 2.2 What must never happen

> **Doctrine 22.** Only `auth::sessions::invalidate_sessions` writes `revoked_at`.

The repository carries a grep proof. CI rejects merges that
introduce a second writer.


> **Trust without rotation.** A session row never upgrades its `trust_level` in place.

Every transition mints a fresh row, revokes the parent, and
swaps the cookie atomically.


> **Authenticated request without filter.** No active-session lookup omits `revoked_at IS NULL`.

A revoked row that returns from a SELECT is a Doctrine 22
violation regardless of cookie validity.


> **Plaintext at rest.** The database stores `token_hash` only.

The legacy `token` column persists for the 0.4.0 transition
window; it is dropped after every legacy session has expired.


> **Cross-session MFA carry.** `mfa_verified` is bound to the row that performed the TOTP step.

Sibling sessions of the same user remain at their own trust
level.


> **Lineage cycles.** `parent_session_id` is one-way.

The parent row is always revoked before the child is
observable. A cycle would imply two live sessions sharing one
escalation event.


> **Reset without revocation.** A password reset always invalidates every session for that user before the new credential lands.

The new credential cannot validate against a row issued under
the old one.


---

## 3. Threat model

The session layer defends a known set of adversaries. Each
adversary names what they have, what they cannot get, and the
property that defeats them.

### 3.1 Adversaries

| Adversary | Has | Cannot get | Defeated by |
|---|---|---|---|
| Cookie thief | Plaintext cookie | Database row | Token rotation on every trust escalation; `revoked_at` invalidates the cookie at the index level |
| Post-DB-leak attacker | `token_hash` row | Plaintext cookie | SHA-256 is one-way; the hash cannot mint a cookie |
| Network MITM | TLS-stripped request | Active session | Cookies are HttpOnly + Secure + SameSite=Strict |
| Malicious admin | Admin login + audit visibility | A revocation that bypasses audit | Every revocation runs through `invalidate_sessions`, which always emits a typed `AuditEvent` |
| Escalation-window attacker | A captured pre-elevation token | A request riding the elevation | The pre-elevation row is revoked atomically when the new row is minted |
| Replay attacker | An expired token | An active session | `expires_at < NOW()` collapses the row to Expired on the read path |

### 3.2 Out of scope

- Compromise of the Postgres host or filesystem.
- Compromise of the operator's password manager.
- Side-channel attacks on the SHA-256 implementation.
- Pre-issuance attacks (TLS handshake, browser extensions).

### 3.3 Disclosure asymmetry

> **The cookie alone is not exploitable; the database alone is not exploitable.**

A breach of either side without the other does not yield an
active session. The cryptographic mechanism is in §8.


---

## 4. State machine

The lifecycle below is the canonical session authority flow.

```text
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

### 4.1 Transition invariants

> **Revoked is terminal.**

A `Revoked` row never re-authenticates regardless of cookie
validity.


> **Expired is treated as Revoked at the auth boundary.**

The row may persist for audit; the session does not.


> **Elevated decays.**

`trust_level = elevated` requires `elevated_until > NOW()`;
otherwise the row reads as `authenticated`.


> **MfaVerified is per-row, not per-user.**

A successful TOTP step on this session does not promote
sibling sessions.


> **Lineage is one-way.**

`parent_session_id` always points at a row whose `revoked_at`
is non-NULL with `revoked_reason = 'trust_escalation'`.


---

## 5. Guarantees

The architectural promises the framework keeps regardless of
caller behaviour.

### 5.1 Single-writer invalidation

> **`auth::sessions::invalidate_sessions` is the only function that writes `revoked_at`.**

Every revocation path — logout, admin reset, password change,
self-service revoke, trust escalation — passes through it. The
function is typed: callers supply a `SessionTarget` and a
`SessionInvalidationReason`. The grep proof is in §12.


### 5.2 Trust-escalation token rotation

> **Trust transitions mint a fresh row and revoke the parent atomically.**

A token captured before an elevation cannot ride the elevation.
The lineage is queryable via `parent_session_id`.


### 5.3 Audit emission per revocation

> **Every revocation emits a typed `AuditEvent` before the cookie is cleared.**

Audit precedes the user-observable response. A successful
logout that fails to audit is a bug, not a feature.


### 5.4 Lineage queryability

> **The authority chain of every `MfaVerified` row resolves through `parent_session_id` traversal.**

The audit answer to "which session triggered this MFA
verification?" is one query.


### 5.5 Disclosure asymmetry

> **Cookie leak ⊕ database leak. Neither alone yields an active session.**

The cookie carries plaintext; the database stores
`sha256(plaintext)`. The asymmetry is the operator's primary
defence against opportunistic breaches.


### 5.6 Forensic correlation

> **Every authenticated request carries a UUID v7 `correlation_id` shared with the audit row.**

The correlation chain crosses middleware, handler, and audit
emission. External tooling pivots on the echoed
`x-correlation-id` response header.


### 5.7 Hard expiry

> **No session survives 14 days from issuance.**

`expires_at` is set at creation and never extended. Read-path
purge runs lazily; the background sweeper runs eagerly.


---

## 6. Implementation notes

The sections below are the engineering reference for the
contract above: schema shape (§7), cryptographic mechanism (§8),
issuance and read paths (§9-§10), trust-escalation procedure
(§11), the invalidation engine (§12), expiry and sweep (§13),
the self-service UI (§14), and forensic correlation (§15).

The doctrine-spec frame above is the contract. The sections
below are the reference for implementing or reviewing it.


---

## 7. Session row schema

The `rustio_sessions` row carries a fixed shape. New columns are
minor-version additions; renames or removals are major.

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

## 8. Cookie ↔ database

The cryptographic mechanism behind §3.3.

The cookie carries the **plaintext** token. The database stores
**SHA-256(token)** in `token_hash`. A cookie leak that doesn't
ship the database remains exploitable; a database leak that
doesn't ship the cookie is **not** — the operator's threat
surface narrows.

```text
Cookie:    rustio_session=Mt7…aBc                       (256-bit URL-safe-b64)
DB row:    token_hash = sha256(Mt7…aBc) → URL-safe-b64  (43 chars)
```

Lookup is constant-time at the index level; `token_hash` carries
a unique partial index `(WHERE revoked_at IS NULL AND token_hash IS NOT NULL)`,
so an active session resolves in one index seek.

The legacy plaintext `token` column persists for the 14-day
transition since 0.4.0 ship — pre-0.4.0 sessions had no hash.
After 14 days every legacy session has expired
(`SESSION_LENGTH_DAYS`); a 0.5.x patch can drop the column.


---

## 9. Issuance

`auth::create_session(db, user_id) -> token`:

1. Generate a 256-bit random token (`rand::thread_rng()`).
2. Compute `token_hash = sha256(token)`, URL-safe-base64.
3. INSERT both `token` (PK, legacy) and `token_hash`.
4. Set `expires_at = NOW() + 14d`, `trust_level = 'authenticated'`,
   `revoked_at = NULL`.
5. Return the plaintext token to the caller (which sets the cookie).

R3+ may add `device_id` to the INSERT when a project enables
device recognition.


---

## 10. Reading a session

`auth::identity_from_session(db, cookie_token)`:

1. Compute `token_hash = sha256(cookie_token)`.
2. SELECT WHERE `token_hash = $1 AND revoked_at IS NULL`. If hit
   → the row.
3. **Fallback**, only for the 0.4.0 transition window:
   SELECT WHERE `token = $1 AND token_hash IS NULL AND revoked_at IS NULL`.
4. If `expires_at < NOW()` → delete the row (legacy purge),
   return `None`.
5. Touch `last_seen = NOW()` async (fire-and-forget).
6. Return `Identity` (the user's row joined into the session).

The `revoked_at IS NULL` filter is the single most important
rule — it makes invalidation trustworthy regardless of which
lookup path matched.


---

## 11. Trust-escalation procedure

Doctrine 17. The procedure for `Authenticated → Elevated → MfaVerified`:

1. Mint a fresh row (`session_id_new`, fresh `token_hash`).
2. Set the new row's `parent_session_id = session_id_old`.
3. Revoke the old row via `invalidate_sessions(target=Single,
   reason=TrustEscalation)`.
4. Replace the user's cookie with the new plaintext token.

The lineage chain (`parent_session_id`) is queryable for audit:
"which session triggered this MFA verification?" is one query.


---

## 12. Invalidation engine

Doctrine 22. Exactly one function writes `revoked_at`:
`auth::sessions::invalidate_sessions`. Every other code path
requesting a revoke calls into it with a typed `SessionTarget`
and `SessionInvalidationReason`. PR review enforces:

```sh
grep -rnE "revoked_at\s*=" --include="*.rs" crates/
# Must return only matches inside auth/sessions.rs::invalidate_sessions.
```

### 12.1 `SessionTarget`

| Variant                        | Use case                              |
|--------------------------------|---------------------------------------|
| `User { user_id }`             | Password reset; "log out everywhere"  |
| `UserExceptCurrent { … }`      | "Sign out other devices"              |
| `Single { session_id }`        | Per-device revoke; logout; trust escalation |

### 12.2 `SessionInvalidationReason` ↔ behaviour

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

## 13. Expiration

- Hard TTL: 14 days from issuance (`SESSION_LENGTH_DAYS`).
- Read path: an `expires_at < NOW()` row is treated as Expired
  and deleted lazily.
- Background: `auth::purge_expired_sessions` runs from
  `background::spawn_session_sweeper` and DELETEs rows where
  `expires_at < NOW()`. It does NOT touch revoked-but-not-expired
  rows — those persist until expiry for audit.


---

## 14. Active-sessions UI

`/admin/account/sessions` is doctrine 7's "core security
surface". Every authenticated user can browse their own
sessions. R0 ships read-only; R1 wires:

- `POST /admin/account/sessions/<id>/revoke` (cannot revoke current)
- `POST /admin/account/sessions/revoke-others` → `UserExceptCurrent`
- `POST /admin/account/sessions/revoke-all` → `User` + cookie clear

Each calls into `invalidate_sessions` with
`reason = UserRequested` and writes one audit row per revoked
session.


---

## 15. Forensic chain — `correlation_id`

Doctrine 8. Every HTTP request gets a UUID v7 stamped by the
`correlation_id` middleware (positioned **before** CSRF so even
rejected requests trace). Audit rows written under that request
share the id. `/admin/history/<correlation_id>` will surface the
chain when it ships.

The middleware honours an inbound `x-correlation-id` header when
its shape looks safe (16-64 chars, ASCII alphanumeric + `-_`);
adversarial inputs are replaced. Echoed back in the response
header so external tooling can pivot.


---

## Appendix A. Versioning

- Token-storage shape change → minor (e.g. moving from SHA-256
  to a KDF-based marker would be a 0.x → 0.(x+1)).
- Trust-level enum addition → minor.
- Trust-level removal or semantics change → major.
- `SessionInvalidationReason` variant addition → minor;
  semantics change → major.
- Schema column rename / removal → major.

CHANGELOG calls every session-impacting change out under
`Sessions`. A consumer pinning `rustio-admin = "0.4"` can rely
on the rotation behaviour, the centralised-invalidation
contract, and the `token_hash` column staying in place across
patch releases.


---

## Appendix B. Deferred work

Items shaped by the existing schema; not yet implemented.

- TOTP / backup codes → R3.
- API tokens / service accounts → out of scope until a future
  phase; will reuse the same invalidation engine.
- WebAuthn / passkeys → out of scope.
- Multi-tenant scoping → schema fields are placeholder-ready
  (`device_id`, `metadata` JSONB on audit rows); semantics
  arrive with the multi-tenancy phase.
