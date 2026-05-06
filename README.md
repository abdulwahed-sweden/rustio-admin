# rustio-admin

Django Admin, but for Rust. A small, focused, beautiful admin framework for Postgres-backed Rust apps.

> Status: pre-alpha. The workspace is being assembled in phases. See
> [`rustio-admin-strategic-reset-plan.md`](./rustio-admin-strategic-reset-plan.md)
> for the architecture and roadmap.

## Workspace layout

| Crate | Purpose |
|---|---|
| `rustio-admin` | The library. Re-exports the macros. |
| `rustio-admin-macros` | Proc-macros (re-exported from `rustio-admin`). |
| `rustio-admin-cli` | The `rustio` binary. |

`examples/minimal` is the canonical consumer — by Phase 1's end it is the ~50-line demo that defines the public API.

## Build

```sh
cargo build --workspace
```
