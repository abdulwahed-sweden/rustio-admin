# AI Assistant Permissions

An AI coding assistant working inside a rustio-admin project may only
do what a developer has allowed, must stop and wait for approval on
anything that touches the database or existing code, and is blocked
outright from security and production. Every suggestion, approval, and
applied change is recorded.

This document is the contract for that system — the buckets a capability
can fall in, the states a change moves through, who may approve, and what
gets written to the record.

Pull request review runs against this document, not only the diff.

> **Doctrine inheritance**
> This layer adds no new trust primitives — it composes the ones the
> framework already owns. Permissions and roles come from
> `DESIGN_PERMISSIONS.md`; the record is the audit trail in
> `DESIGN_AUDIT.md`; applied changes run through the deterministic
> builder in `DESIGN_BUILDER.md` (append-only history, reproducible,
> reversible). The AI is an *external actor* governed by these, never a
> new privileged runtime.

---

## 1. Purpose

### 1.1 What this governs

- The policy that states what an AI assistant may do (`.rustio/ai.toml`).
- The three capability buckets: **Allowed**, **Needs approval**,
  **Blocked** (§3).
- The lifecycle of an AI-touched change: Suggested → Reviewed → Approved
  → Applied / Rejected (§4).
- Who may approve, and what is written to the audit record (§4, §5).
- The `rustio ai` command surface (§6).

### 1.2 What this does not cover

- The AI model itself. The assistant (Claude Code, Copilot, Cursor, …)
  runs **outside** the framework. rustio-admin does not embed a planner
  or generate schemas on its own — it governs what an external assistant
  is permitted to do. See §8 (scope and the "no AI planners" rule).
- Human-initiated changes. A developer using the CLI or editing files by
  hand is bound by the existing role/permission system, not this policy.
- Network access, prompt content, or how the assistant decides what to
  propose. This contract begins at the point a proposal reaches the
  project.

### 1.3 Closing principle

**The AI has fewer privileges than a developer, never more, and no
private path.** It uses the same plan → commit pipeline a human uses; if
a human cannot make a change without approval, neither can the AI. The
position is clarity, not magic: the developer always knows what the AI
can do, what it cannot, what needs approval, and what it changed.

---

## 2. The policy is code

The rules live in one version-controlled file, `.rustio/ai.toml`. It is
the single source of truth. `rustio ai status` reads it directly — seeing
the rules never requires calling an AI.

```toml
[ai]
assistant = "Claude Code"

allowed = [
  "create_model",
  "create_form",
  "create_admin_page",
  "suggest_fields",
  "draft_migration",
]

needs_approval = [
  "apply_migration",
  "modify_table",
  "edit_existing_code",
  "add_dependency",
]

blocked = [
  "security_settings",
  "production_deploy",
  "delete_data",
  "edit_audit_log",
  "edit_ai_policy",
]

[ai.approval]
approver_role      = "administrator"
second_approver_for = ["modify_table", "apply_migration"]
```

Two invariants protect the policy itself:

1. **`edit_ai_policy` is Blocked.** The AI cannot widen its own access.
2. **Changing the policy is a reviewed change.** Moving a capability
   between buckets is a human edit to a version-controlled file — visible
   in a pull request, never a hidden setting.

---

## 3. The permission model

Every capability sits in exactly one bucket. Nothing is ungrouped.

| Bucket | Meaning |
|---|---|
| **Allowed** | The AI does it directly. Recorded and reversible. |
| **Needs approval** | The AI prepares it and stops. A developer must approve before it runs. |
| **Blocked** | The AI cannot do it — not even with approval — until the rule is explicitly moved out of `blocked`. |

### 3.1 The files-vs-database split

The split is what makes "create a model" safe:

- Writing a **model definition, a form, an admin page, or a draft
  migration** produces *files*. These are **Allowed**.
- **Applying** a migration — the step that changes a real Postgres
  table — is a separate capability in **Needs approval**.

