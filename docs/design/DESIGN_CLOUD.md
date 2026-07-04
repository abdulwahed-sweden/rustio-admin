# Project Memory (CLOUD.md)

> **In one minute — what this is.**
> CLOUD.md is your project's **memory of *why*** — its business intent, the
> decisions taken, and especially the **ideas you tried and rejected**. A new
> teammate or an AI assistant reads it to understand years of reasoning in
> minutes, instead of re-deriving — or re-litigating — it.
>
> Three things define it, and they never change:
> - **Subordinate to code.** It is *not* a source of truth — code, schema,
>   and the database are. Memory explains *why*; it never says *how the
>   system works*. On any conflict, **code wins.**
> - **Append-only.** You add to memory; you never rewrite its history. A
>   decision that changes is *superseded* by a new entry, not edited away.
> - **Human-ratified.** Nothing is recorded without a person approving it.
>
> **New here?** That is all you need to use it well — the practical guide is
> [`docs/memory.md`](../memory.md) (commands, recipes, CI). Everything below
> is the formal **contract** (deliberately dense) for people changing how
> memory itself works.

A rustio-admin project keeps a single, project-level memory of *why* it is
the way it is — its business intent, the decisions taken, the ideas
considered and rejected, the assumptions accepted, and the history a new
developer (or a new AI agent) needs to understand years of reasoning
quickly. That memory is called **CLOUD.md**.

CLOUD.md is **not** a source of truth. Code is. Schema is. The database
is. CLOUD.md is the *why-layer* that sits beside them and never above
them. It answers **"why was this decided?"** — never **"how does this
system work?"**

This document is the contract for that memory — what it is, what it may
never become, the invariants that keep it safe, how entries are captured
and governed, how they are retrieved years later, how memory relates to
ADRs, how it behaves across a team, the ways it can fail, and the
criteria by which it is judged a success.

Pull request review runs against this document, not only the diff.

**Status: Approved — 2026-06-02 (Abdulwahed Mansour).** Amended 2026-06-02
(§3 redaction exception clarified: redaction is not history scrubbing —
surfaced by the implementation design, `DESIGN_CLOUD_IMPL.md`). This is an
approved **design contract.** It defines architecture, invariants, and
governance. It deliberately does **not** define file format, syntax,
command surface, storage location, or runtime behaviour — those are
implementation decisions to be designed and approved separately, against
this contract. Implementation may now begin under a separate design that
this document governs; none of it may relax Invariants I–III (§3).

> **Doctrine inheritance**
> This layer adds **no new trust primitives and no new runtime.** It
> composes the ones the framework already owns. The capability to write
> memory is governed by the AI permission/approval lifecycle in
> `DESIGN_AI_ASSISTANT.md`; the durable record of every memory action is
> the audit trail in `DESIGN_AUDIT.md`; the append-only, reversible
> discipline mirrors the deterministic builder in `DESIGN_BUILDER.md`; who
> may ratify a memory comes from `DESIGN_PERMISSIONS.md`. CLOUD.md is an
> *artifact governed by existing systems*, never a privileged actor of its
> own. It does not embed a model, a planner, or retrieval intelligence —
> see §13 (scope).

---

## 1. Purpose

### 1.1 What this governs

- What CLOUD.md **is**: project memory, decision memory, rejected ideas,
  accepted assumptions, business intent, onboarding context, project
  history, and the reasoning behind decisions (§2).
- The three invariants that make it safe: **Subordinate**, **Append-only**,
  **Human-ratified** (§3).
- What may be captured, what must never be captured, and when to ask
  (§5, §6).
- How memory is governed, retrieved, related to ADRs, and reconciled
  across a team (§4, §7, §8, §9).
- How it fails, and how to recognise drift toward failure (§11, §12).

### 1.2 What this does not cover

- **The AI model, planner, or retrieval engine.** The intelligence that
  reads memory and decides relevance runs *outside* the core framework,
  exactly as in `DESIGN_AI_ASSISTANT.md` §8. This contract governs the
  *artifact and its governance*, not the reasoning over it (§13).
- **File format, syntax, storage location, or commands.** Deferred to a
  separately-approved implementation design.
