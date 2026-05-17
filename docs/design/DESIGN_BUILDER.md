# Builder Architecture

A Builder operation is the durable, redacted record of a single
build-time decision about the shape of a Rustio project.

This document is the contract for how those decisions are declared,
ordered, hashed, regenerated, and audited.

Pull request review runs against this document, not only the diff.

> **Doctrine inheritance**
> The Builder is a build-time layer over the Framework's runtime
> surface. It inherits the Framework's existing doctrines on
> append-only audit emission (`DESIGN_AUDIT.md` Doctrines 8 / 11 /
> 18), single-writer enforcement (`DESIGN_SESSIONS.md` Doctrine 22),
> and disclosure asymmetry. Where the Framework's runtime doctrines
> apply by analogy, this document names the Builder-specific
> instances and binds them with the same enforcement style.

---

## 0. Status

| Property | Value |
|---|---|
| Layer | Build-time (CLI), not runtime |
| Crate owner | `crates/rustio-admin-cli` |
| Framework dependency | Builder depends on Framework; Framework never depends on Builder |
| Source of truth | This document |
| Implementation status | Pre-MVP. Doctrine precedes implementation. |
| Forward-looking vision | [`docs/archive/ROADMAP_BUILDER.md`](../archive/ROADMAP_BUILDER.md) |

---

## 1. Purpose

### 1.1 What this governs

- The `.rustio/draft.toml` declarative intent file and its schema
  version lifecycle.
- The `.rustio/history.jsonl` append-only build-time event log.
- The `.rustio/builder.lock` version-pin file.
- The `src/_generated/` directory and the SchemaHash / overwrite /
  deletion contract that protects it.
- The `rustio plan` / `rustio commit` lifecycle and atomicity
  guarantees.
- Migration append-only guarantees and the closed enumeration of
  destructive operations.
- The boundaries between the Framework runtime, the Builder, the
  future Advisory AI layer, and the future Studio surface.
- The canonical-reproducibility property and the environment fixings
  under which it holds.

### 1.2 What this does not cover

- Runtime authority behaviour — `DESIGN_SESSIONS.md`,
  `DESIGN_AUDIT.md`, `DESIGN_RECOVERY.md`,
  `DESIGN_R2_ORGANISATIONAL.md`, `DESIGN_R3_MFA.md`,
  `DESIGN_R4_EMERGENCY.md` are authoritative.
- The exhaustive `draft.toml` field schema — a separate document
  versioned alongside `schema_version`.
- The Advisory AI layer's internal behaviour — boundary only here;
  internal contract in a future `DESIGN_ADVICE.md`.
- The Studio surface — boundary only here; specification in a future
  `DESIGN_STUDIO.md`.
- CLI verb naming and argument shapes — verbs named below describe
  responsibilities, not a frozen API.
- Sequencing, milestones, and version numbers — doctrine outlives
  any schedule.

### 1.3 Closing principle

A Builder operation is a build-time decision about the shape of a
Rustio project. The contract is the document; the implementation
must round-trip against it.

---

## 2. Invariants

### 2.1 Builder Doctrines

The numbered doctrines below are the load-bearing invariants of the
Builder. They are stated in the present tense because a Builder
implementation that violates any of them is not a Builder
implementation. CI enforcement is named under §10.

| Doctrine | Mandate |
|---|---|
| **B1**  | `.rustio/draft.toml` is the sole input to the deterministic generator |
| **B2**  | `.rustio/history.jsonl` is append-only; every reversal is a new compensating event |
| **B3**  | `cli::history::append` is the sole writer of `.rustio/history.jsonl` (Builder analog of Doctrine 22) |
| **B4**  | The Builder's `cli::redact` produces fingerprints, not values, for every secret-category field before emission to `history.jsonl` or `.rustio/proposals/` (Builder analog of Doctrine 11) |
| **B5**  | Every file under `src/_generated/` carries a header and a SchemaHash; overwrite without a matching hash requires explicit `--force` and emits an event |
| **B6**  | Migrations are append-only — no command, with or without `--force`, edits an existing migration file |
| **B7**  | Doctrine-bound features (§8.2) cannot be disabled via `draft.toml`; `rustio commit` refuses such a configuration |
| **B8**  | `rustio plan` has zero filesystem side effects; `rustio commit` is atomic |
| **B9**  | `rustio plan` and `rustio commit` open no network sockets |
| **B10** | The Framework crate never reads Builder-emitted metadata at runtime |
| **B11** | The Builder version is pinned in `.rustio/builder.lock`; mismatch refuses to run without explicit upgrade |
| **B12** | A model or field rename is performed only when the corresponding `rename_*` event exists in `history.jsonl`; otherwise the operation is classified destructive |
| **B13** | The deterministic generator never reads from `.rustio/proposals/`; Advisory output is laundered into `draft.toml` only through an explicit reviewer-attributed Builder command, which emits a verdict event |

### 2.2 What must never happen

> **Doctrine B3.** Only `cli::history::append` writes `.rustio/history.jsonl`.

The repository carries a grep proof (§10). CI rejects merges that
introduce a second writer.

> **Doctrine B4.** A secret value is never written to `history.jsonl` or to `.rustio/proposals/`.

Default values for fields whose declared type matches the secret
category (§4.2.3), environment variables, API keys, SMTP
credentials, and any field marked `redact = true` in `draft.toml`
pass through `cli::redact` before emission. The redaction discipline
is symmetric to `DESIGN_AUDIT.md` §5.3.

