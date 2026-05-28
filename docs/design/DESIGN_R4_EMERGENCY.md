# R4 — CLI Emergency Recovery

R0 shipped session lifecycle. R1 shipped self-service recovery.
R2 shipped organisational recovery (admin acting on other
accounts via the web). R3 shipped TOTP MFA. R4 is the
emergency tier: the path that opens when every in-band recovery
path is closed.

A founder who forgot her password AND lost her TOTP device AND
is the only Administrator on the deployment cannot recover via
R1 (no working password), R2 (no admin to act for her), or R3
(no MFA factor). She still has shell access to the machine
running the framework. The CLI is the last-mile recovery
surface that uses that shell access deliberately, audibly,
auditably.

Pull request review runs against this document, not only the
diff.

> **Doctrine inheritance**
> Doctrines 3, 8, 11, 17, 18, 22 carry through unchanged. D1-D8
> from R3 carry through unchanged. R4 layers four new doctrines
> (D9-D12 below) governing CLI-actor identity, confirmation
> discipline, irreversibility, and audit emission scope.

---

## 1. Purpose

### 1.1 What this governs

R4 governs recovery operations initiated from a shell with
`DATABASE_URL` already configured. The CLI binary is the surface.

- Setting a new password for any user.
- Unlocking a throttled account.
- Disabling MFA on any user.
- Promoting a user to a higher role.
- Issuing a single-use emergency reset URL (mailer-bypass).

### 1.2 What this does not cover

- New schema columns — R4 ships zero migrations.
- New HTTP routes — R4 ships zero web handlers.
- A daemon mode / interactive REPL — every command is one-shot.
- Granular permission delegation — possession of `DATABASE_URL`
  is the authority floor. The host's OS-level access controls
  are the gate; R4 does not re-implement them.
- Undo of emergency operations. Reversal is a fresh emergency
  operation, audited as such.
- Key rotation playbook for `RUSTIO_SECRET_KEY` — separate
  doc when the first rotation lands.

### 1.3 Closing principle

R4 makes the last-mile shell-access recovery path
**indistinguishable in the audit log from any other recovery
operation, but distinguishable in event type**. The auditor
sees `EmergencyRecovery` rows and knows the operator went
around every other tier. That visibility — not the difficulty
of running the command — is the regulatory artefact.

---

## 2. Threat model

R4's gate is OS-level: the operator running the binary has a
shell on the host AND `DATABASE_URL` in env (or `.env`). That
combination already grants unfettered DB write access; an
attacker who has both can run arbitrary SQL on
`rustio_users` and bypass the framework entirely.

R4 therefore does not try to be the security boundary. It is
the **disciplined path** for the operator who could otherwise
do `UPDATE rustio_users SET password_hash = '...'` directly.
The discipline is:

1. The operation is recorded in the audit log with a structured
   reason field.
2. The operator confirms the destructive nature of the
   operation interactively (or with `--yes` for scripting).
3. The operator's identity at the OS level — `whoami` plus
   `hostname` — lands in audit metadata.

What R4 protects against is **operator fat-finger error** and
**plausible-deniability disputes**, not a malicious operator
who already controls the host.

---

## 3. Subcommand catalogue

All R4 commands live under `rustio-admin user` to mirror the existing
CLI shape (`user create`, `user list`, `user role`, `user
delete`). Every emergency operation has the same envelope:
`--email <e> --reason "<text>"` plus operation-specific flags.

### 3.1 `rustio-admin user reset-password`

```
rustio-admin user reset-password --email <e> --reason "<text>" [--temp-password <p>] [--yes]
```

Sets a new password (random if `--temp-password` not given),
sets `must_change_password = TRUE` on the target, revokes
EVERY session for the user, writes one
`AuditEvent::EmergencyRecovery` row with
`metadata.cli_operation = "reset_password"`.

Prints the generated temp password ONCE to stdout. Plaintext
never lands in a log, an audit row, or any other DB column.

### 3.2 `rustio-admin user unlock`

```
rustio-admin user unlock --email <e> --reason "<text>" [--yes]
```

Clears `locked_until` and resets `failed_login_count = 0` on
the target. Does NOT touch sessions; an unlock is not a session
event. Writes `EmergencyRecovery` with `metadata.cli_operation
= "unlock"`.

### 3.3 `rustio-admin user disable-mfa`

```
rustio-admin user disable-mfa --email <e> --reason "<text>" [--yes]
```

