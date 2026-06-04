# Architecture — why the layout is what it is

This project is built to last years and to look the same as every other
RustIO Admin project. The structure is owned by RustIO; the project
adapts to it, not the other way around. That single rule is what makes
any RustIO project legible to any RustIO developer on sight.

## The doctrine

> **One workspace. `crates/` is the unit, fixed from day one.** Every
> project has exactly one foundation crate (`clinic-core`) and one server
> crate (`clinic-server`). Business capabilities are **crates** — coarse,
> plain-English, one per business domain, never one per table; tables are
> **modules inside** them. Capabilities depend only on `clinic-core`,
> never on each other; shared code sinks into `core` reactively, only when
> a *second* capability needs it. Templates and static assets live outside
> the crates with no build step. Migrations are central and append-only.
> Search is opt-in per field; sensitive fields are never indexed. You
> bring your **data**, not your structure. The only thing that ever moves
> is shared code sinking into `core`.

## Why crates from day one (not "modules now, crates later")

A framework's canonical layout is an **invariant** that tools, teams, and
time lean on for years. A structure that changes with project size is not
canonical. So the crate boundary exists from the first model:

- **Consistency at every scale.** 5 models or 500, the shape is identical.
  No one ever asks "did this split into a crate yet?"
- **The compiler enforces the boundary.** Modules in one crate are all
  mutually visible — separation would be honor-system and would erode over
  years. Separate crates make every cross-capability reach a *visible*
  `Cargo.toml` dependency. (And capabilities never depend on each other —
  only on `clinic-core`.)
- **The deferred split never happens cleanly.** Drawing the boundary is
  cheapest when the capability is empty. So draw it now.

The cost is a tiny `Cargo.toml` per capability — paid once, when it's free.

## What controls complexity at scale

**Capability count tracks the business, not the schema.** Five tables that
serve two business capabilities are two crates, not five. This one rule is
why the structure holds at 5, 50, and 500 models. The day it drifts toward
one-crate-per-table is the day the structure starts to die.

## "Bring your data. Leave your old structure behind."

The goal is **business migration, not project migration**. A company
exports its data from an old system and imports it into a *clean,
canonical* RustIO workspace — this one's shape. The destination is always
identical; the data adapts to the structure, never the reverse. RustIO
therefore never adopts a foreign project's layout. (A future
data-importer + read-only readiness report is the right primitive for
that — never a tool that drags a foreign crate, and its structure, in.)

## Search is opt-in and safe

Search is **off** until a model explicitly turns it on. Here, only
`Patient` does, via `search_index_column()`, backed by a Postgres
`tsvector` column the database maintains (see
`migrations/0001_patients.sql`). The framework never indexes anything
outside Postgres, and a model should never expose a sensitive field
through search.

## What this reference intentionally does NOT include

Honesty matters more than features. These were considered and left out
because they are not part of the framework today (inventing them would
make the reference a lie):

- **No external search engine.** Search is Postgres full-text only.
- **No `adopt` of foreign crates.** See the data-migration doctrine above.
- **No CSS framework coupling** (no Tailwind classes, no `ria-*` hooks).
  Theming is the framework's six `--rio-*` override tokens
  (`Admin::accent_color(...)`) plus an optional generated `tokens.css`
  applied via `RUSTIO_TOKENS_CSS`.
- **No `reporting` crate yet.** A capability crate appears when the
  business has that capability — not as an empty placeholder.

## How the wiring actually works (no magic)

1. Each capability crate exposes `pub fn register(admin: Admin) -> Admin`
   and adds its models with `admin.model::<T>()`.
2. `clinic-server/src/main.rs` builds `Admin::new()`, calls each
   capability's `register()`, runs migrations, and serves. It knows
   nothing about any capability beyond that one call.

You can trace every step by reading `main.rs` top to bottom. That is the
point: explicit over magic.
