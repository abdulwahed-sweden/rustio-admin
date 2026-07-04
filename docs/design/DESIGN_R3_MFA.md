# R3 — TOTP Multi-Factor Authentication

R0 shipped session lifecycle. R1 shipped self-service recovery.
R2 shipped organisational recovery. R3 promotes authentication
from a single factor to two — TOTP plus single-use backup codes.

The substrate is unchanged; the second-factor evidence is the
new contract.

`auth::sessions::invalidate_sessions` remains the sole writer of
`revoked_at` (Doctrine 22). The audit chain still ties through
`correlation_id`. The `mfa_verified` trust level on
`rustio_sessions` already exists. R3 lights up what R0-R2
declared.

Pull request review runs against this document, not only the
diff.

> **Doctrine inheritance**
> Doctrines 3, 8, 11, 17, 18, 22 carry through unchanged. R3
> layers eight new doctrines (D1-D8 below) governing TOTP
> secret encryption, backup-code single-use, replay protection,
> and forward-only MFA promotion.

---

## 1. Purpose

### 1.1 What this governs

R3 governs second-factor authentication initiated by
authenticated users on their own accounts.

- TOTP enrolment, verification, and disable.
- Backup-code generation, consumption, and regeneration.
- The `mfa_verified` trust-level promotion path on login and
  re-auth.
- TOTP secret encryption at rest.
- Forward-only `MfaPolicy::Required` enforcement.

### 1.2 What this does not cover

- WebAuthn / passkeys → out of scope.
- SMS / email second factor → out of scope (intentionally —
  both are weaker than TOTP under SIM-swap and email-account
  compromise).
- Hardware tokens (YubiKey, etc.) → out of scope until WebAuthn.
- Admin-driven MFA disable (`MfaDisabledByOther`) → R4 CLI
  emergency surface; R3 declares the audit variant but does
  not wire a web handler for it.
- Key rotation playbook for `RUSTIO_SECRET_KEY` → separate
  `DESIGN_SECRETS.md` when the first rotation lands.
- Backup-code printing / PDF export → operator-side concern.

### 1.3 Closing principle

R3 promotes authentication from a single factor to two. The
substrate is unchanged. The second-factor evidence — a TOTP
code or a single-use backup code — is the new contract.

### 1.4 Foundations from R0-R2

R3 builds on substrate already shipped:

- `SessionInvalidationReason::{MfaEnabled, MfaDisabled, MfaDisabledByOther}`
  declared in 0.5.0; strings frozen; unused until R3 wires them.
- `AuditEvent::{MfaEnabled, MfaDisabled, MfaDisabledByOther}`
  declared in 0.5.0; `as_str()` values frozen.
- `audit::redact::redact_mfa_secret()` and
  `redact_backup_code()` shipped in 0.4.0.
- `rustio_sessions.trust_level` accepts `mfa_verified` (R0).
- `parent_session_id` enables `MfaVerified` lineage queryability
  per DESIGN_SESSIONS §5.4.
- `RecoveryPolicy::reauth_window` and the `/admin/reauth` flow
  (R2) — extended in R3 to require both factors when MFA is
  enrolled.
- `MfaPolicy` enum surface (R1 declared `Optional` as default;
  R3 wires the routing).
- `must_change_password` interstitial pattern (R2) — mirrored
  by R3's `/admin/mfa/enroll` forward-only interstitial.


---

## 2. Invariants

### 2.1 Doctrine inheritance

R3 inherits the cross-document substrate and adds eight
R3-specific doctrines.

| Source | Decision | Implication for R3 |
|--------|----------|---------------------|
| Carry-through | Doctrine 22 — centralised invalidation | `MfaEnabled` and `MfaDisabled` reasons go through `invalidate_sessions`; the framework's only `revoked_at` writer remains `auth::sessions::invalidate_sessions` |
| Carry-through | Doctrine 17 — trust escalation rotates the session token | Successful TOTP step on `/admin/mfa/verify` mints a fresh `mfa_verified` row, revokes the parent, swaps the cookie atomically |
| Carry-through | Doctrine 11 — never log secrets | TOTP secrets and backup codes route through `redact_mfa_secret` / `redact_backup_code` before any log, summary, or audit row |
| Carry-through | Doctrine 8 — audit by default | Every MFA mutation emits a typed `AuditEvent` |
| **D1** (R3) | TOTP secrets encrypted at rest | Plaintext secret never persisted. `mfa_secret_ciphertext` carries AEAD output keyed by `RUSTIO_SECRET_KEY`. The plaintext exists only in process memory during enrolment + verification |
| **D2** (R3) | Backup codes Argon2id-hashed | The plaintext code is shown ONCE on the enrolment success page; never stored, never logged, never re-shown |
| **D3** (R3) | Backup-code regeneration is atomic | New batch INSERT and old batch DELETE happen in one transaction. From the moment regeneration commits, the old batch is unrecoverable |
| **D4** (R3) | TOTP step replay protection | `mfa_last_used_step` is monotonic per user. A TOTP code from a step ≤ `mfa_last_used_step` is rejected even if cryptographically valid |
| **D5** (R3) | MFA mutations require re-auth | Enabling MFA, disabling MFA, regenerating backup codes all require `elevated_until > NOW()`. A stolen cookie cannot mutate authentication factors |
| **D6** (R3) | MFA policy promotion is forward-only | Switching to `MfaPolicy::Required` does not retroactively reject existing sessions. Existing users without MFA are prompted at next login through `/admin/mfa/enroll` |
| **D7** (R3) | Backup codes are single-use | `used_at` is non-NULL after consumption. Verification queries filter `WHERE used_at IS NULL` |
| **D8** (R3) | Key rotation is staged | Adding a new key version stamps `mfa_secret_key_id` on subsequent rows. Existing rows decrypt against their stamped id. Retiring an old key requires a re-encryption sweep — separate doc when the first rotation lands |

### 2.2 What must never happen

> **Doctrine 22.** Only `auth::sessions::invalidate_sessions` writes `revoked_at`.

