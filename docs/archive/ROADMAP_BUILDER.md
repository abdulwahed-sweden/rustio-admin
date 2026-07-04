# Rustio — Forward Roadmap (Builder + Advisory AI)

> Long-horizon vision document. Sits alongside `ROADMAP.md` (near-term
> framework work) and is **not yet** doctrine — the design documents in
> `docs/design/` precede their implementations, but this document
> precedes the design documents.
>
> Mission framing: Rustio solves big-company administrative problems.
> Not a WordPress alternative, not a starter-kit generator. The target
> operator is at a bank, hospital, factory, or regulated enterprise
> who needs administrative software that **audits cleanly, deploys to
> air-gapped environments, and is reviewable line-by-line**. Every
> roadmap item below is filtered through that lens.

---

## Table of contents

- [Part 1 — The Interactive Project Compiler (Builder)](#part-1--the-interactive-project-compiler-builder)
  - [0. The fundamental reframe](#0-the-fundamental-reframe)
  - [1. Mental model — cargo + git, not Yeoman / Rails CLI](#1-mental-model--cargo--git-not-yeoman--rails-cli)
  - [2. CLI surface](#2-cli-surface)
  - [3. The two-file backbone](#3-the-two-file-backbone)
  - [4. Generated code — editability contract](#4-generated-code--editability-contract)
  - [5. Migrations — the most security-sensitive surface](#5-migrations--the-most-security-sensitive-surface)
  - [6. Contracts the Builder must respect](#6-contracts-the-builder-must-respect)
  - [7. Risks and mitigations](#7-risks-and-mitigations)
  - [8. Hard non-goals](#8-hard-non-goals)
  - [9. What this layer protects](#9-what-this-layer-protects)
- [Part 2 — The Advisory AI Layer](#part-2--the-advisory-ai-layer)
  - [10. The one sentence that governs everything](#10-the-one-sentence-that-governs-everything)
  - [11. Why this is structurally different from AI-codegen tools](#11-why-this-is-structurally-different-from-ai-codegen-tools)
  - [12. The four layers](#12-the-four-layers)
  - [13. The proposal contract](#13-the-proposal-contract)
  - [14. The review surface](#14-the-review-surface)
  - [15. Provider abstraction — no Claude lock-in](#15-provider-abstraction--no-claude-lock-in)
  - [16. Domain experts — the deterministic AI tier](#16-domain-experts--the-deterministic-ai-tier)
  - [17. Field-type semantic inference](#17-field-type-semantic-inference)
  - [18. Anti-patterns this design forbids](#18-anti-patterns-this-design-forbids)
  - [19. Privacy and prompt contents](#19-privacy-and-prompt-contents)
  - [20. CLI surface for the AI layer](#20-cli-surface-for-the-ai-layer)
  - [21. The Claude Code integration point](#21-the-claude-code-integration-point)
  - [22. The compliance pitch](#22-the-compliance-pitch)
- [Part 3 — Sequenced roadmap](#part-3--sequenced-roadmap)
- [Part 4 — Open decisions](#part-4--open-decisions)

---

## Part 1 — The Interactive Project Compiler (Builder)

## 0. The fundamental reframe

The existing Rustio surface is governed by:

- **Doctrine 22** — single-writer session invalidation
- **Audit-by-default** — every authority mutation emits a typed event
- **Uniform outward responses** — recovery & login surfaces collapse failure modes
- **No magic / no build step / Postgres only**

Any roadmap that doesn't extend those guarantees into the new layer is
a downgrade dressed as a feature. The Builder is not "faster
scaffolding" — it is **a deterministic project compiler whose every
output is reviewable, auditable, and reproducible by hand**.

That sentence is the design contract for everything below.

The earlier "templates" framing was rejected: templates rot, drift
from the framework, accumulate maintenance debt, and turn into
quasi-independent projects. The Builder replaces templates entirely.

---

## 1. Mental model — `cargo` + `git`, not Yeoman / Rails CLI

Tools that big-company developers already trust shape expectations.
The CLI must feel native to them:

| Pattern | Borrowed from | What it gives the operator |
|---|---|---|
| Working directory becomes project state | `cargo init` | Files on disk inspectable in git; CLI never owns state outside the repo |
| Staging area before commit | `git add` / `git commit` | Reviewable intent. Nothing happens to source until explicit |
| Plan / apply separation | `terraform plan` / `apply` | Preview = no side effects. Apply = atomic, idempotent |
| Lockfile pinning | `Cargo.lock` | Deterministic regeneration months later still emits the same bytes |

The vendor analogy is **Terraform for application code**. Declarative
state file, plan/apply lifecycle, reviewable diffs, idempotent
execution, no surprise.

---

## 2. CLI surface

Every verb is **non-destructive by default**, prints a diff, and
supports `--yes` for CI use. Three groups:

### 2.1 Project lifecycle

```sh
rustio new <name>             # Bootstrap: empty crate + .rustio/draft.toml + .rustio/history.jsonl
rustio status                 # What's staged vs. committed vs. on disk
rustio plan                   # Show every file that would be written, with diffs
rustio commit                 # Atomic: write generated files + emit lockfile entries
rustio doctor                 # Already exists. Extend to validate draft.toml integrity.
```

### 2.2 Schema authoring

```sh
rustio add model <Name>                    [--field name:type[:modifiers] ...]
rustio add field <Model> <name> <type>     [--required] [--unique] [--default ...]
rustio add relation <A> <kind> <B>         [--fk col] [--through pivot]
rustio remove model <Name>                 # Refuses if other models reference it
rustio rename model <Old> <New>            # Tracked as a single op, regenerates cleanly
rustio set <key> <value>                   # rustio set theme.accent "#0F8C7E"
```

### 2.3 History / undo

```sh
rustio log                    # Replays history.jsonl as a human-readable changelog
rustio undo                   # Remove last event, rebuild draft.toml
rustio redo                   # Re-apply removed event
rustio diff <ref>             # Compare current draft to any prior committed state
```

The `log`/`undo`/`redo` triple is what makes this **safe for serious
work**: the developer can experiment freely because every action is
reversible up until `commit`.

---

## 3. The two-file backbone

Everything the CLI does flows through two files in `.rustio/`:

### 3.1 `.rustio/draft.toml` — current intent (mutable, source of truth)

```toml
[project]
name           = "patient-records"
rust_version   = "1.88"
created_at     = "2026-05-15T10:30:00Z"
schema_version = 1                              # bumps with breaking generator changes

[features]
authentication = true
audit          = true
mfa            = false                          # opt-in per Doctrine
soft_delete    = false                          # opt-in; changes ModelAdmin contracts

[[models]]
name = "Patient"
fields = [
  { name = "id",          type = "id" },
  { name = "full_name",   type = "text",  required = true, max = 200 },
  { name = "dob",         type = "date" },
  { name = "national_id", type = "text",  unique = true, redact = true },
]
admin = { list_display = ["full_name", "dob"], search_fields = ["full_name", "national_id"] }

[[relations]]
from      = "Patient"
to        = "Appointment"
kind      = "has_many"
fk        = "patient_id"
on_delete = "restrict"                          # default = restrict (audit-friendly)
```

This is the **declarative contract**. It is the only thing
`rustio commit` reads.

### 3.2 `.rustio/history.jsonl` — append-only event log

```jsonl
{"id": "01HW...", "ts": "...", "op": "add_model",    "args": {"name": "Patient", "fields": [...]}}
{"id": "01HW...", "ts": "...", "op": "add_relation", "args": {"from": "Patient", ...}}
{"id": "01HW...", "ts": "...", "op": "commit",       "args": {"files_written": 12, "migration": "0003"}}
```

Append-only, ULID-keyed, never edited. `undo` writes a compensating
event rather than mutating the log — same discipline as the
framework's audit trail. **The same audit philosophy that protects
production sessions now protects the build-time decisions about the
application itself.**

A security auditor or compliance team can replay exactly how the
project's schema evolved over years.

---

## 4. Generated code — editability contract

The single hardest design problem in this whole roadmap. The clean
answer from Rails/Django is "you cannot edit generated code"; the
realistic answer is "you absolutely will". So we draw the line
explicitly:

```text
src/
├── _generated/           ← regenerated on every commit. DO NOT EDIT.
│   ├── models/
│   ├── admin.rs          ← Admin::new().model::<...>() registrations
│   └── routes.rs
├── app/                  ← developer territory. Generator never touches this.
│   ├── handlers/
│   ├── business/
│   └── extensions.rs     ← ModelAdmin trait impls live here, not in _generated
└── main.rs               ← scaffold once on `new`, then developer-owned
```

Each file in `_generated/` carries the same header:

```rust
// @generated by rustio 0.15.0 from .rustio/draft.toml
// SPDX-SchemaHash: sha256:abc123...   (signs the relevant draft.toml fragment)
// To change, edit draft.toml and run `rustio commit`. Manual edits will be overwritten.
```

`rustio commit` refuses to overwrite a `_generated/` file if its
SchemaHash doesn't match — protecting developers from silent loss of
stale manual edits. Force-overwrite requires `--force` and prints
what's being discarded.

This is the same discipline the framework already enforces between
templates and the embedded `include_str!` copies. **Same doctrine,
new layer.**

---

## 5. Migrations — the most security-sensitive surface

The append-only migration contract is already declared in the
framework. The Builder must extend it without weakening it.

```text
migrations/
├── 0001_initial.sql              ← created on first `rustio commit`
├── 0002_add_appointment.sql      ← created when a model was added later
└── 0003_make_email_unique.sql    ← created when a field constraint changed
```

Rules the Builder will enforce:

1. **Never edit an applied migration.** If a field changes after
   commit, a new migration is written.
2. **Diff against committed state**, not against the database. The
   `draft.toml` carries `applied_migration = "0003"` so regeneration
   on a fresh checkout still emits the same SQL.
3. **Destructive operations require explicit confirmation.**
   `DROP COLUMN`, `DROP TABLE` print a warning and force
   `--accept-destructive` even with `--yes`.
4. **Every migration ships with a roll-back hint** as a SQL comment —
   not auto-applied, but reviewable by the DBA who runs it.
5. **`rustio commit --dry-run`** prints the SQL without writing
   files. Required reading before any production migration.

This is where Rustio earns enterprise trust. A team running Rustio in
a regulated environment can hand any `migrations/*.sql` file to a DBA
who can read it without trusting the tool.

---

## 6. Contracts the Builder must respect

The Builder is a thin layer on top of the framework's existing trait
surface. Every generator decision maps to an existing public API:

| Builder concept | Existing framework binding |
|---|---|
| `[[models]]` entry | `#[derive(RustioAdmin)]` + `impl Model` + `impl ModelAdmin` |
| `admin.list_display` | `ModelAdmin::list_display()` |
| `admin.search_fields` | `ModelAdmin::search_fields()` |
| `[[relations]]` `belongs_to` | `AdminField::ForeignKey` + the FK-hydration on list pages |
| `features.audit` | `audit::record(...)` calls in handlers |
| `features.soft_delete` | (does not exist yet — feature flag must wait until framework adds it) |
| `[theme]` | `Admin::theme(AdminTheme {...})` |

**Critical rule**: the Builder never invents a runtime concept. If
`features.soft_delete` doesn't exist in the framework yet, the Builder
refuses to set it — it doesn't paper over with a half-implementation.
This is the existing "no second runtime" rule extended into the
Builder.

---

## 7. Risks and mitigations

| Risk | Mitigation |
|---|---|
| `draft.toml` schema churn breaks existing projects between 0.15 → 1.0 | `schema_version` field; each bump ships a `rustio migrate-draft` upgrader |
| Developer edits to `_generated/` get silently lost | SchemaHash mismatch refuses overwrite without `--force`; explicit warning lists the diff |
| Generator emits SQL wrong for some Postgres version | `rustio doctor schema` dry-runs every migration against an ephemeral `testcontainers` Postgres before claiming green |
| Builder becomes a second runtime in disguise | The CI grep guard already forbids `HasSchema` / `RustType` etc.; extend to forbid Builder-specific runtime traits leaking into the framework crate |
| Enterprise audit team rejects the tool because the event log is editable | `history.jsonl` is append-only by convention; optional sign-each-line mode for compliance environments — uses the same SHA-256 chain primitive the framework already ships for sessions |
| "Who is this for?" stays unanswered | The first user is **you, building Classrooms/POS on top of the Builder.** Dogfood it. If it doesn't make your own work easier, it ships nothing |

---

## 8. Hard non-goals

- ❌ Templates of any kind. The Builder replaces them entirely.
- ❌ A web wizard before v1.x.
- ❌ Multi-database support.
- ❌ Auto-discovery of models — explicit registration stays.
- ❌ A built-in LLM (handled separately in Part 2 as an opt-in advisor).
- ❌ Editing `_generated/` and expecting persistence.
- ❌ Cloud-hosted state. `.rustio/` lives in the repo. Period.

---

## 9. What this layer protects

A senior engineer at a bank, hospital, or factory can adopt Rustio
and convince their compliance team because:

1. Every file in `migrations/` is hand-reviewable SQL.
2. Every authority decision still emits a typed `AuditEvent`.
3. The build-time decisions (`draft.toml`) are version-controlled and
   the event log is append-only.
4. The tool runs offline, in an air-gapped environment, against a
   single Postgres instance.
5. There's no LLM in the critical path.
6. `cargo expand` still shows the full picture of generated code.

That's the moat. Productivity-tool roadmaps don't protect any of
those properties. This one does — by extending the framework's
existing doctrines into the Builder layer, **rather than inventing
parallel ones**.

---

## Part 2 — The Advisory AI Layer

## 10. The one sentence that governs everything

> **Claude proposes; the developer disposes; Rustio executes.**

Claude (or any LLM) never writes to `draft.toml`, never writes to
`_generated/`, never edits `migrations/`. It writes to one place:
`.rustio/proposals/`. The developer reviews. The deterministic
generator runs only after explicit approval.

This is the same separation-of-powers the framework already enforces
between session lifecycle, audit, and authority — extended to the
build-time AI layer.

---

## 11. Why this is structurally different from AI-codegen tools

The crowded space (Copilot, Cursor, v0, etc.) treats AI as an
authority — it writes code directly into your editor. Rustio's model
is the opposite:

| Capability | Cursor / Copilot | Rustio Advisory AI |
|---|---|---|
| Writes source files directly | ✅ | ❌ |
| Output is the truth | ✅ | ❌ — `draft.toml` is the truth |
| Reproducible after months | ❌ | ✅ — replay `history.jsonl` |
| Reviewable by compliance | ❌ | ✅ — every proposal logged with provenance |
| Works offline / air-gapped | ❌ | ✅ — provider is pluggable, including local |
| Decision is auditable | ❌ | ✅ — proposal + reviewer + verdict in event log |

The framing **"AI as architect, not as scribe"** is what makes this
compatible with regulated environments.

---

## 12. The four layers

```text
┌─────────────────────────────────────────────────────────┐
│  Advisory Layer                                         │
│    Claude / local LLM / domain expert plugin            │
│    Reads: draft.toml, project context, domain hints     │
│    Writes: .rustio/proposals/*.toml                     │
└────────────────────┬────────────────────────────────────┘
                     │  proposes
                     ▼
┌─────────────────────────────────────────────────────────┐
│  Review Layer                                           │
│    Developer runs `rustio review`                       │
│    Verdict: accept / reject / amend                     │
│    Verdict written as event in history.jsonl            │
└────────────────────┬────────────────────────────────────┘
                     │  approves (or doesn't)
                     ▼
┌─────────────────────────────────────────────────────────┐
│  Intent Layer                                           │
│    .rustio/draft.toml                                   │
│    .rustio/history.jsonl                                │
│    Mutated only by `rustio add/remove/set`              │
└────────────────────┬────────────────────────────────────┘
                     │  rustio commit
                     ▼
┌─────────────────────────────────────────────────────────┐
│  Deterministic Core                                     │
│    Generator. Same input → same output. No AI.          │
│    Writes src/_generated/, migrations/                  │
└─────────────────────────────────────────────────────────┘
```

Each layer's output is the *only* input to the next. No backchannels,
no shortcuts, no LLM call from inside the generator.

---

## 13. The proposal contract

Every AI suggestion is a structured document, not free-form text.

```toml
# .rustio/proposals/2026-05-15T1034-add-product-fields.toml

[meta]
id          = "01HW8K3F..."          # ULID
created_at  = "2026-05-15T10:34:21Z"
source      = "claude-sonnet-4-6"     # or "local-llama3", "domain-expert-clinic", ...
prompt_hash = "sha256:abc123..."      # the exact prompt sent — reviewable
draft_hash  = "sha256:def456..."      # draft.toml state when proposal was made
confidence  = 0.91                    # if the model self-reports; advisory only
explanation = """
Suggested fields based on POS retail domain knowledge.
`sku` is the universal SKU field for inventory systems.
`barcode` separated from `sku` because EAN-13 and internal SKU
diverge in practice.
"""

[[patches]]
op        = "add_field"
model     = "Product"
name      = "sku"
type      = "text"
modifiers = { unique = true, max = 64 }
reasoning = "SKUs are unique identifiers in inventory. Without UNIQUE the admin search can return duplicates."

[[patches]]
op        = "add_field"
model     = "Product"
name      = "price"
type      = "decimal"
modifiers = { precision = 12, scale = 2 }
reasoning = "Money should never be float. 12,2 covers values up to 9,999,999,999.99 — enough for any retail item."

[[patches]]
op        = "set_admin"
model     = "Product"
key       = "list_display"
value     = ["sku", "title", "price", "stock_quantity"]
reasoning = "Operators scan inventory by SKU first, then title. Price-right, quantity-right for table alignment."
```

Three properties this structure guarantees:

1. **Atomic patches.** Each `[[patches]]` block is one mutation. The
   developer can accept some and reject others — `rustio review --partial`.
2. **Reasoning per patch.** Every change carries an explanation,
   which lands in `history.jsonl` if accepted. Months later, "why is
   `price` decimal(12,2)?" has an audit answer.
3. **Provenance trail.** `prompt_hash` + `draft_hash` mean a second
   reviewer can rerun the same query against the same model and
   verify the proposal is reproducible from the inputs.

---

## 14. The review surface

```sh
rustio review                          # interactive, opens each pending proposal
rustio review --list                   # list pending proposals
rustio review <id> --accept            # bulk-accept (CI-friendly with --yes)
rustio review <id> --reject "reason"   # reject with rationale in history.jsonl
rustio review <id> --amend             # open in $EDITOR to tweak before accepting
```

Interactive review shows a colored diff:

```text
proposal 01HW8K3F — claude-sonnet-4-6 — 2026-05-15 10:34

┌─ add_field  Product.sku                                    ┐
│ type:      text                                            │
│ modifiers: unique=true, max=64                             │
│                                                            │
│ reasoning: SKUs are unique identifiers in inventory...     │
└────────────────────────────────────────────────────────────┘
[a]ccept  [r]eject  [e]dit  [s]kip  [q]uit:
```

Each verdict writes a row to `history.jsonl` with the proposal ID,
the verdict, the reviewer (from git config), and a timestamp. The
proposal file itself moves to `.rustio/proposals/accepted/` or
`.rustio/proposals/rejected/` — kept forever for audit.

---

## 15. Provider abstraction — no Claude lock-in

The advisory layer is a trait. Claude is one impl. This protects the
project from the deepest single risk: tying enterprise customers to
one closed model provider.

```rust
pub trait AdviceProvider: Send + Sync {
    fn name(&self) -> &str;
    fn supports_domain(&self, hint: &DomainHint) -> bool;

    async fn propose(
        &self,
        request: ProposalRequest,
    ) -> Result<Vec<Patch>, AdviceError>;
}
```

Built-in providers, in `crates/rustio-admin-cli/src/advisors/`:

| Provider | Where it runs | When to use |
|---|---|---|
| `claude` | Anthropic API | Default. Cloud team with API budget |
| `openai` | OpenAI API | Equivalent functionality for shops on OpenAI |
| `local-llama` | Ollama / llama.cpp on `127.0.0.1` | Air-gapped environments. Compliance-mandated |
| `domain-expert-<name>` | Pure Rust, hand-written rules | Deterministic. No API call. The "rule-based" tier |
| `none` | — | Disable AI entirely. Default in CI |

Configured per-project:

```toml
# .rustio/config.toml (committed to git)

[advice]
provider             = "claude"           # or "local-llama" / "none"
require_review       = true               # never auto-accept
include_explanations = true               # log reasoning to history.jsonl

[advice.claude]
model = "claude-opus-4-7"
# api_key is read from ANTHROPIC_API_KEY at runtime — never stored in repo
```

**The `none` setting is the default.** Rustio works perfectly without
any AI. The advisory layer is opt-in, per project, per developer, per
environment.

---

## 16. Domain experts — the deterministic AI tier

The most underrated part of this design. Some "AI" suggestions don't
need an LLM at all — they're domain rules that have been true for 20
years. These ship as deterministic Rust code:

```rust
pub struct ClinicDomainExpert;

impl AdviceProvider for ClinicDomainExpert {
    fn supports_domain(&self, hint: &DomainHint) -> bool {
        hint.matches_any(&["clinic", "medical", "hospital", "patient"])
    }

    async fn propose(&self, req: ProposalRequest) -> Result<Vec<Patch>, AdviceError> {
        if let Some(model) = req.last_added_model() {
            if model.name == "Patient" {
                return Ok(vec![
                    Patch::add_field("Patient", "date_of_birth", "date"),
                    Patch::add_field("Patient", "national_id", "text").unique().redact(),
                    Patch::add_field("Patient", "blood_type", "text").max(3),
                    Patch::add_field("Patient", "allergies", "text").nullable(),
                ]);
            }
        }
        Ok(vec![])
    }
}
```

A clinic developer gets the same suggestions whether Claude is
configured or not. The deterministic experts ship in the binary, work
offline, and have zero per-query cost. **Claude becomes the
"broad-domain fallback" when a deterministic expert doesn't match.**

Domain experts ship for the obvious verticals — clinic, retail/POS,
education, ERP, blog — and the project's `[advice.domain]` setting
controls which are active.

---

## 17. Field-type semantic inference

The cleverest AI capability and also the one with the highest
practical payoff. When the developer types
`rustio add field Patient phone text`, the advisor infers semantic
context from the field name:

```text
field name: "phone"
inferred semantic: PhoneNumber

Suggested modifiers:
  ✓ max = 32                    (E.164 max length)
  ✓ format = "E.164"            (normalize on save)
  ✓ searchable = true           (admin search by phone)
  ✓ redact_in_logs = true       (PII — protect via existing redact helpers)
  ✓ admin.list_display_align = "right"

Suggested validation:
  ✓ regex = "^\\+?[1-9]\\d{1,14}$"

Suggested index:
  ✓ BTREE on phone (for the admin search above)
```

| Field name pattern | Inferred semantic | Suggested treatment |
|---|---|---|
| `*email*`, `*_addr` | Email | Lowercase on save, unique-aware, search-prioritized, format-validated |
| `*phone*`, `*tel*`, `*mobile*` | Phone | E.164 normalize, PII-redact, max=32, indexed |
| `price*`, `*amount`, `*_cost`, `total*` | Money | `decimal(12,2)`, right-align, currency-aware, summary footer |
| `*_id` ending in singular model name | Foreign key | `belongs_to`, FK-hydrated list cell |
| `*_at`, `*_on` | Timestamp | `timestamptz`, monospace render, "x days ago" hover |
| `*_count`, `qty*`, `*_quantity` | Count | `integer`, right-align, no decimal places, summable in footer |
| `national_id`, `ssn`, `passport*` | Sensitive ID | Unique, redact-in-logs, mask-in-admin-list, full only on detail view |
| `*password*`, `*_token`, `*_secret` | Secret | **Refused** — these are framework-managed via `auth::*`, not model fields |
| `*_status`, `*_state` | Enum candidate | Suggest enum with workflow transitions; prompt for the values |

**Pure Rust, no LLM required, deterministic.** This is the
"feels-like-magic" capability that doesn't need a model provider. It
ships as `crates/rustio-admin-cli/src/advisors/semantic.rs` and is on
by default.

---

## 18. Anti-patterns this design forbids

These are the failure modes shipped by other "AI-augmented
frameworks". Each is explicitly blocked:

| Anti-pattern | Why forbidden | How we forbid it |
|---|---|---|
| AI writes directly to `src/_generated/` | Loses determinism; can't reproduce | CLI architecturally cannot — the generator's only input is `draft.toml` |
| AI writes directly to `draft.toml` | Loses human review | Advisor trait returns `Vec<Patch>`, never a draft mutation |
| AI is required for the framework to work | Vendor lock-in | Default `provider = "none"`; every Builder operation works without it |
| AI suggestions silently drift between versions | Compliance review impossible | `prompt_hash` + `draft_hash` + model version recorded in every proposal |
| Hot-path runtime calls an LLM | Air-gap-incompatible; latency unpredictable | The advisory layer is build-time only. Zero runtime LLM calls |
| Generated code edits land in human PR review without provenance | Audit trail breaks | Accepted proposals' `reasoning` is preserved in commit messages by `rustio commit` |
| AI suggests changes to authority code | Doctrine 22 violation risk | Patch ops are a closed enum: model/field/relation/admin/theme. No `op = "modify_auth"` |
| AI hallucinated a feature flag that doesn't exist | Silent half-implementation | Advisor calls into the same `features` registry the Builder uses; unknown features rejected at proposal time |

---

## 19. Privacy and prompt contents

A regulated industry will not accept "we send your schema to a third
party". The privacy contract:

1. **What's sent**: `draft.toml` contents, the developer's CLI
   request, and a project description if `[advice.context]` is
   enabled. Nothing else — no source code, no migrations, no `.env`,
   no database contents.
2. **What's logged locally**: every request as a `prompt_hash` (the
   prompt itself in `.rustio/proposals/<id>.toml` under
   `meta.prompt_text`, opt-in via `[advice.log_prompts = true]`).
3. **What's never sent**: anything under `app/` (developer-owned
   code), anything in `_generated/`, anything in `.env*`, anything
   matching `*.secret`, `*.key`, `*.pem`.
4. **Opt-out is one line**: `provider = "none"` in
   `.rustio/config.toml`, or `RUSTIO_ADVICE=none` in the environment
   for one-off operations.
5. **Air-gap mode**: `[advice] mode = "local-only"` — refuses to
   load any provider that isn't a deterministic expert or `local-*`.
   CI in regulated environments can lock this.

This contract gets its own design doc: `docs/design/DESIGN_ADVICE.md`,
same shelf as `DESIGN_AUDIT.md` and `DESIGN_SESSIONS.md`. Reviewed
against the doctrine, not the implementation.

---

## 20. CLI surface for the AI layer

```sh
# Solicit advice
rustio suggest                          # generic: "what should I do next?"
rustio suggest fields Product           # specific: fields for a model
rustio suggest relations                # cross-model: relations not yet defined
rustio suggest indexes                  # query-pattern-aware
rustio suggest migration                # review pending changes before commit

# Manage proposals
rustio review                           # interactive
rustio review --list                    # pending only
rustio review --history                 # all past, accepted+rejected
rustio review <id>                      # one specific

# Configure
rustio advice status                    # show active provider, recent activity
rustio advice provider <name>           # switch provider
rustio advice off                       # equivalent to `--provider none`

# Audit
rustio advice log                       # filtered history of advice events
rustio advice verify <proposal-id>      # re-runs prompt, checks reproducibility
```

`rustio advice verify` is the killer audit feature: re-run any past
proposal against the same model and confirm the output is
reproducible. If the model has drifted, the verification fails
loudly. This is what makes the AI layer **review-replay-trustable** in
a way no other AI dev tool offers today.

---

## 21. The Claude Code integration point

The same Claude Code agent the developer is using to *write* their
application can also invoke `rustio suggest` / `rustio review` as
part of its workflow:

```text
[Claude Code agent session]
> Let's add a new Order model to the POS project
$ rustio suggest fields Order
  ↳ Proposal 01HW... created
$ rustio review 01HW... --accept
  ↳ Accepted: 6 patches applied to draft.toml
$ rustio commit
  ↳ Generated: src/_generated/order.rs, migrations/0007_create_orders.sql
$ rustio migrate apply
$ cargo run
```

Claude Code doesn't *replace* the Rustio CLI — it *uses* it. Every
change to the application goes through `rustio review`, gets logged
in `history.jsonl`, and is reviewable by a human even when Claude
drove the session. The agent inherits Rustio's audit discipline
rather than bypassing it.

This is the right answer to "how do we let AI agents build with
Rustio?". They use the same CLI as a human, and the audit trail looks
identical.

---

## 22. The compliance pitch

> Rustio's advisory AI never writes to your source tree. It produces
> structured proposals that your developer reviews in a terminal, with
> every patch tagged by its reasoning, prompt hash, and model version.
> Accepted proposals flow into a deterministic generator whose output
> is byte-stable: rebuild the project from `draft.toml` six months
> later, on an air-gapped machine, and the bytes match. You can audit
> every build-time AI decision by reading `.rustio/proposals/` —
> they're part of the repo. You can disable AI entirely by setting
> `provider = "none"`. There is no LLM in the runtime path; the
> framework binary that serves your admin panel has never called an AI
> service in its life.

Every word is enforced by the architecture above, not by policy.

---

## Part 3 — Sequenced roadmap

| Version | Surface | AI involvement |
|---|---|---|
| **v0.13.1** | Lint/CI patch | None |
| **v0.14** | Foundation hardening + `DESIGN_BUILDER.md` | None |
| **v0.15** | Builder MVP (`new`, `add`, `commit`, `status`, `plan`, `log`) | None — deterministic CLI only |
| **v0.16** | Builder safety (`undo`/`redo`, `diff`, SchemaHash) | None |
| **v0.17** | `rustio import postgres://` — introspection of existing DBs | None |
| **v0.18** | Theme + branding via CLI | None |
| **v0.19** | **Semantic inference layer** (pure Rust, no LLM) | Deterministic only — phone/email/money/FK detection. Always on |
| **v0.20** | **Domain experts** (clinic/POS/education) | Deterministic only — hand-written Rust rules per vertical |
| **v0.21** | **LLM advisor — Claude provider** + `rustio suggest` / `review` | Optional, opt-in, behind a feature flag |
| **v0.22** | Provider abstraction + `local-llama` + `openai` | Optional, opt-in |
| **v1.0** | API freeze incl. `AdviceProvider` trait + proposal schema | — |
| **v1.x** | Studio with proposal-review UI | Optional |

**Critical**: 0.19 and 0.20 ship value without any LLM. The
deterministic experts and semantic inference are the AI-shaped
feature that compliance teams will accept on day one. Claude is the
*upgrade*, not the baseline.

---

## Part 4 — Open decisions

These shape downstream work and need answers before implementation
begins.

## Builder questions

1. **Target user for v0.15** — solo dev shipping an MVP, or team
   adopting an admin? (Drives CLI ergonomics.) Recommendation: dogfood
   target is **you** building Classrooms/POS on top of the Builder.
2. **Schema format** — TOML (chosen here, matches cargo ecosystem) or
   alternatives? TOML keeps everything in one syntax family.
3. **`rustio import postgres://...`** — does this stay in v0.17 or
   move earlier? For enterprise teams with existing 50-table legacy
   DBs, this is the killer command and may belong in the MVP.
4. **`history.jsonl` signing** — ship optional SHA-256 chain mode in
   v0.15 or wait? Compliance shops will want it from day one.

## AI layer questions

5. **Provider default for v0.21** — Claude only, or Claude + OpenAI
   from day one? Both early avoids appearing Anthropic-locked but
   doubles the test surface.
6. **Semantic inference (v0.19) policy** — on by default, or opt-in?
   Recommendation: on by default, but with `[advice] semantic = "off"`
   available.
7. **Proposal review in CI** — `rustio review --accept-all` for
   automated agents? Recommendation: yes, but require
   `RUSTIO_AGENT_MODE=1` plus `--reviewer "agent:claude-code"` so the
   verdict in `history.jsonl` records that no human approved it.
   Compliance teams can grep that.
8. **Air-gap mode default** — `[advice] mode = "air-gapped"` refuses
   to load any cloud provider regardless of other config. Worth
   shipping in v0.21 from day one.

## Doctrine questions

9. **`DESIGN_BUILDER.md`** — draft now as a doctrine document, even
   before any code lands? Same way the framework's session/audit
   doctrines preceded their implementations.
10. **`DESIGN_ADVICE.md`** — same question for the AI layer. Probably
    yes; the privacy contract alone justifies it.

---

## TL;DR

> **From `Template Engine` → `Project Compiler` → `Human-controlled
> AI Compiler`.**
>
> `draft.toml` is the declarative source of truth. `history.jsonl` is
> the append-only audit trail of build-time intent. `_generated/` is
> generator-owned, `app/` is developer-owned, `migrations/` is sacred.
> `rustio plan` / `rustio commit` mirror `terraform plan` / `apply`.
> Claude proposes patches, the developer disposes, Rustio executes.
> Same doctrines that protect runtime authority now protect the act
> of building the application itself.
>
> Templates die. The Builder is the answer. The AI layer is the
> upgrade — not the baseline.
