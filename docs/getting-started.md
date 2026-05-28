# Getting started

From an empty directory to a logged-in admin in under five minutes.

## Prerequisites

- Rust 1.88+ (`rustup show` to check).
- A reachable PostgreSQL. A native install is fine; the
  `docker run --name rio-pg -e POSTGRES_PASSWORD=dev -p 5432:5432 -d postgres:16`
  one-liner also works.

## 1. Install the CLI

```sh
cargo install rustio-admin-cli
```

This installs the `rustio` binary into `~/.cargo/bin/`. It's the
project scaffolder, the migrations runner, and the user / group /
permission manager all in one.

## 2. Scaffold

```sh
rustio-admin startproject my-app
cd my-app
```

You get a working project with a demo `Post` model, a starter
migration, an `.env.example`, and a `README.md`:

```
my-app/
├── Cargo.toml
├── .env.example
├── .gitignore
├── README.md
├── migrations/
│   └── 0001_create_posts.sql
└── src/
    ├── main.rs
    └── post.rs
```

## 3. Configure the database

```sh
createdb my_app_dev
cp .env.example .env             # and edit if your Postgres user/password differ
```

The default `DATABASE_URL` in `.env.example` is
`postgres://postgres:dev@localhost/my_app_dev`.

## 4. Apply migrations + seed the first admin

```sh
rustio-admin migrate apply
rustio-admin user create --email admin@my-app.local --role administrator
```

The CLI prompts twice for the password (echo-suppressed). The
plaintext value never appears on `argv`, in `ps` output, or in shell
history — that is the entire reason `--password` is omitted from the
quickstart. Use `--password '…'` only inside CI or scripted bootstrap
flows where the secret is already managed.

Sanity-check the setup at any time:

```sh
rustio-admin doctor
# ✓ DATABASE_URL = postgres://postgres:***@localhost/my_app_dev
# ✓ Connected to Postgres
# ✓ Auth tables present
# ✓ 1 active administrator(s)
```

## 5. Boot

```sh
cargo run
```

The admin lands at <http://127.0.0.1:8000/admin>. Sign in with the
account from step 4.

### Port already in use?

The scaffolded `src/main.rs` binds to `127.0.0.1:8000`. If that port
is taken on your machine, edit the listen address near the bottom of
the file:

```rust
let addr = "127.0.0.1:8001".parse().expect("valid listen address");
```

Pick any free port; nothing else in the framework cares about the
exact value. Re-run `cargo run`.

## What you get after first login

The first sign-in lands on a working admin with everything below
already wired up — no extra configuration, no separate services:

- **Session-backed admin authentication.** Argon2id-hashed passwords;
  cookies carry a 256-bit random token; the database stores only the
  SHA-256 hash (since 0.4.0).
- **Postgres-backed users, groups, and permissions.** A 5-tier role
  ladder (User · Staff · Supervisor · Administrator · Developer) plus
  per-model `<table>.<action>_<singular>` permissions.
- **Permission matrix UI.** Group editing renders permissions as a
  model × action grid (View / Add / Change / Delete) with a per-row
  "All" toggle; codenames that don't match the canonical pattern fall
  through into a collapsible "Other" group.
- **Audit history.** Every authority mutation (user / group create /
  update / delete, future password reset, future MFA toggle) writes a
  row to `rustio_admin_actions`. Browse at `/admin/history`.
- **Active sessions page** at `/admin/account/sessions` — every signed-
  in device with IP, OS · browser summary, created-at, last-seen.
  Revoke buttons land in 0.5.0.
- **Correlation IDs on every request.** `middleware::correlation_id`
  stamps a UUID v7, surfaces it in the `x-correlation-id` response
  header, and threads it into every audit row written under that
  request.
