# Tutorial: from a sentence to a running admin (with rustio-draft)

> **Note:** `rustio-draft` now lives in its own repository
> ([abdulwahed-sweden/rustio-draft](https://github.com/abdulwahed-sweden/rustio-draft)).
> It is a separate, setup-time companion tool — not part of the `rustio-admin`
> workspace and not a runtime dependency. This document describes how it
> integrates with `rustio-admin`. See [`project-status.md`](./project-status.md).

A beginner-friendly, start-to-finish walkthrough. You will describe an app in
one plain-English sentence, turn that into real Rust models, and open a working
admin panel in your browser — step by step, with the exact commands and what you
should see after each one.

---

## What is rustio-draft, and why use it?

**rustio-admin** turns your data models (Rust structs) into a full admin panel —
list, create, edit, delete, search, login, roles, and an audit trail — with no
HTML or frontend to write.

Normally *you* write those models by hand. **rustio-draft** is an optional
helper: you give it a sentence, and it writes a `schema.json` for you (using
Claude). Then rustio-admin turns that JSON into real code — **deterministically**.

```text
   "a blog with posts and comments"        (your sentence)
                │
                ▼
        ┌───────────────┐   uses AI (Claude)
        │  rustio-draft  │   → writes schema.json
        └───────────────┘
                │
                ▼
           schema.json        (you review / edit it — plain JSON)
                │
                ▼
        ┌───────────────┐   no AI, no network, exact + repeatable
        │  rustio-admin  │   import → plan → commit → generated code
        └───────────────┘
                │
                ▼
        a running admin panel
```

Two clean halves:

- **rustio-draft *authors*** the schema (this is the only part that uses AI).
- **rustio-admin *applies*** it (this part is deterministic and runs no AI).

rustio-draft lives in its **own separate repository**
([abdulwahed-sweden/rustio-draft](https://github.com/abdulwahed-sweden/rustio-draft)),
so the framework itself never depends on AI or the network. You are always free
to skip it and write the schema by hand (see [Prefer no AI?](#prefer-no-ai) at
the end).

> **Field types (important):** this flow supports four field types —
> `text`, `integer`, `boolean`, `timestamp`. Money/prices → `integer` (store
> cents). Dates/times → `timestamp`. A link to another model → a plain `integer`
> field named `<thing>_id` (e.g. `post_id`).

---

## Before you start

You need three things:

1. **Rust 1.94+** — check with `rustup show`.
2. **A running PostgreSQL** (rustio-admin is Postgres-only). Quick one via Docker:
   ```sh
   docker run --name rio-pg -e POSTGRES_PASSWORD=dev -p 5432:5432 -d postgres:16
   ```
3. **An Anthropic API key** — only for rustio-draft (the AI step). Get one from
   the Anthropic Console, then:
   ```sh
   export ANTHROPIC_API_KEY=sk-ant-...
   ```

You do **not** need Node, a bundler, or any frontend tools.

---

## Install the two tools

```sh
# 1) the framework CLI — provides the `rustio-admin` binary
cargo install rustio-admin-cli

# 2) rustio-draft — the AI drafting tool (from its repo)
cargo install --git https://github.com/abdulwahed-sweden/rustio-draft
```

Check they are on your PATH:

```sh
rustio-admin --version
rustio-draft --version
```

> If `rustio-admin import` (used below) is missing, your installed CLI is older
> than that feature — install the newest from source:
> `cargo install --git https://github.com/abdulwahed-sweden/rustio-admin rustio-admin-cli`

---

## Step 1 — Create an empty project

`builder new` makes a project that the `import → plan → commit` flow can drive.

```sh
rustio-admin builder new blog
cd blog
```

**You get:**

```text
blog/
├── Cargo.toml            # pins rustio-admin + tokio + chrono
├── src/main.rs           # a placeholder (we replace it in Step 4)
├── migrations/           # empty for now
└── .rustio/              # the builder's bookkeeping (draft, history, lock)
```

---

## Step 2 — Draft the schema from one sentence

```sh
rustio-draft new "a blog: posts have a title, a body, a published flag, and a published-at time; comments have an author name and a body, and belong to a post"
```

**You should see** it print the schema and write `schema.json`:

```text
Designing a schema with claude-opus-4-8…
Wrote schema.json — 2 model(s), 7 field(s).

Review it, then apply deterministically:
    rustio-admin import schema.json
    rustio-admin plan      # preview (read-only)
    rustio-admin commit    # apply atomically
```

`schema.json` will look roughly like this (open it and read it — it is just JSON):

```json
{
  "project": "blog",
  "models": [
    {
      "name": "Post",
      "fields": [
        { "name": "title", "type": "text" },
        { "name": "body", "type": "text" },
        { "name": "published", "type": "boolean" },
        { "name": "published_at", "type": "timestamp" }
      ]
    },
    {
      "name": "Comment",
      "fields": [
        { "name": "post_id", "type": "integer" },
        { "name": "author_name", "type": "text" },
        { "name": "body", "type": "text" }
      ]
    }
  ]
}
```

> Note there is **no `id` and no `created_at`** — rustio-admin adds those to
> every table automatically, so you never write them.

---

## Step 3 — (Optional) Refine the schema

Not quite right? Change it in plain English — rustio-draft rewrites the file:

```sh
rustio-draft refine schema.json "add an integer 'likes' field to Post, default meaning zero"
```

Or just open `schema.json` in your editor and change it by hand. It is only JSON.

---

## Step 4 — Apply it (generate the real code)

Three commands. Each is safe to read before the next.

```sh
rustio-admin import schema.json   # record the models
rustio-admin plan                 # preview what will be written (read-only)
rustio-admin commit               # actually generate the code + migration
```

**`plan`** shows you exactly what `commit` will create:

```text
Plan:
  + create    src/_generated/mod.rs
  + create    src/_generated/admin.rs
  + create    src/_generated/models/mod.rs
  + create    src/_generated/models/post.rs
  + create    src/_generated/models/comment.rs
  + create    migrations/0001_initial.sql
```

**`commit`** writes them:

```text
Committed 6 file(s) …
```

You now have real Rust models in `src/_generated/models/` and a SQL migration in
`migrations/0001_initial.sql`.

> **Shortcut:** `rustio-draft new "…" --apply` runs *draft → import → plan* in
> one go and stops before `commit`, so you can review the plan and then commit
> yourself. (It never commits for you.)

---

## Step 5 — Wire the server (the one bit of code)

The generated code gives you `build_admin()` — a ready-made admin with your
models. You just start a web server around it. Replace `src/main.rs` with this:

```rust
use std::sync::Arc;

use rustio_admin::{
    auth, background, middleware, migrations, register_admin_routes,
    templates::Templates, Db, Result, Router, Server,
};

mod _generated;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Connect to Postgres (reads DATABASE_URL from the environment).
    let db = Db::connect(
        &std::env::var("DATABASE_URL").expect("set DATABASE_URL"),
    )
    .await?;

    // 2. Create the framework's own tables, run your migrations, start
    //    the background housekeeping (session cleanup, etc.).
    auth::init_tables(&db).await?;
    migrations::apply(&db, "migrations").await?;
    background::spawn_housekeeping(db.clone());

    // 3. Build the admin from your generated models, and seed permissions.
    let admin = _generated::admin::build_admin().app_name("Blog");
    admin.seed_permissions(&db).await?;

    // 4. Assemble the router (middleware order matters — keep this order).
    let templates = Templates::new(None)?;
    let router = Router::new()
        .middleware(middleware::logger)
        .middleware(middleware::correlation_id)
        .middleware(middleware::security_headers)
        .middleware(middleware::csrf_protect);
    let router = register_admin_routes(router, admin, db, Arc::clone(&templates));

    // 5. Serve.
    let addr = "127.0.0.1:8000".parse().expect("valid address");
    println!("admin at http://{addr}/admin");
    Server::new(router, addr).run().await
}
```

That is the whole server. (For a fuller example — a homepage, branding, logging —
see [`examples/shop/src/main.rs`](../examples/shop/src/main.rs).)

---

## Step 6 — Point at the database, create a login, run

```sh
createdb blog_dev
export DATABASE_URL=postgres://postgres:dev@localhost:5432/blog_dev

rustio-admin migrate apply                                   # create the tables
rustio-admin user create --email admin@blog.local --role administrator
cargo run                                                    # first build takes a few minutes
```

`user create` asks for a password twice (hidden). When the server prints
`admin at http://127.0.0.1:8000/admin`, open that URL and sign in.

**You should see** your admin, with **Posts** and **Comments** in the sidebar —
list, create, edit, search, delete — plus the login you just used and an audit
trail of every change. You wrote one small `main.rs` and one sentence.

---

## The whole thing, start to finish

```sh
# one-time: install
cargo install rustio-admin-cli
cargo install --git https://github.com/abdulwahed-sweden/rustio-draft
export ANTHROPIC_API_KEY=sk-ant-...

# build the app
rustio-admin builder new blog && cd blog
rustio-draft new "a blog: posts (title, body, published flag) and comments (author name, body)"
rustio-admin import schema.json && rustio-admin plan && rustio-admin commit
#   → then paste the src/main.rs from Step 5

createdb blog_dev
export DATABASE_URL=postgres://postgres:dev@localhost:5432/blog_dev
rustio-admin migrate apply
rustio-admin user create --email admin@blog.local --role administrator
cargo run     # → http://127.0.0.1:8000/admin
```

---

## Prefer no AI?

rustio-draft is optional. The exact same `import → plan → commit` flow works
without it — either hand-write `schema.json`, or build it up command by command:

```sh
rustio-admin add model Post
rustio-admin add field Post title text
rustio-admin add field Post published boolean
rustio-admin plan
rustio-admin commit
```

rustio-draft just writes that `schema.json` for you from a sentence. The
framework never runs AI on its own.

---

## Troubleshooting

| Symptom | Fix |
|---|---|
| Not sure your key works? | Run `rustio-draft doctor` — it checks the key (and lists your models) without spending any tokens. |
| `ANTHROPIC_API_KEY is not set` | `export ANTHROPIC_API_KEY=sk-ant-...` before `rustio-draft`. |
| `API key is invalid or revoked (401)` | Wrong/expired key — get a fresh one and re-`export` it; confirm with `rustio-draft doctor`. |
| `rustio-admin import` not found | Install the newest CLI from source (see [Install](#install-the-two-tools)). |
| `type '…' is not in the closed list` | Use only `text`, `integer`, `boolean`, `timestamp`. |
| Can't connect to Postgres | Check `DATABASE_URL` and that Postgres is running (`docker ps`). |
| Port 8000 in use | Change `127.0.0.1:8000` in `src/main.rs`. |

---

## Where to go next

- [`getting-started.md`](getting-started.md) — the framework's own quickstart
  (scaffold a project, add models with `startapp`, no AI).
- [`modeladmin.md`](modeladmin.md) — customise list columns, search, filters,
  ordering.
- [`cli.md`](cli.md) — the full `rustio-admin` command surface.
