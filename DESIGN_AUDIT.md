# Audit Architecture

Companion to `DESIGN_SYSTEM.md` and `DESIGN_SESSIONS.md`. Codifies
the audit log's shape, the typed-event evolution path, the
no-secrets-in-logs rule, and the forensic-chain story.

---

## 1. Doctrine recap

- **Doctrine 8** — audit logs must be forensically useful. Every
  authority mutation, recovery action, and emergency intervention
  writes a row to `rustio_admin_actions`.
- **Doctrine 11** — never log secrets. `audit::redact` ships
  helpers that produce safe placeholders or short fingerprints; no
  recovery surface logs a plaintext token, password, MFA secret, or
  backup code.
- **Doctrine 18** — audit events evolve toward typed. The internal
  `AuditEvent` enum (pub(crate) in 0.4.0) is the canonical source
  of truth for `action_type` strings; the public typed surface
  lands in 0.5.x.

---

## 2. The row shape (0.4.0)

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

## 3. The typed evolution (`AuditEvent` enum)

`pub(crate) enum AuditEvent` enumerates every `action_type` the
framework writes. A drift test in `audit::tests` asserts:

- Each variant's `as_str()` is unique across the enum.
- Each variant's `as_str()` is snake_case ASCII.

Future SIEM integrations and analytics tooling will tokenise these
strings, so they're pre-normalised at the type level.

The enum is `pub(crate)` in 0.4.0. Promotion to `pub` is planned
for 0.5.x once the recovery flows have shaken out the variant set
(R1 may add new ones). The breaking-change risk is low because
external consumers can already match on `action_type` strings;
moving to the enum is additive for users who want type-checked
matches.

---

## 4. The redaction helpers (`audit::redact`)

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

## 5. The forensic chain

Three columns participate:

- `correlation_id` — the request boundary. Every audit row written
  under one HTTP request shares this id.
- `session_id` — the session boundary. Every audit row caused by
  one signed-in session shares this id (NULL for CLI / system actions).
- `metadata` — JSONB. Carries before/after diff, supplied reason,
  and any per-action structured payload.

Forensic queries land naturally:

```sql
-- "What happened in request 7f4e…?"
SELECT * FROM rustio_admin_actions WHERE correlation_id = '7f4e…';

-- "Did this session perform any privileged action?"
SELECT * FROM rustio_admin_actions WHERE session_id = 42 AND
    action_type IN ('password_reset_by_other', 'mfa_disabled', 'sessions_revoked_by_other');
```

The future `/admin/history/<correlation_id>` page is a thin wrapper
over the first query.

---

## 6. The middleware contract

`middleware::correlation_id` is **mandatory** for any project that
wants forensic chain semantics. Install it **before**
`csrf_protect` so even 403/429 rejections trace:

```rust
Router::new()
    .middleware(middleware::logger)
    .middleware(middleware::correlation_id)   // ← BEFORE csrf_protect
    .middleware(middleware::security_headers)
    .middleware(middleware::csrf_protect)
```

A project that omits the middleware lands NULL `correlation_id`s in
their audit rows. The framework does not silently fabricate ids —
better a missing trace than a fake one.

---

## 7. What goes into `metadata`

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

R0 ships the column without populating it from the existing handler
call sites (those still use the human-readable `summary` string). R1
introduces a builder pattern that writes both the legacy `summary`
text and the structured `metadata` JSON for handlers that want to
pivot via the JSONB column.

Reserved keys (do not collide with these in user-supplied
extensions):

- `before`, `after` — the diff
- `reason` — operator-supplied reason text
- `email_send_status` — set by recovery flows when a `Mailer.send`
  failure was non-fatal
- `cli_user` — the operating-system user who ran a Layer-3 CLI
  command

---

## 8. Performance

- INSERT is a single statement; no joins, no triggers.
- Reads come in three shapes (`recent`, `for_object`,
  per-correlation_id) — all index-backed.
- `metadata` JSONB has no GIN index in 0.4.0; add one if a project
  starts running `metadata @> '…'` queries at scale. The framework
  does not.

---

## 9. Out of scope (today)

- Append-only / cryptographically-signed audit chain (Merkle log).
  If a regulator demands tamper-evidence, ship a project-level
  audit forwarder and let the framework's row remain the source of
  the structured event.
- Cross-system correlation (OpenTelemetry trace id, etc.).
  `correlation_id` is intentionally rustio-scoped; a project that
  wants OTel can stamp the inbound `traceparent` into the inbound
  `x-correlation-id` header (the framework will accept it).
- Retention / purge policy. Operators set their own.