> **Mutated history line.** No code path runs file-truncating or in-place edits against `history.jsonl`.

Reversal is always a new line with `op = "undo"` referencing the
ULID of the event being reversed. The same append-only contract
`DESIGN_AUDIT.md` §5.6 binds on `rustio_admin_actions` binds on
`history.jsonl` here.

> **Disabled audit-by-default.** `audit = false` in `draft.toml` is not a permitted configuration.

`DESIGN_AUDIT.md` §5.1 (Doctrine 8) is framework-wide. The Builder
cannot bypass it via configuration. The closed list of toggleable
features lives in §8.1; the closed list of doctrine-bound features
lives in §8.2.

> **Hash-bypassing overwrite.** No code path overwrites a file under `src/_generated/` whose header hash mismatches without `--force`, and no `--force` operation runs without emitting a `forced_overwrite` event.

The Builder's only sanctioned way to lose generated work must itself
be auditable.

> **Edited migration.** No command alters a file in `migrations/` after it has been written.

A schema change always produces a new numerically prefixed migration.
The Framework's own migration append-only doctrine (declared in
`CLAUDE.md` and the workspace `architecture.md`) binds on the Builder
unchanged.

> **Silent rename collapse.** A model or field rename never collapses into a drop+create without explicit destructive opt-in.

The discrimination rule lives in §7.4 and Doctrine B12.

> **Framework reads Builder metadata at runtime.** The `// @generated by rustio …` header is human-facing metadata only.

A grep proof in `crates/rustio-admin/` rejects any code path that
parses generator header content. Doctrine B10.

---

## 3. Threat model

The Builder layer defends a known set of adversaries. Each adversary
names what they have, what they cannot get, and the property that
defeats them.

### 3.1 Adversaries

| Adversary | Has | Cannot get | Defeated by |
|---|---|---|---|
| History forger | Write access to `.rustio/history.jsonl` | A row that survives PR review | All appends route through `cli::history::append`; orphan lines fail the drift test (§10) |
| History dropper | The ability to skip an event in a Builder command | A schema change without an event | Generator refuses to commit if `draft.toml` diverges from the replay of `history.jsonl` |
| Tamperer with quiet edits | Direct file edits to `history.jsonl` | A modified event that survives replay | Replay invariant (§4.2.2); optional SHA-256 chain (§4.2.5) detects mid-stream tampering |
| Secret leak via event log | Access to `.rustio/history.jsonl` in source control | Plaintext token / key / default secret | Doctrine B4 redaction; property test asserts no 4-char input substring leaks |
| Doctrine bypass via config | Direct `draft.toml` edit | A project whose generated code disables audit / CSRF / single-writer invalidation | Doctrine B7; closed feature lists in §8 |
| Reproducibility forger | A different Builder version | Bytes that match the originally generated project | Doctrine B11; `builder.lock` mismatch refuses to run |
| Silent data loss via rename | A `draft.toml` edit that drops and re-adds a model with a different name | A migration that preserves data without `rename_*` event | Doctrine B12; absence of event ⇒ operation classified destructive (§7.3) |

### 3.2 Out of scope

- Compromise of the developer's workstation / git remote.
- Compromise of the operator's CI runner.
- Adversaries with simultaneous filesystem write and Builder source
  modification.
- Network adversaries — the Builder makes no network calls under
  `plan` or `commit` (Doctrine B9); other Builder verbs (`suggest`,
  `import`) have their own contracts.
- Determinism under intentionally hostile environments (modified
  TOML parser, custom rustc fork). The reproducibility property in
  §4.4 holds under the named environment fixings; outside them it
  is unspecified.

---

## 4. The `.rustio/` backbone

Every Builder operation flows through three files in the project's
`.rustio/` directory. All three are checked into version control.

```text
.rustio/
├── draft.toml          ← declarative intent (current state)
├── history.jsonl       ← append-only event log
└── builder.lock        ← Builder version pin
```

The directory also hosts transient subdirectories during operations
in flight (`tmp/`, `forced/`, `proposals/`) — see §5 and §9.

### 4.1 `draft.toml` — declarative intent

The single declarative source of truth for what the project is. The
generator's only input (Doctrine B1).

**Format.** TOML. UTF-8 NFC-normalized, LF line endings, no trailing
whitespace, sorted keys within each table, 2-space indent. Canonical
serialization is enforced by `cli::toml::emit_canonical`; the
function is the sole emitter for any Builder-written TOML. PRs that
introduce a second emitter fail the grep proof in §10.

**Schema version.** A top-level `schema_version` integer pins the
expected structure. Bumps are migrations (§4.6); reads of an unknown
schema version refuse to run.

**Mutability.** Mutated only by Builder commands. Hand-editing is
permitted but treated as an unverified mutation: the next
`rustio status` invocation detects drift between `draft.toml` and
the replay of `history.jsonl`, prints the divergence, and refuses
subsequent mutating commands until the developer either accepts the
hand edit (which emits a `hand_edit` event recording the drift) or
discards it.

**Top-level sections.** The exhaustive schema is a separate document
versioned independently. At minimum:

```toml
schema_version = 1

[project]
name           = "..."
rust_version   = "..."
builder_pinned = "..."        # mirror of .rustio/builder.lock for grep visibility
created_at     = "..."

[features]
# Toggleable members per §8.1. Doctrine-bound members per §8.2
# may appear but only with the doctrine-required value.

[[models]]
# ...

[[relations]]
# ...
```

