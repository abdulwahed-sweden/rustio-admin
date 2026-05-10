# Audit Architecture

An audit row is the durable, redacted record of a single
authority mutation.

This document is the contract for how those rows are typed,
emitted, correlated, redacted, and queried.

Pull request review runs against this document, not only the
diff.

> **Doctrine inheritance**
> Doctrine 8 makes audit logs forensically useful. Doctrine 11
> forbids secrets in logs. Doctrine 18 evolves audit toward
> typed events.

---

## 1. Purpose

### 1.1 What this governs

- The `rustio_admin_actions` row schema and indexes.
- The typed `AuditEvent` enum and its evolution path.
- Redaction helpers and the no-secrets-at-rest rule.
- The forensic chain — `correlation_id`, `session_id`, `metadata`.
- The middleware ordering contract.
- Append-only emission semantics.

### 1.2 What this does not cover

- Cryptographically-signed audit chains (Merkle log) — Appendix B.
- Cross-system trace correlation (OpenTelemetry) — Appendix B.
- Retention and purge policy — operator-owned, Appendix B.
- The SIEM / analytics pipeline that consumes the rows.

### 1.3 Closing principle

An audit row is the durable, redacted record of a single
authority mutation. The contract is the document; the
implementation must round-trip against it.


---

## 2. Invariants

### 2.1 Doctrine inheritance

| Doctrine | Mandate                                                                |
|----------|------------------------------------------------------------------------|
| 8        | Audit logs must be forensically useful                                 |
| 11       | Never log secrets; redaction is type-level                             |
| 18       | Audit events evolve toward typed; `AuditEvent` is the source of truth  |

### 2.2 What must never happen

> **Doctrine 11.** A secret value is never written to an audit row.

Plaintext tokens, passwords, MFA seeds, and backup codes pass
through `audit::redact` before reaching `summary` or `metadata`.


> **Fabricated correlation_id.** The framework never invents an id.

A project that omits the middleware lands NULL `correlation_id`
on its audit rows. A NULL is honest; an invented id is not.


> **Mutated audit row.** No code path runs `UPDATE` or `DELETE` against `rustio_admin_actions` outside operator-owned retention jobs.

Audit rows are append-only at the framework level. Any mutation
inside the framework crate is a Doctrine 8 violation.


> **Untyped `action_type`.** No `INSERT` writes a string that does not correspond to an `AuditEvent` variant.

The drift test in `audit::tests` rejects un-enumerated values.
SIEM consumers tokenise on the enum surface.


> **Disclosed redacted field.** A `before` or `after` payload never carries the un-redacted form of a redacted field.

`before.password_hash`, `before.mfa_secret`, and equivalent keys
are stripped at the audit-emission boundary.


> **Skipped emission.** Every authority mutation emits an audit row.

A handler that mutates user state without emitting is a
Doctrine 8 violation regardless of operator intent.


> **Out-of-order middleware.** `correlation_id` is installed before `csrf_protect`.

A 403 from CSRF rejection must still trace. The framework
neither autocorrects nor warns; PR review enforces the order.


---

## 3. Threat model

The audit layer defends a known set of adversaries. Each
adversary names what they have, what they cannot get, and the
property that defeats them.

### 3.1 Adversaries

| Adversary | Has | Cannot get | Defeated by |
|---|---|---|---|
| Log forger | Database write access | A row that survives PR review | All inserts route through the typed `audit::emit` path; orphan rows fail the drift test |
| Log dropper | Handler-level code | An authority mutation that lands without an audit row | Doctrine 8; PR review against handlers that mutate state without emission |
| Log tamperer | Database `UPDATE` access | A modified row that the framework wrote | Append-only emission; framework crate carries no audit `UPDATE` / `DELETE` |
| Secrets-in-log scanner | Read access to log lines | A plaintext secret | `audit::redact` produces fingerprints, not values; property test asserts no 4-char input substring leaks |
| Unauthorised reader | Read access to `rustio_admin_actions` | The full forensic chain without authority | RBAC on the admin surface; future `/admin/history/<correlation_id>` honours the same role gate |

### 3.2 Out of scope

- Compromise of the Postgres host or filesystem.
- Compromise of the operator's log retention pipeline.
- Side-channel attacks against the SHA-256 fingerprinting helper.
- Adversaries with simultaneous database write access and crate-source modification.


---

## 4. Emission lifecycle

The flow below is the canonical authority chain from inbound
HTTP request to durable audit row.

```text
       HTTP request
            │
            ▼
   ┌──────────────────────────────┐
   │  middleware::correlation_id  │  UUID v7 stamped
   │  (before csrf_protect)       │  honours safe-shape inbound header
   └────────────────┬─────────────┘
                    │
                    ▼
   ┌──────────────────────────────┐
   │  handler                     │  performs authority mutation
   │                              │  builds typed AuditEvent
   └────────────────┬─────────────┘
                    │
                    ▼
   ┌──────────────────────────────┐
   │  audit::emit(...)            │  INSERT into rustio_admin_actions
   │                              │  carries correlation_id, session_id,
   │                              │  metadata, redacted summary
   └────────────────┬─────────────┘
                    │
                    ▼
   ┌──────────────────────────────┐
   │  rustio_admin_actions        │  append-only durable row
   └────────────────┬─────────────┘
                    │
                    ▼
       forensic query
       /admin/history/<correlation_id>
```