So the AI can hand a developer a complete, ready model. Only the
developer turns it into a live table. The database is never touched on
the AI's own authority.

### 3.2 Default buckets

| Capability | Default bucket |
|---|---|
| `create_model`, `create_form`, `create_admin_page` | Allowed |
| `suggest_fields` | Allowed |
| `draft_migration` (write a migration file) | Allowed |
| `apply_migration` (run it against Postgres) | Needs approval |
| `modify_table` (alter an existing table) | Needs approval |
| `edit_existing_code` (hand-written, non-generated) | Needs approval |
| `add_dependency` | Needs approval |
| `security_settings` (auth, roles, permissions, secret key, CSRF, rate limits) | Blocked |
| `production_deploy` | Blocked |
| `delete_data` | Blocked |
| `edit_audit_log` | Blocked |
| `edit_ai_policy` | Blocked |

Blocked capabilities still allow the AI to **describe** a change in plain
text (no files written) so a developer can act on the suggestion by hand.

---

## 4. The approval model

Every AI-touched change moves through a small, visible set of states:

```
Suggested  →  Reviewed  →  Approved  →  Applied
                                    ↘  Rejected
```

- **Suggested** — a proposal exists as files/diffs, not a live change.
- **Reviewed** — a developer has seen the preview (`rustio ai review`).
- **Approved** — a developer signed off. The approver is a **real
  rustio-admin user**, authenticated through the framework's identity
  system — not an anonymous confirmation.
- **Applied** — the change ran. For database changes, this is when the
  migration reaches Postgres.
- **Rejected** — declined, with an optional reason; the proposal is kept
  for the record.

### 4.1 Who approves

`approver_role` sets the minimum role (default `administrator`). A
capability listed in `second_approver_for` requires **two distinct
approvers** before it can be applied — used for database and other
high-risk changes.

### 4.2 Apply is always separate

Nothing applies on approval alone. `apply` is an explicit, separate step.
Preview-by-default means a developer can always see the exact files or
SQL before anything runs.

### 4.3 Reversibility

Applied changes run through the deterministic builder, so they are
reproducible and can be reverted. The history is append-only — a revert
is a new recorded step, not an erasure.

---

## 5. The record

For every proposal, the following is written to the project's audit trail
(`rustio_admin_actions`, with the correlation id the framework already
threads through every action):

- proposal id and a description of what it changes
- the files touched and the exact preview / diff
- the capability and its bucket
- **who approved it, and when** (by authenticated user)
- the outcome: applied, rejected, or reverted

"Who changed the schema, and who approved it?" always has an answer. A
Blocked attempt is recorded too — the record shows the AI was told no.

> **Implementation status.** The current slice keeps this record in a
> local append-only `.rustio/ai/log.jsonl` (one JSON object per line),
> and proposals as JSON under `.rustio/ai/proposals/`. The approver is
> captured as a `--by <name>` string. Mirroring the record into
> `rustio_admin_actions` and authenticating the approver against a live
> admin both need a database; they are a later slice, so this surface
> stays offline like `rustio ai status`.

---

## 6. Command surface

The command set stays small and obvious.

| Command | Does | Status |
|---|---|---|
| `rustio ai status` | Show the policy (Allowed / Needs approval / Blocked), pending proposals, and recent actions. Reads the policy + log; no AI call. | shipped |
| `rustio ai init [--force]` | Write a default `.rustio/ai.toml`. | shipped |
| `rustio ai propose --capability <k> --title <t> [--stage DEST=SRC]…` | Register a change. Refused when the capability is Blocked. | shipped |
| `rustio ai list [--all]` | List proposals (pending by default). | shipped |
| `rustio ai review <id>` | Show a proposal's details and staged changes. | shipped |
| `rustio ai approve <id> --by <name>` | Record one approval (distinct approvers enforced). | shipped |
| `rustio ai reject <id> --reason "…"` | Decline, keeping the record. | shipped |
| `rustio ai apply <id>` | Write the staged files of an approved (or Allowed) proposal. | shipped |
| `rustio ai log [--limit N] [--proposal <id>] [--all]` | The record of suggestions, approvals, rejections, applies, and blocked attempts, newest first. | shipped |
| `rustio ai allow <cap> [--needs-approval]` / `deny <cap>` | Move a capability between buckets (`allow` → allowed, `--needs-approval` → needs_approval, `deny` → blocked). Edits `.rustio/ai.toml` in place, preserving comments, and prints the diff. | shipped |

