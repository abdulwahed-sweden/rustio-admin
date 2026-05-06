# Getting started

From an empty directory to a logged-in admin in under five minutes.

## Prerequisites

- Rust 1.80+ (`rustup show` to check).
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
rustio startproject my-app
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
rustio migrate apply
rustio user create --email admin@my-app.local --role administrator
```

The CLI prompts twice for the password (echo-suppressed). Use
`--password '…'` to skip the prompt for CI / scripting.

Sanity-check the setup at any time:

```sh
rustio doctor
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

## 6. Add a model

The scaffolded `src/post.rs` is the canonical pattern — copy it,
rename, and add a new entry to `Admin::new()…model::<…>()` in
`src/main.rs`. The four moving parts:

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
rustio migrate apply
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
- Write a migration runner — `rustio migrate apply` walks the
  numerically prefixed `*.sql` files in `migrations/` transactionally
  with a tracking table.
- Hand-craft your first admin user — `rustio user create` does the
  Argon2 hashing and seeds the auth tables on a fresh DB.

## Next

- Customise the chrome with `Admin::theme(...)` and
  `Admin::site_branding(...)`.
- Override `templates/admin/_base.html` etc. to ship your own layout
  — set `RUSTIO_TEMPLATE_DIR=templates` and disk overrides win
  against the embedded copies.
- Read [`docs/architecture.md`](./architecture.md) to understand
  which module owns what.
