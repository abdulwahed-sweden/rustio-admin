# Project memory — `rustio-admin memory`

A guide to recording **why** your project is the way it is, so the reasoning
survives — and a new teammate or AI assistant can absorb years of it in
minutes.

> The contract that governs this feature is
> [`docs/design/DESIGN_CLOUD.md`](./design/DESIGN_CLOUD.md). This page is the
> practical guide; read it first.
>
> Commands are written as `rustio-admin memory …` — the name of the
> installed binary. The framework's prose sometimes shortens this to
> `rustio memory`; they mean the same command.

---

## What is project memory?

Code tells you **what** a project does. Git history tells you **what
changed**. Neither reliably tells you **why** — why this approach, why *not*
the obvious alternative, what you assumed, what you're building toward.

That "why" is the first thing lost and the most expensive to recover. People
re-litigate settled decisions, re-implement ideas that were already tried and
rejected, and new joiners spend weeks asking "but why is it like this?"

**Project memory** is where that "why" lives. You record decisions, accepted
assumptions, business intent, and — most valuably — **the ideas you tried and
rejected, with the reason**. It's stored as plain files under
`.rustio/memory/` and rendered into a human-readable **`CLOUD.md`** at your
project root. An AI assistant (or a person) reads it to get the context that
isn't in the code.

**How it's different from a wiki or a doc folder:** project memory is
governed. It can't drift into a second source of truth, it can't be quietly
rewritten, and nothing lands in it without a human saying yes (next section).

**Who it's for:** any team — solo or many — that wants its reasoning to
outlive the conversation it happened in, and AI-assisted teams that want the
assistant to stop re-deriving context every session.

---

## The mental model

Three rules define project memory. They never change — everything else is
built to protect them.

### 1. Subordinate to code

Memory is **not a source of truth.** Code, your schema, and the database are.
Memory explains *why*; it never describes *how the system works*. **On any
conflict, code wins.** If an entry and the code disagree, the entry is stale
history, not instruction.

### 2. Append-only

