use std::sync::Arc;

use rustio_admin::admin::Admin;
use rustio_admin::middleware;
use rustio_admin::templates::Templates;
use rustio_admin::{
    auth, background, migrations, register_admin_routes, Db, Error, Response, Result, Router,
    Server,
};

// rustio: modules — `mod <name>;` declarations from `rustio-admin startapp` go directly under this marker.
mod address;
mod cart_item;
mod category;
mod customer;
mod order;
mod order_item;
mod payment;
mod payment_method;
mod product;
mod product_image;

// rustio: imports — `use <name>::<Name>;` lines from `rustio-admin startapp` go directly under this marker.
use address::Address;
use cart_item::CartItem;
use category::Category;
use customer::Customer;
use order::Order;
use order_item::OrderItem;
use payment::Payment;
use payment_method::PaymentMethod;
use product::Product;
use product_image::ProductImage;

// The first-boot homepage at `/`. Baked at compile time so the
// binary stays single-file. Replace by editing this file and
// re-running `cargo run`, or remove the `.get("/", …)` route below
// to free the path for your own handler.
const HOMEPAGE_HTML: &str = include_str!("../templates/home.html");

#[tokio::main]
async fn main() -> Result<()> {
    // .env is optional; production deploys typically use real env vars.
    let _ = dotenvy::dotenv();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let db_url = std::env::var("DATABASE_URL").map_err(|_| {
        Error::Internal(
            "DATABASE_URL is not set. Copy .env.example to .env and edit it before running.".into(),
        )
    })?;

    let db = Db::connect(&db_url).await?;
    auth::init_tables(&db).await?;
    migrations::apply(&db, "migrations").await?;
    background::spawn_housekeeping(db.clone());

    // `.app_name(...)` flows your project name into the admin tab
    // title, login header, and dashboard h1 so the framework reads as
    // "powered by RustIO", not "inside RustIO". The `// rustio: models`
    // marker below is the insertion point for `.model::<YourModel>()`
    // calls printed by `rustio-admin startapp` — chain them between the
    // marker and the closing `;`.
    let admin = Admin::new()
        .app_name("Shop")
        // rustio: models
        .model::<Product>()
        .model::<Category>()
        .model::<Customer>()
        .model::<Order>()
        .model::<OrderItem>()
        .model::<Address>()
        .model::<ProductImage>()
        .model::<CartItem>()
        .model::<Payment>()
        .model::<PaymentMethod>();

    admin.seed_permissions(&db).await?;

    // Project template overrides in ./templates win over the framework's
    // embedded defaults (e.g. templates/admin/index.html restyles the
    // dashboard). Default to "templates" so any `cargo run` picks them up;
    // RUSTIO_TEMPLATE_DIR can still point somewhere else if set.
    let template_dir = std::env::var("RUSTIO_TEMPLATE_DIR").unwrap_or_else(|_| "templates".into());
    let templates = Templates::new(Some(template_dir.into()))?;

    // Middleware order is locked by DESIGN_AUDIT.md §11:
    //
    //   logger → correlation_id → security_headers → csrf_protect
    //
    // `correlation_id` MUST sit between logger and csrf_protect so
    // every audit row written under one HTTP request shares one
    // UUID v7 in `rustio_admin_actions.correlation_id`. The
    // built-in `/admin/history` page surfaces this column as a
    // pivot; downstream audit-log views (your project's, if you
    // add one) can join framework + project rows on the same
    // column without per-source post-processing.
    // Clone the DB handle for the homepage route before `db` is moved
    // into `register_admin_routes` below. The handler re-clones per
    // request so its future stays `'static`.
    let home_db = db.clone();
    let router = Router::new()
        .middleware(middleware::logger)
        .middleware(middleware::correlation_id)
        .middleware(middleware::security_headers)
        .middleware(middleware::csrf_protect)
        .get("/", move |_req| {
            let db = home_db.clone();
            async move { Ok(Response::html(render_home(&db).await)) }
        });

    let router = register_admin_routes(router, admin, db, Arc::clone(&templates));

    let addr = "127.0.0.1:8000".parse().expect("valid listen address");
    log::info!("listening on http://{addr}/admin");
    Server::new(router, addr).run().await
}

/// Render the public homepage with live figures pulled from Postgres at
/// request time. One round-trip fills the "Live overview" cards: the
/// connected database + server version, catalogue counts, and order
/// totals. Every value falls back to a dash if the query fails, so the
/// page always renders even if the database hiccups.
async fn render_home(db: &Db) -> String {
    // `sqlx` is re-exported by the framework, so this example doesn't
    // declare a direct `sqlx` dependency.
    use rustio_admin::sqlx::{self, Row};

    let mut db_name = "—".to_string();
    let mut pg_major = "—".to_string();
    let (mut products, mut customers, mut orders) = (0_i64, 0_i64, 0_i64);
    let mut revenue = 0.0_f64;
    // Count of the project's own model tables present in Postgres — the
    // live source for the "Data models" header (matches the grid below).
    let mut models = 0_i64;

    let query = "SELECT current_database() AS db, \
                 split_part(current_setting('server_version'), '.', 1) AS pg, \
                 (SELECT count(*) FROM products)  AS products, \
                 (SELECT count(*) FROM customers) AS customers, \
                 (SELECT count(*) FROM orders)    AS orders, \
                 (SELECT COALESCE(SUM(total), 0) FROM orders)::float8 AS revenue, \
                 (SELECT count(*) FROM pg_class c \
                    JOIN pg_namespace n ON n.oid = c.relnamespace \
                  WHERE n.nspname = 'public' AND c.relkind = 'r' \
                    AND c.relname IN ('products', 'categories', 'customers', 'orders', \
                                      'order_items', 'addresses', 'product_images', \
                                      'cart_items', 'payments', 'payment_methods')) AS models";

    if let Ok(row) = sqlx::query(query).fetch_one(db.pool()).await {
        db_name = row.try_get::<String, _>("db").unwrap_or(db_name);
        pg_major = row.try_get::<String, _>("pg").unwrap_or(pg_major);
        products = row.try_get::<i64, _>("products").unwrap_or(0);
        customers = row.try_get::<i64, _>("customers").unwrap_or(0);
        orders = row.try_get::<i64, _>("orders").unwrap_or(0);
        revenue = row.try_get::<f64, _>("revenue").unwrap_or(0.0);
        models = row.try_get::<i64, _>("models").unwrap_or(0);
    } else {
        log::warn!("homepage: live stats query failed; rendering with placeholders");
    }

    HOMEPAGE_HTML
        .replace("{{DB_NAME}}", &db_name)
        .replace("{{PG_MAJOR}}", &pg_major)
        .replace("{{PRODUCTS}}", &products.to_string())
        .replace("{{CUSTOMERS}}", &customers.to_string())
        .replace("{{ORDERS}}", &orders.to_string())
        .replace("{{REVENUE}}", &group_thousands(revenue))
        .replace("{{MODELS}}", &models.to_string())
}

/// Format a money amount with thousands separators and two decimals,
/// e.g. `5432.1` -> `5,432.10`.
fn group_thousands(v: f64) -> String {
    let s = format!("{v:.2}");
    let (int_part, frac) = s.split_once('.').unwrap_or((s.as_str(), "00"));
    let neg = int_part.starts_with('-');
    let digits = int_part.trim_start_matches('-');
    let n = digits.len();
    let mut grouped = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (n - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    format!("{}{}.{}", if neg { "-" } else { "" }, grouped, frac)
}