- **Runtime behaviour.** CLOUD.md never participates in request handling,
  never gates a runtime decision, and is never read by the running admin.

### 1.3 Closing principle

**The developer must eventually trust CLOUD.md, but must never fear it.**
Fear comes from memory that acts on its own or that a developer has to
defend against. Trust comes from memory that is subordinate to code,
honest about its own age, captured only with human consent, and never
rewritten. If a developer ever thinks *"I need to go fix the memory
file,"* the design has failed.

---

## 2. What CLOUD.md is — and is not

### 2.1 The mental model

CLOUD.md is a **project conversation history**, not a project
specification. It is a commentary track on the codebase: it explains why
the system was built this way and preserves the paths not taken. It is
worthless — and dangerous — the moment it is mistaken for the system
itself.

A specification is *normative* (states what must be), read as current,
and rewritten to stay true. A conversation history is *descriptive*
(records what was decided and why), read as chronology, and never
rewritten — you only append the next turn. CLOUD.md is strictly the
latter.

### 2.2 What it preserves

| Category | Example | Why it has no other home |
|---|---|---|
| **Business intent** | "Built for small clinics; never multi-region." | Code cannot express purpose. |
| **Decisions + rationale** | "Chose double-submit CSRF because…" | Code shows the *what*, not the *why*. |
| **Rejected ideas + reasons** | "Rejected LISTEN/NOTIFY for jobs because…" | *Nothing else in the stack records what was killed.* Highest value. |
| **Accepted assumptions** | "Assume single-tenant; admins are trusted." | Soft constraints that shape reasoning but are enforced nowhere. |
| **Onboarding context** | "Read `DESIGN_SESSIONS.md` before touching auth." | The orientation a senior engineer gives verbally. |
| **Project history** | "Pivoted off the original billing model in Q3 because a client churned." | The story, not the diff. |

### 2.3 What it must never become or contain

CLOUD.md must **never** be, duplicate, or contain:

- A **source of truth** — code, schema, and database are truth, forever.
- **Model definitions, field definitions, migrations, runtime
  configuration, or database structure** — the *what* lives in code; any
  copy here will drift and contradict.
- **Migration history** — that is the migrations' own append-only record.
- **A permissions engine or executable instruction** — memory describes;
  it never commands and never gates behaviour.
- **Secrets, credentials, tokens, PII** — a memory an AI reads first and
  may quote is a permanent exfiltration surface. This is categorical, not
  discretionary.
- **Operational present-tense state** — current deploy, current incident,
  "the build is broken." It rots in hours; memory is the wrong place.

The line is sharp and always under erosion pressure: memory captures the
**why**, never the **what**. The instant an entry narrates schema or
structure, it has acquired a duplicate that will drift — and every
objection this contract exists to prevent returns.

---

## 3. The invariants

Three invariants are the spine of the entire design. Everything else is
mechanism in service of them. **Violating any one reintroduces a whole
class of failure that the other two cannot contain.** They are to be
enforced the way "Postgres only" and "no second runtime" are enforced.

### Invariant I — Subordinate forever

CLOUD.md never holds authority. **On any conflict between memory and
code, code wins — silently and always.** An AI reading memory reads
*context*, never *law*. Memory may inform a proposal; it may never
override the codebase, grant a permission, or instruct a tool.

### Invariant II — Append-only

Memory is a **log, not a document.** Entries are **superseded, never
overwritten or deleted.** Correcting memory means appending a new entry
that references and supersedes the old one. Old reasoning is *history*,
not *error* — a 2026 assumption is a true fact about 2026 even after it
is superseded in 2029. This single rule defeats the lossy-recompression
("telephone game") failure mode and makes old memory safe to read.