R3 inherits the substrate unchanged. `MfaEnabled` and
`MfaDisabled` reasons are passed to `invalidate_sessions`; no
direct UPDATE on `revoked_at` exists in `auth::mfa`.


> **Plaintext TOTP secret at rest.** The database stores ciphertext only.

`mfa_secret_ciphertext` carries `nonce || ciphertext || tag`
under AES-256-GCM. The plaintext secret exists only in process
memory during enrolment + verification.


> **Plaintext backup code at rest.** The database stores Argon2id hashes only.

The plaintext code is rendered ONCE on the enrolment / regeneration
success page. After the response is sent, the plaintext is dropped
from memory; only the hash remains.


> **Replayed TOTP step.** A code from a step ≤ `mfa_last_used_step` is rejected.

Replay protection runs after cryptographic verification. A
network-captured TOTP code cannot ride a second login attempt.


> **MFA mutation without re-auth.** Enrol, disable, and regenerate routes refuse without `elevated_until > NOW()`.

Same gate as R2's destructive admin actions. A stolen cookie
cannot turn off the second factor.


> **Retroactive policy lockout.** `MfaPolicy::Required` does not revoke existing sessions.

Existing sessions remain valid. At next login (or next
authenticated request that triggers `login_guard`), users
without MFA enrolled are redirected to `/admin/mfa/enroll`.


> **Skipped enrolment when policy = Required.** Non-whitelisted routes redirect to the enrolment interstitial.

Whitelist mirrors R2's `must_change_password` whitelist:
`/admin/mfa/enroll`, `/admin/logout`, `/admin/account/sessions`.
The check sits in `login_guard` after the must-change-password
check.


> **Reused backup code.** A code with non-NULL `used_at` is never accepted.

Verification queries filter `WHERE used_at IS NULL`. The
constraint is at the index level.


---

## 3. Threat model

The MFA layer defends a known set of adversaries. Each
adversary names what they have, what they cannot get, and the
property that defeats them.

### 3.1 Adversaries

| Adversary | Has | Cannot get | Defeated by |
|---|---|---|---|
| **TOTP-secret extractor** (post-DB-leak) | `mfa_secret_ciphertext` rows | Plaintext TOTP secret | AES-256-GCM keyed by `RUSTIO_SECRET_KEY`; the database leak alone yields ciphertext, not a working second factor |
| **Backup-code-DB-leak attacker** | `code_hash` rows | Plaintext backup codes | Argon2id (low-memory params); SHA-256 fingerprint reveals nothing reversible |
| **Step-replay attacker** | A network-captured valid TOTP code | A second authenticated login from the same code | `mfa_last_used_step` monotonic; the second use rejects regardless of cryptographic validity |
| **MFA-bypass-via-reset attacker** | Compromised email account; ability to trigger password reset | An MFA-bypassed session | Per Doctrine 9 (DESIGN_RECOVERY): a reset never lifts trust above `Authenticated`. The next route requiring MFA still demands the second factor |
| **MfaPolicy=Required-bypass attacker** | An authenticated session without MFA enrolled (after policy was switched to Required) | Access to any non-whitelisted admin route | `login_guard` redirects to `/admin/mfa/enroll`. The interstitial mirrors `must_change_password`'s forward-only enforcement |
| **MFA-disable-via-stolen-cookie attacker** | A valid session cookie, no password | The ability to disable MFA | Re-auth wall — disable refuses without `elevated_until > NOW()`. Re-auth requires both factors when MFA is enrolled |
| **Key-rotation-window attacker** | Read access to the database during a key rotation | Plaintext secrets across the transition | `mfa_secret_key_id` stamps each row; the framework decrypts against the row's stamped id, never against a "current" key. A retired key version may stay loaded for the duration of a rotation sweep |

### 3.2 Out-of-scope adversaries

- **Physical device theft.** A user with the unlocked authenticator app can complete TOTP. Out of scope; the operator's response is to revoke the session and disable MFA via R4 CLI.
- **Screen recording the QR code at enrolment.** The QR exposes the plaintext secret for ~30 seconds. Mitigations are operational (private screen, no shoulder-surfing). Out of scope for the framework.
- **Side-channel attacks against AES-256-GCM.** Out of scope; the framework uses the standard `aes-gcm` crate's constant-time implementation.
- **Social engineering of backup codes.** A user who reads their backup code aloud over the phone is out of scope.
- **Compromise of the Postgres host or `RUSTIO_SECRET_KEY` simultaneously.** Out of scope; this is the layered-defence assumption.

### 3.3 Disclosure asymmetry

> **The cookie alone is not exploitable. The database alone is not exploitable. The `RUSTIO_SECRET_KEY` alone is not exploitable.**

A breach of any single side without the others does not yield
authenticated MFA-verified access. The operator's primary
defence is layered: TLS protects cookies, AES-256-GCM protects
secrets at rest, Argon2id protects backup codes. The
cryptographic mechanisms are in §8.


---

## 4. Authority flows

R3 ships five authority flows. Each names the actor (always the
user themselves except where noted), the guards that must pass,
and the audit emission.

### 4.1 Enrolment

The user provisions a TOTP secret, scans the QR code, verifies
the first code, and receives backup codes.

