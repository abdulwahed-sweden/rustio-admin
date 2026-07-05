# Getting started

From nothing to a **running admin panel** — login, roles, and an audit trail
— in about ten minutes. Two ways in:

- **See it work now** — run the bundled **translation-agency** example and sign
  in to a real, seeded admin, having written no code.
- **Build your own** — describe your data as Rust structs and watch RustIO
  render the admin.

RustIO (`rustio-admin`) is Postgres-first: you describe your data as Rust
structs, and RustIO renders the admin — no HTML, no separate frontend, no
extra services.

---

## Before you start

- **Rust 1.94 or newer** — check with `rustup show`. (The floor tracks
  `sqlx` 0.9's MSRV.)
- **A reachable PostgreSQL** (RustIO is Postgres-only, by design — no SQLite
  fallback). A native install is fine; or run one in Docker:

  ```sh
  docker run --name rio-pg -e POSTGRES_PASSWORD=dev -p 5432:5432 -d postgres:16
  ```

You do **not** need Node, a bundler, or any frontend toolchain.

---

## See it work now — the translation-agency example

The fastest way to understand RustIO is to run a real admin. The repository
ships a **translation-agency** example — `Translator` and `Task` models, already
registered, with example rows — so the first thing you do is sign in to
something real.

```sh
git clone https://github.com/abdulwahed-sweden/rustio-admin
cd rustio-admin/examples/translation-agency
cp .env.example .env                # defaults are fine for local Postgres
createdb translation_agency_dev
cargo run                           # applies migrations + seeds, then serves
```

In another shell, install the CLI and create your login (run it from the same
example directory so it reads that `.env`):

```sh
cargo install rustio-admin-cli      # provides the `rustio-admin` binary
rustio-admin user create --email coordinator@agency.local --role administrator
```

Open **<http://127.0.0.1:8000/admin>**, sign in, and you have a working
dispatch admin:

```text
Translators (3)           Tasks (3)
  Amina Hassan              Medical discharge summary · ar · in_progress
  Pierre Dubois             Rental contract           · en · review
  Sven Karlsson             Court hearing transcript  · en · available
```

List, create, edit, search, delete — plus the login you just used and an
audit trail of every change — and you wrote **zero lines of code**. That's
the whole idea. The rest of this guide shows how to build your own.

---

## How a RustIO project fits together

Before you build one yourself, the shape — so each step makes sense:

- You write a **Rust struct** for each thing you manage (a `Translator`, a
  `Task`), plus a **SQL migration** that creates its table.
- Three small pieces turn that struct into admin pages:
  - `#[derive(RustioAdmin)]` — generates the list / create / edit / delete UI.
  - `impl Model` — the contract with Postgres (table, columns, row mapping).
    The CLI writes this for you.
  - `impl ModelAdmin` — optional customisation (columns shown, what's
    searchable, ordering). Empty `{}` accepts sensible defaults.
- You **register** each model by hand — `Admin::new().model::<Translator>()`.
  The sidebar matches your source; nothing is auto-discovered behind your back.

The translation-agency example above ships all of this; below you build the
same models yourself, one command at a time.

---

## Build your own — from struct to admin

### 1. Create a clean-slate project

```sh
rustio-admin new agency             # at "Project type", choose 1) custom
cd agency
```

`new` runs a short, calm wizard (project name → type → database), then writes
a matching `.env`. Scripting or in CI? `rustio-admin startproject agency` skips
the wizard and writes `.env.example` instead.

The `custom` scaffold is neutral — no demo models, nothing to delete later:

```text
agency/
├── Cargo.toml
├── .env                    # written by the wizard (.env.example for startproject)
├── .gitignore
├── README.md
├── migrations/             # empty; your first model adds the first migration
├── src/
│   └── main.rs             # wires the server + an empty Admin::new(), with // rustio: markers
└── templates/
    └── home.html
```

### 2. Point it at your database

```sh
createdb agency_dev         # the database name the wizard showed you
```

> RustIO reads `DATABASE_URL` from `.env` in the project directory. The
> scaffold's `.gitignore` already keeps `.env` out of version control.

### 3. Create your admin login and run

```sh
rustio-admin migrate apply  # a brand-new custom project has no model migrations yet — expected
rustio-admin user create --email coordinator@agency.local --role administrator
cargo run
```

After each step the CLI points you to the next one — `migrate apply` → *create
your admin login*; `user create` → *launch with `cargo run`* — so you always
know where you stand. `user create` prompts for the password twice (input
hidden); there's deliberately no `--password` flag in everyday use.

Open **<http://127.0.0.1:8000/>** (your homepage) and
**<http://127.0.0.1:8000/admin>** (sign in with the account above).

> **Port already in use?** Edit the listen address near the bottom of
> `src/main.rs` (e.g. `127.0.0.1:8001`) and re-run.

### 4. Add your first model

This is the loop you repeat for every kind of thing you manage. One command
scaffolds the model and its migration:

```sh
rustio-admin startapp translator \
  --field name:str \
  --field email:email \
  --field languages:str \
  --field active:bool
```

**Creates** a `Translator` with four columns:

```text
Translator
├── name        str        (TEXT)
├── email       email      (TEXT, format-validated)
├── languages   str        (TEXT)
└── active      bool       (BOOLEAN)
```

**Generates**, and tells you exactly what to wire:

```text
✓ src/translator.rs                       the Rust model (+ admin defaults)
✓ migrations/0001_create_translators.sql  the table

  …plus an admin page, search, and per-model permissions at boot.

3 edits in src/main.rs
  under  // rustio: modules  add  mod translator;
  under  // rustio: imports   add  use translator::Translator;
  under  // rustio: models    add  .model::<Translator>()
```

**`startapp` never edits `main.rs` — you stay the author.** Paste each line
under its matching `// rustio:` marker:

```rust
mod translator;                      // under `// rustio: modules`
use translator::Translator;          // under `// rustio: imports`

let admin = Admin::new()
    .model::<Translator>();          // under `// rustio: models`
```

Then apply the migration and re-run:

```sh
rustio-admin migrate apply           # creates the `translators` table
cargo run
```

Your pages are live at **<http://127.0.0.1:8000/admin/translators>** — list,
create, edit, search, delete — permission-seeded (`view_translator`,
`add_translator`, `change_translator`, `delete_translator`) at boot.

To add the second model — a translation `Task` with a status and a foreign key
to the translator — follow the same loop, or see the full walkthrough in the
[translation-agency Quick Start](./quickstart-translation-agency.md).

### Field types — and the best reference

`--field <name>:<type>` accepts a closed vocabulary, so model and migration
always line up:

```text
str   text   int   bigint   float   decimal   bool
timestamp   date   time   uuid   email   phone   json
fk:<Model>            a foreign key — renders as a link to the related row
choice:a,b,c          a fixed set — a CHECK constraint and a dropdown
```

Two more examples by way of illustration:

```sh
--field author:fk:Author             # links each row to an Author
--field status:choice:draft,published,archived
```

The command's own help is the live, example-first reference:

```sh
rustio-admin startapp --help         # examples, every field type, relations, choices
```

> **Tip:** name fields after your business domain. And avoid SQL keywords as
> bare names — `order` works (RustIO quotes it), but `order_id` reads better
> for a foreign key.

### Customising a model

The generated `impl ModelAdmin for Translator {}` accepts every default. Fill
it in to shape the pages — the Django-style hooks:

```rust
impl ModelAdmin for Translator {
    fn list_display()  -> &'static [&'static str] { &["name", "email", "languages", "active"] }
    fn search_fields() -> &'static [&'static str] { &["name", "email", "languages"] }
    fn list_filter()   -> &'static [&'static str] { &["active"] }
    fn ordering()      -> &'static [&'static str] { &["name"] }
}
```

See the [`ModelAdmin` reference](./modeladmin.md) for every method.

---

## What you got for free

After your first sign-in, all of this is already wired in — no extra
configuration, no separate services:

- **Session-backed admin login.** Argon2id-hashed passwords; session tokens
  are random and stored only as a SHA-256 hash.
- **Users, groups, and a 5-tier role ladder** (User · Staff · Supervisor ·
  Administrator · Developer), with per-model permissions and a permission-grid
  editor for groups.
- **Password recovery** — self-service forgot/reset, plus admin-driven reset,
  lock, unlock, and session revoke.
- **An audit trail.** Every authority change writes a typed row to
  `rustio_admin_actions` with a per-request correlation ID; browse it at
  `/admin/history`.
- **Foreign keys that read like data** — `fk:` columns show the related row's
  label and link through, not a raw `5`.
- **Server-rendered pages, override-ready.** Every page is baked into the
  binary; set `RUSTIO_TEMPLATE_DIR=templates` and a disk-side
  `templates/admin/<page>.html` wins, no recompile.

You wrote no HTML, CSS, JS, router boilerplate, ORM, search engine, or
migration runner — the framework ships all of it.

---

## Where to go next

- **See a full worked domain** — [Quick Start: a translation-agency
  admin](./quickstart-translation-agency.md) builds translators, translation
  tasks with a status state, and an automatic audit trail — and draws a clear
  line between what RustIO enforces (authority, permissions, audit) and what you
  write (the matching rule, the legal transitions).
- **Customise the look** — `Admin::accent_color("#…")`,
  `Admin::site_branding(…)`, or generate a full palette with
  `rustio-admin theme generate --brand "#…"`.
- **Record the *why* behind your project** — `rustio-admin memory` keeps a
  governed project memory (decisions, rejected ideas, intent) a teammate or AI
  assistant reads for context. Guide: [`memory.md`](./memory.md).
- **Working with an AI coding assistant?** `rustio-admin ai init` then
  `rustio-admin ai status` set what it may do here (Allowed / Needs approval /
  Blocked) — changes are proposed, approved, and applied as explicit steps.
  RustIO runs no AI itself; it governs the one you bring.
- **Wonder why it says "no" by default?** — [Why
  refusal-first?](./why-refusal-first.md) explains why strict-by-default makes
  you faster, and how to grant explicitly when you mean to.
- **See what you're protected from** — [What RustIO stops out of the
  box](./security.md) lists the shipping security defaults (CSRF, refusal-first
  permissions, rate limiting, hashed-at-rest credentials, audit) — and, just as
  honestly, what it does *not* do for you.
- **Understand the internals** — [`architecture.md`](./architecture.md) maps
  which module owns what; the [`design/`](./design/) contracts govern the
  security-sensitive surfaces.

---

## If you remember nothing else

```sh
rustio-admin new <name>          # scaffold a clean-slate project (choose "custom")
cd <name> && createdb <name>_dev
rustio-admin migrate apply
rustio-admin user create --email admin@<name>.local --role administrator
cargo run                        # homepage at /, admin at /admin

# add a model, any time:
rustio-admin startapp <model> --field name:str --field …
#   → paste its 3 `// rustio:` lines into src/main.rs, then migrate apply + cargo run
```

Everything else — auth, roles, recovery, audit — is already there.
