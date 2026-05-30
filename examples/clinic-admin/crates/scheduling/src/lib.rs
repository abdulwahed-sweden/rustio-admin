//! Scheduling — appointments booked for patients.

use clinic_core::prelude::*;

mod appointment;

pub use appointment::Appointment;

/// Add this capability's models to the admin. Called by `clinic-server`.
pub fn register(admin: Admin) -> Admin {
    admin.model::<Appointment>()
}