```text
   user GET /admin/account/mfa/enroll
                       │
                       ▼
              ┌─────────────────┐
              │ Authority guard │
              │ + re-auth wall  │  (re-auth required: D5)
              │   (password)    │
              └────────┬────────┘
                       │ pass
                       ▼
              ┌─────────────────┐
              │ Generate TOTP   │  20 random bytes
              │   secret        │  base32-encoded for QR
              └────────┬────────┘
                       │
                       ▼
              ┌─────────────────┐
              │ Render QR       │  otpauth:// URL
              │ + manual key    │  shown ONCE
              │ (server keeps   │
              │  secret in      │
              │  session-state) │
              └────────┬────────┘
                       │ user POSTs
                       │ first 6-digit code
                       ▼
              ┌─────────────────┐
              │ Verify TOTP     │  current step or ±1
              │ (RFC 6238)      │
              └────────┬────────┘
                       │ pass
                       ▼
   AES-256-GCM encrypt secret with RUSTIO_SECRET_KEY
   UPDATE rustio_users
     SET mfa_enabled = TRUE,
         mfa_secret_ciphertext = ?,
         mfa_secret_key_id = current_key_version,
         mfa_last_used_step = step_used
                       │
                       ▼
   Generate 8 backup codes (XXXX-XXXX format)
   Argon2id-hash (low-memory) and INSERT into
   rustio_mfa_backup_codes
                       │
                       ▼
   emit AuditEvent::MfaEnabled
   metadata: { backup_codes_count: 8 }
                       │
                       ▼
   Render backup codes ONCE — rest of flow unreachable
   without re-enrolment if the user navigates away
```

