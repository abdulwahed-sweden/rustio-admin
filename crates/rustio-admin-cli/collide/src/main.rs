//! Project entry point. Generator scaffolds this once;
//! thereafter it is developer-owned.
//!
//! Run `rustio add model <Name>` and `rustio commit` to
//! populate `src/_generated/`. Then wire your server here.

mod _generated;

#[tokio::main]
async fn main() {
    let _admin = _generated::admin::build_admin();
    println!("rustio: scaffold ready. Edit src/main.rs to wire your server.");
}
