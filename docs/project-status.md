# RustIO Project Status

A map of what is active, what is earlier/legacy, what is a separate companion,
and what is reserved — so a new developer, sponsor, or crates.io user knows what
to use today.

## Use today

The active, canonical RustIO Admin crates:

- `rustio-admin` — the library (admin engine).
- `rustio-admin-cli` — the CLI; provides the `rustio-admin` binary. Lightweight by default; database & authority verbs need `--features db`.
- `rustio-admin-macros` — proc-macros, re-exported from `rustio-admin`.
- `rustio-admin-assets` — std-only leaf crate holding the embedded admin templates, shared by the library and the CLI.
- `rio-theme` — build-time theme engine.

```bash
cargo install rustio-admin-cli
rustio-admin --help
```

These crates ship together at the same version (see [versioning.md](./versioning.md)).

## Active canonical line

`rustio-admin` is the current active project: the Rust-first admin and
operational systems engine — Postgres-first, single-binary, with authority,
sessions, recovery, RBAC, and audit designed as one system.

- It is **not** the earlier monolithic `rustio-core` line.
- It is **not** the unrelated `rustio` crate on crates.io.
- The latest published version is **0.31.0** (released 2026-07-03).
- `main` may contain unreleased work committed after the `v0.31.0` tag.

## Earlier / legacy line

`rustio-core` is an **earlier RustIO line**, published separately:

- An earlier RustIO runtime/core (HTTP server, router, middleware, ORM, admin,
  migrations), associated with the sibling [`rustio`](https://github.com/abdulwahed-sweden/rustio)
  repository.
- Its version may look higher (the `2.x` line) than `rustio-admin`'s `0.x` — the
  numbers are independent and do not indicate which is current.
- It is **not** the current canonical project; it is **superseded by the current
  `rustio-admin` direction**.
- It is preserved for history and compatibility. It has not been deleted, and
  ownership is unchanged.

If you are starting today, use `rustio-admin`, not `rustio-core`.

## Separate companion project

`rustio-draft` turns a natural-language project brief into a safe `schema.json`
for `rustio-admin`.

- It lives in its **own repository**:
  <https://github.com/abdulwahed-sweden/rustio-draft>.
- It is a **setup-time** schema drafting tool, not part of the `rustio-admin`
  workspace and not a runtime dependency.
- Its output (`schema.json`) is handed off to `rustio-admin import` → `plan` →
  human-approved `commit`.

Its boundary:

> **AI drafts. RustIO validates. Diff protects. Human approves.**

It is not an AI runtime, not an ORM, and not a migration engine, and it does not
make `rustio-admin` depend on any LLM at runtime.

## Reserved commercial direction

`rustio-pro-*` is a **reserved** future commercial/pro layer. Advanced or
operational capabilities that would widen the doctrine-governed core live in a
separate `rustio-pro-*` family of crates, **never inside `rustio-admin`**. It is
a direction, not a shipped product — nothing here claims otherwise. See
[`commercial-model.md`](./commercial-model.md).

## Name to avoid

The crate name **`rustio`** on crates.io is **not** the current install target
and is unrelated to this project. Do not use `cargo install rustio` in docs or
setup steps. The install target is `rustio-admin-cli`, which provides the
`rustio-admin` binary.

## Release status

- **crates.io:** the four active crates are published at **0.31.0**.
- **Git tags:** latest is `v0.31.0` (annotated `vX.Y.Z` tags).
- **GitHub Releases:** current — see the
  [v0.31.0 release](https://github.com/abdulwahed-sweden/rustio-admin/releases/tag/v0.31.0).
- **Latest release:** **0.31.0** (2026-07-03) — tag, GitHub Release, and
  crates.io, cut per [`RELEASING.md`](../RELEASING.md).