**The single exception — redaction of prohibited content.** Append-only
governs *reasoning*; it does not trap *prohibited content* (§2.3) that a
fallible human gate mistakenly ratified in. Removing a secret, credential,
token, PII, or operational-state leak is the **one permitted destructive
operation.** It is not a correction of reasoning and may never be used to
revise or erase a decision — it excises content that should never have
existed. The redaction is **itself recorded** (the class of content
removed, by whom, when) so the *act* stays append-only even though the
content is gone, and it requires the same human ratification as any other
write (§4.1). The line is bright: anything merely *wrong* or *outdated* is
**superseded, never redacted**; redaction is reserved for content that is
*prohibited*, not content that is *mistaken*. This exception exists solely
to keep Invariant II from colliding with the absolute prohibition in §2.3
— without it, the strongest invariant and the most absolute rule are
mutually unsatisfiable. **Redaction removes prohibited content going
forward; it does not remove it from version-control history, which
additionally requires secret rotation and an out-of-band history rewrite** —
"redacted" does not mean "the secret is gone."

### Invariant III — Human-ratified

AI **proposes** memory; a human **accepts** it. **Nothing enters memory
autonomously.** The human gate is the hallucination firewall — it is
where a fabricated rationale is caught before it becomes "memory." Memory
is captured at moments where a human is *already* exercising judgment, not
as an independent chore.

> If only three things are ever enforced about CLOUD.md, enforce these. A
> design that holds all three is safe; a design that drops one is the
> original rejected proposal in disguise.

---

## 4. Governance — how memory is written

Memory writing is **not a new pathway.** It reuses the AI
proposal→approval→record lifecycle already defined in
`DESIGN_AI_ASSISTANT.md` **unchanged** — including its terminal state
names. Writing a memory entry is an AI **capability** in that system, and
it moves through the same states:

```text
Suggested  →  Reviewed  →  Approved  →  Applied
                                    ↘  Rejected
```

The vocabulary is the canonical one; this contract introduces no new
state. **"Applied," for the memory capability, means the approved entry is
appended to CLOUD.md** — "append" is the *effect* of `Applied`, not a
separate state. Two consequences follow directly from reusing the
existing machine:

- A **Suggested** memory entry is **not yet in CLOUD.md.** Exactly as a
  suggested migration is a staged proposal and not a live table
  (`DESIGN_AI_ASSISTANT.md` §3.1, §4), a suggested memory lives as a
  proposal until `Applied`. Nothing reaches the memory log on suggestion
  alone.
- **Apply is always separate from approval.** Approval does not append;
  the append is the explicit `Applied` step, preserving preview-by-default
  over the exact entry text before it lands.

- **Proposal, not assertion.** An AI-suggested memory is a *candidate*
  until a human accepts it (Invariant III). This is the
  anti-hallucination gate (§10).
- **Ratification rides existing judgment moments.** Capture happens where
  a human is already deciding — approving a decision, accepting a
  trade-off, superseding an assumption — never as a separate step the
  developer must remember to perform. One ratification, two effects.
- **Attribution and dating are mandatory** on every entry, including
  whether the author was human or AI. Provenance is what makes
  non-authoritative memory trustworthy *as memory*: a dated, attributed
  belief is weighed differently from an anonymous assertion.
- **Memory rides the same review pipeline as code** — reviewed,
  attributed, reversible, version-controlled. No out-of-band edits. This
  one rule makes the team-collaboration problem tractable (§9).
- **Supersession is the only form of change** (Invariant II). No entry is
  ever edited or deleted in place.

**What is reused, and what is not.** Memory reuses the AI lifecycle's
**governance semantics** — the states (§4), the human-ratification gate,
attribution, and the audit record. It does **not** automatically inherit
that lifecycle's **storage substrate.** The JSON-proposal /
append-only-`log.jsonl` / `--by`/`--as` machinery in
`DESIGN_AI_ASSISTANT.md` §5 was designed for *code and migration*
proposals; whether prose memory rides the same physical substrate or a
purpose-fit one is an implementation decision (§14), not a commitment of
this contract. Reusing the *lifecycle* is mandatory; reusing the *storage*
is not assumed. This distinction matters because over-coupling memory to
the migration-proposal substrate is a real complexity risk the contract
deliberately leaves open rather than presumes closed.

What governance explicitly does **not** permit: memory that gates runtime
behaviour, grants permissions, or instructs tools. Per
`DESIGN_AI_ASSISTANT.md`, *the AI has fewer privileges than a developer,
never more, and no private path* — writing memory is no exception.

### 4.1 Human control — what must never happen automatically

The workflow is **Developer → AI proposes → Human approves → memory
appends.**

