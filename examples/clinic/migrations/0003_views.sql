-- =============================================================================
-- Clinic helper views — idempotent (CREATE OR REPLACE).
-- =============================================================================
--
-- Three derived-data views that turn previously-recurring ad-hoc queries into
-- single-table reads. The base schema is unchanged; these views are
-- additive sugar and can be dropped or replaced without touching data.
--
--   v_invoice_outstanding_by_source
--     Split each invoice's outstanding amount into "owed by patient" vs
--     "owed by insurer" by partitioning payments on payment_method =
--     'insurance'. Without this view the front desk + billing dept can't
--     tell at a glance whether to chase a patient or file an insurance
--     claim — the raw `invoices.amount_paid` aggregates both sources.
--
--   v_doctor_utilisation_7d
--     Rolling 7-day booked-hours vs scheduled-hours per doctor, with a
--     utilisation percentage. Rolling (not calendar week) because real
--     dashboards want "next 7 days from now," and calendar-week boundaries
--     misread near-zero when bookings cluster at the week edge.
--
--   v_active_medications
--     Every active prescription per patient. "Active" = no defined
--     duration (standing PRN) OR issued_at + duration_days hasn't
--     elapsed. Cross-cuts the prescriptions table so a third doctor
--     seeing the patient sees the current medication list at a glance —
--     the EMR convention of "active medications panel."
--
-- Apply (the clinic crate has no migration runner — run via psql):
--
--     PGPASSWORD='…' psql -h localhost -U clinic_app -d clinic_prod \
--         -f examples/clinic/migrations/0003_views.sql
--
-- Re-runnable: CREATE OR REPLACE VIEW is idempotent for these definitions
-- (no column-type changes). Drop manually with `DROP VIEW v_…` if needed.
-- =============================================================================

BEGIN;

-- -----------------------------------------------------------------------------
-- v_invoice_outstanding_by_source
-- -----------------------------------------------------------------------------
-- Computed columns per invoice:
--
--   patient_share              total × (100 - coverage_percent) / 100
--                              (or full total when no insurance)
--   insurance_share            total × coverage_percent / 100
--                              (or 0 when no insurance)
--   paid_by_patient            sum of completed payments where
--                              payment_method <> 'insurance'
--   paid_by_insurer            sum of completed payments where
--                              payment_method = 'insurance'
--   outstanding_from_patient   patient_share - paid_by_patient
--   outstanding_from_insurer   insurance_share - paid_by_insurer
--
-- Useful query: find invoices where the patient still owes money
-- (front-desk follow-up):
--
--     SELECT invoice_number, patient_id, outstanding_from_patient
--     FROM   v_invoice_outstanding_by_source
--     WHERE  outstanding_from_patient > 0;
--
-- Or where insurer hasn't paid yet (claims follow-up):
--
--     SELECT invoice_number, insurance_provider, outstanding_from_insurer
--     FROM   v_invoice_outstanding_by_source
--     WHERE  outstanding_from_insurer > 0;

CREATE OR REPLACE VIEW v_invoice_outstanding_by_source AS
SELECT
  i.id,
  i.invoice_number,
  i.patient_id,
  i.appointment_id,
  i.status,
  i.due_date,
  i.total_amount,
  i.amount_paid,

  -- Insurance link
  ip.id                                          AS insurance_policy_id,
  ip.provider_name                               AS insurance_provider,
  ip.policy_number                               AS insurance_policy_number,
  ip.coverage_percent,

  -- Expected shares per source
  CASE WHEN ip.id IS NOT NULL
       THEN round(i.total_amount * (100 - ip.coverage_percent) / 100, 2)
       ELSE i.total_amount
  END                                            AS patient_share,
  CASE WHEN ip.id IS NOT NULL
       THEN round(i.total_amount * ip.coverage_percent / 100, 2)
       ELSE 0::numeric(12,2)
  END                                            AS insurance_share,

  -- Actual payments received per source
  coalesce((
    SELECT sum(py.amount)
    FROM payments py
    WHERE py.invoice_id = i.id
      AND py.status = 'completed'
      AND py.payment_method <> 'insurance'
  ), 0)                                          AS paid_by_patient,
  coalesce((
    SELECT sum(py.amount)
    FROM payments py
    WHERE py.invoice_id = i.id
      AND py.status = 'completed'
      AND py.payment_method = 'insurance'
  ), 0)                                          AS paid_by_insurer,

  -- Outstanding per source = expected share - actual paid
  (CASE WHEN ip.id IS NOT NULL
        THEN round(i.total_amount * (100 - ip.coverage_percent) / 100, 2)
        ELSE i.total_amount
   END)
  - coalesce((
      SELECT sum(py.amount)
      FROM payments py
      WHERE py.invoice_id = i.id
        AND py.status = 'completed'
        AND py.payment_method <> 'insurance'
    ), 0)                                        AS outstanding_from_patient,
  (CASE WHEN ip.id IS NOT NULL
        THEN round(i.total_amount * ip.coverage_percent / 100, 2)
        ELSE 0::numeric(12,2)
   END)
  - coalesce((
      SELECT sum(py.amount)
      FROM payments py
      WHERE py.invoice_id = i.id
        AND py.status = 'completed'
        AND py.payment_method = 'insurance'
    ), 0)                                        AS outstanding_from_insurer