### 4.1 Emission invariants

> **`correlation_id` is stamped before CSRF.**

A request rejected at the CSRF boundary still carries an id;
audit rows from middleware-level rejections trace.


> **`correlation_id` is not fabricated.**

If the middleware is absent, the audit row's `correlation_id`
is NULL. The framework never invents.


> **The typed event is built before `INSERT`.**

`audit::emit` accepts an `AuditEvent`; the variant's `as_str()`
populates `action_type`. No call site composes strings.


> **`session_id` is set when an authenticated session performed the action.**

CLI / system actions emit with `session_id = NULL`. The filter
in §10 honours this without a join.


> **No row is mutated post-INSERT.**

The framework crate runs no `UPDATE` or `DELETE` against
`rustio_admin_actions`. Operator retention jobs are
out-of-scope and project-owned.


---

## 5. Guarantees

The architectural promises the framework keeps regardless of
caller behaviour.

### 5.1 Audit-by-default

> **Every authority mutation emits a typed `AuditEvent` row.**

User, group, permission, session, and recovery surfaces all
emit. The canonical example is `auth::sessions::invalidate_sessions`,
which emits before clearing the cookie (see DESIGN_SESSIONS §5.3).


### 5.2 Typed-event evolution

> **`AuditEvent` is the source of truth for `action_type` strings.**

A drift test asserts each variant's `as_str()` is unique and
snake_case ASCII. `pub(crate)` in 0.4.0; `pub` in 0.5.x once the
recovery flows have shaken out the variant set.


### 5.3 No secrets at rest

> **`audit::redact` produces fingerprints, not values.**

Plaintext tokens, passwords, MFA seeds, and backup codes are
redacted at the type level. A property test asserts no 4-char
input substring leaks through `redact_token`.


### 5.4 Forensic correlation

> **Every audit row written under one HTTP request shares one `correlation_id`.**

A future `/admin/history/<correlation_id>` page is a thin
wrapper over a single index-backed query.


### 5.5 Session linkage

> **`session_id` ties every authenticated mutation back to the device-context that performed it.**

CLI and system actions carry `session_id = NULL`; queries that
filter on session boundaries honour this without a join.


### 5.6 Append-only emission

> **No code path inside the framework crate runs `UPDATE` or `DELETE` on `rustio_admin_actions`.**

Tamper resistance comes from the append-only contract. Operator
retention jobs are project-owned and out-of-scope.


### 5.7 Middleware ordering

> **`correlation_id` middleware installs before `csrf_protect`.**

Rejections at the CSRF boundary still carry forensic ids. PR
review enforces the order; the framework does not autocorrect.


---

## 6. Implementation notes

The sections below are the engineering reference for the
contract above: row schema (§7), typed event surface (§8),
redaction helpers (§9), forensic queries (§10), middleware
contract (§11), `metadata` payload shape (§12), and performance
notes (§13).

The doctrine-spec frame above is the contract. The sections
below are the reference for implementing or reviewing it.


---

## 7. Row schema

The `rustio_admin_actions` row carries a fixed shape. New
columns are minor-version additions; renames or removals are
major.

```sql
rustio_admin_actions(
    id              BIGSERIAL,
    user_id         BIGINT,           -- the actor
    action_type     TEXT,             -- AuditEvent::as_str() value
    model_name      TEXT,             -- "User" / "Group" / future "Session" / etc.
    object_id       BIGINT,           -- the target row id
    timestamp       TIMESTAMPTZ,
    ip_address      TEXT,             -- best-effort (x-forwarded-for / x-real-ip)
    summary         TEXT,             -- human-readable diff
    correlation_id  TEXT,             -- per-request UUID v7; ties chains together
    session_id      BIGINT,           -- the session that performed the action
    metadata        JSONB             -- structured before/after; reason; etc.
)
```

Indexes:

- `(model_name, object_id)` — per-object history
- `(timestamp DESC)` — recent-activity feed
- `(correlation_id) WHERE correlation_id IS NOT NULL`
- `(session_id) WHERE session_id IS NOT NULL`


---

## 8. Typed event surface

`pub(crate) enum AuditEvent` enumerates every `action_type` the
framework writes. A drift test in `audit::tests` asserts:

- Each variant's `as_str()` is unique across the enum.
- Each variant's `as_str()` is snake_case ASCII.

Future SIEM integrations and analytics tooling tokenise these
strings, so they are pre-normalised at the type level.

The enum is `pub(crate)` in 0.4.0. Promotion to `pub` is planned
for 0.5.x once the recovery flows have shaken out the variant
set (R1 may add new ones). The breaking-change risk is low —
external consumers can already match on `action_type` strings;
moving to the enum is additive for users who want type-checked
matches.