This **fixes the capability's bucket** (`DESIGN_AI_ASSISTANT.md` §3):
**memory-write defaults to `needs_approval`.** The AI may *prepare* an
entry (Suggested) on its own authority, but the *append* (Applied)
requires human approval. It is never `Allowed` — that would permit
autonomous append and violate Invariant III — and never `Blocked` — that
would forbid memory entirely. **Redaction (§3) and supersession are not
separate buckets; they are memory-write operations and inherit the same
`needs_approval` gate.**

Always requiring human approval:

- **Any** memory write (no silent autonomous append, ever).
- **Any** supersession of a prior entry.
- **Any** redaction of prohibited content (§3).
- **Any** promotion of a memory to an ADR (§8).

Never happening automatically:

- Rewriting or deleting a prior entry.
- Resolving a contradiction between two entries or two agents (§9).
- An AI acting on memory in preference to code.

---

## 5. The capture model — what is worth remembering

**A memory is worth keeping when it will still matter after this
conversation ends *and* cannot be recovered from code.** That single test
yields the categories in §2.2 and one sharp exclusion.

**Never capture** (noise, not signal):

- **Conversational scaffolding** — dead-ends *worked through and resolved
  in the same session* are thinking-out-loud, not rejected *decisions*.
  Logging them as "rejected ideas" is the most common form of pollution.
- Anything recoverable from code/schema/migrations (the *what*).
- Anything in the §2.3 prohibition list (secrets, PII, operational
  state).

### 5.1 When to ask "remember this?" — without becoming annoying

The asymmetry is the design key: **interrupting to ask is expensive**
(breaks flow, trains reflexive dismissal); **failing to capture is cheap
to recover** if manual capture is always available. Therefore:

- **Never interrupt mid-flow to ask.** Tie any prompt to the moments a
  developer has *already* stopped to exercise judgment (a decision, a
  trade-off, a supersession). There, a single lightweight prompt is
  welcome because the developer is already in decision-mode.
- **Bias toward under-capture with frictionless manual capture.** A memory
  that occasionally misses is trusted; one that constantly nags is
  dismissed. Missing is recoverable; annoyance is not.
- **Batch, never pepper.** Surface several candidates together at a
  natural pause, not one interruption each.
- **Silence is the default.** The correct number of prompts in a routine
  working session is usually zero.

The capture model's success metric is counterintuitive: **the developer
should rarely notice it.**

---

## 6. The memory model

Each entry is a small, dated, attributed unit carrying its reasoning. The
model has three structural properties, all downstream of the invariants:

- **Atomic** — one decision / assumption / rejected idea per entry, so it
  can be retrieved, referenced, and superseded independently.
- **Attributed and dated** — who (human or AI) and when, always.
- **Superseded, never mutated** — change is a new entry linking the old;
  superseded entries remain visible but demoted (§7).

Relative dates are forbidden; record absolute dates (the same discipline
the project applies elsewhere). The *layering* of supersessions over time
is the asset — a year-3 reader traces any current shape back through its
supersessions to the original intent, with every fork-not-taken preserved.

---

## 7. The retrieval model

This is the concern most likely to be underestimated and the one that
decides the *good* outcome from the *mediocre* one (§11). **A three-year
log that can only be read chronologically is an archive nobody opens.**
The governing shift: **memory is summoned by relevance, not browsed by
date.**

The following are **requirements the retrieval design must satisfy — the
*what*, not the *how***. They state the behaviour the system owes; the
mechanism that delivers it (and where it lives — see §13) is deferred to
the implementation design (§14). The contract does not prescribe an
indexing or matching technique.

1. **Contextual resurfacing of rejected ideas — the decisive requirement.**
   The moment that matters for a rejected idea is *when someone
   re-proposes it.* Memory's highest-value behaviour is: a developer or
   AI proposes X, and memory surfaces "X was considered and rejected on
   [date] because Y." Rejected ideas must be discoverable **at the moment
   of re-proposal**, not by someone happening to scroll an archive. If
   this works, the feature pays for itself; if it does not, rejected ideas
   are dead weight.

