//! Patient management — the people receiving care and their vitals.
//!
//! A capability crate owns a slice of the business. Its tables are
//! modules inside it (`patient`, `vitals`), and it exposes ONE function:
//! [`register`], which the server calls to add this capability's models
//! to the admin. The server knows nothing about patients beyond this.

use clinic_core::prelude::*;

mod patient;
mod vitals;

pub use patient::Patient;
pub use vitals::Vitals;

/// Add this capability's models to the admin. Called by `clinic-server`.
pub fn register(admin: Admin) -> Admin {
    admin.model::<Patient>().model::<Vitals>()
}
