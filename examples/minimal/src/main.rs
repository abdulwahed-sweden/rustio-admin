//! Minimal consumer of `rustio-admin`.
//!
//! Phase 1 bootstrap: this binary just signals that the workspace builds.
//! Later phases evolve it into the ~50-line "from struct to admin" demo
//! described in `rustio-admin-strategic-reset-plan.md` Section 10.

fn main() {
    println!("rustio-admin alive");
}
