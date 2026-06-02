# Project Memory — Implementation Design

This is the implementation design for **CLOUD.md**, the project-memory
why-layer whose architecture and invariants are fixed by
[`DESIGN_CLOUD.md`](./DESIGN_CLOUD.md). That contract deliberately deferred
six decisions (its §14): artifact format/location/storage, the
substrate-fit choice, the command surface, retrieval, capture-prompt
mechanics, the `.rustio/ai.toml` capability keys, and the audit record
shape. This document resolves them.

It **may not relax** the contract's three invariants (`DESIGN_CLOUD.md`
§3): **Subordinate forever**, **Append-only**, **Human-ratified**. Every
decision below cites the invariant or contract clause it serves, and §13
is an explicit conformance check.

Pull request review runs against this document *and* `DESIGN_CLOUD.md` —
the contract is the higher authority; where this design and the contract
ever disagree, the contract wins and this document is the bug.

**Status: Approved — 2026-06-02 (Abdulwahed Mansour).** This design is
approved as the basis for implementation. No code exists yet; naming, exact
flags, and on-disk byte layout remain open to refinement in implementation
PRs (§14), but the *shape* of the decisions is ratified and binding.
Implementation PRs are reviewed against this document and the governing
contract (`DESIGN_CLOUD.md`); none may relax Invariants I–III.

> **Doctrine inheritance**
> This design adds **no new runtime and no new trust primitive**
> (`DESIGN_CLOUD.md` §13). Memory-write reuses the AI
> proposal→approval→audit lifecycle of `DESIGN_AI_ASSISTANT.md` unchanged;
> the record is the audit trail of `DESIGN_AUDIT.md`; approver roles come
> from `DESIGN_PERMISSIONS.md`. The CLI verbs follow the
> printer/generator pattern of `rustio ai` / `rustio theme` — pure
> local tools that never call a model. The running admin never reads
> CLOUD.md (§12).

---

## 1. Purpose and scope

### 1.1 What this resolves

| Contract §14 item | Resolved in |
|---|---|
| Format, location, storage | §2 |
| Substrate-fit (reuse vs purpose-fit) | §2.4 |
| Entry model | §3 |
| Lifecycle / governance integration | §4 |
| `.rustio/ai.toml` capability keys + buckets | §5 |
| Audit record shape | §6 |
| Command surface | §7 |
| Capture-prompt mechanics (contract §5.1) | §8 |
| Retrieval (contract §7) + foundational protection | §9 |
| ADR promotion mechanics (contract §8) | §10 |

### 1.2 What this does not cover

- **Assistant prompt engineering.** *How* an external assistant decides
  relevance or phrases a capture suggestion is assistant-side behaviour,
  not framework code (§9, §12). This design specifies the *surfaces and
  conventions* the assistant uses, never the model.
- **The ADR system itself.** Promotion (§10) writes an ADR via the
  project's existing ADR process; this design only defines the *link*
  between a memory entry and the ADR it spawns.

---

## 2. Storage model — per-entry files, generated human view

### 2.1 Decision — entries are immutable per-entry files; CLOUD.md is a generated view

The **canonical store is a directory of immutable, per-entry files** under
`.rustio/memory/entries/` — one file per applied entry, version-controlled.
The project-root **`CLOUD.md` is a generated, human-readable concatenation**
of those entries, produced by `rustio memory render` and **never
hand-edited**.

This **reverses an earlier draft** (a single canonical append-structured
CLOUD.md) after review. A single shared file reintroduced three failure
modes a per-entry store eliminates:

- **Merge safety (review #2).** Each entry is its own file named by its
  ULID, so two branches that each add an entry create two *different* new
  files and **never conflict.** This is what the contract's §9 ("append-only
  makes concurrent writers safe") actually requires — a property a single
  shared-tail file did **not** have, since concurrent appends collide on the
  same trailing lines. Conflicts can now occur only in the *generated*
  CLOUD.md, and those are resolved by re-running `render`, never by hand.
- **Structural append-only (review #1, #3).** Entry files are **write-once.**
  Append-only stops being a convention policed against git history and
  becomes a property of how the store is written: tooling creates new files
  and never rewrites an existing one — with the single bounded exception of
  redaction (§3.4). `verify` (§11) shrinks from load-bearing archaeology to
  a light backstop.
- **Derived status (review #1).** Whether an entry is superseded is
  **computed**, not stored (§3.3) — so no prior entry is ever mutated to
  flip a status flag, closing the in-place-edit contradiction the earlier
  draft carried.

### 2.2 Why a generated CLOUD.md is *not* a "build step"

The earlier draft rejected a generated view as "smelling like a build
step." Review showed that objection was mis-scoped. The **no-build-step
rule governs the runtime path** (no bundler/transpiler in the request
path), and there is a direct, sanctioned precedent: **`rustio theme
generate` emits `tokens.css`** (`DESIGN_THEME.md`) — an opt-in developer
command whose committed output is read directly. CLOUD.md stands in exactly
that relation to its entry files as `tokens.css` does to a brand color:
canonical input → generated artifact → no runtime involvement. The
two-source-ambiguity worry is resolved the same way theme resolves it — the
generated CLOUD.md carries a **"generated by `rustio memory render` — edit
entries, not this file"** banner, and the entry files are the unambiguous
source.

> **Confirmed intent — 2026-06-02 (Abdulwahed Mansour).** This reframing was
> raised in review (finding I): making the feature's namesake file a
> *generated projection* over the canonical `.rustio/memory/` store shifts
> where the truth-of-memory lives relative to the evocative filename. It is
> consistent with the contract's own "CLOUD.md names the concept, not a
> committed format" (`DESIGN_CLOUD.md` §14), and is **confirmed as the
> intended shape**: `entries/` is canonical, CLOUD.md is the enforced-fresh
> human/assistant view (§2.6).

### 2.3 Layout

```
<project root>/
├── CLOUD.md                       # generated human view (committed, do-not-edit banner)
└── .rustio/memory/
    ├── entries/<ulid>.md          # canonical, write-once, one per applied entry
    ├── proposals/                 # Suggested entries (staging) — §2.4
    └── index.json                 # derived cache — §2.5
```

ULID filenames give each entry a **stable, collision-free identity** and a
cheap default sort. One subtlety (review pass 2, finding F): an entry's
ULID is its *proposal* id, stamped at **Suggested** time, while the entry
enters memory at **Applied** time (after ratification, possibly later). So
filename order is order-of-*suggestion*. Where chronology matters, `render`
and `show` order by the `date:` field — defined as the **apply
(ratification) date** — so the human-facing timeline reflects when memory
actually recorded a decision, not when it was first drafted. The audit join
remains the UUID v7 `correlation_id`.

### 2.4 Substrate-fit decision (contract §14 / review #5)

Unchanged in spirit, now concrete: **Suggested** entries are staged as
ordinary AI proposals under `.rustio/memory/proposals/` (a sibling of
`entries/`, mirroring the `DESIGN_AI_ASSISTANT.md` proposal substrate). On
`Applied`, the proposal is written **once** into `entries/<ulid>.md` and
CLOUD.md is re-rendered. Lifecycle reuse is total; the durable artifact is
purpose-fit.

### 2.5 Derived index — a cache, never a source

`.rustio/memory/index.json` is a **regenerable mechanical cache**: per
entry, its id / type / subjects / supersedes-link (status is *derived* from
the supersedes-links, §3.3), plus reference and supersession counts, plus
**the set of subjects in use** (for capture-time suggestion, §3.5). It
exists to make filtering and ADR-candidate counting fast. It is **never a
source of truth**, holds **no content interpretation** (counts and links
only — `DESIGN_CLOUD.md` §8.2, §13), and is fully rebuilt from `entries/` by
`rustio memory index`. Deleting it loses nothing.

### 2.6 The assistant's read path and view freshness (review pass 2, finding B)

The canonical source is **always `entries/`**; CLOUD.md is a convenience
projection. Two read paths, with one freshness rule:

- **Broad / session-start read → CLOUD.md.** The assistant reads the
  generated CLOUD.md for the narrative overview, as the contract frames it
  (`DESIGN_CLOUD.md` §1). Because CLOUD.md is *derived*, this is safe only
  if it is **never stale**, so render-freshness is **enforced, not
  optional**: `apply` always re-renders (§2.4), and `rustio memory verify` /
  CI **fails if CLOUD.md does not equal `render(entries/)`** (§11, §14). A
  stale view is a build failure, not a nicety.
- **Targeted retrieval → tooling.** Relevance queries — notably
  rejected-idea resurfacing (§9.2) — go through `rustio memory show`, which
  reads `entries/` directly and is therefore **always fresh** regardless of
  CLOUD.md's render state.

This closes the view-can-lag-canonical gap the per-entry model would
otherwise introduce: the toolchain keeps CLOUD.md in lock-step with its
source, and anything precise reads the source directly.

---

## 3. The entry model

### 3.1 Per-entry file — Markdown with TOML frontmatter

Each entry is a Markdown file with a **TOML frontmatter** header (fenced by
`+++`) and a prose body. This replaces the prior draft's bespoke
HTML-comment metadata block (review #7): frontmatter is a boring, ubiquitous
format with off-the-shelf parsers, diffs cleanly line-by-line, and renders
natively in common viewers — consistent with "no magic," and far less
fragile than parsing structured data out of a comment.

**Refined from YAML to TOML (implementation finding).** The contract left
metadata encoding to this design, and the original §3.1 draft picked YAML.
Implementation showed YAML would add a **new dependency** (`serde_yaml`),
whereas TOML is already the project's config language (`.rustio/ai.toml`)
and `toml_edit` is **already a CLI dependency** — so TOML frontmatter costs
**zero new dependencies** while keeping the exact property that motivated
frontmatter (a boring, off-the-shelf parser, no bespoke format). This
refines an encoding detail only; it touches **no invariant** (CLAUDE.md:
"don't add dependencies unless they clearly earn their place").

```
+++
id = "01J9Z…"                # ULID; also the filename
type = "rejected"            # see §3.2
subjects = ["auth", "sessions"]  # mechanical retrieval keys (§3.5)
supersedes = ""              # entry id this replaces, if any ("" = none)
foundational = false         # exempt from recency demotion (§9); curated, rare (§3.6)
sources = ["pr#41"]          # cite-or-hedge (DESIGN_CLOUD.md §10)
author = "ai:claude-code"
ratified_by = "amir@team"
date = "2026-06-02"          # apply (ratification) date — the render/show order key (§2.3)
correlation_id = "0190…"     # UUID v7; joins to rustio_admin_actions
+++

We considered LISTEN/NOTIFY for the job runner and rejected it because …
```

No field is free-form authority; the prose is *reasoning*, never
specification (`DESIGN_CLOUD.md` §2.3). There is deliberately **no `status`
field** — status is derived (§3.3).

### 3.2 Entry types

`decision`, `rejected`, `assumption`, `intent`, `onboarding`, `history`,
and `open-tension`. **`open-tension` is not a special mechanism** — it is
an ordinary entry whose prose records an unresolved disagreement and that
**closes by supersession** like any other (`DESIGN_CLOUD.md` §9).

### 3.3 Supersession — derived, never mutated (review #1)

An entry file is **immutable once written.** "Superseded" is **not a stored
flag**; it is **derived**: entry A is superseded iff some later entry names
A in `supersedes:`. The index (§2.5) computes this, and the generated
CLOUD.md marks superseded entries — visibly demoted, **never removed**
(`DESIGN_CLOUD.md` §3 II, §7). No prior entry is ever edited to record that
it was superseded; this is what closes the in-place-edit contradiction the
earlier draft carried.

**Referential integrity (review pass 2, finding D).** `supersedes:` links
are validated by `rustio memory index` / `verify`, which reject a
**dangling** link (target id never applied) and a **cyclic** graph. Because
ids are time-ordered and files are write-once, a cycle should be impossible
in normal operation; validation guards against a malformed import or a hand
edit.

**The merge-fork case (review pass 2, finding E).** Two branches may each
supersede the same entry A, so after a merge A has **two live successors**.
The derived-status logic must **not** silently pick one as "current": per
the contract's surface-don't-resolve rule (`DESIGN_CLOUD.md` §9), a
multiply-superseded entry whose successors are themselves unreconciled is
rendered as an **open tension** (§3.2) — both successors shown, the fork
made visible — until a human appends a superseding entry that resolves it.
Append-only made *adding* entries conflict-free (§2.1); this is where the
collision it relocated into the supersession graph is handled.

### 3.4 Redaction — the one in-place operation (aligned taxonomy, review #8)

Redaction (`DESIGN_CLOUD.md` §3, the bounded exception) is the **sole**
operation that rewrites an existing entry file. It replaces the prohibited
**body** with a recorded marker, leaving the frontmatter (id, type, date,
`correlation_id`) intact so the *act* stays bounded and attributable:

```
> [content removed: class=token · 2026-06-02 · by=amir@team · audit=0190…]
```

The `class` **reuses the framework's existing redaction taxonomy** rather
than a parallel one (`admin/redact.rs` + `filters.rs::mask_pii`):
`password`, `token`, `mfa_secret`, `backup_code`, `pii` — plus `credential`
and `operational` for the memory-specific cases. Redaction is used **only**
for prohibited content, **never** to revise reasoning — wrong or outdated
entries are *superseded*, not redacted. It is the only write `verify` (§11)
permits against an existing entry file.

**Redaction is not history scrubbing (review pass 2, finding A).**
Redaction removes prohibited content from the **working tree and the
going-forward record** only. Because entry files *and* the generated
CLOUD.md are git-committed, the pre-redaction bytes **persist in git
history**, and in every existing clone and fork. Redaction therefore stops
the leak *forward*; it does not erase the past. A genuinely-leaked
credential additionally requires, **out of band and outside this system**:
(1) rotating/invalidating the exposed secret, and (2) rewriting history
across all copies (`git filter-repo` / BFG). The framework's responsibility
ends at the working tree and the audit record (`memory_redacted`, §6); true
remediation of a live secret is an operational action this design neither
performs nor claims to. Operators must not read "redacted" as "the secret
is gone."

> **Proposed amendment to the approved contract (`DESIGN_CLOUD.md` §3).**
> The redaction exception should carry one clarifying sentence — *"Redaction
> removes prohibited content going forward; it does not remove it from
> version-control history, which additionally requires secret rotation and
> an out-of-band history rewrite."* This is honesty, not a relaxation of
> Invariant II, but it edits an approved contract and so is the owner's
> call (§14).

### 3.5 Subjects — a convention-controlled vocabulary (review #4)

`subjects` are the mechanical retrieval keys, and the decisive
rejected-idea-resurfacing feature (§9.2) depends on them matching. To fight
three-year tag drift ("auth" vs "authentication" vs "login"),
`rustio memory remember` **suggests existing subjects** from the index at
capture time (mechanical listing of known tags — *counting, not
interpretation*). Subjects are thus a **convention-controlled vocabulary**,
not free-for-all. Crucially, **resurfacing does not rely on exact subject
match alone**: the assistant's *semantic* match over subjects **and** prose
is the primary retrieval path (§9); `--subject` is a deterministic aid, not
the sole mechanism — so a tag mismatch degrades, rather than silently
disables, the feature.

### 3.6 Foundational — rare and curated (review #10)

`foundational: true` exempts an entry from recency demotion (§9.3). If
over-applied it refloods retrieval and recreates the archive problem
(`DESIGN_CLOUD.md` §11.2), so it is **curated, not casual**: marking an
entry foundational — and superseding one that is — **requires the second
approver** (§5). `rustio memory stats` reports the foundational count so
proliferation is visible and reviewable.

---

## 4. Lifecycle and governance integration

Memory-write **is an AI capability** and flows through the existing
pipeline unchanged (`DESIGN_AI_ASSISTANT.md` §4):

```
Suggested  →  Reviewed  →  Approved  →  Applied
                                    ↘  Rejected
```

- A **Suggested** memory entry is a staged proposal, **not yet in
  CLOUD.md** (§2.4).
- **Apply is separate from approval**; `Applied` is the step that **writes
  the immutable `entries/<ulid>.md` and re-renders CLOUD.md** (for
  `write_memory`/`supersede_memory`) or **performs the recorded excision and
  re-renders** (for `redact_memory`).
- There is exactly **one approval pipeline** — memory does not get a
  parallel governance path. This is what keeps the contract's
  human-ratification gate (Invariant III) honest and singular.

---

## 5. Capability keys and policy (`.rustio/ai.toml`)

The contract fixed the bucket to `needs_approval` (`DESIGN_CLOUD.md`
§4.1). This design proposes **three distinct keys** for differential risk
and clear audit, rather than one opaque `write_memory`:

| Capability | Bucket | Notes |
|---|---|---|
| `write_memory` | `needs_approval` | Append a new entry. |
| `supersede_memory` | `needs_approval` | Append an entry that supersedes another. |
| `redact_memory` | `needs_approval` **+ `second_approver_for`** | The destructive, irreversible excision (§3.4) — two approvers, matching how the existing policy treats `modify_table` / `apply_migration`. |

Notes:

- **There is no `edit_memory` key** — by omission, in-place revision of
  reasoning is *uncapability*, reinforcing append-only at the policy layer.
- **Foundational entries are second-approver-gated.** Marking an entry
  `foundational: true`, or superseding one that is, requires two approvers
  (§3.6) — the curation guard against foundational proliferation.
- The AI may **prepare** any of these (Suggested) on its own authority but
  may never **apply** one without human approval — never `Allowed`, never
  `Blocked` (`DESIGN_CLOUD.md` §4.1).
- `edit_ai_policy` remains **Blocked** (`DESIGN_AI_ASSISTANT.md` §2) — the
  AI still cannot widen its own memory access.

---

## 6. Audit record shape

Reuse the existing typed events for the lifecycle, add one for the
destructive op:

- `write_memory` / `supersede_memory` approve/reject/apply reuse the
  existing **`ai_proposal_approved` / `_rejected` / `_applied`** variants
  (`DESIGN_AI_ASSISTANT.md` §5), with `metadata` carrying the capability,
  entry id, subjects, and (for supersede) the superseded id. No new
  variant needed — the lifecycle is identical.
- **Redaction adds one new typed variant — `memory_redacted`** — because
  the contract *requires* recording the **class of content removed, by
  whom, when** (`DESIGN_CLOUD.md` §3), which `ai_proposal_applied` does not
  capture. It records `class` (the §3.4 taxonomy, aligned with
  `admin/redact.rs`), `entry_id`, approver, and correlation id.

Every memory action is therefore attributable and correlation-joined per
`DESIGN_AUDIT.md`, satisfying audit-by-default.

---

## 7. Command surface

**One namespace, one pipeline (review #5).** The whole memory workflow
lives under `rustio memory`, so a developer never crosses namespaces
mid-task. The lifecycle verbs (`pending` / `approve` / `apply` / `reject`)
are **thin wrappers that delegate to the single shared governance engine of
`DESIGN_AI_ASSISTANT.md`** — identical approval rules, identical audit, no
parallel governance. The `rustio ai` verbs continue to work on memory
proposals too; `rustio memory` is the memory-scoped front door over the
same machinery, not a second pipeline.

**Scope (review #6).** To keep the surface narrow, only a **v1 core** ships
first; analytical verbs are **deferred** until the core is in use.

### 7.1 v1 core

| Command | Does |
|---|---|
| `rustio memory remember --type <t> --subject <s,…> [--foundational] [--source <ref>] --note "<prose>"` | Create a `write_memory` proposal (Suggested). Suggests existing subjects (§3.5). |
| `rustio memory supersede <id> --note "<prose>" […]` | Create a `supersede_memory` proposal (sets `supersedes:`). |
| `rustio memory redact <id> --class <password\|token\|mfa_secret\|backup_code\|pii\|credential\|operational> --reason "<…>"` | Create a `redact_memory` proposal (two approvers). |
| `rustio memory pending` / `approve <id>` / `apply <id>` / `reject <id>` | The memory-scoped lifecycle front door — delegates to the shared pipeline (§4). |
| `rustio memory show [--subject S] [--type T] [--active\|--include-superseded] [--grep TEXT]` | Deterministic filtered listing — mechanical only, never relevance ranking (§9). |
| `rustio memory render` | (Re)generate the human-readable CLOUD.md from `entries/`. Idempotent; the only writer of CLOUD.md. |
| `rustio memory verify` | Backstop check that committed entry files are unchanged except authorised redactions — the `DESIGN_CLOUD.md` §11.4 breach signal (§11). |

Writes **only create Suggested proposals**; nothing bypasses approval.
`apply` is the step that writes the immutable `entries/<ulid>.md` and
re-renders CLOUD.md (§2.4).

### 7.2 Deferred (later, once the core is in use)

| Command | Does |
|---|---|
| `rustio memory index` | Rebuild the derived cache (§2.5). Normally auto-maintained on apply; manual rebuild is a recovery tool. |
| `rustio memory chain <id>` | Show an entry's supersession lineage. |
| `rustio memory stats` | Mechanical counts per subject/type; reports the foundational count (§3.6). |
| `rustio memory promote-candidates` | Entries crossing mechanical count thresholds, flagged as ADR candidates — suggests, never promotes (§10). |

---

## 8. Capture-prompt mechanics (contract §5.1)

The contract's rule: **never interrupt mid-flow; bias to under-capture;
batch; silence by default.** This design realises capture **as ordinary
low-priority proposals, not modal prompts**:

- At a natural decision boundary — typically right after a developer
  ratifies a code/migration decision (`rustio ai apply`) — the assistant
  *may* create a `write_memory` proposal capturing the *why*. It enters the
  **Suggested** queue; it does **not** pop a dialog.
- The developer reviews candidates **at their own pace** via `rustio memory
  pending` and ratifies in batch through the normal apply flow. The "ask"
  is a pending item, not an interruption.
- **Default is silence**: a routine session that made no durable decision
  produces zero memory proposals. Manual capture is always one
  `rustio memory remember` away (frictionless under-capture, contract §5.1).
- The framework cannot and does not force assistant prompting behaviour
  (§12); it provides the queue and the convention. The assistant's own
  instructions (e.g. the project's `CLAUDE.md`) carry the "capture only
  durable, non-recoverable reasoning; never conversational scaffolding"
  rule from `DESIGN_CLOUD.md` §5.

---

## 9. Retrieval and the assistant workflow (contract §7, §13)

Per the contract, the framework keeps memory **well-formed and findable**;
the **external assistant** performs relevance and resurfacing. No semantic
index and no model live in the framework.

**Framework provides (mechanical only):** structured `subjects`/`type`/
`supersedes` fields and derived status (§3), the deterministic filters of
`rustio memory show` (`--subject`, `--type rejected`, `--active`,
`--grep`), supersession chains, and counts. These are the *retrievability*
guarantees.

**Assistant performs (documented workflow, not framework code):**

1. **Session start** — read the (enforced-fresh, §2.6) CLOUD.md for active
   intent and assumptions; use `rustio memory show` for anything targeted.
2. **Rejected-idea resurfacing (the decisive requirement,
   `DESIGN_CLOUD.md` §7.1)** — *before proposing a change on topic X*, the
   assistant checks memory for a prior rejection and, if one exists,
   surfaces it ("X was rejected on … because …") instead of re-proposing.
   The **primary** match is the assistant's *semantic* read of CLOUD.md
   over both `subjects` and prose; the mechanical `rustio memory show
   --type rejected --subject X` is a deterministic **aid**, not the sole
   path — so subject-tag drift (§3.5) *degrades* rather than silently
   *disables* resurfacing. Two honest dependencies remain, both already
   conceded by the contract (`DESIGN_CLOUD.md` §7, §12.6): the assistant
   must *remember to check* before proposing, and its retrieval must be
   good. The framework guarantees findability, not that memory is found.
3. **Foundational protection (review #4/#10, contract §7 req 3)** —
   entries with `foundational: true` (curated, second-approver-gated, §3.6)
   are always surfaced for their subject and **exempt from recency
   demotion**. The framework stores and exposes the flag; the assistant
   honours it. The failure mode guarded against: the oldest load-bearing
   intent becoming the least findable.

The contract is honest that resurfacing is **conditional on the assistant's
retrieval quality** (`DESIGN_CLOUD.md` §7, §12.6): the framework guarantees
findability, not that it is found.

---

## 10. ADR promotion mechanics (contract §8)

- **Detection is mechanical** (`DESIGN_CLOUD.md` §8.2): `rustio memory
  promote-candidates` flags entries with high reference/supersession counts
  — *counts, not content*. The assistant may *suggest* promotion;
  interpretation of meaning stays assistant-side (§13 of the contract).
- **Promotion is human-ratified and is link-not-move**
  (`DESIGN_CLOUD.md` §8.3): the memory entry is **not deleted**. Promotion
  appends a `supersede_memory` entry that carries an `adr: <path>` field
  and marks the original `superseded`. The ADR (written via the project's
  ADR process) becomes the normative record; memory keeps the narrative and
  points at it. Memory must never re-narrate the ADR's content.

---

## 11. Append-only enforcement and the redaction exception

Append-only is **structural, not merely policed** (the point of the §2
per-entry model). Two layers:

- **Tooling-immutable (primary).** Entry files are **write-once *by the
  tooling***: applying a proposal creates a new `entries/<ulid>.md` and no
  command ever rewrites an existing one. Status is derived (§3.3), so
  supersession adds a file rather than editing one — there is **no edit
  path** in the command surface or the capability keys (§5, §7), and
  CLOUD.md is a generated artifact (§2.1), not an integrity target. Be
  precise about the strength of this claim (review pass 2, finding C): the
  filesystem and git do **not** enforce immutability — a hand or a stray
  tool can still edit a file. What the per-entry model buys over the
  single-file draft is that any such edit is an isolated, obvious per-file
  diff that the backstop below catches.
- **Backstop (secondary).** **`rustio memory verify`** confirms that
  committed entry files are byte-unchanged across history *except* for
  authorised, audit-recorded redactions — the `DESIGN_CLOUD.md` §11.4 breach
  signal, surfaced mechanically. Honest limitation: this check reads git
  history, so it **degrades under squash-merge or force-push**; it is a
  backstop on the structural property, not the primary guarantee.
  Recommended as a CI status on PRs that touch `entries/` (§14).
- **The one exception.** Redaction (§3.4) is the **sole** operation that
  rewrites an existing entry file's body — gated by two approvers (§5),
  recorded via `memory_redacted` (§6), and used only to satisfy the
  contract's absolute prohibition (§2.3) without colliding with append-only
  (`DESIGN_CLOUD.md` §3). Note its real limit: redaction is **not**
  git-history scrubbing (§3.4) — it cleans the working tree forward, not
  the past.

---

## 12. What this design explicitly does NOT add

- **No runtime read of CLOUD.md.** The running admin (hyper request path)
  never reads project memory. CLOUD.md is a **dev-time artifact** consumed
  by an external assistant and the CLI only. This preserves "no second
  runtime" and keeps memory entirely outside request handling.
- **No embedded model, planner, or semantic index** in the framework
  (`DESIGN_CLOUD.md` §13). Relevance is the assistant's; the framework
  counts and filters, never interprets.
- **No new database table is required** for memory content — the
  version-controlled per-entry files are the store (§2). (The audit
  *mirror* uses the existing `rustio_admin_actions`; no schema-driven memory
  engine, no Tier-2 symbols.)
- **No widening of AI authority** — `edit_ai_policy` stays Blocked; memory
  keys are `needs_approval`.

---

## 13. Invariant conformance check

| Contract invariant | How this design conforms |
|---|---|
| **I — Subordinate forever** | CLOUD.md is dev-time only, never read at runtime (§12); entries are reasoning, never specification (§3.1); the assistant treats it as context, never authority. |
| **II — Append-only** | Per-entry files are **tooling-immutable** (write-once by the toolchain, not filesystem-enforced — §2.1, §11/C); status is **derived, not mutated** (§3.3); no `edit_memory` capability (§5); redaction is the single bounded, recorded exception and is *not* history scrubbing (§3.4); `verify` backstops it and checks view-freshness (§11, §2.6). |
| **III — Human-ratified** | All three capabilities are `needs_approval` (§5); apply is separate from approval (§4); capture is a queued proposal, never an autonomous write (§8). |

If any future PR breaks a cell in this table, it breaks the contract, not
just this design.

---

## 14. Open questions — resolution

The implementation (PRs #7–#12 + the lifecycle refactor and the analytics /
audit-mirror / tamper slices) closed every open item. None relaxed
Invariants I–III.

**Settled by implementation:**

- **Metadata encoding** — TOML frontmatter (§3.1; refined from YAML to add
  no dependency).
- **Staging location** — `.rustio/memory/proposals/` (§2.4).
- **Entry id** — ULID filename + UUID v7 audit join (§2.3/§3.1).
- **Foundational second-approver gate** — `required_approvals` floor (§3.6/§5).
- **Contract amendment (`DESIGN_CLOUD.md` §3)** — **applied** (redaction is
  not history scrubbing), and surfaced at apply time as an operator warning.
- **Freshness + tamper enforcement mechanism** — `rustio memory verify` is
  the gate: it fails (non-zero exit) on a stale `CLOUD.md`, a referential
  error (dangling/cyclic supersedes), or a working-tree entry change outside
  a ratified redaction (§11). **Run it in CI** on any change touching
  `.rustio/memory/` or `CLOUD.md`. Honest caveat (already in §11): the
  git-based tamper check compares the working tree against HEAD only — it is
  a backstop and degrades under squash-merge / force-push; it does not
  forensically audit committed history.

**Considered and declined** (not worth the surface, per the design's own
scale and ethos):

- **CLOUD.md git merge driver** — manual `rustio memory render` on a
  conflict in the generated view is sufficient; a `.gitattributes` merge
  driver is not warranted.
- **`gitignore`-and-generate VCS mode** — `CLOUD.md` stays committed so the
  assistant and reviewers read it without running tooling. (Reviewer
  ergonomics: an applied entry shows two diffs — the canonical
  `entries/<ulid>.md` and the regenerated `CLOUD.md`; review the entry file,
  the view diff is generated.)
- **`entries/` directory sharding** — the under-capture design keeps the
  count to tens–low-hundreds; a flat directory is fine. Blowing past it is a
  capture-discipline smell (§11.2 archive death), not a storage problem.

The implementation now matches this design end to end.
