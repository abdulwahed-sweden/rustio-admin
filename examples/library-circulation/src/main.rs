//! library-circulation example — boots an admin panel for the
//! 4-table library schema defined in D.2.
//!
//! Run:
//!
//!   createdb library_circulation_demo
//!   DATABASE_URL=postgres://localhost/library_circulation_demo \
//!     cargo run -p library-circulation
//!
//! First-time admin superuser creation is out of scope for the
//! example main.rs; use the framework CLI (`rustio user create`)
//! against the same DATABASE_URL.

mod models;

use std::net::SocketAddr;

use rustio_admin::templates::Templates;
use rustio_admin::{
    auth, middleware, migrations, register_admin_routes, Admin, Db, Result, Router, Server,
};

use crate::models::branch::Branch;
use crate::models::item::Item;
use crate::models::loan::Loan;
use crate::models::patron::Patron;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Connection string — env override, localhost dev fallback.
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@127.0.0.1:5432/library_circulation_demo".into()
    });

    // 2. Connect; fails fast if URL or server is wrong.
    let db = Db::connect(&database_url).await?;

    // 3. Framework's own auth / session / permission / audit tables.
    auth::init_tables(&db).await?;

    // 4. Example domain migrations (branches, patrons, items, loans).
    //    Run before route registration so admin queries never target
    //    missing tables.
    migrations::apply(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).await?;

    // 5. Register the four models on the admin builder.
    let admin = Admin::new()
        .model::<Branch>()
        .model::<Patron>()
        .model::<Item>()
        .model::<Loan>();

    // 6. Materialise view / add / change / delete perms per model.
    admin.seed_permissions(&db).await?;

    // 7. Embedded-template engine (no disk override directory).
    let templates = Templates::new(None)?;

    // 8. R0-canonical middleware chain. Order is load-bearing.
    let router = Router::new()
        .middleware(middleware::logger)
        .middleware(middleware::correlation_id)
        .middleware(middleware::security_headers)
        .middleware(middleware::csrf_protect);

    // 9. Mount /admin/* onto the middleware-wrapped router.
    let router = register_admin_routes(router, admin, db.clone(), templates);

    // 10. Bind and run; blocks until Ctrl-C.
    let addr: SocketAddr = "127.0.0.1:3000"
        .parse()
        .expect("hard-coded bind address parses");
    println!("library-circulation admin listening on http://{addr}/admin/");
    Server::new(router, addr).run().await?;

    Ok(())
}
