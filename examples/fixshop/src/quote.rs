//! The price the shop sent the customer, and whether they said yes.
//!
//! `approved` is the single source of truth for approval in this
//! project. The trigger in migration 0003 mirrors it onto
//! `jobs.quote_approved` on every write here — including the owner
//! editing this row by hand — so the job's copy can never disagree.
//!
//! Who can do what, from migration 0005:
//!
//! - technician — `quotes.add_quote`, `quotes.view_quote`. Writes the
//!   quote, then cannot go back and change the number.
//! - front desk — `quotes.view_quote` only. Records the customer's
//!   answer with the "Customer approved" button on the job, which is
//!   gated on `jobs.approve_job` instead.
//! - owner — signs in as `administrator`, bypasses group checks, and
//!   is therefore the only one who can edit or delete a quote.

use rustio_admin::{DateTime, Decimal, FieldValidationError, ModelAdmin, RustioAdmin, Utc};

#[derive(RustioAdmin)]
#[rustio(table = "quotes")]
pub struct Quote {
    pub id: i64,
    #[rustio(belongs_to = "Job", display = "ticket_no")]
    pub job_id: i64,
    /// `Decimal` maps to `NUMERIC(10, 2)` — money, not a float.
    pub amount: Decimal,
    pub approved: bool,
    pub created_at: DateTime<Utc>,
}

impl ModelAdmin for Quote {
    fn list_display() -> &'static [&'static str] {
        &["job_id", "amount", "approved", "created_at"]
    }

    fn list_filter() -> &'static [&'static str] {
        &["approved"]
    }

    fn ordering() -> &'static [&'static str] {
        &["-created_at"]
    }

    fn validate(quote: &Self) -> std::result::Result<(), Vec<FieldValidationError>> {
        if quote.amount <= Decimal::ZERO {
            return Err(vec![FieldValidationError::field(
                "amount",
                "must be more than zero.",
            )]);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quote(amount: &str) -> Quote {
        Quote {
            id: 1,
            job_id: 1,
            amount: amount.parse().unwrap(),
            approved: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn a_real_price_is_accepted() {
        assert!(Quote::validate(&quote("145.00")).is_ok());
    }

    #[test]
    fn zero_and_negative_prices_are_refused() {
        for amount in ["0", "-10.00"] {
            let errs = Quote::validate(&quote(amount)).unwrap_err();
            assert_eq!(errs[0].field, Some("amount"));
        }
    }
}
