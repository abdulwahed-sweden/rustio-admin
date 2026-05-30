//! Billing — invoices raised against patients.

use clinic_core::prelude::*;

mod invoice;

pub use invoice::Invoice;

/// Add this capability's models to the admin. Called by `clinic-server`.
pub fn register(admin: Admin) -> Admin {
    admin.model::<Invoice>()
}