### 4.2 `history.jsonl` — append-only event log

The audit trail of every build-time decision.

#### 4.2.1 Line format

One JSON object per line. Each line carries the following fields, in
this order, with no embedded newlines:

```jsonc
{
  "id":        "01HW...",          // ULID, time-ordered
  "ts":        "2026-05-15T10:30:00Z",
  "op":        "add_model",        // closed enum; drift test below
  "actor":     "alice@example.com",// from git config user.email or RUSTIO_AGENT_ID
  "args":      { ... },            // op-specific payload, post-redaction
  "prev_hash": "sha256:...",       // optional; only present when chain mode is on
  "schema_v":  1                   // schema version of this line's format
}
```

#### 4.2.2 Replay invariant

> **The replay-rebuild property.** Replaying every event in `history.jsonl` in line order against an empty `draft.toml`, using `cli::toml::emit_canonical`, produces the current `draft.toml` byte-for-byte under the environment fixings of §4.4.

The property is verified by `rustio doctor builder` (§10) and by a
unit-level drift test in CI.

#### 4.2.3 Redaction (Doctrine B4)

Before any payload reaches `args`, it passes through `cli::redact`.
The function is the sole redaction boundary; a grep proof in §10
forbids any other code path from constructing `args` directly.

Categories that must never appear in plaintext:

1. Default values for fields whose declared type is one of
   `password`, `secret`, `token`, `api_key`, `private_key`,
   `encryption_key`.
2. The value of any field whose modifier set in `draft.toml`
   includes `redact = true`.
3. Values that match the framework's secret-fingerprint heuristic
   used by `admin::redact::redact_token` (re-used here to avoid two
   discordant heuristics).
4. Environment-variable contents passed via CLI flags
   (`--from-env <VAR>`).
5. Any string the developer marks with the `@redact:` sigil at the
   CLI boundary.

The redacted form is `"sha256:<first-16-hex>...truncated"`. A
property test asserts no 4-char input substring leaks through the
redactor, symmetric to `DESIGN_AUDIT.md` §5.3.

#### 4.2.4 Single-writer rule (Doctrine B3)

> **`cli::history::append` is the sole function that appends to `history.jsonl`.**

Every Builder command that mutates state routes through this
function. The repository carries a grep proof (§10). CI rejects
merges that introduce a second writer. The discipline is the Builder
analog of `DESIGN_SESSIONS.md` Doctrine 22.

The closed enumeration of `op` values lives in a single Rust
`enum HistoryOp` whose `as_str()` is the source of truth for the
serialized values. A drift test asserts the enum and the on-disk
log values stay in lockstep, symmetric to `DESIGN_AUDIT.md` §5.2.

#### 4.2.5 Optional chain mode

Projects may opt into a SHA-256 chain — each line carries
`prev_hash = sha256(prior line bytes including its prev_hash)`.
When chain mode is enabled in `builder.lock`, every appender must
populate the field; verification is part of `rustio doctor builder`.
Chain mode is off by default; off projects omit the field entirely.

### 4.3 `builder.lock` — version pin

Format: TOML.

```toml
schema_version = 1
builder        = "0.15.0"        # exact semver
toml_emitter   = "rio-canon-1"   # canonical emitter version
chain_mode     = false
```

**Ownership.** Written on first `rustio commit` of a project.
Updated only via `rustio upgrade` (or whichever name the eventual
CLI uses for the same responsibility).

**Enforcement (Doctrine B11).** Every Builder command except
`rustio upgrade` reads `builder.lock` first and refuses to proceed
if the executing Builder's semver does not match. The error message
prints the exact upgrade command. There is no implicit upgrade.

**Why this is not a second runtime.** `builder.lock` is consumed by
the CLI at build time only. Doctrine B10 forbids the Framework
runtime crate from reading it; the grep proof in §10 enforces this.

### 4.4 Canonical reproducibility

Reproducibility is the load-bearing property of the Builder. It is
asserted, not assumed, under a closed set of environment fixings.

> **Canonical reproducibility property.** Given an unchanged `draft.toml`, an unchanged `history.jsonl`, an unchanged `builder.lock`, and an execution environment matching the fixings below, two `rustio commit --dry-run` runs produce byte-identical outputs.

**Required fixings:**

1. **Locale.** `cli::main` sets `LC_ALL=C` for the duration of any
   command that emits text. Hostile shells that override this are
   detected and refused.
2. **Line endings.** All Builder-emitted text files use LF
   exclusively. CRLF input is normalized at read time.
3. **Encoding.** UTF-8 NFC. Input bytes that are not valid NFC are
   normalized on read (with an event entry recording the
   normalization).
4. **TOML emitter.** `cli::toml::emit_canonical` is the sole
   emitter. The emitter's contract is alphabetical key order within
   each table, two-space indent, no trailing whitespace, no comments
   emitted (Builder TOML is machine-managed; comments are not
   round-tripped).
5. **Time.** Timestamps in `history.jsonl` use ISO-8601 with UTC
   `Z` suffix, second-precision. Sub-second precision is forbidden
   because two same-second events with different fractional
   timestamps are visually distinct without semantic difference.
6. **Identifier generation.** ULIDs are derived from the wall-clock
   timestamp plus 80 bits of randomness. Replay does not regenerate
   ULIDs; reading an existing log does not assign new IDs.