`<id>` accepts the full ULID or the short suffix handle `status` / `list`
print. The default `--by` is the OS user.

### 6.1 `rustio ai status` — reference output

```
AI Assistant: Claude Code
Policy:       .rustio/ai.toml   (approver: administrator)

Allowed:
  ✓ Create models
  ✓ Create forms
  ✓ Create admin pages
  ✓ Suggest fields
  ✓ Draft migrations

Needs approval:
  ⚠ Apply migrations
  ⚠ Change existing tables   (2 approvers)
  ⚠ Edit existing code
  ⚠ Add dependencies

Blocked:
  ✗ Security settings
  ✗ Production deployment
  ✗ Delete data
  ✗ Edit the audit log
  ✗ Edit the AI policy

Pending your review:
  #42  Add phone column to Customer   (needs approval)

Recent actions:
  #41  Added Customer model           applied   by amir@team
  #40  Suggested search fields        suggested
```

---

## 7. Worked workflows

The shape is always: **Developer asks → AI suggests → Developer reviews →
Developer approves → Framework applies.**

### 7.1 Create a new model — Allowed

```
Developer: "Add a Customer model with name, email, signup date."

AI:        Suggested #41
           + src/models/customer.rs        (new file)
           + migrations/0007_customer.sql  (draft — not applied)

Developer: rustio ai review 41
           rustio ai apply 41       # files written; table NOT yet created
```

The model and form exist immediately. The table does not — the migration
is still a draft (§7.2 applies it).

### 7.2 Modify a database table — Needs approval

```
Developer: "Add a phone column to Customer."

AI:        Suggested #42
           migrations/0008_customer_phone.sql:
             ALTER TABLE customer ADD COLUMN phone text;
           Risk: changes an existing table  →  Needs approval (2 approvers)

Developer: rustio ai review 42
           rustio ai approve 42     # approved by amir@team, then sara@team
           rustio ai apply 42       # migration runs against Postgres
```

Nothing reaches the database before authenticated approval.

### 7.3 Change permissions — Blocked

```
Developer: "Let the editor role delete users."

AI:        This is a security setting. I cannot make this change.
           Suggested (text only, no files):
             "Grant 'auth_user.delete_user' to the editor group."
           To allow it: a developer makes the change by hand, or moves
           'security_settings' out of `blocked` in .rustio/ai.toml
           (itself a reviewed change; the AI cannot edit that file).
```

---

## 8. Scope and the "no AI planners" rule

The ROADMAP lists **embedded AI planners** as out of scope for the core
crate. This system does not conflict with that rule, and the boundary is
deliberate:

- **In scope here:** a *permissions, approval, and audit layer* over an
  external assistant. These are the framework's existing strengths
  (permissions, audit, deterministic builder) applied to a new kind of
  actor. The policy file, the approval lifecycle, and the record are
  ordinary rustio-admin surfaces.
- **Out of scope here:** the AI itself. rustio-admin does not host a model,
  does not design schemas on its own, and does not call out to an
  assistant. The intelligence stays in the external tool; the framework
  only governs it.

If a deeper integration ever embeds model calls or schema planning inside
the product, that integration ships in a separate `rustio-pro-*` crate,
per the strategic-reset rules — never inside `rustio-admin`. This contract
governs the **permission and approval primitives**, which are native.