Clears `mfa_enabled`, `mfa_secret_ciphertext`,
`mfa_secret_key_id`, `mfa_last_used_step`. Deletes every row
in `rustio_mfa_backup_codes` for the user. Revokes every
session whose `trust_level = 'mfa_verified'` for the user (a
deferred MFA-bypass attack via stale elevated session is the
only realistic post-disable threat). Writes
`EmergencyRecovery` with `metadata.cli_operation =
"disable_mfa"`.

Refuses with a non-zero exit if `MfaPolicy::Required` is the
active policy AND the target carries a role that would
re-enrol them on next login. The operator must downgrade the
policy first or accept that the user will be redirected to MFA
enrolment on next login. (Acceptance via a second confirmation
prompt: "Policy requires MFA; user will be re-enrolled on next
login. Proceed?")

### 3.4 `rustio-admin user promote`

```
rustio-admin user promote --email <e> --to-role <role> --reason "<text>" [--yes]
```

Sets `rustio_users.role` to `<role>`. Revokes the target's
OTHER sessions (the current sessions still belong to the
pre-promotion role; revoking forces a fresh login that picks
up the new tier). Writes `EmergencyRecovery` with
`metadata.cli_operation = "promote"` and
`metadata.previous_role` / `metadata.new_role`.

Refuses to demote the sole Administrator. The check is
`COUNT(*) FROM rustio_users WHERE role = 'administrator' AND
is_active = TRUE AND id <> $target` — must be ≥ 1 for a
demote-from-administrator operation.

### 3.5 `rustio-admin user emergency-access`

```
rustio-admin user emergency-access --email <e> --reason "<text>" [--ttl-minutes <n>] [--yes]
```

Issues a single-use password-reset URL bypassing the mailer.
Reuses R1's `rustio_password_reset_tokens` machinery: insert
a hashed token, return the plaintext URL, print to stdout. TTL
defaults to 15 minutes; max via `--ttl-minutes` is 60
(beyond that, prefer `reset-password` instead). Writes
`EmergencyRecovery` with `metadata.cli_operation =
"emergency_access"` and `metadata.token_id` referencing the
inserted row.

The plaintext URL is printed exactly once to stdout. The
operator hands it to the target via whatever out-of-band
channel makes sense (in person, secure messaging, etc.). The
URL itself contains the only copy of the token; no DB column
holds the plaintext.

---

## 4. Confirmation banner

Every emergency command prints a red ANSI banner before the
mutation lands. The banner format is locked:

```
┌──────────────────────────────────────────────────────────────┐
│  ⚠  EMERGENCY OPERATION — RUSTIO ADMIN CLI                   │
├──────────────────────────────────────────────────────────────┤
│  Operation:  reset-password                                  │
│  Target:     alice@example.com (user_id=42, role=admin)      │
│  Reason:     Lost MFA device + locked out, no other admins   │
│  Operator:   mansour@studio.local                            │
│  Time:       2026-05-11T16:42:00Z                            │
├──────────────────────────────────────────────────────────────┤
│  This action is audited and irreversible.                    │
└──────────────────────────────────────────────────────────────┘

Type 'yes' to confirm, anything else to abort:
```

Lock points:
- Header colour: red (ANSI `\x1b[31m`). The body box is the
  default terminal colour.
- The target row is loaded from the DB BEFORE the banner
  renders. Operator typos on `--email` surface here, not
  inside the mutation.
- The reason text is echoed verbatim in the banner. The
  operator sees what will land in the audit row.
- The operator/time/reason lines exist even with `--yes`
  (skipped only the prompt itself).

`--yes` bypasses the prompt but still prints the banner. CI
runs do not lose the forensic echo.

If the operator's stdin is not a TTY and `--yes` is absent,
the command exits with status 2 and the message "Refusing to
run without a TTY (or pass --yes for scripting)". This
prevents accidental piping from running an emergency op
unnoticed.

---

## 5. Audit emissions

Every R4 command writes exactly one `rustio_admin_actions`
row with `AuditEvent::EmergencyRecovery` (already declared in
the R0-era enum, never previously emitted).

Schema of the row:

| Column | Value |
|---|---|
| `user_id` | The target user's id (the subject) |
| `action_type` | `"emergency_recovery"` (snake_case per audit drift test) |
| `model_name` | `"rustio_users"` |
| `object_id` | The target user's id |
| `summary` | `"<cli_operation>: <reason (first 200 chars)>"` |
| `metadata.cli_operation` | `"reset_password" \| "unlock" \| "disable_mfa" \| "promote" \| "emergency_access"` |
| `metadata.reason` | The full reason text |
| `metadata.os_actor` | `"<whoami>@<hostname>"` |
| `metadata.cli_invocation` | argv joined with spaces, EXCLUDING the `--reason` value (the reason lives in its own field; the argv echo is for forensics) |
| `metadata.previous_role` / `metadata.new_role` | promote only |
| `metadata.token_id` | emergency-access only |
| `correlation_id` | a fresh UUID v7 stamped by the CLI |
| `session_id` | NULL (the CLI has no session row) |
| `ip_address` | NULL (no HTTP request) |

The web handlers do NOT emit `EmergencyRecovery`. A grep test
at the framework crate boundary asserts the enum variant is
referenced only from the CLI crate. (See §9.3.)

---

## 6. Building blocks reused

R4 leans on the R0-R3 primitives:

| Operation | Primitive |
|---|---|
| `reset-password` set password | `auth::users::set_password(db, user_id, plain)` |
| `reset-password` flag must-change | direct SQL `UPDATE rustio_users SET must_change_password = TRUE` |
| `reset-password` revoke sessions | `auth::sessions::invalidate_sessions(db, SessionTarget::User { user_id }, SessionInvalidationReason::PasswordResetByOther)` |
| `unlock` clear lock | direct SQL `UPDATE rustio_users SET locked_until = NULL, failed_login_count = 0` |
| `disable-mfa` clear MFA cols | direct SQL `UPDATE rustio_users SET mfa_enabled = FALSE, mfa_secret_ciphertext = NULL, mfa_secret_key_id = NULL, mfa_last_used_step = NULL` |
| `disable-mfa` delete backup codes | direct SQL `DELETE FROM rustio_mfa_backup_codes WHERE user_id = $1` |
| `disable-mfa` revoke MFA-elevated | `auth::sessions::invalidate_sessions(db, SessionTarget::User { user_id }, SessionInvalidationReason::MfaResetByOther)` filtered to `trust_level='mfa_verified'` |
| `promote` change role | direct SQL `UPDATE rustio_users SET role = $1` |
| `promote` revoke other sessions | `invalidate_sessions(SessionTarget::User, RoleChangedByOther)` |
| `emergency-access` issue token | refactor the token-insert path out of `auth::recovery::issue_reset_token` into a `pub` helper that the CLI calls directly, OR call the existing pub fn with a mailer that drops the email — TBD in commit #5. |

R4 deliberately does NOT call the R2 `recovery_admin`
functions (`issue_admin_reset_token`, `admin_set_temp_password`,
`lock_user_account`, `unlock_user_account`) — those emit R2's
audit variants (`PasswordResetByOther`, etc.), not
`EmergencyRecovery`. The R4 audit chain must be visually
distinct in the log.

A new module `auth::emergency` (in the framework crate) holds
the thin wrappers each CLI command calls. The CLI module
calls a single `auth::emergency::<op>(db, target, reason,
metadata)` per command and the framework writes the audit row.

---

## 7. Session-revocation policy per operation

| Operation | Revokes |
|---|---|
| `reset-password` | All sessions for the target. Reason: `PasswordResetByOther`. |
| `unlock` | None. Reason: an unlock is not a session event. |
| `disable-mfa` | Sessions where `trust_level = 'mfa_verified'`. Reason: `MfaResetByOther`. Other sessions stay valid. |
| `promote` | All sessions EXCEPT the target's none-current — but the CLI has no current. Effectively: all sessions. Reason: `RoleChangedByOther` (new enum variant in `SessionInvalidationReason`). |
| `emergency-access` | None. The target opens the URL with no session yet; the URL flow rotates them itself. |

`SessionInvalidationReason::RoleChangedByOther` is the only
new variant. Reuses the existing column type. No migration.

---

## 8. What R4 does NOT ship

- **A daemon-mode CLI** — every command is one-shot.
- **An interactive REPL** — clap subcommands only.
- **Email/SMS the target** — `emergency-access` prints the URL
  to stdout. The operator delivers it out-of-band.
- **A `dry-run` flag** — banner + interactive confirm IS the
  dry-run. Adding `--dry-run` invites the operator to skip
  confirm and run for real, which is the opposite of the
  discipline.
- **Bulk operations** — every command takes exactly one
  `--email`. A bulk requirement is a script around the CLI,
  not a CLI feature.
- **A CHANGELOG entry until R4 ships** — placeholder
  `[Unreleased]` block updated in the final commit.

---

## 9. Test strategy

### 9.1 Unit tests

- Banner rendering: golden-string match on the rendered banner
  for each operation.
- Reason validation: `<8 chars`, all-whitespace, missing
  `--reason` flag, `--reason ""` — each surfaces a distinct
  exit code.
- `--yes` works without a TTY; absent `--yes` and no TTY
  fails fast.
- Sole-administrator-demote refused.
- `disable-mfa` against `MfaPolicy::Required` user surfaces
  the second-confirm prompt.

### 9.2 Integration tests (testcontainers)

Reuses the R3 testcontainers harness pattern. One scenario
per command:

- `reset_password_writes_audit_row_and_invalidates_sessions`
- `unlock_clears_lock_state_only`
- `disable_mfa_clears_columns_and_backup_codes`
- `promote_refuses_sole_admin_demote`
- `emergency_access_issues_single_use_url`

### 9.3 Cross-crate visibility test

A unit test in the framework crate asserts that
`AuditEvent::EmergencyRecovery` is referenced ONLY from
`rustio-admin-cli`. Implementation: walk
`crates/rustio-admin/src/` for `EmergencyRecovery` references;
assert zero. Walk `crates/rustio-admin-cli/src/`; assert ≥ 1.
Std-only walk, same pattern as the 0.7.1 template-resolution
test.

---

## 10. Doctrines added by R4

### D9. CLI-actor identity is OS-level

`metadata.os_actor` is `<whoami>@<hostname>`. The CLI does
not invent a synthetic admin user; the audit row's target
is the user being acted on, and the CLI operator is in
metadata.

Why: introducing a `cli@system.local` synthetic user would
create a row in `rustio_users` whose authentication semantics
are undefined (can it log in? what role?). Better to record
the OS identity directly.

### D10. Confirmation banner is irreducible

Every R4 command prints the banner. `--yes` skips the prompt,
not the banner. No flag suppresses the banner. The banner is
the visible artefact for over-the-shoulder review and CI log
review alike.

### D11. R4 operations are atomic per command

One command = one audit row = one transaction (where DB
operations span multiple statements). A partial failure
(e.g. password set but session revoke failed) rolls back the
password change. Half-applied state is the worst possible
outcome for emergency recovery.

### D12. `EmergencyRecovery` is CLI-only by code-walk

The framework crate must not emit `EmergencyRecovery`. The
§9.3 cross-crate test enforces this. A future web handler
that needs an emergency-shaped operation introduces a new
audit variant rather than reusing this one.

---

## 11. Commit chain (locked order)

R4 ships as 9 small commits on `feat/r4-cli-emergency-recovery`:

1. **This DESIGN doc.** Reviewed in isolation; locks scope.
2. `feat(r4): AuditEvent + SessionInvalidationReason scaffolding` — add `RoleChangedByOther`, doc-comment `EmergencyRecovery` as CLI-only, plus the §9.3 cross-crate test.
3. `feat(r4): auth::emergency module skeleton` — framework-side thin wrappers, one function per CLI command, no banner / no confirm logic (those live in CLI).
4. `feat(r4): CLI banner + reason validation` — std-only, no DB. Unit tests for §9.1 banner/reason cases.
5. `feat(r4): rustio-admin user reset-password` — first end-to-end command. Audit row written, sessions revoked, temp password printed.
6. `feat(r4): rustio-admin user unlock` — simplest mutation. Used to validate the banner+confirm chain on a no-side-effects command first.
7. `feat(r4): rustio-admin user disable-mfa + promote` — bundled, both lean on §6's primitives.
8. `feat(r4): rustio-admin user emergency-access` — the token-issuance path. Refactor `issue_reset_token` to expose the no-mail variant cleanly.
9. `test(r4): testcontainers integration suite` — §9.2 scenarios.
10. `docs(changelog): R4 entry under [Unreleased] targeting 0.8.0`.
11. `chore: bump workspace to 0.8.0 + CHANGELOG release flip` (gated on explicit publish approval).

Commits 1-9 land on the feature branch. Commit 10 lands before
the version bump per the R3 cycle's discipline. The 0.8.0
publish is a separate step gated on explicit "publish 0.8.0"
approval AND a live-DB validation pass against lursystem.

---

## 12. Done definition

R4 is done when:

- Every command in §3 runs end-to-end against a real Postgres.
- Every audit row carries the §5 metadata.
- The §9.3 cross-crate visibility test passes.
- `cargo fmt && cargo check --workspace && cargo clippy
  --workspace --all-targets -- -D warnings && cargo test
  --workspace` is green.
- lursystem can run a downstream-validation scenario:
  simulate "Administrator lost MFA, no other admins", recover
  via `rustio-admin user disable-mfa`, confirm the recovered admin
  can re-enrol MFA cleanly on next login.
- `[0.8.0]` CHANGELOG entry references this doc.

---

R4 closes the recovery-roadmap loop opened in R0. After R4 the
framework supports four tiers — self (R1), peer-admin (R2),
two-factor (R3), shell (R4) — and refuses to silently lose the
audit chain at any tier transition.