7. **Builder version.** Pinned by `builder.lock`. Different
   versions are not part of the same reproducibility frame.
8. **Toolchain version.** The project's `rust-toolchain.toml`
   pins the rustc the generated code is compiled against. The
   Builder's own MSRV is independent and pinned in its `Cargo.toml`.

Outside these fixings, the reproducibility property is unspecified.
Doctrine B11 enforces #7; a CI check in the Builder crate enforces
#4 (single-emitter grep proof).

### 4.5 Concurrent writers and git merge

Real teams develop on branches. A doctrine that does not address
merge cannot survive an enterprise environment.

> **Mergeability of `history.jsonl`.** Two `history.jsonl` streams from divergent branches are mergeable iff the textual three-way merge produces a line sequence that replays to a well-formed `draft.toml` under §4.2.2.

`rustio merge` (CLI verb naming non-normative) performs the
verification. The operation:

1. Reads the working-tree `history.jsonl` (assumed to be the
   merged-but-unverified state from git).
2. Replays it from an empty `draft.toml`.
3. Compares the replay output to the working-tree `draft.toml`.
4. On match: emits a `merge_verified` event carrying the merge
   parent SHAs from `git`.
5. On mismatch: refuses, prints the first divergent line, and exits
   non-zero. The developer must resolve `history.jsonl` or
   `draft.toml` by hand and re-run.

Conflicting renames across branches (both branches renamed the same
model to different targets) surface as merge failures at step 5 and
require human resolution. The Builder does not auto-resolve.

### 4.6 `schema_version` migration

When the Builder advances its internal schema, projects on older
versions must migrate forward.

> **Forward-only.** Schema downgrades are not supported by the Builder.

A `rustio upgrade --schema` command (verb naming non-normative):

1. Reads the existing `draft.toml`.
2. Writes a snapshot to `.rustio/draft.toml.v<old>.bak`.
3. Applies the migration transformer for the target version.
4. Emits a `schema_upgraded` event in `history.jsonl` carrying the
   from/to versions and a digest of the prior `draft.toml`.
5. Updates `builder.lock` if the new schema requires a new Builder
   semver.

The `.bak` file is the documented recovery path for a botched
upgrade. Manual restoration is the only downgrade procedure.

**Replay across upgrades.** Events written under an older
`schema_v` field remain valid in `history.jsonl`. The replay logic
in `cli::history::replay` is responsible for migrating older event
shapes to the current `draft.toml` representation. The drift test
in §10 covers every supported `(from_schema, to_schema)` pair.

---

## 5. `src/_generated/` — generator-owned territory

The Builder emits all generated Rust source files under a single
top-level directory. The directory's contents are owned by the
generator. The developer's contract is enforced by file headers, a
SchemaHash, and per-file event emission.

### 5.1 Directory boundary

```
src/
├── _generated/           ← generator territory. Regenerated on every commit.
│   ├── models/
│   ├── admin.rs          ← Admin::new().model::<...>() registrations + `build_admin()`
│   ├── routes.rs
│   └── ...
└── app/                  ← developer territory. Generator never touches this.
```

**`main.rs` boundary.** The generator never edits `main.rs` after
project creation. To avoid the "needs to register new models in
main.rs" trap, `_generated/admin.rs` exposes a single public
function `build_admin() -> rustio_admin::Admin` that `main.rs`
calls. Future model additions update `build_admin`, not `main.rs`.

### 5.2 File header (metadata, not contract)

Every file in `_generated/` carries a fixed header:

```text
// @generated by rustio <semver> from .rustio/draft.toml
// SPDX-SchemaHash: sha256:<64-hex-chars>
// SPDX-EmitterVersion: rio-canon-1
// To change, edit draft.toml and run `rustio commit`. Manual edits
// will be overwritten.
```

The header is human-facing metadata. **It is not a runtime contract**
(Doctrine B10). The Framework crate must not parse it. A grep proof
in `crates/rustio-admin/` forbids reading the marker strings:

```sh
git grep -nE '@generated by rustio|SPDX-SchemaHash|SPDX-EmitterVersion' -- crates/rustio-admin/src/
```

The above must produce zero matches. The CLI crate freely reads its
own emitted markers.

### 5.3 SchemaHash

The SchemaHash is a SHA-256 over a deterministic projection of
`draft.toml`. The projection is the **closed slice** of `draft.toml`
that determines the file's contents:

| File | Projection input |
|---|---|
| `_generated/models/<name>.rs` | The `[[models]]` entry with matching `name` + every `[[relations]]` entry where `from = <name>` or `to = <name>` |
| `_generated/admin.rs` | All `[[models]]` entries (names + admin sections) + `[project]` + `[theme]` |
| `_generated/routes.rs` | All `[[models]]` names + `[features]` doctrine-bound members (§8.2) |

The projection function `cli::hash::projection(path, draft)` is the
sole producer of SchemaHash inputs; a grep proof in §10 forbids any
other path from synthesizing a hash. The header text is **not** in
the projection — Doctrine B10's grep proof is symmetric with this
design.

### 5.4 Overwrite contract

`rustio commit` operates on each file in `_generated/` under these
rules:

1. **File absent on disk.** Write the new file. Emit a
   `file_created` event. No prompt.
2. **File present, header parses, SchemaHash matches.** Overwrite
   silently. No event (the file is in lockstep with `draft.toml`;
   the parent `commit` event covers the operation).
