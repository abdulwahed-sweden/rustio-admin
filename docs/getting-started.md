# Getting started

From `cargo new` to a logged-in admin in under 10 minutes.

## Prerequisites

- Rust 1.80+ (`rustup show` to check).
- A reachable PostgreSQL (`docker run --name rio-pg -e POSTGRES_PASSWORD=postgres -p 5432:5432 -d postgres:16` works).
- The `rustio-admin` repo checked out somewhere — `rustio-admin` is path-only until the first crates.io publish.

## 1. Scaffold

```sh
cargo new my-app --bin
cd my-app
```

Edit `Cargo.toml`:

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"

[dependencies]
rustio-admin = { path = "../rustio-admin/crates/rustio-admin" }
tokio        = { version = "1", features = ["macros", "rt-multi-thread"] }
chrono       = { version = "0.4", features = ["serde"] }
sqlx         = { version = "0.8", default-features = false, features = ["runtime-tokio", "postgres"] }
dotenvy      = "0.15"
env_logger   = "0.11"
log          = "0.4"
```

## 2. Define a model

`src/post.rs`:

```rust
use chrono::{DateTime, Utc};
use rustio_admin::{Error, Model, ModelAdmin, Row, RustioAdmin, Value};

#[derive(RustioAdmin)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub published: bool,
    pub created_at: DateTime<Utc>,
}

impl Model for Post {
    const TABLE: &'static str   = "posts";
    const COLUMNS: &'static [&'static str] =
        &["id", "title", "body", "published", "created_at"];
    const INSERT_COLUMNS: &'static [&'static str] =
        &["title", "body", "published", "created_at"];

    fn id(&self) -> i64 { self.id }

    fn from_row(row: Row<'_>) -> Result<Self, Error> {
        Ok(Self {
            id:           row.get_i64("id")?,
            title:        row.get_string("title")?,
            body:         row.get_string("body")?,
            published:    row.get_bool("published")?,
            created_at:   row.get_datetime("created_at")?,
        })
    }

    fn insert_values(&self) -> Vec<Value> {
        vec![
            self.title.clone().into(),
            self.body.clone().into(),
            self.published.into(),
            self.created_at.into(),
        ]
    }
}

// Empty impl picks up every framework default. Override individual
// methods to customise — see docs/modeladmin.md.
impl ModelAdmin for Post {
    fn list_display()  -> &'static [&'static str] { &["title", "published", "created_at"] }
    fn list_filter()   -> &'static [&'static str] { &["published"] }
    fn search_fields() -> &'static [&'static str] { &["title", "body"] }
    fn ordering()      -> &'static [&'static str] { &["-created_at"] }
}
```

## 3. Add a migration

`migrations/0001_create_posts.sql`:

```sql
CREATE TABLE posts (
    id          BIGSERIAL    PRIMARY KEY,
    title       TEXT         NOT NULL,
    body        TEXT         NOT NULL DEFAULT '',
    published   BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX posts_created_at_idx ON posts (created_at DESC);
```

## 4. Wire main.rs

`src/main.rs`:

```rust
use std::sync::Arc;

use rustio_admin::admin::Admin;
use rustio_admin::middleware;
use rustio_admin::templates::Templates;
use rustio_admin::{auth, background, migrations, register_admin_routes, Db, Router, Server};

mod post;
use post::Post;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let db_url = std::env::var("DATABASE_URL")?;
    let db = Db::connect(&db_url).await?;

    auth::init_tables(&db).await?;
    migrations::apply(&db, "migrations").await?;
    background::spawn_housekeeping(db.clone());

    let admin = Admin::new().model::<Post>();
    admin.seed_permissions(&db).await?;

    let templates = Templates::new(None)?;

    let router = Router::new()
        .middleware(middleware::logger)
        .middleware(middleware::security_headers)
        .middleware(middleware::csrf_protect);

    let router = register_admin_routes(router, admin, db, Arc::clone(&templates));

    let addr = "127.0.0.1:8000".parse()?;
    println!("listening on http://{addr}/admin");
    Server::new(router, addr).run().await?;
    Ok(())
}
```

## 5. Boot

```sh
echo 'DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/postgres' > .env
cargo run
```

The migration creates `posts`; auth tables come up automatically. Hit `http://127.0.0.1:8000/admin` — you'll be redirected to the login page.

## 6. Create the first admin user

A CLI is on the way; today the path is direct SQL via psql or any sqlx caller. The framework's `auth::create_user` does Argon2 hashing for you:

```rust
// In a one-off binary (or a `#[tokio::test]` you discard after running):
let id = rustio_admin::auth::create_user(
    &db, "admin@example.local", "supersecret", rustio_admin::Role::Administrator,
).await?;
```

Sign in at `/admin/login`; the dashboard shows the registered models, and `/admin/posts` is the live changelist.

## What you didn't have to do

- Write any HTML, CSS, or JS — the framework ships every page baked in.
- Wire any router boilerplate — `register_admin_routes` mounts every URL the admin needs.
- Reach for a separate ORM, search engine, or templating crate — sqlx + minijinja are wrapped behind the `Model` and `Templates` types.
- Write a single migration runner — `migrations::apply` walks the numerically prefixed `*.sql` files in your directory, transactionally, with a tracking table.

## Next

- Customise the chrome with [`Admin::theme(...)`](./modeladmin.md#theming) and `Admin::site_branding(...)`.
- Override `templates/admin/_base.html` etc. to ship your own layout — disk overrides win against the embedded copies.
- Read [`docs/architecture.md`](./architecture.md) to understand which module owns what.
