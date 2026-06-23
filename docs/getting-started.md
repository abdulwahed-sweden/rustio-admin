# Getting started

From nothing to a **running admin panel** — login, roles, and an audit trail
— in about ten minutes. Two ways in:

- **See it work now** — scaffold a ready-made clinic and sign in to something
  real, having written no code.
- **Build your own** — describe your data as a Rust struct and watch RustIO
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

## See it work now — the clinic example

The fastest way to understand RustIO is to run a real admin. The `new`
wizard's **Clinic** type scaffolds a working clinic — `Patient` and
`Appointment` models, already registered, with example rows — so the first
thing you do is sign in to something real.

```sh
cargo install rustio-admin-cli      # installs the `rustio-admin` binary
rustio-admin new clinic             # at "Project type", choose 2) clinic
```

The wizard asks three short questions and writes a matching `.env`:

```
Step 2 of 3 · Project type
  clinic and blog come with example models you can run right away;
  the rest start clean.

    1  custom      clean slate (no models yet)
    2  clinic      example models — patients, appointments
    3  school      clean slate (no models yet)
    4  inventory   clean slate (no models yet)
    5  blog        example models — posts, comments

  › Type [1]: 2
```

Then follow the steps it prints (the database name is the one it shows you):

```sh
cd clinic
createdb clinic_dev
rustio-admin migrate apply          # creates patients + appointments (with example rows)
rustio-admin user create --email admin@clinic.local --role administrator
cargo run                           # first build takes a few minutes; later runs are fast
```

Open **<http://127.0.0.1:8000/admin>**, sign in, and you have a working
clinic:

```
Patients (3)              Appointments (3)
  Sarah Ahmed               Sarah Ahmed · Annual checkup · scheduled
  John Okoro                John Okoro  · Follow-up      · scheduled
  Maria Lopez               Maria Lopez · Lab results    · completed
```

List, create, edit, search, delete — plus the login you just used and an
audit trail of every change — and you wrote **zero lines of code**. That's
the whole idea. The rest of this guide shows how to build your own.

---

## How a RustIO project fits together

Before you build one yourself, the shape — so each step makes sense:

- You write a **Rust struct** for each thing you manage (a `Product`, an
  `Invoice`), plus a **SQL migration** that creates its table.
- Three small pieces turn that struct into admin pages:
  - `#[derive(RustioAdmin)]` — generates the list / create / edit / delete UI.
  - `impl Model` — the contract with Postgres (table, columns, row mapping).
    The CLI writes this for you.
  - `impl ModelAdmin` — optional customisation (columns shown, what's
    searchable, ordering). Empty `{}` accepts sensible defaults.
- You **register** each model by hand — `Admin::new().model::<Product>()`. The
  sidebar matches your source; nothing is auto-discovered behind your back.

The `clinic` preset above did all of this for you. Below you do it yourself,
one command at a time.

---

## Build your own — from struct to admin

### 1. Create a clean-slate project

```sh
rustio-admin new shop               # at "Project type", choose 1) custom
cd shop
```

`new` runs a short, calm wizard (project name → type → database), then writes
a matching `.env`. Scripting or in CI? `rustio-admin startproject shop` skips
the wizard and writes `.env.example` instead.

The `custom` scaffold is neutral — no demo models, nothing to delete later:

```
shop/
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
createdb shop_dev           # the database name the wizard showed you
```

> RustIO reads `DATABASE_URL` from `.env` in the project directory. The
> scaffold's `.gitignore` already keeps `.env` out of version control.

### 3. Create your admin login and run

```sh
rustio-admin migrate apply  # a brand-new custom project has no model migrations yet — expected
rustio-admin user create --email admin@shop.local --role administrator
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
rustio-admin startapp product \
  --field name:str \
  --field price:decimal \
  --field in_stock:bool
```

**Creates** a `Product` with three columns:

```
Product
├── name       str        (TEXT)
├── price      decimal    (NUMERIC)
└── in_stock   bool       (BOOLEAN)
```

**Generates**, and tells you exactly what to wire:

```
✓ src/product.rs                       the Rust model (+ admin defaults)
✓ migrations/0001_create_products.sql  the table

  …plus an admin page, search, and per-model permissions at boot.

3 edits in src/main.rs
  under  // rustio: modules  add  mod product;
  under  // rustio: imports   add  use product::Product;
  under  // rustio: models    add  .model::<Product>()
```

**`startapp` never edits `main.rs` — you stay the author.** Paste each line
under its matching `// rustio:` marker:

```rust
mod product;                         // under `// rustio: modules`
use product::Product;                // under `// rustio: imports`

let admin = Admin::new()
    .model::<Product>();             // under `// rustio: models`
```

Then apply the migration and re-run:

```sh
rustio-admin migrate apply           # creates the `products` table
cargo run
```

Your pages are live at **<http://127.0.0.1:8000/admin/products>** — list,
create, edit, search, delete — permission-seeded (`view_product`,
`add_product`, `change_product`, `delete_product`) at boot.

### Field types — and the best reference

`--field <name>:<type>` accepts a closed vocabulary, so model and migration
always line up:

```
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

The generated `impl ModelAdmin for Product {}` accepts every default. Fill it
in to shape the pages — the Django-style hooks:

```rust
impl ModelAdmin for Product {
    fn list_display()  -> &'static [&'static str] { &["name", "price", "in_stock"] }
    fn search_fields() -> &'static [&'static str] { &["name"] }
    fn list_filter()   -> &'static [&'static str] { &["in_stock"] }
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
- **Understand the internals** — [`architecture.md`](./architecture.md) maps
  which module owns what; the [`design/`](./design/) contracts govern the
  security-sensitive surfaces.

---

## If you remember nothing else

```sh
rustio-admin new <name>          # pick "clinic" to see real models now, or "custom" for a clean slate
cd <name> && createdb <name>_dev
rustio-admin migrate apply
rustio-admin user create --email admin@<name>.local --role administrator
cargo run                        # homepage at /, admin at /admin

# add a model, any time:
rustio-admin startapp <model> --field name:str --field …
#   → paste its 3 `// rustio:` lines into src/main.rs, then migrate apply + cargo run
```

Everything else — auth, roles, recovery, audit — is already there.