2. **Reasoning must be retrievable by subject.** The system must be able
   to answer "*why* is the auth layer shaped this way?" by surfacing the
   reasoning attached to that subject, with superseded entries clearly
   marked as past — recalled the way a senior engineer recalls "oh, we
   decided that because…", not by re-reading the whole history. *How*
   subject-association is achieved (indexing, tagging, semantic match) is
   left to the design.

3. **Recency and supersession must be first-class signals — but recency
   must not bury foundational intent.** Newer context generally outranks
   older, and superseded entries are visibly demoted yet **never hidden**
   (the dead architecture's rationale must stay findable to prevent its
   reintroduction). **Age is always surfaced; staleness disguised as
   currency is a trust-killer.** Recency is a *default heuristic, not a
   truth ranking*: a **still-valid, never-superseded foundational
   assumption** (e.g. a day-one "internal-only" constraint) must remain
   retrievable on its subject and must **not** be demoted into invisibility
   merely for being old. The failure mode to design against is the inverse
   of staleness — the most load-bearing entries becoming the least
   findable over three years precisely because they are the oldest. *Age
   demotes; supersession demotes; being foundational protects.*

The design test for retrieval: **at year 3, the relevant three entries
surface and the irrelevant three thousand stay out of the way.** A
retrieval model that returns "everything since project start" has failed.

**Where this capability lives — and the dependency it creates.**
Retrieval is performed by the **external assistant** (§13), not the
framework. The framework's obligation is to keep memory *retrievable* —
atomic, attributed, dated, with superseded entries marked — but judging
relevance and detecting a re-proposal is the assistant's work. This means
rejected-idea resurfacing, the decisive feature, is **conditional on the
external assistant's retrieval quality, which the framework does not
own.** A poor assistant can fail it even over well-formed memory. The
contract is therefore honest about the split: the framework guarantees
memory is *well-formed and findable*; it does not — and cannot alone —
guarantee that it is *found*. The success criterion in §12.6 is a bar on
the **system as a whole**, not a promise the framework makes by itself.

---

## 8. ADR interaction — promotion

Memory and ADRs are different altitudes, not competitors:

- **ADR** — a *ratified, consequential, architectural* decision: formal,
  reviewed, normative about architecture.
- **CLOUD.md** — intent, soft assumptions, and the *long tail* of
  rejected and lightweight reasoning that never reached ADR status, plus
  onboarding narrative.

Most rejected ideas die before deserving an ADR; that long tail is
memory's domain.

### 8.1 Promotion criteria

Graduate a memory to an ADR when it is **all** of:

- **Consequential** — it constrains future work, and
- **Architectural / cross-cutting** — it shapes the system, not one
  feature, and
- **Likely to be referenced repeatedly** — people keep asking "why
  this?", and
- **Stable enough to ratify formally** — no longer in flux.

A memory that is none of these stays memory. A memory that is all four
wants to be an ADR.

### 8.2 Who participates

- **AI agents suggest promotion** — proposal, never autonomous. When an
  entry is referenced repeatedly, or a soft assumption hardens into a
  load-bearing constraint, the AI flags "this looks like architecture —
  promote to an ADR?"
- **A diagnostic may surface candidates from mechanical signals only** —
  consistent with the existing doctor role as a *detector that reports and
  recommends*, not a decider. It may count **structural metadata** — how
  often an entry is referenced, how many times it has been superseded —
  and flag high-count entries as promotion candidates. It reads **counts,
  not content**: it never interprets what an entry *means* or whether two
  entries conflict — that semantic judgement is the external assistant's
  (§13). It never promotes anything itself.
- **The human ratifies promotion** — exactly as with every other memory
  act (§4.1).

### 8.3 Promotion mechanics

Promotion is **link, not move.** The memory entry is **not deleted**
(Invariant II) — it gains a reference to the ADR it spawned, and the ADR
becomes the normative record. Memory stays the narrative waiting-room and
margin notes; ADRs are the formal record. The taboo: memory must never
re-narrate an ADR'd decision in its own words, or two accounts of one
decision will drift apart.

---

## 9. Team collaboration model

Multiple developers, branches, and AI agents will hold potentially
conflicting assumptions. The governing principle: **surface
contradictions; never auto-resolve them.**

