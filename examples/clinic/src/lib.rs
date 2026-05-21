//! `clinic-management` — typed data layer for the clinic schema in
//! `migrations/0001_clinic_schema.sql`.
//!
//! Standalone library. Not wired to rustio-admin: the schema uses
//! UUID primary keys, which the rustio-admin admin runtime does not
//! support today (every URL / ORM call assumes `i64`). This crate
//! gives you a clean typed surface (sqlx + chrono + rust_decimal +
//! uuid + serde) that any HTTP framework — or a future
//! UUID-aware admin layer — can wrap.
//!
//! All status / channel fields use Rust enums backed by `text`
//! columns; the SQL schema validates them via CHECK constraints,
//! so the wire format must stay in sync. `sqlx::Type` +
//! `rename_all = "snake_case"` keeps both sides aligned.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// =============================================================================
// Enums — kept as Rust enums but serialized as the snake_case strings the
// CHECK constraints expect. sqlx::Type with rename_all keeps the wire format
// aligned with what Postgres validates.
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum VisitType {
    InPerson,
    Telemedicine,
    HomeVisit,
    FollowUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AppointmentStatus {
    Booked,
    Confirmed,
    CheckedIn,
    InProgress,
    Completed,
    Cancelled,
    NoShow,
}

impl AppointmentStatus {
    pub fn is_active(self) -> bool {
        !matches!(self, Self::Cancelled | Self::NoShow | Self::Completed)
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        use AppointmentStatus::*;
        matches!(
            (self, next),
            (Booked, Confirmed | Cancelled)
                | (Confirmed, CheckedIn | Cancelled | NoShow)
                | (CheckedIn, InProgress | NoShow)
                | (InProgress, Completed)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BookingChannel {
    WalkIn,
    Phone,
    Web,
    MobileApp,
    Referral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReminderChannel {
    Sms,
    Email,
    Push,
    Whatsapp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReminderStatus {
    Pending,
    Sent,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Issued,
    Partial,
    Paid,
    Void,
    Overdue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    Cash,
    Card,
    BankTransfer,
    Insurance,
    MobilePay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Completed,
    Failed,
    Refunded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TimeOffStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum LabOrderStatus {
    Ordered,
    Collected,
    Processing,
    Completed,
    Cancelled,
}

// =============================================================================
// Reference tables
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Clinic {
    pub id: Uuid,
    pub name: String,
    pub organization_no: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Branch {
    pub id: Uuid,
    pub clinic_id: Uuid,
    pub name: String,
    pub address: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub phone: Option<String>,
    pub timezone: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Room {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub label: String,
    pub room_type: Option<String>,
    pub floor: Option<i16>,
    pub is_available: bool,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Specialty {
    pub id: Uuid,
    pub code: String,
    pub name_ar: String,
    pub name_en: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Service {
    pub id: Uuid,
    pub specialty_id: Option<Uuid>,
    pub code: String,
    pub name: String,
    pub base_price: Decimal,
    pub duration_minutes: i16,
    pub is_active: bool,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Medication {
    pub id: Uuid,
    pub name: String,
    pub generic_name: Option<String>,
    pub strength: Option<String>,
    pub form: Option<String>,
    pub atc_code: Option<String>,
}

// =============================================================================
// People
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Patient {
    pub id: Uuid,
    pub medical_record_no: String,
    pub full_name: String,
    pub national_id: Option<String>,
    pub date_of_birth: Option<NaiveDate>,
    pub gender: Option<Gender>,
    pub blood_type: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub allergies: Option<String>,
    pub chronic_conditions: Option<String>,
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Doctor {
    pub id: Uuid,
    pub full_name: String,
    pub license_number: String,
    pub specialty_id: Option<Uuid>,
    pub years_of_experience: Option<i16>,
    pub consultation_fee: Decimal,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub bio: Option<String>,
    pub accepts_new_patients: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// Scheduling
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DoctorSchedule {
    pub id: Uuid,
    pub doctor_id: Uuid,
    pub branch_id: Uuid,
    pub weekday: i16,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub slot_minutes: i16,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DoctorTimeOff {
    pub id: Uuid,
    pub doctor_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub reason: Option<String>,
    pub status: TimeOffStatus,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// Appointments
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Appointment {
    pub id: Uuid,
    pub appointment_number: String,
    pub patient_id: Uuid,
    pub doctor_id: Uuid,
    pub branch_id: Uuid,
    pub room_id: Option<Uuid>,
    pub service_id: Option<Uuid>,
    pub scheduled_start: DateTime<Utc>,
    pub scheduled_end: DateTime<Utc>,
    pub visit_type: VisitType,
    pub status: AppointmentStatus,
    pub chief_complaint: Option<String>,
    pub notes: Option<String>,
    pub checked_in_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub booking_channel: Option<BookingChannel>,
    pub cancellation_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAppointment {
    pub appointment_number: String,
    pub patient_id: Uuid,
    pub doctor_id: Uuid,
    pub branch_id: Uuid,
    pub room_id: Option<Uuid>,
    pub service_id: Option<Uuid>,
    pub scheduled_start: DateTime<Utc>,
    pub scheduled_end: DateTime<Utc>,
    pub visit_type: VisitType,
    pub chief_complaint: Option<String>,
    pub booking_channel: Option<BookingChannel>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AppointmentStatusHistory {
    pub id: Uuid,
    pub appointment_id: Uuid,
    pub from_status: Option<AppointmentStatus>,
    pub to_status: AppointmentStatus,
    pub changed_by: Option<Uuid>,
    pub reason: Option<String>,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AppointmentReminder {
    pub id: Uuid,
    pub appointment_id: Uuid,
    pub channel: ReminderChannel,
    pub send_at: DateTime<Utc>,
    pub status: ReminderStatus,
    pub sent_at: Option<DateTime<Utc>>,
}

// =============================================================================
// Clinical
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MedicalRecord {
    pub id: Uuid,
    pub appointment_id: Uuid,
    pub patient_id: Uuid,
    pub doctor_id: Uuid,
    pub subjective: Option<String>,
    pub objective: Option<String>,
    pub assessment: Option<String>,
    pub plan: Option<String>,
    pub bp_systolic: Option<i16>,
    pub bp_diastolic: Option<i16>,
    pub temperature: Option<Decimal>,
    pub pulse_rate: Option<i16>,
    pub weight_kg: Option<Decimal>,
    pub height_cm: Option<Decimal>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Diagnosis {
    pub id: Uuid,
    pub medical_record_id: Uuid,
    pub icd10_code: Option<String>,
    pub description: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Prescription {
    pub id: Uuid,
    pub medical_record_id: Uuid,
    pub prescribed_by: Uuid,
    pub medication_id: Uuid,
    pub dosage: Option<String>,
    pub frequency: Option<String>,
    pub duration_days: Option<i16>,
    pub instructions: Option<String>,
    pub is_dispensed: bool,
    pub issued_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LabOrder {
    pub id: Uuid,
    pub medical_record_id: Uuid,
    pub test_code: Option<String>,
    pub test_name: String,
    pub status: LabOrderStatus,
    pub result_value: Option<String>,
    pub reference_range: Option<String>,
    pub ordered_at: DateTime<Utc>,
    pub resulted_at: Option<DateTime<Utc>>,
}

// =============================================================================
// Billing
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct InsurancePolicy {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub provider_name: String,
    pub policy_number: String,
    pub holder_name: Option<String>,
    pub coverage_percent: Decimal,
    pub valid_from: Option<NaiveDate>,
    pub valid_to: Option<NaiveDate>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Invoice {
    pub id: Uuid,
    pub invoice_number: String,
    pub patient_id: Uuid,
    pub appointment_id: Option<Uuid>,
    pub insurance_policy_id: Option<Uuid>,
    pub subtotal: Decimal,
    pub discount_amount: Decimal,
    pub tax_amount: Decimal,
    pub total_amount: Decimal,
    pub amount_paid: Decimal,
    pub status: InvoiceStatus,
    pub due_date: Option<NaiveDate>,
    pub issued_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Invoice {
    pub fn outstanding(&self) -> Decimal {
        self.total_amount - self.amount_paid
    }

    pub fn is_settled(&self) -> bool {
        self.outstanding() <= Decimal::ZERO
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub amount: Decimal,
    pub payment_method: PaymentMethod,
    pub transaction_ref: Option<String>,
    pub status: PaymentStatus,
    pub paid_at: DateTime<Utc>,
}

// =============================================================================
// Errors
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ClinicError {
    #[error("appointment slot is already taken")]
    SlotConflict,

    #[error("invalid status transition: {from:?} -> {to:?}")]
    InvalidStatusTransition {
        from: AppointmentStatus,
        to: AppointmentStatus,
    },

    #[error("appointment not found: {0}")]
    AppointmentNotFound(Uuid),

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

// =============================================================================
// Tests — pure logic, no DB needed
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appointment_status_transitions_are_one_way() {
        use AppointmentStatus::*;
        // Happy path
        assert!(Booked.can_transition_to(Confirmed));
        assert!(Confirmed.can_transition_to(CheckedIn));
        assert!(CheckedIn.can_transition_to(InProgress));
        assert!(InProgress.can_transition_to(Completed));
        // Cancel allowed up to checked-in, then must go through no_show
        assert!(Booked.can_transition_to(Cancelled));
        assert!(Confirmed.can_transition_to(Cancelled));
        assert!(!CheckedIn.can_transition_to(Cancelled));
        // Terminal states don't transition
        assert!(!Completed.can_transition_to(Booked));
        assert!(!Cancelled.can_transition_to(Booked));
        assert!(!NoShow.can_transition_to(Confirmed));
    }

    #[test]
    fn appointment_status_is_active_matches_check_constraint_intent() {
        use AppointmentStatus::*;
        for active in [Booked, Confirmed, CheckedIn, InProgress] {
            assert!(active.is_active(), "{active:?} should be active");
        }
        for terminal in [Cancelled, NoShow, Completed] {
            assert!(!terminal.is_active(), "{terminal:?} should not be active");
        }
    }

    #[test]
    fn invoice_outstanding_and_settled_round_trip() {
        let mut inv = Invoice {
            id: Uuid::nil(),
            invoice_number: "INV-1".into(),
            patient_id: Uuid::nil(),
            appointment_id: None,
            insurance_policy_id: None,
            subtotal: Decimal::new(10_000, 2),
            discount_amount: Decimal::ZERO,
            tax_amount: Decimal::ZERO,
            total_amount: Decimal::new(10_000, 2),
            amount_paid: Decimal::ZERO,
            status: InvoiceStatus::Issued,
            due_date: None,
            issued_at: None,
            created_at: Utc::now(),
        };
        assert_eq!(inv.outstanding(), Decimal::new(10_000, 2));
        assert!(!inv.is_settled());
        inv.amount_paid = Decimal::new(10_000, 2);
        assert_eq!(inv.outstanding(), Decimal::ZERO);
        assert!(inv.is_settled());
    }
}