3. **File present, header parses, SchemaHash mismatches.** Refuse
   to overwrite. Print a diff. Require `--force` to proceed. On
   `--force`: copy the prior file to
   `.rustio/forced/<timestamp>/<path>` before overwrite, emit a
   `forced_overwrite` event carrying the prior file's SHA-256 and
   the actor.
4. **File present, header absent or malformed.** Refuse to
   overwrite. Same `--force` discipline as case 3.

`--force` refuses to run against a dirty git working tree unless
`--accept-dirty` is also passed. The two flags are independent;
neither implies the other. `--accept-dirty` itself emits an event.

The same `--force` flag never bypasses migration safety (§7).

### 5.5 Deletion contract

When a model removal (or any other `draft.toml` change) leaves a
`_generated/` file no longer required:

1. The Builder verifies the file's SchemaHash matches expectation.
   On mismatch, refuses (same as overwrite case 3) unless
   `--force` is supplied.
2. The deletion is part of the atomic `rustio commit`. Either every
   intended deletion happens or none do.
3. A `file_deleted` event is emitted carrying the prior path and
   the prior file's SHA-256.
4. The deleted file's prior content is copied to
   `.rustio/forced/<timestamp>/<path>` when removed via `--force`,
   matching §5.4 case 3.

### 5.6 `app/` is sacred

The Builder must not, under any circumstances, read or write files
under `src/app/`. Custom `ModelAdmin` trait implementations,
business logic, and developer-authored handlers live here. The
generator's output references types defined in `app/` only by name;
it never modifies them. A grep proof (§10) forbids any Builder code
path from constructing a path under `src/app/`.

---

## 6. The `plan` / `commit` lifecycle

The Builder's mutating verbs follow a Terraform-style two-phase
contract.

### 6.1 `rustio plan`

Read-only. Has zero filesystem side effects (no writes, no creates,
no deletes, no permission changes — Doctrine B8). Output is a
structured diff that describes what `rustio commit` would do:

- Files that would be created.
- Files that would be overwritten (SchemaHash before/after).
- Files that would be deleted (prior SHA-256).
- Migrations that would be generated (full SQL inline).
- Destructive operations that would be emitted (highlighted, with
  the closed enum classification from §7.3).
- Redaction summaries for any `args` payload that would be written
  to `history.jsonl`.

`rustio plan` is the contract surface for review tools and CI. A
pull request that touches `draft.toml` must include the `plan`
output, the same way a Terraform PR includes a plan diff.

### 6.2 `rustio commit`

Atomic. Either every file in the plan is written, or none are.
Implementation uses a staged-write strategy: produce all outputs
into `.rustio/tmp/<txn-id>/`, validate the complete set, then swap
files into place. Partial writes are forbidden — a `commit` that
fails halfway must leave the project on disk indistinguishable from
its pre-commit state.

After a successful `commit`:

- One `commit` event is appended to `history.jsonl` with the list of
  files written, deleted, and the migration filename emitted (if
  any). This is a single line; per-file events from §5.4 / §5.5 are
  separate child entries with a `parent` field referencing the
  `commit` event's ULID.
- `draft.toml` is unchanged. `commit` consumes `draft.toml`; it
  does not mutate it.
- `builder.lock` is unchanged unless an explicit upgrade ran.

### 6.3 No network in `plan` or `commit`

Doctrine B9. Neither verb opens a socket. The deterministic core is
network-free. The Advisory layer's `rustio suggest` and any future
import command have their own contracts and never reach `plan` or
`commit` directly.

This rule is what makes Rustio adoptable in air-gapped environments.
The same rule does not bind the Framework runtime — that is the
Framework's own contract surface (`DESIGN_RECOVERY.md`,
`DESIGN_EMAIL.md` govern its runtime network discipline).

### 6.4 Idempotence under canonical conditions

Running `rustio commit` twice in succession with no intervening
changes to `draft.toml`, `history.jsonl`, or `builder.lock`, and
under the environment fixings of §4.4, produces no on-disk
modification on the second run beyond timestamp updates the OS may
impose. The SchemaHash check in §5.4 case 2 guarantees this for
generated files; the replay invariant guarantees this for
`history.jsonl`.

---

## 7. Migrations under the Builder

The Framework's existing migration contract is **append-only by
contract**. Doctrine B6 binds the Builder to extend that contract,
never weaken it.

### 7.1 New migrations only

Every change to `draft.toml` that affects the database schema
produces a new numerically prefixed migration file. The Builder
never edits an existing migration, regardless of whether it has been
applied.

### 7.2 Diff against committed state

The Builder generates migrations by comparing the current
`draft.toml` to the schema captured at the last `commit` event in
`history.jsonl`. It does not connect to the running database.
Reproducibility on a fresh checkout against any database state is
preserved.

**Operator responsibility.** Whether the database matches the
committed migration state is the operator's concern. The Builder
makes no guarantees about runtime applicability; the `rustio
migrate apply` runner is the framework-level boundary for that
question.

### 7.3 Destructive-operation enumeration (closed list)

A migration emitting any of the following is **destructive** and
requires `--accept-destructive` on `rustio commit`. The flag is
independent of `--yes` and `--force`; none implies the others.