You **add** to memory; you never rewrite its history. A decision that changes
isn't edited — a new entry **supersedes** the old one, and the old one stays,
visibly marked as superseded. So the record of *what you used to believe, and
when* is never lost. (There is exactly one exception — removing a leaked
secret; see [Redaction](#redacting-a-leaked-secret).)

### 3. Human-ratified

Nothing is recorded automatically. An assistant (or you) **proposes** an
entry; a **human approves** it; only then is it **applied**. This is the gate
that keeps a confident-but-wrong AI suggestion from becoming "fact."

### What goes in — and what never does

| Record this (the *why*) | Never record this |
|---|---|
| Decisions + their rationale | Anything in code/schema (the *what*) |
| **Rejected ideas + why rejected** | Secrets, tokens, credentials, PII |
| Accepted assumptions ("internal-only") | Current operational state ("deploy is broken") |
| Business intent, project history | Anything you'd treat as an instruction to act |

If an entry starts describing *how the system works*, it's in the wrong
place — that belongs in code or a design doc.

---

## How it works

### The lifecycle

Every change to memory runs through the same **propose → approve → apply**
pipeline that `rustio-admin ai` uses — one governance path, no exceptions:

```
remember / supersede / redact   →   approve (a human)   →   apply
        (proposes)                                       (writes the entry,
                                                          re-renders CLOUD.md)
```

- **Propose** stages the change; nothing is in memory yet.
- **Approve** records a human's sign-off. `write`/`supersede` need **one**
  approver; **redaction needs two**, as does marking an entry *foundational*.
- **Apply** writes the immutable entry file and regenerates `CLOUD.md`.

An entry's date, the approver, and its audit id are stamped **at apply** —
the moment a human ratifies it.

### Where things live

```
your-project/
├── CLOUD.md                     ← generated, human-readable; do not hand-edit
└── .rustio/memory/
    ├── entries/<id>.md          ← the canonical entries (one file each)
    ├── proposals/               ← staged, not-yet-approved changes
    └── log.jsonl                ← the append-only record of every action
```

`CLOUD.md` is **generated** from the entries (like `tokens.css` is generated
by the theme engine). Edit entries through the commands, never `CLOUD.md`
directly — `rustio-admin memory verify` will flag a hand-edit.

### Offline by default; authenticated when you want it

- **`--by <name>`** (default) records an approver by name, offline. No
  database needed; the `log.jsonl` is your record. Great for solo use.
- **`--as <email>`** authenticates the approver against your user table
  (active user, sufficient role) and **mirrors the decision into the audit
  trail** (`rustio_admin_actions`). Use this when you want
  authenticated, audited approvals.

### Permissions

Memory reuses your `.rustio/ai.toml` policy. The capabilities
`write_memory`, `supersede_memory`, and `redact_memory` default to **needs
approval** (redaction additionally needs a second approver). Run
`rustio-admin ai init` to write the policy file so those buckets are explicit
and version-controlled; memory also works against the built-in defaults if
the file is absent.

---

## Recipes

### Remember a decision

```bash
rustio-admin memory remember \
  --type decision \
  --subject auth \
  --note "Use double-submit CSRF cookies — simplest token strategy that fits our stateless sessions." \
  --by amir
# → proposal created; needs 1 approval
rustio-admin memory approve <id> --by sara
rustio-admin memory apply   <id> --by sara      # writes the entry, re-renders CLOUD.md
```

`--type` is one of: `decision`, `rejected`, `assumption`, `intent`,
`onboarding`, `history`, `open-tension`. `--subject` is repeatable and is the
key others retrieve by; reuse existing subjects (see `stats`) so related
entries cluster.

### Remember a rejected idea (the highest-value kind)

```bash
rustio-admin memory remember \
  --type rejected \
  --subject jobs \
  --note "Rejected LISTEN/NOTIFY for the job runner — polling is simpler to operate and we don't need sub-second latency." \
  --source "pr#41" \
  --by amir
```

Now, when someone proposes LISTEN/NOTIFY again next year, the reason it was
dropped is one `rustio-admin memory show --type rejected --subject jobs` away.

### Supersede a decision that changed

```bash
rustio-admin memory supersede <old-id> \
  --type decision --subject auth \
  --note "Moved to SameSite=Strict cookies; the double-submit token is redundant now." \
  --by amir
# approve + apply as above
```

The old entry stays in the record, marked superseded — never deleted.

### Mark something foundational

Add `--foundational` to `remember`/`supersede` for a load-bearing assumption
you don't want buried over time (e.g. "internal-only, never multi-tenant").
Foundational entries require **two approvers**.

### Read it back

```bash
rustio-admin memory show                       # everything
rustio-admin memory show --subject auth        # by subject
rustio-admin memory show --type rejected       # only rejected ideas
rustio-admin memory show --active              # hide superseded entries
rustio-admin memory show --grep CSRF           # body contains text
rustio-admin memory chain <id>                 # an entry's supersession lineage
```

### Redacting a leaked secret

Redaction is the **one** way to change an existing entry — for removing a
secret/PII that slipped in. It needs **two approvers**.

```bash
rustio-admin memory redact <id> --class token --reason "API key pasted into the note" --by amir
rustio-admin memory approve <id-of-redaction> --by amir
rustio-admin memory approve <id-of-redaction> --by sara     # second approver
rustio-admin memory apply   <id-of-redaction> --by lee
```

`--class` is one of `password`, `token`, `mfa_secret`, `backup_code`, `pii`,
`credential`, `operational`.

> **Redaction is not history scrubbing.** It removes the content from the
> working tree going forward — but the secret is still in your **git
> history**. A real leak also requires **rotating the secret** and rewriting
> history (`git filter-repo` / BFG). "Redacted" does not mean "gone."

### Keep `CLOUD.md` honest (and put it in CI)

```bash
rustio-admin memory render     # regenerate CLOUD.md from entries (rarely needed; apply does it)
rustio-admin memory verify     # fails if CLOUD.md is stale, links are broken, or an entry was hand-edited
```

Run `rustio-admin memory verify` in CI on any change touching
`.rustio/memory/` or `CLOUD.md`. It exits non-zero on a stale `CLOUD.md`, a
dangling/cyclic supersedes link, or an entry changed outside a redaction.
(It's a best-effort backstop: it compares the working tree against `HEAD` and
skips when there's no git repo — it doesn't audit committed history.)

### See the shape of your memory

```bash
rustio-admin memory stats                 # counts by type/status, foundational/redacted, subjects
rustio-admin memory promote-candidates     # entries revised ≥2× — candidates to graduate into an ADR
rustio-admin memory pending                # proposals awaiting a decision
```

`promote-candidates` only *suggests*: when a decision keeps getting revised,
it's probably architectural enough to deserve a real ADR. Promotion is a
human action — write the ADR, then `supersede` the entry with a link to it.

---

## Command reference

| Command | What it does |
|---|---|
| `memory remember --type T --subject S --note "…"` | Propose a new entry. `--foundational`, `--source REF`, `--by NAME`. |
| `memory supersede <id> --type T --note "…"` | Propose an entry that replaces an existing one. |
| `memory redact <id> --class C --reason "…"` | Propose removing prohibited content (two approvers). |
| `memory pending` | List proposals awaiting a decision. |
| `memory approve <id>` | Approve a proposal. `--by NAME` (offline) or `--as EMAIL` (authenticated + audited). |
| `memory reject <id> --reason "…"` | Decline a proposal, keeping the record. |
| `memory apply <id>` | Write the approved entry and re-render `CLOUD.md`. |
| `memory render` | Regenerate `CLOUD.md` from the entries. |
| `memory show [--subject S] [--type T] [--active] [--grep TEXT]` | List entries (exact filters, no ranking). |
| `memory chain <id>` | Show an entry's supersession lineage. |
| `memory verify` | Check `CLOUD.md` freshness + referential integrity + working-tree tampering. |
| `memory stats` | Mechanical counts (types, statuses, subjects, foundational/redacted). |
| `memory promote-candidates [--min N]` | Flag entries revised ≥ N times as ADR candidates. |
| `memory index` | Rebuild the `.rustio/memory/index.json` cache. |

---

## If you remember nothing else

- Project memory records **why**, never **what**. **Code always wins.**
- Capture **rejected ideas with their reasons** — that's the payoff.
- It's **append-only** (supersede, don't rewrite) and **human-approved**
  (propose → approve → apply).
- `CLOUD.md` is **generated** — edit entries, not the file; run
  `rustio-admin memory verify` in CI.
- The running app never reads any of this. It's a tool for the humans (and
  assistants) building the project.

Deeper still: the formal contract is
[`DESIGN_CLOUD.md`](./design/DESIGN_CLOUD.md); the implementation design is
[`DESIGN_CLOUD_IMPL.md`](./design/DESIGN_CLOUD_IMPL.md).