FROM invoices i
LEFT JOIN insurance_policies ip ON ip.id = i.insurance_policy_id;

-- -----------------------------------------------------------------------------
-- v_doctor_utilisation_7d
-- -----------------------------------------------------------------------------
-- Rolling 7-day forward window. Excludes cancelled / no_show appointments
-- because they don't consume capacity. Doctors without any doctor_schedules
-- rows report 0% utilisation (avoids divide-by-zero).
--
-- Useful query: doctors below 20% utilisation in the next week (capacity
-- to absorb new patients):
--
--     SELECT doctor, scheduled_h_per_wk, booked_h_next_7d, utilisation_pct
--     FROM   v_doctor_utilisation_7d
--     WHERE  utilisation_pct < 20
--     ORDER  BY utilisation_pct;

CREATE OR REPLACE VIEW v_doctor_utilisation_7d AS
WITH per_doctor_schedule AS (
  SELECT
    s.doctor_id,
    sum(extract(epoch FROM s.end_time - s.start_time)) AS scheduled_seconds
  FROM doctor_schedules s
  GROUP BY s.doctor_id
),
per_doctor_booked AS (
  SELECT
    a.doctor_id,
    sum(extract(epoch FROM a.scheduled_end - a.scheduled_start)) AS booked_seconds,
    count(*)                                                      AS slot_count
  FROM appointments a
  WHERE a.status NOT IN ('cancelled', 'no_show')
    AND a.scheduled_start >= now()
    AND a.scheduled_start <  now() + interval '7 days'
  GROUP BY a.doctor_id
)
SELECT
  d.id                                                            AS doctor_id,
  d.full_name                                                     AS doctor,
  sp.code                                                         AS specialty,
  round(coalesce(s.scheduled_seconds, 0)::numeric / 3600, 1)      AS scheduled_h_per_wk,
  round(coalesce(b.booked_seconds, 0)::numeric / 3600, 2)         AS booked_h_next_7d,
  coalesce(b.slot_count, 0)                                       AS slots_next_7d,
  CASE
    WHEN coalesce(s.scheduled_seconds, 0) = 0 THEN 0.0
    ELSE round(100.0 * coalesce(b.booked_seconds, 0) / s.scheduled_seconds, 1)
  END                                                             AS utilisation_pct
FROM doctors d
LEFT JOIN specialties sp        ON sp.id = d.specialty_id
LEFT JOIN per_doctor_schedule s ON s.doctor_id = d.id
LEFT JOIN per_doctor_booked b   ON b.doctor_id = d.id
WHERE d.is_active;

-- -----------------------------------------------------------------------------
-- v_active_medications
-- -----------------------------------------------------------------------------
-- "Active" = either no defined duration (standing PRN order) OR
-- issued_at + duration_days has not yet elapsed. Expired short courses
-- (the typical 5-day paracetamol etc.) drop out automatically without
-- needing a separate `archived` flag.
--
-- `expires_at` is NULL for indefinite-duration prescriptions and a
-- timestamptz for time-limited courses.
--
-- Useful queries:
--
--   All active meds for one patient (cross-cut across visits):
--     SELECT medication, dosage, frequency, expires_at, prescribed_by
--     FROM   v_active_medications
--     WHERE  patient_id = '77777777-7777-7777-7777-000000000006';
--
--   Patients on a specific ATC class (drug-drug-interaction screen):
--     SELECT patient, medication, dosage
--     FROM   v_active_medications
--     WHERE  atc_code LIKE 'C09%';        -- ACE inhibitors + ARBs

CREATE OR REPLACE VIEW v_active_medications AS
SELECT
  p.id                                                            AS patient_id,
  p.full_name                                                     AS patient,
  p.medical_record_no,
  m.id                                                            AS medication_id,
  m.name                                                          AS medication,
  m.generic_name,
  m.strength,
  m.form,
  m.atc_code,
  rx.dosage,
  rx.frequency,
  rx.duration_days,
  rx.issued_at,
  CASE WHEN rx.duration_days IS NOT NULL
       THEN rx.issued_at + (rx.duration_days || ' days')::interval
       ELSE NULL
  END                                                             AS expires_at,
  rx.is_dispensed,
  d.id                                                            AS prescribed_by_id,
  d.full_name                                                     AS prescribed_by,
  rx.id                                                           AS prescription_id,
  mr.id                                                           AS medical_record_id
FROM prescriptions rx
JOIN medical_records mr  ON mr.id = rx.medical_record_id
JOIN patients p          ON p.id  = mr.patient_id
JOIN medications m       ON m.id  = rx.medication_id
JOIN doctors d           ON d.id  = rx.prescribed_by
WHERE rx.duration_days IS NULL
   OR (rx.issued_at + (rx.duration_days || ' days')::interval) >= now();

COMMIT;