The QR code contains an `otpauth://totp/` URL with the issuer
(operator's project name from `Admin::project_name()`), the
account email, and the base32-encoded secret. Standard format;
interoperable with Google Authenticator, 1Password, Authy,
Bitwarden, Aegis, Raivo, etc.

### 4.2 Verification (login second factor)

After password verification, MFA-enrolled users are challenged
on a dedicated page before the session promotes to
`mfa_verified`.

```text
   user POST /admin/login (success on password)
        │
        ▼
   identity.mfa_enabled
        │
        ├── FALSE ──► normal /admin flow
        │              (trust_level = authenticated)
        │
        └── TRUE ───► create pending session row
                      trust_level = authenticated
                      with metadata.awaiting_mfa = TRUE
                      redirect 303 → /admin/mfa/verify
                            │
                            ▼
                    GET /admin/mfa/verify
                    (form: 6-digit code OR
                          `XXXX-XXXX` backup code)
                            │
                            ▼ POST
                    Try TOTP first:
                    - decrypt secret with row's mfa_secret_key_id
                    - verify (current step or ±1)
                    - reject if step ≤ mfa_last_used_step (D4)
                            │
                            ├── valid ──► update mfa_last_used_step
                            │              trust escalation:
                            │              mint new session row,
                            │              parent = pending row,
                            │              trust_level = mfa_verified
                            │              revoke pending via
                            │              invalidate_sessions(Single,
                            │                TrustEscalation)
                            │              swap cookie
                            │              emit AuditEvent::SessionPromoted
                            │              redirect 303 → /admin
                            │
                            └── TOTP fail ──► try backup code:
                                              - normalise input
                                                (strip hyphen, uppercase)
                                              - SELECT ... WHERE used_at IS NULL
                                              - Argon2id verify each candidate
                                              - on match: UPDATE used_at = NOW()
                                                       same trust escalation
                                                       emit MfaCodeConsumed
                                                       (with remaining_codes count)
                                              - no match: uniform 401 retry
```

The pending-session pattern preserves the device-context
(`session_id`, `created_at`, `ip`, `user_agent`) across the
TOTP step. A pending session that does not complete MFA within
its 14-day expiry decays naturally; an explicit `/admin/logout`
on `/admin/mfa/verify` invalidates it.

### 4.3 Disable

The user disables MFA from their account page. Re-auth requires
both factors.

```text
   user GET /admin/account/mfa/disable
                       │
                       ▼
              ┌─────────────────┐
              │ Authority guard │
              │ + re-auth wall  │  re-auth REQUIRES both
              │ (password+TOTP) │  factors when mfa_enabled
              │                 │  per §12.2
              └────────┬────────┘
                       │ pass
                       │ user POSTs confirmation
                       ▼
   UPDATE rustio_users
     SET mfa_enabled = FALSE,
         mfa_secret_ciphertext = NULL,
         mfa_secret_key_id = NULL,
         mfa_last_used_step = NULL
                       │
                       ▼
   DELETE FROM rustio_mfa_backup_codes WHERE user_id = ?
                       │
                       ▼
   invalidate_sessions(
     SessionTarget::User { user_id },
     SessionInvalidationReason::MfaDisabled)
                       │
                       ▼
   emit AuditEvent::MfaDisabled
   metadata: { reason: "self_disabled", … }
                       │
                       ▼
   redirect 303 → /admin/login
   (user re-authenticates with password only)
```

When `MfaPolicy::Required` is active, the disable route returns
403 — a user under a Required policy cannot self-disable.
Disable under Required policy requires R4 emergency CLI.

### 4.4 Backup-code consume

Backup codes are tried as a fallback when the user does not have
their authenticator. The verification path in §4.2 wraps both
TOTP and backup-code attempts.

```text
   user submits XXXX-XXXX on /admin/mfa/verify
                       │
                       ▼
   normalise: strip hyphen, uppercase
                       │
                       ▼
   SELECT id, code_hash
     FROM rustio_mfa_backup_codes
     WHERE user_id = ? AND used_at IS NULL
                       │
                       ▼
   Argon2id verify input against each candidate
   (constant-time iteration; do not break on first match
    to avoid timing leak about candidate ordering)
                       │
                       ▼
              ┌────────┴─────────┐
              │ no match         │ match
              ▼                  ▼
   uniform 401              UPDATE used_at = NOW()
   (no enumeration leak)    WHERE id = matched_id
                                  │
                                  ▼
                            same trust escalation
                            as TOTP path:
                            mint mfa_verified session
                                  │
                                  ▼
                            emit AuditEvent::MfaCodeConsumed
                            metadata: {
                              code_id: matched_id,
                              remaining_codes: <count>
                            }
                                  │
                                  ▼
                            if remaining_codes <= 2,
                            flash warning on /admin
                            ("regenerate backup codes")
```

A code consumes regardless of which TOTP step was last used —
backup codes are an out-of-band path; they are not subject to
the `mfa_last_used_step` replay rule.

### 4.5 Backup-code regeneration

The user generates a new batch. The old batch is destroyed
atomically.

```text
   user POST /admin/account/mfa/regenerate-codes
                       │
                       ▼
              ┌─────────────────┐
              │ Authority guard │
              │ + re-auth wall  │  re-auth REQUIRES both
              │ (password+TOTP) │  factors when mfa_enabled
              └────────┬────────┘
                       │ pass
                       ▼
   BEGIN TRANSACTION;
     DELETE FROM rustio_mfa_backup_codes WHERE user_id = ?;
     INSERT 8 fresh hashed codes;
   COMMIT;
                       │
                       ▼
   emit AuditEvent::BackupCodesRegenerated
   metadata: {
     previous_codes_invalidated: <count>,
     new_codes_count: 8
   }
                       │
                       ▼
   Render new codes ONCE on success page
```

The transaction is the doctrine D3 enforcement point. A reader
of the database mid-transaction sees either all old codes or
all new codes, never a mix.


---

## 5. Guarantees

The architectural promises R3 keeps regardless of caller
behaviour.

### 5.1 Single-writer invalidation

> **Doctrine 22 carries through unchanged.**

Every R3 revocation path passes
`auth::sessions::invalidate_sessions` with `MfaEnabled`,
`MfaDisabled`, or `TrustEscalation`. No direct `revoked_at`
write exists in `auth::mfa`.


### 5.2 At-rest encryption

> **TOTP secrets are encrypted with AES-256-GCM. Backup codes are hashed with Argon2id.**

A database leak alone yields ciphertext and hashes — never
plaintext. The cryptographic mechanism is in §8.


### 5.3 Replay protection

> **A TOTP step ≤ `mfa_last_used_step` is rejected.**

A network-captured TOTP code cannot ride a second login attempt.
The check runs after cryptographic verification.


### 5.4 Re-auth gating

> **Enrol, disable, and regenerate routes refuse without `elevated_until > NOW()`.**

The window is 15 minutes from a successful `/admin/reauth` POST.
For users with MFA enrolled, re-auth requires both factors.


### 5.5 Single-use backup codes

> **A backup code with non-NULL `used_at` is never accepted.**

The constraint is at the index level. Verification queries
filter `WHERE used_at IS NULL`.


### 5.6 Forward-only policy promotion

> **`MfaPolicy::Required` does not revoke existing sessions.**

Existing sessions remain valid. `login_guard` redirects users
without MFA to `/admin/mfa/enroll` at the next request.


### 5.7 Trust-level promotion via token rotation

> **Successful TOTP or backup-code verification mints a fresh `mfa_verified` session row.**

Doctrine 17 unchanged. The pending session is revoked atomically
when the new row is minted.


### 5.8 Audit emission per mutation

> **Every MFA mutation emits a typed `AuditEvent`.**

`MfaEnabled`, `MfaDisabled`, `MfaCodeConsumed`,
`BackupCodesRegenerated` are emitted before any user-observable
response.


---

## 6. Implementation notes

The sections below are the engineering reference for the
contract above: schema deltas (§7), audit event plan (§8),
module + types layout (§9), routes (§10), trait extensions
(§11), existing-handler integration deltas (§12), and the test
plan (§13).

The doctrine-spec frame above is the contract. The sections
below are the reference for implementing or reviewing it.


---

## 7. Schema deltas

R3 adds four columns on `rustio_users` and one new table.
Column names and types are stable across the 0.7.x line.

```sql
-- TOTP secret + replay protection
ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS
    mfa_enabled            BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS
    mfa_secret_ciphertext  BYTEA;       -- nonce || ciphertext || tag
ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS
    mfa_secret_key_id      INT;          -- which RUSTIO_SECRET_KEY version
ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS
    mfa_last_used_step     BIGINT;       -- monotonic; replay protection

-- Backup codes (single-use, Argon2id-hashed)
CREATE TABLE IF NOT EXISTS rustio_mfa_backup_codes (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES rustio_users(id) ON DELETE CASCADE,
    code_hash   TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used_at     TIMESTAMPTZ
);

-- Per-user partial index supports the verification query
CREATE INDEX IF NOT EXISTS rustio_mfa_backup_codes_user_unused_idx
    ON rustio_mfa_backup_codes (user_id)
    WHERE used_at IS NULL;
```

**Backfill:** existing rows get `mfa_enabled = FALSE`,
`mfa_secret_ciphertext = NULL`, `mfa_secret_key_id = NULL`,
`mfa_last_used_step = NULL`. No pre-existing user is auto-enrolled.

**Migration function shape:**

```rust
pub(crate) async fn migrate_user_mfa_schema(db: &Db) -> Result<()> {
    // ALTER TABLE rustio_users ADD COLUMN IF NOT EXISTS …
    // CREATE TABLE IF NOT EXISTS rustio_mfa_backup_codes …
    // CREATE INDEX IF NOT EXISTS …
}
```

Called from `auth::init_tables` after the R2 migration.
Idempotent. Rolling back to R2 is data-safe (the new columns
become unreferenced; the new table is unused; nothing
hard-fails).


---

## 8. Audit event plan

R3 emits **3 already-declared** AuditEvent variants and adds
**2 new variants**:

| Variant | `as_str()` | Status | Wired in R3 commit |
|---------|-----------|--------|---------------------|
| `MfaEnabled` | `"mfa_enabled"` | declared 0.5.0 | enrolment success |
| `MfaDisabled` | `"mfa_disabled"` | declared 0.5.0 | self-disable success |
| `MfaDisabledByOther` | `"mfa_reset_by_other"` | declared 0.5.0 | (deferred to R4 CLI) |
| `MfaCodeConsumed` | `"mfa_code_consumed"` | **NEW in 0.7.0** | backup-code verification |
| `BackupCodesRegenerated` | `"backup_codes_regenerated"` | **NEW in 0.7.0** | regeneration success |

Both new variants are added with `#[non_exhaustive]` ergonomics —
additive, no breaking match-arms for external consumers.

### 8.1 Cryptographic primitives

**TOTP secret encryption:** AES-256-GCM via the `aes-gcm` crate.

- **Key:** 32 raw bytes derived from `RUSTIO_SECRET_KEY` env var
  (32-byte URL-safe-base64 input).
- **Nonce:** 12 random bytes per encryption (per `aes-gcm`
  recommendation).
- **Ciphertext:** the encrypted secret + 16-byte authentication
  tag.
- **Stored layout:** `mfa_secret_ciphertext = nonce || ciphertext_with_tag`.
- **Decryption:** the framework reads `mfa_secret_key_id`, looks
  up the corresponding key (active key by default; key registry
  for staged rotation), splits nonce / ciphertext, decrypts.

If `RUSTIO_SECRET_KEY` is unset and `MfaPolicy != Disabled`, the
framework refuses to boot. The error message is explicit:
`RUSTIO_SECRET_KEY is required when MfaPolicy != Disabled`.

**Backup code hashing:** Argon2id via the `argon2` crate, with
low-memory params:

```rust
Argon2::new(
    Algorithm::Argon2id,
    Version::V0x13,
    Params::new(
        16 * 1024,  // m = 16 MB
        2,          // t = 2 iterations
        1,          // p = 1 lane
        None,       // default 32-byte output
    )?,
)
```

Rationale: backup codes carry ~41 bits of entropy (8 chars from
a 32-symbol alphabet — exclude ambiguous chars 0/O/1/I/L). Full
password-strength Argon2id is unnecessary; low-memory params
balance verification cost across an 8-row scan (one query per
login attempt that tries a backup code).

### 8.2 `metadata` shapes

For `MfaEnabled`:
```json
{
  "backup_codes_count": 8,
  "key_id": 1
}
```

For `MfaDisabled` (self):
```json
{
  "reason": "self_disabled",
  "previous_backup_codes_count": 8
}
```

For `MfaCodeConsumed`:
```json
{
  "code_id": 4231,
  "remaining_codes": 7,
  "via": "login" | "reauth"
}
```

For `BackupCodesRegenerated`:
```json
{
  "previous_codes_invalidated": 6,
  "new_codes_count": 8
}
```

### 8.3 `correlation_id` chains

R3 chains:

- **enrolment**: `MfaEnabled` (single row, the user's POST).
- **verification (TOTP)**: no audit row — TOTP success is part
  of normal login flow; the session-promotion is recorded via
  `parent_session_id` lineage, not a separate audit event.
- **verification (backup code)**: `MfaCodeConsumed` (single row,
  ties via the user's POST `correlation_id`).
- **disable**: `MfaDisabled` → N × `SessionsRevokedByOther` (one
  per revoked session, except the current device which gets a
  fresh login session). All share one `correlation_id`.
- **regenerate**: `BackupCodesRegenerated` (single row, the
  user's POST `correlation_id`).


---

## 9. Module + types layout

### 9.1 New submodule

`crates/rustio-admin/src/auth/mfa/` — sibling to
`auth/recovery_admin.rs`. A directory module rather than a
single file because R3 carries non-trivial cryptographic
helpers that warrant separation.

```text
crates/rustio-admin/src/auth/mfa/
  mod.rs         — public surface re-exports + outcome enums
  totp.rs        — RFC 6238 hand-rolled implementation
  secret.rs      — AES-256-GCM wrap / unwrap helpers
  backup.rs      — Argon2id hashing + single-use consume
  policy.rs      — MfaPolicy routing + login_guard integration
```

Public surface (re-exported from `auth::mfa`):

```rust
// schema migration
pub(crate) async fn migrate_user_mfa_schema(db: &Db) -> Result<()>;

// enrolment
pub(crate) async fn provision_secret(user_id: i64) -> ProvisionedSecret;
pub(crate) async fn confirm_enrolment(
    db: &Db,
    user_id: i64,
    secret: &Secret,
    candidate_code: &str,
    key_id: u32,
) -> Result<EnrolOutcome>;

// verification
pub(crate) async fn verify_totp(
    db: &Db,
    user_id: i64,
    candidate_code: &str,
) -> Result<VerifyOutcome>;
pub(crate) async fn consume_backup_code(
    db: &Db,
    user_id: i64,
    candidate_code: &str,
) -> Result<BackupConsumeOutcome>;

// disable + regenerate
pub(crate) async fn disable_mfa(
    db: &Db,
    user_id: i64,
    actor: AdminActor<'_>,  // for self-disable, actor == target
) -> Result<DisableOutcome>;
pub(crate) async fn regenerate_backup_codes(
    db: &Db,
    user_id: i64,
) -> Result<RegenOutcome>;
```

### 9.2 Outcome enums

All outcome enums follow R1/R2's pattern (typed-info per
variant; handler decides page rendering).

```rust
pub enum EnrolOutcome {
    Enrolled { plain_backup_codes: Vec<String> },  // 8 codes shown ONCE
    InvalidCode,
    AlreadyEnrolled,
}

pub enum VerifyOutcome {
    Verified { step_used: u64 },
    Replay { last_used_step: u64 },
    Invalid,
    NotEnrolled,
}

pub enum BackupConsumeOutcome {
    Consumed { code_id: i64, remaining: u32 },
    Invalid,
    AlreadyUsed,  // unreachable in normal flow (filtered at SELECT)
}

pub enum DisableOutcome {
    Disabled,
    NotEnrolled,
    PolicyRequired,  // refuses self-disable under MfaPolicy::Required
}

pub enum RegenOutcome {
    Regenerated { plain_backup_codes: Vec<String> },
    NotEnrolled,
}
```

### 9.3 New handler module

`crates/rustio-admin/src/admin/mfa_handlers.rs` — wires the
runtime functions to HTTP. 8 handlers (4 routes × GET + POST).

### 9.4 Hand-rolled RFC 6238

`auth::mfa::totp` is ~80 lines of HMAC-SHA1-based generation +
verification. The framework already pulls `sha2` (for session
tokens) and `hmac` (for HMAC primitives elsewhere). No new
external dependency.

Key functions:

```rust
pub fn current_step(now_unix: u64, step_seconds: u64) -> u64;
pub fn generate(secret: &[u8], step: u64) -> u32;  // 6-digit
pub fn verify(
    secret: &[u8],
    candidate: u32,
    now_unix: u64,
    step_seconds: u64,
    skew_steps: u32,  // 1 by default
) -> Option<u64>;  // returns the step used, for replay tracking
```

Tested against the canonical RFC 6238 Appendix B test vectors.


---

## 10. Routes

R3 adds eight routes — four pairs (GET form / POST action).

| Method | Path | Handler | Guard | Re-auth required |
|--------|------|---------|-------|------------------|
| `GET` | `/admin/account/mfa/enroll` | `show_enroll` | `Role::User` | yes (password) |
| `POST` | `/admin/account/mfa/enroll` | `do_enroll` | same | same |
| `GET` | `/admin/mfa/verify` | `show_verify` | login_guard pending-mfa only | n/a |
| `POST` | `/admin/mfa/verify` | `do_verify` | same | n/a |
| `GET` | `/admin/account/mfa/regenerate-codes` | `show_regenerate` | `Role::User` | yes (password + TOTP) |
| `POST` | `/admin/account/mfa/regenerate-codes` | `do_regenerate` | same | same |
| `GET` | `/admin/account/mfa/disable` | `show_disable` | `Role::User` | yes (password + TOTP) |
| `POST` | `/admin/account/mfa/disable` | `do_disable` | same | same |

All POST routes inherit the project's `csrf_protect` middleware
(unchanged from R0 / R1 / R2).

### 10.1 Modifications to existing routes

- **`POST /admin/login`** (R2 handler): on password success,
  check `identity.mfa_enabled`. If TRUE, mint a *pending* session
  row (`trust_level = authenticated`, `metadata.awaiting_mfa = TRUE`),
  redirect to `/admin/mfa/verify`. If FALSE, normal flow
  (current R0 behaviour).
- **`/admin/reauth`** (R2 handler): when `identity.mfa_enabled`
  is TRUE, the form requires both password AND a 6-digit TOTP
  code (or `XXXX-XXXX` backup code). On success, stamp
  `elevated_until = NOW() + reauth_window()` AND
  `trust_level = mfa_verified`. When `mfa_enabled = FALSE`, the
  flow is unchanged (password only).
- **`login_guard`**: extends with the must-MFA check. After the
  must-change-password check (R2):
  - If `MfaPolicy::Required` and `!identity.mfa_enabled` and the
    current path is NOT in the MFA-enrol whitelist
    (`/admin/mfa/enroll`, `/admin/logout`,
    `/admin/account/sessions`), redirect to
    `/admin/mfa/enroll`.

### 10.2 Registration order

R3 routes register between R2's admin-recovery routes and the
`/admin/:admin_name` model wildcards. The
`/admin/mfa/verify` pair sits with the login-flow routes (it is
not under `/admin/account/`); the rest sit under `/admin/account/mfa/`.


---

## 11. Trait extensions

### 11.1 `MfaPolicy` enum (lighting up declared surface)

```rust
#[derive(Debug, Clone, Copy)]
pub enum MfaPolicy {
    Disabled,
    Optional,                       // default
    Required,
    RequiredForRoles(&'static [Role]),
}

impl Default for MfaPolicy {
    fn default() -> Self { Self::Optional }
}
```

The variant has been declared since R1; R3 adds the routing.

### 11.2 `RecoveryPolicy` adds 2 methods

Both have provided defaults so existing impls don't break.

- `fn mfa_step_seconds(&self) -> u64 { 30 }` — TOTP step
  interval.
- `fn mfa_skew_steps(&self) -> u32 { 1 }` — TOTP step skew
  tolerance.

### 11.3 New `Admin` builder method

```rust
impl Admin {
    pub fn require_mfa(self, policy: MfaPolicy) -> Self;
}
```

Wires `MfaPolicy` into the framework's authentication pipeline.
Default behaviour (no `require_mfa` call) is `MfaPolicy::Optional`.

### 11.4 No new traits

R3's behaviour fits in `RecoveryPolicy` + a small `MfaSecretKeyResolver`
trait for staged key rotation. The resolver has a default impl
that always returns the active key — operators who never rotate
the key never override it.

```rust
pub trait MfaSecretKeyResolver: Send + Sync {
    fn active_key_id(&self) -> u32 { 1 }
    fn key_for(&self, key_id: u32) -> Option<&[u8; 32]>;
}
```


---

## 12. Existing-handler integration deltas

### 12.1 Login flow (`do_login` in `admin/handlers.rs`)

Pre-R3 (post-R2):

```rust
// 1. user lookup; 2. lockout check; 3. password verify;
// 4. record_successful_login; 5. create_session; 6. set cookie
```

Post-R3:

```rust
// 1. user lookup; 2. lockout check; 3. password verify;
// 4. record_successful_login;

// 5. R3 — MFA branch
if user.mfa_enabled {
    let pending = auth::create_pending_mfa_session(&ctx.db, user.id).await?;
    cookie::set(ctx, pending.token);  // pending session cookie
    return redirect("/admin/mfa/verify");
}

// 6. (no MFA) — normal session mint
let token = auth::create_session(&ctx.db, user.id).await?;
// ... existing cookie + redirect logic
```

`create_pending_mfa_session` creates a session row with
`trust_level = authenticated` and `metadata.awaiting_mfa = TRUE`.
The login_guard treats pending sessions specially — they only
match the `/admin/mfa/verify` and `/admin/logout` routes.

### 12.2 `/admin/reauth` form

```rust
async fn do_reauth(ctx: &AdminCtx, req: &Request) -> Response {
    // 1. parse password (and TOTP code if mfa_enabled)
    // 2. verify password
    // 3. if user.mfa_enabled, verify TOTP or consume backup code
    // 4. on full success:
    //    UPDATE rustio_sessions
    //      SET elevated_until = NOW() + reauth_window(),
    //          trust_level = if user.mfa_enabled
    //                          { 'mfa_verified' }
    //                        else
    //                          { 'elevated' }
    //      WHERE session_id = current
    // 5. redirect to validated return_to
}
```

### 12.3 `login_guard` adds MFA-required check

After the R2 must-change-password check:

```rust
// R3 — MFA-required gate (forward-only)
let policy = ctx.admin.active_mfa_policy();
let needs_mfa = match policy {
    MfaPolicy::Required => true,
    MfaPolicy::RequiredForRoles(roles) => roles.contains(&ident.role),
    _ => false,
};

if needs_mfa
    && !ident.mfa_enabled
    && !is_mfa_enroll_whitelisted_path(req.path())
{
    return Ok(Guard::Redirect(
        Response::redirect("/admin/mfa/enroll")
    ));
}
```

The whitelist is:

```rust
const MFA_ENROLL_WHITELIST: &[&str] = &[
    "/admin/mfa/enroll",
    "/admin/logout",
    "/admin/account/sessions",
];
```

The `Identity` struct gains `mfa_enabled: bool` (additive;
defaults to FALSE for pre-R3 sessions). Same pattern as R2's
`Identity::must_change_password`.


---

## 13. Test plan

### 13.1 Unit (pure)

- **RFC 6238 test vectors** — the canonical 8-row Appendix B
  test table (HMAC-SHA1 with 30s step, T0 = 0). Tests both
  `generate(secret, step)` and `verify(secret, candidate, …)`.
- **TOTP step skew** — `verify` with `skew_steps = 1` accepts
  current ±1 step but rejects current ±2.
- **Replay detection** — calling `verify` twice with the same
  step value returns `VerifyOutcome::Replay` on the second call.
- **AES-256-GCM round trip** — `secret::wrap` then `secret::unwrap`
  recovers the input. Tampering with the ciphertext rejects
  with auth-tag failure.
- **Argon2id round trip** — `backup::hash` then `backup::verify`
  succeeds for the matching code, fails for a mismatch.
- **Backup code generator** — 8 codes per call, all 8-char
  alphanumeric in the safe alphabet (no 0/O/1/I/L).

### 13.2 Schema migration

- Boot fresh DB → migrations apply, columns + index + table
  appear.
- Boot 0.6.0 DB → idempotent re-application, no errors.
- Re-boot → no errors.

### 13.3 DB integration (testcontainers)

R2's testcontainers suite extends to R3:

- `provision_secret` + `confirm_enrolment` round-trip writes
  ciphertext + first `mfa_last_used_step`, INSERTs 8 backup
  code rows, audit-emits `MfaEnabled`.
- `verify_totp` accepts current step, rejects replay, accepts
  ±1 skew.
- `consume_backup_code` matches one of 8 inserted codes,
  UPDATEs `used_at`, audit-emits `MfaCodeConsumed`.
- `disable_mfa` clears all four columns, DELETEs all backup
  codes, calls `invalidate_sessions(User, MfaDisabled)`,
  audit-emits `MfaDisabled`.
- `regenerate_backup_codes` atomically replaces the batch,
  audit-emits `BackupCodesRegenerated`.
- `RUSTIO_SECRET_KEY` rotation simulation — encrypt with key
  v1, retire to v2, decrypt against the row's stamped
  `mfa_secret_key_id`.

Located under `crates/rustio-admin/tests/integration/` (extends
the existing `integration_recovery.rs` pattern). Gated behind
`--features integration-test`.

### 13.4 End-to-end (downstream validation pass)

Stockholm POS smoke test against the live DB before publish:

- Admin user enrols → scans QR with phone authenticator → enters
  first code → 8 backup codes shown ONCE → MFA-enabled status
  visible on account page.
- Admin signs out → signs in → enters TOTP → arrives at admin.
- Admin re-auths to perform a destructive action → password +
  TOTP both required → action proceeds.
- Admin loses phone → signs in with backup code → backup code
  consumed; account page shows "7 codes remaining".
- Admin regenerates backup codes → old codes rejected; new
  codes work.
- Admin disables MFA (re-auth with both factors) → MFA-disabled;
  next login is password-only.
- Operator switches `MfaPolicy::Required` → existing user
  without MFA is redirected to `/admin/mfa/enroll` at next
  request.


---

## Appendix A. Versioning

R3 ships as `rustio-admin@0.7.0`. Patch releases on the 0.7.x
line are reserved for fixes that don't change semantics.

- New columns on existing table: minor.
- New table: minor.
- New `AuditEvent` variants (`MfaCodeConsumed`,
  `BackupCodesRegenerated`): minor (additive; `#[non_exhaustive]`
  covers).
- New `Identity::mfa_enabled` field: minor (additive;
  defaults to FALSE for pre-R3 sessions).
- New `RecoveryPolicy` methods (`mfa_step_seconds`,
  `mfa_skew_steps`): minor (provided defaults).
- New `Admin::require_mfa(MfaPolicy)` builder: minor.
- New routes (8): minor.
- New `MfaSecretKeyResolver` trait: minor (default impl).
- `do_login` flow change (MFA branch): minor (behaviour change
  visible to end-users — `Behaviour change` CHANGELOG section,
  same as R2's lockout-check treatment).
- `RUSTIO_SECRET_KEY` env var requirement when
  `MfaPolicy != Disabled`: behaviour change; CHANGELOG
  documents the boot-refusal explicitly.
- TOTP step interval / skew change → would be major (operators
  rely on interop with installed authenticator apps).
- Encryption algorithm change → major.
- Backup code shape change → major.


---

## Appendix B. Locked decisions

| Decision | Value | Override |
|----------|-------|----------|
| TOTP step interval | **30 seconds** | none (RFC 6238 standard; interop) |
| TOTP step skew tolerance | **±1 step** | `RecoveryPolicy::mfa_skew_steps` |
| Backup code count per user | **8** | none |
| Backup code shape | **`XXXX-XXXX` (8 alphanumeric, hyphen-separated)** — alphabet excludes 0/O/1/I/L | none |
| Backup code Argon2id params | **m=16MB, t=2, p=1** (low-memory) | none |
| TOTP secret encryption | **AES-256-GCM** | none |
| Encryption key source | **`RUSTIO_SECRET_KEY` env var** (32-byte URL-safe-base64) | `MfaSecretKeyResolver` for staged rotation |
| Boot refusal on missing key | **Yes** when `MfaPolicy != Disabled` | none |
| Re-auth required for MFA mutations | **Yes** (enrol, disable, regenerate) | none (doctrine D5) |
| MFA mutations require both factors at re-auth | **Yes** when `mfa_enabled` | none |
| `MfaPolicy::Required` enforcement | **Forward-only** via `/admin/mfa/enroll` interstitial | none (doctrine D6) |
| `mfa_verified` trust level scope | **Extend re-auth gate**, no new gates | none (doctrine — see §10.1) |
| Verification page | **Dedicated `/admin/mfa/verify`** | none |
| Backup code consume — out-of-band of step replay | **Yes** | none (doctrine — see §4.4) |
| Backup-code regeneration | **Atomic** (DELETE + INSERT in one transaction) | none (doctrine D3) |
| TOTP implementation | **Hand-rolled** RFC 6238 in `auth::mfa::totp` | none (no `totp-rs` dependency added) |


---

## Appendix C. PR review checklist

R3-specific additions, walked alongside the existing visual
regression checklist + token-disclosure section in
`.github/pull_request_template.md`:

- [ ] Grep proof: `revoked_at\s*=` returns only
      `auth/sessions.rs::invalidate_sessions`.
- [ ] Grep proof: no plaintext TOTP secret in any template,
      log statement, or audit summary. The grep for
      `Secret::expose_secret` returns only the verify and
      encrypt sites.
- [ ] Grep proof: no plaintext backup code in any template,
      log statement, or audit summary. The plaintext exists
      only in the response body for the enrolment + regenerate
      success pages.
- [ ] Grep proof: `mfa_secret_ciphertext` only ever appears in
      the framework's encrypt / decrypt boundary; no other
      handler reads or writes it.
- [ ] Manual: enrol → first TOTP code accepts → 8 backup codes
      shown ONCE → leaving the page and returning shows MFA
      enabled, no codes re-shown.
- [ ] Manual: TOTP replay — capture the same code with two
      tabs, submit second after first → second rejects.
- [ ] Manual: backup code consume → single code works once →
      same code submitted again rejects with uniform "invalid
      code".
- [ ] Manual: regenerate codes → old codes rejected; new codes
      work.
- [ ] Manual: disable MFA (re-auth with both factors) →
      mfa_enabled = FALSE; next login is password-only;
      backup_codes table is empty for that user.
- [ ] Manual: `MfaPolicy::Required` → existing user without
      MFA → next request redirects to `/admin/mfa/enroll`.
      Whitelist (`/admin/logout`, `/admin/account/sessions`)
      reachable.
- [ ] Manual: `RUSTIO_SECRET_KEY` unset + `MfaPolicy != Disabled`
      → framework refuses to boot with explicit error message.
- [ ] Manual: re-auth with both factors → `trust_level = mfa_verified`
      stamped on the session row.
- [ ] `cargo test --workspace` passes at every commit.
- [ ] `cargo test --workspace --features integration-test`
      passes (testcontainers Postgres suite includes R3
      coverage).
- [ ] CHANGELOG entry placed under `[Unreleased]`,
      sectioned by `Recovery / Sessions / Audit / MFA / Security
      / Behaviour change / Documentation / Internal`.
- [ ] `DESIGN_R3_MFA.md` entries updated if any locked
      decision was amended during implementation.


---

## Appendix D. Implementation history

*(Populated as commits land. R3 is in active development; this
section will carry the atomic commit plan and any kickoff Q&A
once R3 ships.)*


---

## Appendix E. Deferred work

Items shaped by the R3 substrate; not yet implemented or
explicitly out of scope.

- **WebAuthn / passkeys** — out of scope for the foreseeable
  future. The framework's narrow surface keeps WebAuthn out
  unless a downstream demands it.
- **SMS second factor** — explicitly out of scope. SIM-swap
  attacks have made SMS a weaker factor than TOTP.
- **Email second factor** — explicitly out of scope. Email
  account compromise is the doctrine-9 floor in
  DESIGN_RECOVERY; an email-based factor would re-introduce
  the floor as a ceiling.
- **Hardware tokens (YubiKey, etc.)** — out of scope until
  WebAuthn lands.
- **Admin-driven MFA disable (`MfaDisabledByOther`)** — declared
  in 0.5.0 audit surface; wires up in R4 CLI emergency
  recovery, not in R3 web routes.
- **Key rotation playbook** — separate `DESIGN_SECRETS.md` when
  the first rotation lands. R3 ships the staging mechanism
  (`mfa_secret_key_id` column + `MfaSecretKeyResolver` trait);
  the operator-facing rotation procedure documents the sweep.
- **TOTP step interval projection-time configuration** — locked
  at 30s for interop. A future minor could expose
  `RecoveryPolicy::mfa_step_seconds` if a project produces a
  documented use case; currently no such case exists.
- **Backup code printing / PDF export** — operator-side concern.
  The framework renders the codes on a clean HTML page;
  printing is a browser concern.
- **Trusted-device "remember me"** — out of scope. A trusted
  device that bypasses MFA for N days re-introduces a long-lived
  authentication artefact that the framework's session model
  already provides via `expires_at`. Operators who want
  trusted devices would extend `mfa_verified` session lifetime
  rather than introduce a new mechanism.
- **MFA recovery via email** — explicitly out of scope. A user
  who loses both their authenticator and their backup codes
  recovers via R4 CLI emergency surface or operator-mediated
  process — not via an email reset link, which would re-introduce
  the doctrine-9 floor.
