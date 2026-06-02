# Getting started

This guide takes you from nothing to a **running admin panel for your own
data** — with login, roles, and an audit trail — in about ten minutes. By
the end you'll have created a project, added your first model, and signed in
to manage it.

RustIO (`rustio-admin`) is a Postgres-first admin framework for Rust: you
describe your data as Rust structs, and RustIO renders the admin — no HTML,
no separate frontend, no extra services.

---

## Before you start

- **Rust 1.88 or newer.** Check with `rustup show`.
- **A reachable PostgreSQL** (RustIO is Postgres-only, by design — there is
  no SQLite fallback). A native install is fine; or run one in Docker:

  ```sh
  docker run --name rio-pg -e POSTGRES_PASSWORD=dev -p 5432:5432 -d postgres:16
  ```

That's all. You do not need Node, a bundler, or any frontend toolchain.

---

## How a RustIO project fits together

Before the commands, the shape — so each step makes sense:

- You write a **Rust struct** for each thing you manage (a `Student`, an
  `Invoice`), plus a **SQL migration** that creates its table.
- Three small pieces turn that struct into admin pages:
  - `#[derive(RustioAdmin)]` — generates the list / create / edit / delete UI.
  - `impl Model` — the contract with Postgres (table name, columns, row
    mapping). The CLI writes this for you.
  - `impl ModelAdmin` — optional customisation (which columns to show, what's
    searchable, ordering). Empty `{}` accepts sensible defaults.
- You **register** each model by hand: `Admin::new().model::<Student>()`. The
  sidebar matches your source — nothing is auto-discovered behind your back.

You'll see all three when you add a model in step 6. The CLI generates them;
you stay the author of `main.rs`.

---

## 1. Install the CLI

```sh
cargo install rustio-admin-cli
```

This installs the **`rustio-admin`** binary into `~/.cargo/bin/`. It's your
scaffolder, migrations runner, and user / group / permission manager in one.

```sh
rustio-admin --help     # leads with "Start here"; lists every command below it
```

---

## 2. Create a project

```sh
rustio-admin new my-app
```

`new` runs a short, calm wizard: it confirms the project name, asks what
you're building, prints PostgreSQL guidance, and lets you name your dev
database — then writes a matching `.env` for you. (Scripting or in CI? Use
`rustio-admin startproject my-app`, which skips the wizard and writes
`.env.example` instead of `.env`.)

```sh
cd my-app
```

The scaffold is **intentionally neutral** — no demo models, nothing to delete
later:

```
my-app/
├── Cargo.toml
├── .env / .env.example      # .env if you used the wizard; .env.example otherwise
├── .gitignore
├── README.md
├── migrations/              # empty; your first model adds the first migration
├── src/
│   └── main.rs              # carries `// rustio:` markers for adding models
└── templates/
    └── home.html
```

Your `src/main.rs` already wires up the server and an empty `Admin::new()`.
You'll add models to it in step 6.

---

## 3. Point it at your database

The wizard's `.env` (or `.env.example`) defines `DATABASE_URL` from a few
components — edit them if your Postgres user, password, host, or port differ:

```sh
createdb my_app_dev          # the database name you chose in the wizard
cp .env.example .env         # only if you used `startproject` (the wizard already wrote .env)
```

> RustIO reads `DATABASE_URL` from `.env` in the project directory (and from
> the environment). Keep `.env` out of version control — the scaffold's
> `.gitignore` already excludes it.

---

## 4. Apply migrations and create your first admin

```sh
rustio-admin migrate apply
# A brand-new project has no model migrations yet — that's expected.
rustio-admin user create --email admin@my-app.local --role administrator
```

`user create` prompts for the password twice (input hidden). The plaintext
never appears in `argv`, `ps` output, or shell history — that's why there's
no `--password` flag in this guide. (Use `--password '…'` only in CI or a
scripted bootstrap where the secret is already managed.)

Sanity-check anytime:

```sh
rustio-admin doctor
# ✓ DATABASE_URL set
# ✓ Connected to Postgres
# ✓ Auth tables present
# ✓ 1 active administrator
```

If a check fails, `doctor` explains what's wrong and how to fix it.

---

## 5. Run it

```sh
cargo run
```

> The **first** `cargo run` compiles the framework and can take a few minutes
> — that's normal for a fresh Rust project. Later runs are fast.

Then open:

- **<http://127.0.0.1:8000/>** — your project's homepage; it's alive.
- **<http://127.0.0.1:8000/admin>** — the admin panel. Sign in with the
  account from step 4.

**Port already in use?** Edit the listen address near the bottom of
`src/main.rs` (e.g. `127.0.0.1:8001`) and re-run. Nothing else depends on the
exact port.

---

## 6. Add your first model

This is the loop you'll repeat for every kind of thing you manage. One
command scaffolds the model and its migration:

```sh
rustio-admin startapp student --field name:str --field email:email
```

It creates two files and tells you exactly what to add to `main.rs`:

```
MODEL CREATED  Student  (2 fields)
  ✓ src/student.rs
  ✓ migrations/0001_create_students.sql