| Class | SQL pattern | Notes |
|---|---|---|
| D1 | `DROP TABLE` | Includes implicit drops via model removal |
| D2 | `DROP COLUMN` | Includes implicit drops via field removal |
| D3 | `ALTER COLUMN ... TYPE` with a lossy cast | e.g. `text → varchar(N)`, narrower integer, scale reduction in `decimal` |
| D4 | `ADD COLUMN ... NOT NULL` without a `DEFAULT` | Fails on existing rows |
| D5 | `ALTER COLUMN ... SET NOT NULL` | When existing rows may be NULL |
| D6 | `ALTER TABLE ... ADD UNIQUE` | When existing rows may contain duplicates |
| D7 | `ALTER TABLE ... ADD CHECK` | When existing rows may violate the predicate |
| D8 | `DROP CONSTRAINT` | When constraint is referenced by FK or trigger |
| D9 | `ALTER TABLE ... RENAME COLUMN` without an enabling `rename_field` event | Doctrine B12; otherwise non-destructive (§7.4) |
| D10 | `ALTER TABLE ... RENAME` without an enabling `rename_model` event | Doctrine B12; otherwise non-destructive (§7.4) |

"Any other data-loss operation" is not a doctrine-grade phrase and
is not part of the enumeration. Future destructive classes are
added to this table via doctrine amendment, not by implementer
judgement.

### 7.4 Rename semantics (Doctrine B12)

> **A model or field rename is performed only when the corresponding `rename_*` event exists in `history.jsonl`.**

The discrimination rule:

1. Developer runs `rustio rename model Patient Resident`.
2. Builder emits a `rename_model` event:
   `{"op": "rename_model", "args": {"from": "Patient", "to": "Resident"}}`.
3. On the next `rustio commit`, the generator detects that
   `Patient` is absent from `draft.toml` but `Resident` is present
   with a matching field set, and the `rename_model` event is the
   most recent operation affecting either name.
4. The generated migration emits `ALTER TABLE patients RENAME TO
   residents` (D9/D10 are reclassified non-destructive only in this
   case).

If the developer hand-edits `draft.toml` to drop `Patient` and add
`Resident` without emitting the rename event:

- The Builder sees a drop+create.
- D1 + create classification applies.
- `--accept-destructive` is required.
- Data is lost.

The doctrine intentionally privileges *event-recorded intent* over
*final-state similarity*. Silent data preservation through structural
inference is not a Builder feature; it is a footgun.

### 7.5 Rollback hints (free-form, not contract)

Every migration the Builder emits ships with a SQL comment block
describing how to reverse it. The hint is informational only:

- It is **free-form human-readable text**, not parseable by any
  machine consumer.
- The Framework's migration runner must not parse it. A grep proof
  in §10 enforces this.
- Future Builder needs for structured per-migration metadata belong
  in a sidecar file (`migrations/<n>.sidecar.toml`), not in SQL
  comments.

### 7.6 Migrations are part of the project contract surface

Once written, a migration file is owned by the project, not the
Builder. The Builder never reads its own past migrations to decide
what to do next. Past migrations are inputs to the database, not to
the Builder. Builder state about migrations lives exclusively in
`history.jsonl`.

---

## 8. Feature toggles

`draft.toml` carries a `[features]` table. Members fall into two
disjoint classes.

### 8.1 Toggleable features (developer choice)

The Builder accepts `true` or `false` for these. Each maps to a
Framework capability that is optional by design:

| Feature | Effect when `true` | Effect when `false` | Framework binding |
|---|---|---|---|
| `mfa` | Enables R3 TOTP enrolment surfaces | Generated admin omits MFA pages | `DESIGN_R3_MFA.md` |
| `email` | Wires the framework's email sender into recovery flows | Generated recovery uses log-only mailer | `DESIGN_EMAIL.md`, `DESIGN_RECOVERY.md` |
| `soft_delete` | (When Framework ships it.) Generates soft-delete ModelAdmin overrides | Omits | Pending Framework support |
| `dark_mode_default` | Project ships with `data-rio-theme="dark"` as the bootstrap default | Light default | `DESIGN_SYSTEM.md` |

A new feature lands here only after the corresponding Framework
contract document accepts it. Until then, the Builder rejects
unknown feature names.

### 8.2 Doctrine-bound features (never disable)

The Builder refuses any `draft.toml` that sets these to `false`
(Doctrine B7). The features encode framework-wide doctrine; the
Builder is not authorized to bypass them by configuration.

| Feature | Framework doctrine | Why this cannot be disabled |
|---|---|---|
| `authentication` | `DESIGN_SESSIONS.md` whole | The framework is an admin framework; an unauthenticated admin is not a Rustio project |
| `audit` | `DESIGN_AUDIT.md` Doctrine 8 | Every authority mutation emits a typed audit row. A project that disables this is not Rustio |
| `csrf_protection` | Framework default; covered by `DESIGN_SESSIONS.md` middleware ordering | CSRF on mutating routes is non-negotiable |
| `correlation_id` | `DESIGN_AUDIT.md` §11 + middleware ordering | Forensic correlation is non-optional |
| `redaction` | `DESIGN_AUDIT.md` §5.3 (Doctrine 11) | Secrets at rest in audit rows is forbidden framework-wide |
| `single_writer_invalidation` | `DESIGN_SESSIONS.md` Doctrine 22 | Doctrine 22 is enforced at the Framework grep proof; the Builder cannot emit code that subverts it |

`draft.toml` may name these for documentation purposes but only
with value `true`. A `commit` against a doctrine-bound member set
to `false` fails with `error: doctrine-bound feature 'X' cannot be
disabled (DESIGN_BUILDER.md §8.2)`.