- **FK-hydrated list cells.** `belongs_to` foreign-key columns on list
  pages render as the target row's display field (e.g. `Anna
  Lindqvist`, not `5`) and link through to the related row.
- **Server-rendered templates with override paths.** Every page is
  embedded into the binary via `include_str!`. Set
  `RUSTIO_TEMPLATE_DIR=templates` and any disk-side
  `templates/admin/<page>.html` wins over the embedded copy without
  recompilation.

## Project philosophy

The framework's design choices, in one place:

- **Postgres-first.** No SQLite, no MySQL backend; every query and
  every migration assumes Postgres semantics so the admin can rely on
  them.
- **Operational clarity over magic.** No risk scoring, no AI
  heuristics, no automatic decisions the operator can't read in three
  lines and reproduce by hand.
- **Explicit model registration.** `Admin::new().model::<X>()` —
  models are listed by hand, not auto-discovered. The admin sidebar
  matches the source code.
- **Server-rendered admin UI.** No SPA, no JSON API at the admin
  layer, no separate frontend build step. One Rust binary serves
  everything.
- **Security and auditability built into the lifecycle.** Authority
  guards, hashed-at-rest sessions, centralised invalidation, audit
  forensic chain — all baked in before the first feature flag.
- **No AI, no cloud lock-in, no frontend build step.** The framework
  ships the same way to a $5 VPS, a kubernetes cluster, and an
  air-gapped factory floor.

## 6. Add a model

`rustio-admin startproject` already creates `migrations/` containing
`0001_create_posts.sql` and the matching `src/post.rs`. **Don't run
`mkdir -p migrations`** — the directory exists. From here you have
two clean paths:

### Option A — keep `Post` as a smoke test

Useful while you learn the framework. Leave `0001_create_posts.sql`
applied; add new tables as `0002_*.sql`, `0003_*.sql`, etc. via
`rustio-admin startapp`:

```sh
rustio-admin startapp patient        # writes src/patient.rs + migrations/0002_create_patients.sql
rustio-admin startapp appointment    # 0003_*
rustio-admin startapp treatment      # 0004_*
rustio-admin startapp invoice        # 0005_*
```

Each `startapp` prints the `mod` / `use` / `.model::<>()` lines to
paste into `src/main.rs`. Run `rustio-admin migrate apply` and re-boot;
the new pages light up alongside `/admin/posts`.

### Option B — replace `Post` with the real domain

Useful when you're confident in the layout and want a clean
namespace. **Do this BEFORE the migration has been applied to a real
database** — once `0001_create_posts.sql` has run on production, you
own the table.

```sh
rm migrations/0001_create_posts.sql
rm src/post.rs
# Edit src/main.rs: remove `mod post;` and the `.model::<Post>()` line.
$EDITOR migrations/0001_create_patients.sql       # write your real first table
$EDITOR src/patient.rs                            # write the matching Rust model
# Add `mod patient; use patient::Patient;` to src/main.rs and chain
# `.model::<Patient>()` onto Admin::new().
rustio-admin migrate apply
```

If the demo migration has already touched a real database, do **not**
delete the file — write `0002_drop_posts.sql` with `DROP TABLE
posts;` and let it run forward. Migrations are append-only by
contract.

### The four moving parts of any model

The scaffolded `src/post.rs` is the canonical pattern — copy it,
rename, and add a new entry to `Admin::new()…model::<…>()` in
`src/main.rs`:

```rust
#[derive(RustioAdmin)]
pub struct Course { /* … */ }

impl Model      for Course { /* TABLE, COLUMNS, from_row, insert_values */ }
impl ModelAdmin for Course {
    fn list_display()  -> &'static [&'static str] { &["code", "title", "is_published"] }
    fn ordering()      -> &'static [&'static str] { &["code"] }
}
```

`#[derive(RustioAdmin)]` produces the `AdminModel` impl from the
struct; the hand-written `impl Model` is the contract with Postgres;
`impl ModelAdmin` is the Django-style customisation hook. See the
[`ModelAdmin` reference](./modeladmin.md) for every available method.

Don't forget the migration:

```sql
-- migrations/0002_create_courses.sql
CREATE TABLE courses (
    id          BIGSERIAL    PRIMARY KEY,
    code        TEXT         NOT NULL UNIQUE,
    title       TEXT         NOT NULL,
    is_published BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);
```

```sh
rustio-admin migrate apply
```

The new model is automatically permission-seeded (`courses.add_course`,
`change`, `delete`, `view`) on the next `Admin::seed_permissions(&db)`
call at app boot.

## What you didn't have to do

- Write any HTML, CSS, or JS — the framework ships every page and a
  hand-written stylesheet baked in.
- Wire any router boilerplate — `register_admin_routes` mounts every
  URL the admin needs.
- Reach for a separate ORM, search engine, or templating crate —
  sqlx + minijinja are wrapped behind the `Model` and `Templates`
  types.
- Write a migration runner — `rustio-admin migrate apply` walks the
  numerically prefixed `*.sql` files in `migrations/` transactionally
  with a tracking table.
- Hand-craft your first admin user — `rustio-admin user create` does the
  Argon2 hashing and seeds the auth tables on a fresh DB.

## Next

- Customise the chrome with `Admin::theme(...)` and
  `Admin::site_branding(...)`.
- Override `templates/admin/_base.html` etc. to ship your own layout
  — set `RUSTIO_TEMPLATE_DIR=templates` and disk overrides win
  against the embedded copies.
- Read [`docs/architecture.md`](./architecture.md) to understand
  which module owns what.