- **Memory follows code through the same merge process** (§4). Because
  memory is append-only, branches *add* turns rather than editing a shared
  passage — the prose-merge-conflict and telephone-game failure modes both
  require in-place rewriting, which Invariant II forbids.
- **Contradictions become explicit, never silently merged.** Two
  conflicting branch assumptions resolve as either an explicit
  **supersession** (one replaces the other, with reason and date) or an
  **open tension**. An open tension is **not a new entry type** — it is an
  ordinary memory entry whose content records an unresolved disagreement
  ("two parts of the team currently assume different things about X —
  unresolved"). It is captured, ratified, attributed, and retrieved by the
  same rules as any other entry, and it **closes the way everything else
  changes: by supersession** — when the disagreement is resolved, a new
  entry supersedes it with the decision and its reason. No separate
  lifecycle, status machine, or retrieval path is introduced. An
  honestly-recorded open tension is far safer than a silently-merged
  contradiction no compiler will catch.
- **Superseded assumptions stay visible, marked as past.** Memory evolves
  without self-contradiction because contradiction is represented as
  supersession *chronology*, not as competing current-tense claims.
- **Conflicting agents surface to a human.** When two AI agents disagree,
  neither wins by recency or last-write. The disagreement is surfaced for
  human resolution — the same gate that governs every write (§4.1).

The whole model reduces to **append + attribute + supersede +
surface-don't-resolve.**

---

## 10. Anti-hallucination

Memory must not become a laundering channel where a fabricated claim
becomes "established fact" that later reasoning builds on. Three defences,
all already implied by the invariants:

- **The human ratification gate** (Invariant III) — the primary firewall.
  A fabricated rationale ("benchmarks showed…") is caught before it
  enters memory.
- **Cite or hedge.** A claim with a source links it; a claim without one
  is phrased as belief ("we assumed…"), never as fact. An unsourced entry
  is non-authoritative *as memory* — it may be recorded as a belief, never
  cited as a fact.
- **Provenance** (§4, §6) — every entry is attributed and dated, so a
  reader always knows *who believed this and when*, and can weigh it
  accordingly.

---

## 11. Failure modes — *How CLOUD.md Dies*

### 11.1 The bad death (catastrophic, trust-destroying)

Memory acquired authority. It became AI-writable without a human gate,
began overriding code in practice, accumulated a hallucinated rationale
that became load-bearing, or leaked a secret. The AI did the wrong thing
*"because CLOUD.md said so."* Memory converted from asset to hazard,
permanently.

**Warning signs — these are alarms, not gauges; one occurrence is a
breach:** an AI action justified by memory over code; an entry changing
without a review trail; secrets/operational state appearing; an
**unauthorised** in-place edit of a prior entry (an authorised §3
redaction of prohibited content — ratified and recorded — is the sole
exception); "the memory says so, so we should…" replacing "the code does
X."

### 11.2 The mediocre death (quiet waste)

Memory stayed safe but became useless. Either it nagged until developers
reflexively dismissed every prompt (**noise death**), or it grew into a
chronological pile nobody can retrieve from (**archive death**).
Technically subordinate, practically ignored — new hires are told "ignore
CLOUD.md, it's out of date."

**Warning signs — gradual gauges; watch the trend:** rising
capture-prompt dismissal rate; nobody reads it during onboarding;
rejected ideas recur *despite* being recorded (retrieval failure); memory
volume grows while memory *reads* flatten.

### 11.3 The successful version

A clean, layered, append-only chronology. Subordinate to code and visibly
so. Rejected ideas resurface at the moment of re-proposal. New hires
absorb years of reasoning in minutes. The right memories graduate to
ADRs. Every entry dated, attributed, never rewritten. Developers trust it
precisely because it has never acted on its own or changed its own past.

### 11.4 Recognising drift

The two deaths fail differently and need different detection:

- **Toward the bad death** — detected by *category violation*
  (autonomous write, code override, taboo content, in-place edit). Any
  single occurrence is a breach to be treated as a stop-the-line event.
- **Toward the mediocre death** — detected only by *usage trend*
  (dismissal rate up, reads down, volume up, rejected ideas recurring).

---

## 12. Success criteria

CLOUD.md has succeeded if, at year 3:

1. It **never once overrode code** — and developers know it never will.
2. **No entry was ever rewritten or deleted** — only superseded.
3. **No autonomous write ever occurred** — every entry passed a human
   gate.
4. **No secret, PII, or operational state persisted in it** — nothing
   prohibited entered, or where the fallible human gate let something
   through, it was **redacted under §3 and the redaction recorded.**
5. A **new developer can reconstruct years of reasoning from it in
   minutes** — and knows to verify against code.
6. A **rejected idea resurfaced and prevented a re-implementation** at
   least once — the feature paid for itself. *(Conditional on the external
   assistant's retrieval quality (§7, §13): the framework keeps memory
   retrievable, but the assistant performs the retrieval. This criterion
   judges the system as a whole, not the framework alone.)*
7. The **relevant few entries surface on demand** while the thousands of
   irrelevant ones stay out of the way.
8. Developers **neither maintain it nor fear it** — they trust it and
   mostly forget it is there until they need "why."
9. The **right memories graduated to ADRs**, and memory never re-narrated
   them.

Criteria **1–4 are absolute.** If any of them ever broke, CLOUD.md failed
regardless of the rest — they are the invariant violations.

---

## 13. Scope and the "no second runtime" rule

CLOUD.md must not become a backdoor to an embedded AI runtime, which the
strategic-reset rules and `DESIGN_AI_ASSISTANT.md` §8 forbid in the core
crate. The boundary:

- **In scope here:** a *memory artifact and its governance* — the
  append-only record, the human-ratification gate, attribution, and
  supersession. These reuse the framework's existing strengths
  (permissions, audit, deterministic builder, the AI proposal lifecycle)
  applied to a new kind of artifact. No new runtime, no new trust
  primitive.
- **Out of scope here:** the *intelligence over the memory* — the model
  that reads CLOUD.md, decides relevance, performs retrieval, judges what
  an entry means, or detects that two entries conflict. That reasoning
  runs in the **external** assistant, exactly as today. The framework
  governs the artifact and may keep **mechanical bookkeeping** over it —
  counting references and supersessions as structural metadata (§8.2) —
  but it does not **interpret** memory: it reads *counts, never content
  meaning*. The line between native and external is the line between
  *counting* and *understanding*.

If a deeper integration ever embeds retrieval intelligence or model calls
inside the product, that integration ships in a separate `rustio-pro-*`
crate — never inside `rustio-admin`. This contract governs the **memory
and governance primitives**, which are native in kind to what the
framework already does.

---

## 14. Open questions for implementation review

Deferred deliberately; to be designed and approved separately, bound by
the invariants above:

- **Format, location, and storage** of the memory artifact (the contract
  is silent by design; "CLOUD.md" names the concept, not a committed
  format). This includes the **substrate-fit decision** (§4): whether
  prose memory reuses the `DESIGN_AI_ASSISTANT.md` §5 proposal/log
  substrate or a purpose-fit one — reusing the *lifecycle* is mandatory,
  reusing the *storage* is open.
- **Command surface**, if any, and how it composes with the existing
  `rustio ai` lifecycle in `DESIGN_AI_ASSISTANT.md`.
- **Retrieval implementation** — how relevance and rejected-idea
  resurfacing (§7) are realised in the external assistant without
  embedding a runtime (§13).
- **Capture-prompt mechanics** — the concrete moments and surface for the
  §5.1 "ask without annoying" rule.
- **Relationship to `.rustio/ai.toml`** — the bucket is **settled**
  (`needs_approval`, §4.1); what remains is the capability *key name(s)*
  (e.g. one `write_memory` key versus distinct keys for append /
  supersede / redact) and whether redaction or supersession warrants a
  `second_approver_for` entry given their higher risk.
- **Audit record shape** — whether a memory append/supersede/redact emits
  **new typed `AuditEvent` variants** or reuses the existing
  `ai_proposal_applied` family (`DESIGN_AI_ASSISTANT.md` §5). The contract
  requires only that every memory action be recorded with attribution and
  correlation id per `DESIGN_AUDIT.md`; the variant taxonomy is deferred.

None of these may relax Invariants I–III.
