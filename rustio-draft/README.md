# rustio-draft

Setup-time **genesis** for [rustio-admin](../README.md): turn a natural-language
brief into a `schema.json`, which `rustio-admin import` then applies
deterministically.

> **rustio-draft is the only part of the ecosystem that calls an LLM.** It is a
> standalone workspace, excluded from the framework, so the runtime library and
> CLI never gain a network or LLM dependency. RustIO itself runs no AI — rustio-draft
> *authors* a schema; `rustio-admin` *applies* it. Full design:
> [`../docs/RUSTIO_DRAFT_SCOPE.md`](../docs/RUSTIO_DRAFT_SCOPE.md).

## Usage

```sh
export ANTHROPIC_API_KEY=sk-ant-...

cargo run -- new "a booking system for a salon: clients, staff, appointments"
# → writes ./schema.json, then prints:
#     rustio-admin import schema.json
#     rustio-admin plan
#     rustio-admin commit
```

Flags: `--out <path>` (default `schema.json`), `--model <id>` (default
`claude-opus-4-8`), `--max-tokens <n>` (default 8000), `--force` (overwrite).

## How it works

1. Sends the brief to the Claude Messages API with **structured outputs** — a
   JSON Schema whose `type` field is an `enum` of the builder's `FIELD_TYPES`,
   so the model cannot emit a type `import` would reject.
2. Re-validates the output with the same name/type rules the builder uses.
3. Writes `schema.json`. **It never applies the schema** — you review it and run
   `rustio-admin import` / `plan` / `commit` yourself.

## Status

Phase **F1** (the engine). Field types are limited to the builder's MVP set
(`text`, `integer`, `boolean`, `timestamp`); relations are modelled as plain
`integer` `*_id` fields. See the scope doc for F2–F5.