---

## 9. Layer boundaries

The Builder is one of four layers. The boundary between each pair
is strict.

### 9.1 Framework ↔ Builder

| Direction | Permitted |
|---|---|
| Builder → Framework | Calls public Framework APIs (`Model`, `ModelAdmin`, …) in the code it emits |
| Framework → Builder | **Forbidden.** Framework code never references Builder concepts. Doctrine B10 enforces this with a grep proof |

The Framework remains usable without the Builder. A project written
by hand against the Framework's public trait surface is a
first-class consumer; the Builder is a convenience, not a
prerequisite.

The CI guard that forbids Tier-2 symbols
(`HasSchema`, `ModelSchema`, …) extends to the Builder: the
Builder cannot introduce a parallel runtime concept and call it a
non-Tier-2 abstraction. The "no second runtime" rule applies
unchanged.

### 9.2 Builder ↔ generated code

The Builder emits source files. Once emitted, those files are
compiled by `cargo` like any other. There is no Builder-specific
runtime library that the generated code links against. Generated
code depends only on `rustio-admin` and the developer's other
declared dependencies.

The Builder never injects runtime behaviour. It does not register
hooks, ship a privileged daemon, or open a control socket. After
`rustio commit` exits, no Builder code is running anywhere.

### 9.3 Builder ↔ Advisory AI (boundary only)

The Advisory AI layer's internal contract lives in a future
`DESIGN_ADVICE.md`. This document binds only the *boundary*:

| Direction | Permitted |
|---|---|
| Advisory → `.rustio/proposals/` | Yes |
| Advisory → `.rustio/draft.toml` | **Forbidden** |
| Advisory → `src/_generated/` | **Forbidden** |
| Advisory → `migrations/` | **Forbidden** |
| Advisory → `history.jsonl` | **Forbidden** — only `cli::history::append` writes there |
| Generator → `.rustio/proposals/` | **Forbidden** — Doctrine B13. The deterministic core never reads proposals |

> **Doctrine B13. Advisory output is never source of truth.**

A future `rustio review <id> --accept` command takes Advisor-authored
content and writes it through Builder commands to `draft.toml`. The
operation is human-initiated; it carries a verdict event in
`history.jsonl` that records the proposal ID, reviewer identifier,
and verdict. Acceptance under `RUSTIO_AGENT_MODE=1` is permitted but
the verdict event uses an `actor` prefixed `agent:` so a grep on the
log distinguishes human-approved from agent-approved writes. No flag
suppresses the verdict event.

### 9.4 Builder ↔ Studio (consumption only)

Studio's specification lives in a future `DESIGN_STUDIO.md`. This
document binds only the consumption rule:

| Direction | Permitted |
|---|---|
| Studio → Builder (via CLI / stable RPC) | Yes |
| Studio → `.rustio/*` directly | **Forbidden.** Studio must invoke Builder commands |
| Studio → Framework directly | **Forbidden.** Studio is a Builder consumer |

Studio does not retroactively shape Builder design.

---

## 10. Grep proofs and CI-enforceable obligations

The following invariants are enforced by automated checks at PR
time. Each line is a command that must produce the stated result.
The checks live alongside the existing Tier-2-symbol guard in
`.github/workflows/ci.yml`.

### 10.1 Single-writer for `history.jsonl` (Doctrine B3)

```sh
# Exactly one function definition writing to history.jsonl:
git grep -nE 'fn[[:space:]]+append.*history\.jsonl' -- crates/rustio-admin-cli/src/history.rs

# No other file constructs a write handle to history.jsonl:
git grep -nE 'OpenOptions.*append|File::create.*history\.jsonl' -- crates/rustio-admin-cli/src/ \
  | grep -v 'crates/rustio-admin-cli/src/history.rs' \
  | wc -l   # must be 0
```

### 10.2 Framework does not read Builder metadata (Doctrine B10)

```sh
git grep -nE '@generated by rustio|SPDX-SchemaHash|SPDX-EmitterVersion' -- crates/rustio-admin/src/
# must produce zero matches
```

### 10.3 Audit-by-default cannot be disabled (Doctrine B7)

The CLI's `draft.toml` parser refuses any of the §8.2 features set
to `false`. A drift test in `cli::config::tests` asserts the closed
list is exhaustive against the feature parser's accepted values.
The doctrine-bound feature names are also grepped:

```sh
git grep -nE 'audit[[:space:]]*=[[:space:]]*false' -- '*.toml' 'crates/' 'examples/'
# must produce zero matches outside test fixtures intended to verify rejection
```

### 10.4 No second TOML emitter (Doctrine B1 / §4.4)

```sh
git grep -nE 'toml::to_string|toml_edit::.*to_string' -- crates/rustio-admin-cli/src/ \
  | grep -v 'crates/rustio-admin-cli/src/toml.rs' \
  | wc -l   # must be 0
```

### 10.5 No second redactor (Doctrine B4)

```sh
git grep -nE 'fn[[:space:]]+redact' -- crates/rustio-admin-cli/src/ \
  | grep -v 'crates/rustio-admin-cli/src/redact.rs' \
  | wc -l   # must be 0
```

### 10.6 Migration files are not edited (Doctrine B6)

A pre-merge check inspects the diff for any modification (not
addition) to a file under `migrations/` and refuses the merge. The
check is implemented as a separate CI step; the doctrine-level
obligation is that an applied migration file is immutable.

