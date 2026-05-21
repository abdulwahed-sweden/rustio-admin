-- =============================================================================
-- Clinic Appointments Schema (PostgreSQL 14+)
-- =============================================================================
-- Conventions:
--   * snake_case table and column names, plural table names
--   * UUID primary keys (gen_random_uuid)
--   * timestamptz everywhere (no naive timestamps)
--   * money as numeric(12,2)
--   * soft enums via CHECK constraints (cheaper to evolve than native enums)
-- =============================================================================

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- -----------------------------------------------------------------------------
-- Reference: clinics, branches, rooms, specialties, services, medications
-- -----------------------------------------------------------------------------

CREATE TABLE clinics (
    id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name             varchar(200) NOT NULL,
    organization_no  varchar(50),
    contact_email    varchar(150),
    contact_phone    varchar(30),
    created_at       timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE branches (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    clinic_id    uuid NOT NULL REFERENCES clinics(id) ON DELETE RESTRICT,
    name         varchar(200) NOT NULL,
    address      text,
    city         varchar(100),
    postal_code  varchar(20),
    phone        varchar(30),
    timezone     varchar(50) NOT NULL DEFAULT 'Europe/Stockholm',
    is_active    boolean NOT NULL DEFAULT true,
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_branches_clinic ON branches(clinic_id);

CREATE TABLE rooms (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    branch_id     uuid NOT NULL REFERENCES branches(id) ON DELETE CASCADE,
    label         varchar(50) NOT NULL,
    room_type     varchar(50),
    floor         smallint,
    is_available  boolean NOT NULL DEFAULT true,
    UNIQUE (branch_id, label)
);

CREATE TABLE specialties (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    code         varchar(50) UNIQUE NOT NULL,
    name_ar      varchar(150) NOT NULL,
    name_en      varchar(150),
    description  text
);

CREATE TABLE services (
    id                uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    specialty_id      uuid REFERENCES specialties(id) ON DELETE SET NULL,
    code              varchar(50) UNIQUE NOT NULL,
    name              varchar(200) NOT NULL,
    base_price        numeric(12,2) NOT NULL DEFAULT 0,
    duration_minutes  smallint NOT NULL DEFAULT 30,
    is_active         boolean NOT NULL DEFAULT true,
    CHECK (duration_minutes > 0),
    CHECK (base_price >= 0)
);

CREATE TABLE medications (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name          varchar(200) NOT NULL,
    generic_name  varchar(200),
    strength      varchar(50),
    form          varchar(50),
    atc_code      varchar(20)
);
CREATE INDEX idx_medications_name ON medications(name);

-- -----------------------------------------------------------------------------
-- People: patients, doctors
-- -----------------------------------------------------------------------------

CREATE TABLE patients (
    id                   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    medical_record_no    varchar(30) UNIQUE NOT NULL,
    full_name            varchar(200) NOT NULL,
    national_id          varchar(30) UNIQUE,
    date_of_birth        date,
    gender               varchar(10) CHECK (gender IN ('male', 'female', 'other')),
    blood_type           varchar(3),
    phone                varchar(30),
    email                varchar(150),
    address              text,
    allergies            text,
    chronic_conditions   text,
    registered_at        timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_patients_phone ON patients(phone);
CREATE INDEX idx_patients_name ON patients(full_name);

CREATE TABLE doctors (
    id                     uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    full_name              varchar(200) NOT NULL,
    license_number         varchar(50) UNIQUE NOT NULL,
    specialty_id           uuid REFERENCES specialties(id) ON DELETE SET NULL,
    years_of_experience    smallint DEFAULT 0,
    consultation_fee       numeric(12,2) NOT NULL DEFAULT 0,
    phone                  varchar(30),
    email                  varchar(150),
    bio                    text,
    accepts_new_patients   boolean NOT NULL DEFAULT true,
    is_active              boolean NOT NULL DEFAULT true,
    created_at             timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_doctors_specialty ON doctors(specialty_id);

-- -----------------------------------------------------------------------------
-- Scheduling: doctor weekly schedules + time off
-- -----------------------------------------------------------------------------

CREATE TABLE doctor_schedules (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    doctor_id     uuid NOT NULL REFERENCES doctors(id) ON DELETE CASCADE,
    branch_id     uuid NOT NULL REFERENCES branches(id) ON DELETE CASCADE,
    weekday       smallint NOT NULL CHECK (weekday BETWEEN 0 AND 6),
    start_time    time NOT NULL,
    end_time      time NOT NULL,
    slot_minutes  smallint NOT NULL DEFAULT 30,
    CHECK (end_time > start_time),
    CHECK (slot_minutes > 0)
);
CREATE INDEX idx_schedules_doctor ON doctor_schedules(doctor_id, weekday);

CREATE TABLE doctor_time_off (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    doctor_id   uuid NOT NULL REFERENCES doctors(id) ON DELETE CASCADE,
    starts_at   timestamptz NOT NULL,
    ends_at     timestamptz NOT NULL,
    reason      varchar(200),
    status      varchar(20) NOT NULL DEFAULT 'approved'
                CHECK (status IN ('pending', 'approved', 'rejected')),
    created_at  timestamptz NOT NULL DEFAULT now(),
    CHECK (ends_at > starts_at)
);
CREATE INDEX idx_timeoff_doctor_range ON doctor_time_off(doctor_id, starts_at, ends_at);

-- -----------------------------------------------------------------------------
-- Appointments (the central table)
-- -----------------------------------------------------------------------------

CREATE TABLE appointments (
    id                    uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    appointment_number    varchar(20) UNIQUE NOT NULL,
    patient_id            uuid NOT NULL REFERENCES patients(id) ON DELETE RESTRICT,
    doctor_id             uuid NOT NULL REFERENCES doctors(id) ON DELETE RESTRICT,
    branch_id             uuid NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    room_id               uuid REFERENCES rooms(id) ON DELETE SET NULL,
    service_id            uuid REFERENCES services(id) ON DELETE SET NULL,
    scheduled_start       timestamptz NOT NULL,
    scheduled_end         timestamptz NOT NULL,
    visit_type            varchar(20) NOT NULL DEFAULT 'in_person'
                          CHECK (visit_type IN ('in_person', 'telemedicine', 'home_visit', 'follow_up')),
    status                varchar(20) NOT NULL DEFAULT 'booked'
                          CHECK (status IN ('booked', 'confirmed', 'checked_in', 'in_progress',
                                            'completed', 'cancelled', 'no_show')),
    chief_complaint       text,
    notes                 text,
    checked_in_at         timestamptz,
    completed_at          timestamptz,
    booking_channel       varchar(20) DEFAULT 'walk_in'
                          CHECK (booking_channel IN ('walk_in', 'phone', 'web', 'mobile_app', 'referral')),
    cancellation_reason   text,
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz NOT NULL DEFAULT now(),
    CHECK (scheduled_end > scheduled_start)
);

CREATE INDEX idx_appointments_patient ON appointments(patient_id, scheduled_start DESC);
CREATE INDEX idx_appointments_doctor_day ON appointments(doctor_id, scheduled_start);
CREATE INDEX idx_appointments_status ON appointments(status) WHERE status IN ('booked', 'confirmed', 'checked_in');
CREATE INDEX idx_appointments_branch_day ON appointments(branch_id, scheduled_start);

-- Prevent double-booking the same doctor at overlapping times (excluding cancelled/no_show)
CREATE EXTENSION IF NOT EXISTS btree_gist;
ALTER TABLE appointments
    ADD CONSTRAINT no_doctor_overlap
    EXCLUDE USING gist (
        doctor_id WITH =,
        tstzrange(scheduled_start, scheduled_end, '[)') WITH &&
    ) WHERE (status NOT IN ('cancelled', 'no_show'));

CREATE TABLE appointment_status_history (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    appointment_id  uuid NOT NULL REFERENCES appointments(id) ON DELETE CASCADE,
    from_status     varchar(20),
    to_status       varchar(20) NOT NULL,
    changed_by      uuid,
    reason          text,
    changed_at      timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_status_history_appt ON appointment_status_history(appointment_id, changed_at);

CREATE TABLE appointment_reminders (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    appointment_id  uuid NOT NULL REFERENCES appointments(id) ON DELETE CASCADE,
    channel         varchar(20) NOT NULL CHECK (channel IN ('sms', 'email', 'push', 'whatsapp')),
    send_at         timestamptz NOT NULL,
    status          varchar(20) NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'sent', 'failed', 'cancelled')),
    sent_at         timestamptz
);
CREATE INDEX idx_reminders_pending ON appointment_reminders(send_at) WHERE status = 'pending';

-- -----------------------------------------------------------------------------
-- Clinical: medical records, diagnoses, prescriptions, lab orders
-- -----------------------------------------------------------------------------

CREATE TABLE medical_records (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    appointment_id  uuid UNIQUE NOT NULL REFERENCES appointments(id) ON DELETE CASCADE,
    patient_id      uuid NOT NULL REFERENCES patients(id) ON DELETE RESTRICT,
    doctor_id       uuid NOT NULL REFERENCES doctors(id) ON DELETE RESTRICT,
    subjective      text,
    objective       text,
    assessment      text,
    plan            text,
    bp_systolic     smallint,
    bp_diastolic    smallint,
    temperature     numeric(4,2),
    pulse_rate      smallint,
    weight_kg       numeric(5,2),
    height_cm       numeric(5,2),
    recorded_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_records_patient ON medical_records(patient_id, recorded_at DESC);

CREATE TABLE diagnoses (
    id                 uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    medical_record_id  uuid NOT NULL REFERENCES medical_records(id) ON DELETE CASCADE,
    icd10_code         varchar(10),
    description        text NOT NULL,
    is_primary         boolean NOT NULL DEFAULT false
);
CREATE INDEX idx_diagnoses_record ON diagnoses(medical_record_id);

CREATE TABLE prescriptions (
    id                 uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    medical_record_id  uuid NOT NULL REFERENCES medical_records(id) ON DELETE CASCADE,
    prescribed_by      uuid NOT NULL REFERENCES doctors(id) ON DELETE RESTRICT,
    medication_id      uuid NOT NULL REFERENCES medications(id) ON DELETE RESTRICT,
    dosage             varchar(100),
    frequency          varchar(100),
    duration_days      smallint,
    instructions       text,
    is_dispensed       boolean NOT NULL DEFAULT false,
    issued_at          timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_prescriptions_record ON prescriptions(medical_record_id);

CREATE TABLE lab_orders (
    id                 uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    medical_record_id  uuid NOT NULL REFERENCES medical_records(id) ON DELETE CASCADE,
    test_code          varchar(50),
    test_name          varchar(200) NOT NULL,
    status             varchar(20) NOT NULL DEFAULT 'ordered'
                       CHECK (status IN ('ordered', 'collected', 'processing', 'completed', 'cancelled')),
    result_value       text,
    reference_range    varchar(100),
    ordered_at         timestamptz NOT NULL DEFAULT now(),
    resulted_at        timestamptz
);
CREATE INDEX idx_lab_orders_record ON lab_orders(medical_record_id);

-- -----------------------------------------------------------------------------
-- Billing: insurance, invoices, payments
-- -----------------------------------------------------------------------------

CREATE TABLE insurance_policies (
    id                uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id        uuid NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    provider_name     varchar(150) NOT NULL,
    policy_number     varchar(50) NOT NULL,
    holder_name       varchar(200),
    coverage_percent  numeric(5,2) NOT NULL DEFAULT 0
                      CHECK (coverage_percent BETWEEN 0 AND 100),
    valid_from        date,
    valid_to          date,
    UNIQUE (provider_name, policy_number)
);
CREATE INDEX idx_insurance_patient ON insurance_policies(patient_id);

CREATE TABLE invoices (
    id                    uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_number        varchar(30) UNIQUE NOT NULL,
    patient_id            uuid NOT NULL REFERENCES patients(id) ON DELETE RESTRICT,
    appointment_id        uuid REFERENCES appointments(id) ON DELETE SET NULL,
    insurance_policy_id   uuid REFERENCES insurance_policies(id) ON DELETE SET NULL,
    subtotal              numeric(12,2) NOT NULL DEFAULT 0,
    discount_amount       numeric(12,2) NOT NULL DEFAULT 0,
    tax_amount            numeric(12,2) NOT NULL DEFAULT 0,
    total_amount          numeric(12,2) NOT NULL DEFAULT 0,
    amount_paid           numeric(12,2) NOT NULL DEFAULT 0,
    status                varchar(20) NOT NULL DEFAULT 'draft'
                          CHECK (status IN ('draft', 'issued', 'partial', 'paid', 'void', 'overdue')),
    due_date              date,
    issued_at             timestamptz,
    created_at            timestamptz NOT NULL DEFAULT now(),
    CHECK (amount_paid >= 0 AND amount_paid <= total_amount + 0.01),
    CHECK (subtotal >= 0 AND discount_amount >= 0 AND tax_amount >= 0)
);
CREATE INDEX idx_invoices_patient ON invoices(patient_id, created_at DESC);
CREATE INDEX idx_invoices_status ON invoices(status) WHERE status IN ('issued', 'partial', 'overdue');

CREATE TABLE payments (
    id                uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_id        uuid NOT NULL REFERENCES invoices(id) ON DELETE RESTRICT,
    amount            numeric(12,2) NOT NULL CHECK (amount > 0),
    payment_method    varchar(20) NOT NULL
                      CHECK (payment_method IN ('cash', 'card', 'bank_transfer', 'insurance', 'mobile_pay')),
    transaction_ref   varchar(100),
    status            varchar(20) NOT NULL DEFAULT 'completed'
                      CHECK (status IN ('pending', 'completed', 'failed', 'refunded')),
    paid_at           timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_payments_invoice ON payments(invoice_id);

-- -----------------------------------------------------------------------------
-- Updated-at trigger (keep updated_at fresh on appointments)
-- -----------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION touch_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_appointments_touch
    BEFORE UPDATE ON appointments
    FOR EACH ROW
    EXECUTE FUNCTION touch_updated_at();