---

## 9. Redaction helpers

`audit::redact` ships type-level helpers. Every recovery and
authority surface routes secrets through them before they reach
`summary` or `metadata`.

| Helper                 | Returns                              | Use for                          |
|------------------------|--------------------------------------|----------------------------------|
| `redact_password()`    | `"<password>"`                       | log lines & summary text         |
| `redact_token(t)`      | `"<token:…XXXXXXXX>"` (8-char fingerprint) | session cookies, reset tokens, API keys |
| `redact_mfa_secret()`  | `"<mfa-secret>"`                     | TOTP seeds                       |
| `redact_backup_code()` | `"<backup-code>"`                    | one-time codes                   |

The fingerprint variant exists so an operator can pivot two log
lines about the same token without disclosing it. SHA-256-based;
not reversible to the input.

A unit-test property check asserts `redact_token` returns no
4-char substring of the input — catches accidental "show last N
chars" regressions.


---

## 10. Forensic queries

Three columns participate in the forensic chain:

- `correlation_id` — the request boundary. Every audit row
  written under one HTTP request shares this id.
- `session_id` — the session boundary. Every audit row caused
  by one signed-in session shares this id (NULL for CLI /
  system actions).
- `metadata` — JSONB. Carries before/after diff, supplied
  reason, and any per-action structured payload.

Forensic queries land naturally:

```sql
-- "What happened in request 7f4e…?"
SELECT * FROM rustio_admin_actions WHERE correlation_id = '7f4e…';

-- "Did this session perform any privileged action?"
SELECT * FROM rustio_admin_actions WHERE session_id = 42 AND
    action_type IN ('password_reset_by_other', 'mfa_disabled', 'sessions_revoked_by_other');
```

The future `/admin/history/<correlation_id>` page is a thin
wrapper over the first query.


---

## 11. Middleware contract

`middleware::correlation_id` is **mandatory** for any project
that wants forensic chain semantics. Install it **before**
`csrf_protect` so even 403/429 rejections trace:

```rust
Router::new()
    .middleware(middleware::logger)
    .middleware(middleware::correlation_id)   // ← BEFORE csrf_protect
    .middleware(middleware::security_headers)
    .middleware(middleware::csrf_protect)
```

A project that omits the middleware lands NULL `correlation_id`s
in their audit rows. The framework does not silently fabricate
ids — better a missing trace than a fake one.


---

## 12. `metadata` payload

Free-form JSONB at the SQL layer; conventional shape inside the
codebase:

```json
{
  "before": { "role": "staff", "is_active": true },
  "after":  { "role": "supervisor", "is_active": true },
  "reason": "promoted to shift lead per HR",
  "added_groups":   [3, 7],
  "removed_groups": [9]
}
```

R0 ships the column without populating it from the existing
handler call sites (those still use the human-readable `summary`
string). R1 introduces a builder pattern that writes both the
legacy `summary` text and the structured `metadata` JSON for
handlers that want to pivot via the JSONB column.

Reserved keys (do not collide with these in user-supplied
extensions):

- `before`, `after` — the diff
- `reason` — operator-supplied reason text
- `email_send_status` — set by recovery flows when a
  `Mailer.send` failure was non-fatal
- `cli_user` — the operating-system user who ran a Layer-3 CLI
  command


---

## 13. Performance

- `INSERT` is a single statement; no joins, no triggers.
- Reads come in three shapes — recent-activity feed,
  per-object history, per-correlation_id — all index-backed.
- `metadata` JSONB has no GIN index in 0.4.0; add one if a
  project starts running `metadata @> '…'` queries at scale.
  The framework does not.


---

## Appendix A. Versioning

- `AuditEvent` variant addition → minor.
- `AuditEvent` variant removal or `as_str()` rename → major.
- Enum visibility change (`pub(crate)` → `pub`) → minor; the
  string surface was already public.
- Schema column addition → minor; rename or removal → major.
- Index addition → minor; index removal that breaks an
  index-backed query shape → major.
- `correlation_id` format change (UUID v7 → other) → major.
- Reserved-key collision (`before` / `after` / `reason` / etc.)
  is a deprecation cycle; the new key wins after one minor
  version of warnings.

CHANGELOG calls every audit-impacting change out under `Audit`.
A consumer pinning `rustio-admin = "0.4"` can rely on the row
schema, the redaction helper signatures, the middleware order,
and the `correlation_id` UUID v7 shape across patch releases.


---

## Appendix B. Deferred work

Items shaped by the existing schema; not yet implemented.

- Cryptographically-signed audit chain (Merkle log). If a
  regulator demands tamper-evidence, ship a project-level audit
  forwarder and let the framework's row remain the source of
  the structured event.
- Cross-system correlation (OpenTelemetry trace id, etc.).
  `correlation_id` is intentionally rustio-scoped; a project
  that wants OTel can stamp the inbound `traceparent` into the
  inbound `x-correlation-id` header (the framework will accept
  it).
- Retention / purge policy. Operators set their own. The
  framework does not run scheduled deletes.