3 edits in src/main.rs
  under  // rustio: modules  add  mod student;
  under  // rustio: imports   add  use student::Student;
  under  // rustio: models    add  .model::<Student>()
```

**`startapp` does not edit `main.rs` for you — you stay the author.** Open
`src/main.rs` and paste each line under its matching `// rustio:` marker. The
result looks like:

```rust
mod student;                         // under `// rustio: modules`
use student::Student;                // under `// rustio: imports`

let admin = Admin::new()
    .model::<Student>();             // under `// rustio: models`
```

Then apply the migration and re-run:

```sh
rustio-admin migrate apply           # creates the `students` table
cargo run
```

Your new pages are live at **<http://127.0.0.1:8000/admin/students>** —
list, create, edit, delete — and the model is automatically permission-seeded
(`students.view_student`, `add_`, `change_`, `delete_`) at boot.

### Field types

`--field <name>:<type>` accepts a closed vocabulary, so generated models and
migrations always line up:

```
str   text   int   bigint   bool   timestamp   json   fk:<Model>
float   date   time   decimal   uuid   email   phone   choice:a,b,c
```

`email` / `phone` are validated strings; `choice:draft,published` becomes a
`CHECK` constraint and a dropdown; `fk:Author` is a foreign key that renders
as a link to the related row. Omit `--field` to get a one-field placeholder
you can edit by hand.

### Customising a model

Generated `impl ModelAdmin for Student {}` accepts every default. Fill it in
to shape the pages — the Django-style hooks:

```rust
impl ModelAdmin for Student {
    fn list_display()  -> &'static [&'static str] { &["name", "email", "created_at"] }
    fn search_fields() -> &'static [&'static str] { &["name", "email"] }
    fn ordering()      -> &'static [&'static str] { &["-created_at"] }
}
```

See the [`ModelAdmin` reference](./modeladmin.md) for every available method.

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

You didn't write any HTML, CSS, JS, router boilerplate, an ORM, a search
engine, or a migration runner — the framework ships all of it.

---

## Where to go next

- **Customise the look** — `Admin::accent_color("#…")`, `Admin::site_branding(…)`,
  or generate a full palette with `rustio-admin theme generate --brand "#…"`.
- **Record the *why* behind your project** — `rustio-admin memory` keeps a
  governed project memory (decisions, rejected ideas, intent) that a teammate
  or AI assistant reads for context. Guide: [`memory.md`](./memory.md).
- **Working with an AI coding assistant?** `rustio-admin ai init` then
  `rustio-admin ai status` set what it may do here (Allowed / Needs approval /
  Blocked), with changes proposed, approved, and applied as explicit steps.
- **Understand the internals** — [`architecture.md`](./architecture.md) maps
  which module owns what; the [`design/`](./design/) contracts govern the
  security-sensitive surfaces.

---

## If you remember nothing else

1. `rustio-admin new <name>` → a neutral, ready-to-run project (with `.env`).
2. `createdb`, then `rustio-admin migrate apply` and `rustio-admin user create`.
3. `cargo run` → homepage at `/`, admin at `/admin`.
4. `rustio-admin startapp <name> --field …` → paste its three `// rustio:`
   lines into `main.rs`, `migrate apply`, re-run. Repeat per model.

Everything else — auth, roles, recovery, audit — is already there.
