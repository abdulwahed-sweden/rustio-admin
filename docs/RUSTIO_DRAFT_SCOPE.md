# rustio-draft — Scope (proposal)

**Status:** proposal / not built. Tracks DEFERRED.md item 6.
**One line:** a *separate* setup-time tool that turns a natural-language brief into a `schema.json`, which `rustio-admin import` then applies deterministically.

`rustio-draft` is the **only** place in the ecosystem that calls an LLM. It lives
outside the OSS `rustio-admin` runtime and CLI on purpose — see "Boundary".

---

## 1. Why a separate tool (the boundary)

The OSS repo's stated stance is **"RustIO runs no AI"** (the `ai` CLI verb says
so literally; `.rustio/ai.toml` governs an *external* assistant, not an embedded
one). The runtime is Postgres-only, network-free, dependency-disciplined, and
deterministic. A live LLM call would break all of that at once: a network
client, an API-key path, and non-determinism on a code path that must stay
reproducible.

So genesis is split in two, and the split is the whole point:

| Half | Where | Determinism |
|---|---|---|
| **Author** a `schema.json` from a brief (AI) | `rustio-draft` (this tool) | non-deterministic, network, opt-in |
| **Apply** a `schema.json` to a project (codegen) | `rustio-admin import` → `plan` → `commit` (already shipped) | deterministic, network-free, atomic |

`rustio-draft` produces a *file*. It never edits the user's `main.rs`, never runs
codegen, never touches the database. The deterministic half already exists and is
tested (Phase 4). rustio-draft only has to fill in the JSON.

## 2. Architecture

```
   natural-language brief
   "a booking system for a salon: clients, staff, appointments"
            │
            ▼
   ┌───────────────────────┐     Claude API (/v1/messages)
   │   rustio-draft         │ ──▶ model: claude-opus-4-8
   │   (Rust CLI)           │ ◀── structured output: json_schema
   └───────────────────────┘     (constrained to the import contract)
            │
            ▼
      schema.json  ── developer reviews / edits (plain JSON) ──┐
            │                                                  │
            ▼                                                  │
   rustio-admin import schema.json   ◀───────────────────────┘
   rustio-admin plan        # preview (read-only)
   rustio-admin commit      # apply atomically
            │
            ▼
      generated models + migration
```

The contract between the two halves is the `schema.json` that `import` already
consumes:

```json
{ "project": "salon",
  "models": [
    { "name": "Client", "fields": [
      { "name": "full_name", "type": "text" },
      { "name": "joined_at", "type": "timestamp" } ] } ] }
```

`type` ∈ the builder's closed `FIELD_TYPES` (`text · integer · boolean ·
timestamp` today). rustio-draft must emit only these — see §4.

## 3. Surfaces

- **MVP — CLI.** `rustio-draft new "<brief>" [--out schema.json] [--apply]`
  - default: write `schema.json`, print it, and print the exact
    `rustio-admin import … && plan && commit` next-steps (do **not** auto-apply).
  - `--apply`: chain into `rustio-admin import` for the impatient — still stops at
    `plan` for human review; never auto-`commit`.
- **Later (optional) — local web wizard.** The "rustio-draft/Genesis Studio" screen:
  brief in, proposed schema rendered as editable cards, "download schema.json".
  Same engine; a nicer front-end. Out of MVP scope.

Gate every run behind `.rustio/ai.toml` (reuse the existing `ai` policy): if the
policy forbids AI authoring, rustio-draft refuses with the four-part onboarding error
shape. This keeps the *one* permission surface the repo already defines.

## 4. LLM integration (the AI half)

Grounded in the current Claude API reference, not memory.

- **Provider / model.** Claude (Anthropic). Default **`claude-opus-4-8`** (the
  reference's mandated default; let the user override with `--model`). This is a
  single-call extraction/generation task → **not** an agent; no tool loop, no
  managed-agents surface.
- **Transport.** rustio-draft is Rust, and there is **no official Anthropic Rust
  SDK** → call the REST API directly over `reqwest` + `serde_json` (`POST
  https://api.anthropic.com/v1/messages`, headers `x-api-key`,
  `anthropic-version: 2023-06-01`). This is the reference's prescribed path for
  languages without an SDK.
- **Force a valid shape with structured outputs.** Send
  `output_config: { format: { type: "json_schema", schema: <IMPORT_SCHEMA> } }`
  where `<IMPORT_SCHEMA>` is a JSON Schema mirroring the import contract —
  crucially, `type` is an `enum` of the live `FIELD_TYPES`. The model then
  *cannot* emit a field type `import` would reject. Our contract (object / array
  / string / enum / `additionalProperties:false`) sits inside the supported
  structured-output subset, so this works without client-side repair.
- **Thinking.** `thinking: { type: "adaptive" }` — schema design benefits from
  reasoning; adaptive lets the model size it.
- **Output is small** (a JSON schema doc) → streaming optional; a single
  non-streaming call with a modest `max_tokens` (~4–8k) is fine.
- **Key handling.** `ANTHROPIC_API_KEY` from the environment only. rustio-draft never
  stores or logs it. No key → clean error pointing at the env var.
- **Determinism guard.** Whatever the model returns is **untrusted input**:
  re-validate it against the *same* validators `import` uses (`validate_model_name`,
  `validate_field_name`, `FIELD_TYPES`, secret-type refusal) before writing the
  file. rustio-draft's output is only ever a candidate `schema.json` a human approves.

## 5. Where it lives / dependencies

- A **separate workspace** (own repo, or a `rustio-draft/` sibling excluded from
  the framework workspace exactly like `examples/`). It may depend on
  `rustio-admin-cli`'s import path *or* simply shell out to the installed
  `rustio-admin` binary — preference: shell out, to keep zero coupling.
- New deps live **only** here: `reqwest` (rustls), `serde`/`serde_json`,
  `tokio`. The runtime crate's dependency budget is untouched.
- The runtime library **must not** gain a path/dev dependency on rustio-draft. CI's
  Tier-2 guard and the "no second runtime / no build step" rules are unaffected
  because rustio-draft is not in the workspace.

## 6. Phasing

1. **F1 — engine.** `brief → Claude (json_schema) → validated schema.json`. CLI
   `rustio-draft new`. Re-uses the import contract + validators. (Smallest useful
   slice; everything else is polish.)
2. **F2 — apply chain.** `--apply` to run `import` + `plan` and stop for review.
3. **F3 — refinement loop.** `rustio-draft refine schema.json "add a status enum
   to Appointment"` — feed the current schema + instruction back for an edit.
4. **F4 — web wizard.** The editable-cards Studio front-end over the same engine.
5. **F5 — richer contract.** Track `FIELD_TYPES` as the builder grows
   (relations, enums); the `enum` in the structured-output schema is generated
   from the live list so they never drift.

## 7. Non-goals

- **Not** in the OSS runtime or CLI. No `rustio-admin` verb calls an LLM.
- **No runtime AI**, ever — rustio-draft is setup-time only.
- **No auto-commit.** A human reviews the schema and runs `commit`.
- **Not an agent / no tool loop / no managed-agents** — it's one constrained call.
- **No new runtime dependencies** and no second runtime.

## 8. Open decisions

- Shell out to the `rustio-admin` binary vs. depend on the CLI crate's import
  function. (Lean: shell out.)
- Whether `FIELD_TYPES` is hand-mirrored in the structured-output schema or
  exported from `rustio-admin-cli` as a small public const for rustio-draft to read.
- Repo placement: separate repo vs. excluded sibling workspace.
- Provider-agnostic later? (MVP is Claude-only; the contract makes swapping the
  author trivial since the artifact is just JSON.)