### 10.7 No Builder access to `src/app/` (§5.6)

```sh
git grep -nE 'src/app/' -- crates/rustio-admin-cli/src/
# must produce zero matches outside doc strings
```

### 10.8 Drift tests

| Drift test | Asserts |
|---|---|
| `cli::history::tests::op_enum_matches_serialized` | The `HistoryOp` enum and the on-disk `op` strings stay in lockstep |
| `cli::config::tests::doctrine_bound_features_rejected` | Every §8.2 feature set to `false` is refused by the parser |
| `cli::redact::tests::no_4char_input_substring_leaks` | Property test, symmetric to `DESIGN_AUDIT.md` §5.3 |
| `cli::toml::tests::canonical_round_trip` | Canonical emitter produces byte-identical output across two runs |
| `cli::history::tests::replay_rebuilds_draft` | Replay invariant §4.2.2 against a fixture corpus |
| `cli::schema_migration::tests::all_pairs_covered` | Every supported `(from_schema, to_schema)` pair has a transformer |

---

## 11. What this doctrine does NOT govern

- **The exhaustive `draft.toml` schema.** The full field list,
  permitted modifiers, and `schema_version` migration procedure are
  a separate document versioned alongside the schema.
- **The Advisory AI's internal contract.** Provider abstraction,
  proposal format, prompt-hash discipline, and privacy guarantees
  live in `DESIGN_ADVICE.md` (future). This document binds only the
  boundary.
- **The Studio surface.** UI, RPC protocol, authentication, and
  packaging belong to `DESIGN_STUDIO.md` (future). This document
  binds only Studio's consumption rights.
- **CLI verb naming.** The verbs named in this document (`plan`,
  `commit`, `add`, `remove`, `set`, `status`, `log`, `undo`, `redo`,
  `diff`, `merge`, `upgrade`, `review`, `suggest`) describe
  responsibilities, not a frozen API. The CLI may rename, split, or
  merge verbs as long as every invariant in §2 continues to hold.
- **Sequencing.** Version numbers, milestone ordering, and target
  release dates live in `archive/ROADMAP_BUILDER.md`. Doctrine outlives any
  schedule.
- **Runtime behaviour.** Nothing in this document changes how the
  framework binary serves requests, manages sessions, records audit
  events, or invalidates state at runtime. The Framework's existing
  doctrine is authoritative for the runtime surface and is
  unaffected by the Builder layer.

---

## 12. Companion doctrine

This document is one of a family. The others that interact with the
Builder at a contract level:

| Doctrine | Relationship |
|---|---|
| [`DESIGN_AUDIT.md`](DESIGN_AUDIT.md) | Doctrine 8 (audit-by-default) is doctrine-bound in §8.2; Doctrine 11 (no secrets) extends to `history.jsonl` via Doctrine B4; Doctrine 18 (typed events) shapes the `HistoryOp` enum; the append-only contract in §5.6 binds on `history.jsonl` via Doctrine B2 |
| [`DESIGN_SESSIONS.md`](DESIGN_SESSIONS.md) | Doctrine 22 (single-writer for `revoked_at`) is the model for Doctrine B3 (single-writer for `history.jsonl`); Doctrine 22 itself is doctrine-bound in §8.2 (`single_writer_invalidation`) |
| [`DESIGN_RECOVERY.md`](DESIGN_RECOVERY.md), [`DESIGN_R2_ORGANISATIONAL.md`](DESIGN_R2_ORGANISATIONAL.md), [`DESIGN_R3_MFA.md`](DESIGN_R3_MFA.md), [`DESIGN_R4_EMERGENCY.md`](DESIGN_R4_EMERGENCY.md) | The Builder may emit code that wires recovery / MFA / emergency flows; it cannot redefine their contracts |
| [`DESIGN_EMAIL.md`](DESIGN_EMAIL.md) | `[features] email` toggles the Builder's wiring of the framework's email sender; the email contract itself is unaffected |
| [`DESIGN_SYSTEM.md`](DESIGN_SYSTEM.md), [`DESIGN_CHROME.md`](DESIGN_CHROME.md), [`DESIGN_DOCTRINE.md`](DESIGN_DOCTRINE.md) | Theme tokens emitted by the Builder must respect token ownership rules; the Builder is one consumer of the design-system contract, not its author |
| `DESIGN_ADVICE.md` (future) | Specifies the Advisory AI's internal behaviour. This document specifies only its boundary |
| `DESIGN_STUDIO.md` (future) | Specifies Studio's surface. This document specifies only its consumption rights |

---

## 13. Doctrine review

Any pull request that adds or modifies code under
`crates/rustio-admin-cli/` outside the existing migration / user /
group / doctor subcommands must reference this document. Reviewers
are expected to read the relevant section before approving.

If an invariant in §2 needs to change, the change lands here first,
before any code. Updates to this document carry a CHANGELOG entry the
same way framework runtime changes do.

The doctrine is not closed. Significant gaps still exist —
`main.rs` splice details, `rustio doctor builder` obligations, and
the Advisory verdict-event field schema all remain undocumented. The
companion review document [`REVIEW_BUILDER_DOCTRINE.md`](../archive/REVIEW_BUILDER_DOCTRINE.md)
tracks resolved and outstanding findings. Future doctrine revisions
close those gaps; until they do, the doctrine is implementation-grade
for the blockers listed in §2 but not yet complete for v1.0 freeze.
