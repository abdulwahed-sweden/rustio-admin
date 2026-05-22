-- =============================================================================
-- Clinic seed — small but representative dataset for an empty clinic_prod.
-- =============================================================================
--
-- Context:
--   * Sweden / Arabic bilingual clinic — matches `specialties.name_ar` and
--     the `Europe/Stockholm` default branch timezone.
--   * Currency SEK (numeric(12,2)).
--   * Time anchor: 2026-05-22 (today). Past appointments are completed with
--     medical records + invoices; today's are in-progress / checked-in;
--     future ones are booked / confirmed.
--   * No `gen_random_uuid()` — literal UUIDs with namespace prefixes so cross-
--     INSERT references read cleanly:
--         11.. clinics      22.. branches    33.. rooms
--         44.. specialties  55.. services    66.. medications
--         77.. patients     88.. doctors     99.. appointments
--         aa.. doctor_schedules    bb.. doctor_time_off
--         cc.. appointment_status_history   dd.. appointment_reminders
--         ee.. medical_records     ff.. diagnoses
--         a1.. prescriptions       a2.. lab_orders
--         b1.. insurance_policies  b2.. invoices    b3.. payments
--
-- Apply (the clinic crate has no migration runner — run via psql):
--
--     PGPASSWORD='…' psql -h localhost -U clinic_app -d clinic_prod \
--         -f examples/clinic/migrations/0002_seed.sql
--
-- This file expects a virgin schema (0001 just applied). It does NOT use
-- `ON CONFLICT`; re-running on populated tables will error on UNIQUE
-- constraints. To re-seed: `TRUNCATE ... RESTART IDENTITY CASCADE` first,
-- or drop+reapply 0001.
--
-- All people / clinics / addresses / national-id numbers in this file are
-- fictional. ICD-10 codes (J06.9, I10, E11.9, L70.0, R51) and ATC codes
-- are real; their attachment to invented patients is illustrative only.
-- =============================================================================

BEGIN;

-- -----------------------------------------------------------------------------
-- Reference: clinic, branches, rooms
-- -----------------------------------------------------------------------------

INSERT INTO clinics (id, name, organization_no, contact_email, contact_phone, created_at) VALUES
  ('11111111-1111-1111-1111-000000000001',
   'Stockholm Health Center',
   '556677-8899',
   'info@stockholmhealthcenter.se',
   '+46 8 555 0100',
   '2025-09-01 09:00:00+02');

INSERT INTO branches (id, clinic_id, name, address, city, postal_code, phone, timezone, is_active, created_at) VALUES
  ('22222222-2222-2222-2222-000000000001',
   '11111111-1111-1111-1111-000000000001',
   'Östermalm Branch',
   'Karlavägen 100', 'Stockholm', '11526',
   '+46 8 555 0101', 'Europe/Stockholm', true, '2025-09-01 09:00:00+02'),
  ('22222222-2222-2222-2222-000000000002',
   '11111111-1111-1111-1111-000000000001',
   'Söder Branch',
   'Hornsgatan 50',   'Stockholm', '11821',
   '+46 8 555 0102', 'Europe/Stockholm', true, '2025-09-15 09:00:00+02');

INSERT INTO rooms (id, branch_id, label, room_type, floor, is_available) VALUES
  ('33333333-3333-3333-3333-000000000001', '22222222-2222-2222-2222-000000000001', 'A1', 'consultation', 1, true),
  ('33333333-3333-3333-3333-000000000002', '22222222-2222-2222-2222-000000000001', 'A2', 'consultation', 1, true),
  ('33333333-3333-3333-3333-000000000003', '22222222-2222-2222-2222-000000000001', 'B1', 'treatment',    2, true),
  ('33333333-3333-3333-3333-000000000004', '22222222-2222-2222-2222-000000000002', 'S1', 'consultation', 1, true),
  ('33333333-3333-3333-3333-000000000005', '22222222-2222-2222-2222-000000000002', 'S2', 'consultation', 1, true),
  ('33333333-3333-3333-3333-000000000006', '22222222-2222-2222-2222-000000000002', 'S3', 'examination',  1, true);

-- -----------------------------------------------------------------------------
-- Specialties + services + medications
-- -----------------------------------------------------------------------------

INSERT INTO specialties (id, code, name_ar, name_en, description) VALUES
  ('44444444-4444-4444-4444-000000000001', 'GP',   'طب عام',           'General Practice',   'Family medicine / primary care.'),
  ('44444444-4444-4444-4444-000000000002', 'CARD', 'أمراض القلب',       'Cardiology',         'Heart and circulatory system.'),
  ('44444444-4444-4444-4444-000000000003', 'PED',  'طب الأطفال',       'Pediatrics',         'Infant, child, and adolescent care.'),
  ('44444444-4444-4444-4444-000000000004', 'DERM', 'الأمراض الجلدية', 'Dermatology',        'Skin, hair, and nail conditions.'),
  ('44444444-4444-4444-4444-000000000005', 'IM',   'الباطنية',          'Internal Medicine',  'Adult internal-organ conditions.');

INSERT INTO services (id, specialty_id, code, name, base_price, duration_minutes, is_active) VALUES
  ('55555555-5555-5555-5555-000000000001', '44444444-4444-4444-4444-000000000001', 'GP-CONS',   'GP consultation',          500.00, 30, true),
  ('55555555-5555-5555-5555-000000000002', '44444444-4444-4444-4444-000000000002', 'CARD-CONS', 'Cardiology consultation', 1200.00, 45, true),
  ('55555555-5555-5555-5555-000000000003', '44444444-4444-4444-4444-000000000002', 'ECG',       'ECG test',                 400.00, 20, true),
  ('55555555-5555-5555-5555-000000000004', '44444444-4444-4444-4444-000000000003', 'PED-CONS',  'Pediatric consultation',   700.00, 30, true),
  ('55555555-5555-5555-5555-000000000005', '44444444-4444-4444-4444-000000000004', 'DERM-CONS', 'Dermatology consultation', 900.00, 30, true),
  ('55555555-5555-5555-5555-000000000006', '44444444-4444-4444-4444-000000000004', 'BIOPSY',    'Skin biopsy',             1500.00, 60, true),
  ('55555555-5555-5555-5555-000000000007', '44444444-4444-4444-4444-000000000003', 'VACC',      'Vaccination',              350.00, 15, true),
  ('55555555-5555-5555-5555-000000000008', '44444444-4444-4444-4444-000000000005', 'IM-CHECK',  'Annual health check',     1800.00, 60, true);

INSERT INTO medications (id, name, generic_name, strength, form, atc_code) VALUES
  ('66666666-6666-6666-6666-000000000001', 'Alvedon',    'Paracetamol',  '500 mg',  'tablet',  'N02BE01'),
  ('66666666-6666-6666-6666-000000000002', 'Amimox',     'Amoxicillin',  '500 mg',  'capsule', 'J01CA04'),
  ('66666666-6666-6666-6666-000000000003', 'Ibumetin',   'Ibuprofen',    '400 mg',  'tablet',  'M01AE01'),
  ('66666666-6666-6666-6666-000000000004', 'Glucophage', 'Metformin',    '500 mg',  'tablet',  'A10BA02'),
  ('66666666-6666-6666-6666-000000000005', 'Zestril',    'Lisinopril',   '10 mg',   'tablet',  'C09AA03'),
  ('66666666-6666-6666-6666-000000000006', 'Ventoline',  'Salbutamol',   '100 mcg', 'inhaler', 'R03AC02');

-- -----------------------------------------------------------------------------
-- People: patients + doctors
-- -----------------------------------------------------------------------------

INSERT INTO patients (id, medical_record_no, full_name, national_id, date_of_birth, gender, blood_type,
                      phone, email, address, allergies, chronic_conditions, registered_at) VALUES
  ('77777777-7777-7777-7777-000000000001', 'MRN-2025-0001', 'Anna Lindqvist',     '19850412-1234', '1985-04-12', 'female', 'O+',
   '+46 70 111 0001', 'anna.lindqvist@example.se', 'Sveavägen 1, Stockholm', NULL, NULL, '2025-09-10 10:00:00+02'),
  ('77777777-7777-7777-7777-000000000002', 'MRN-2025-0002', 'Erik Johansson',     '19720228-2345', '1972-02-28', 'male',   'A+',
   '+46 70 111 0002', 'erik.johansson@example.se', 'Götgatan 50, Stockholm', 'Penicillin', 'Hypertension', '2025-09-12 11:30:00+02'),
  ('77777777-7777-7777-7777-000000000003', 'MRN-2025-0003', 'Fatima Karim',       '19900803-3456', '1990-08-03', 'female', 'B+',
   '+46 70 111 0003', 'fatima.karim@example.se', 'Folkungagatan 12, Stockholm', NULL, 'Asthma', '2025-09-20 09:15:00+02'),
  ('77777777-7777-7777-7777-000000000004', 'MRN-2025-0004', 'Lukas Bergman',      '20171115-4567', '2017-11-15', 'male',   'O-',
   '+46 70 111 0004', 'parent.bergman@example.se', 'Sankt Eriksgatan 88, Stockholm', NULL, NULL, '2025-10-01 14:00:00+02'),
  ('77777777-7777-7777-7777-000000000005', 'MRN-2025-0005', 'Sofia Andersson',    '19951001-5678', '1995-10-01', 'female', 'AB+',
   '+46 70 111 0005', 'sofia.andersson@example.se', 'Kungsholms Strand 5, Stockholm', 'Latex', NULL, '2025-10-10 10:45:00+02'),
  ('77777777-7777-7777-7777-000000000006', 'MRN-2025-0006', 'Omar Saleh',         '19690705-6789', '1969-07-05', 'male',   'A-',
   '+46 70 111 0006', 'omar.saleh@example.se', 'Birger Jarlsgatan 30, Stockholm', NULL, 'Type 2 diabetes, Hypertension', '2025-10-15 11:00:00+02'),
  ('77777777-7777-7777-7777-000000000007', 'MRN-2025-0007', 'Maja Karlsson',      '20080530-7890', '2008-05-30', 'female', 'O+',
   '+46 70 111 0007', 'parent.karlsson@example.se', 'Vasagatan 22, Stockholm', 'Peanuts', NULL, '2025-11-02 09:30:00+02'),
  ('77777777-7777-7777-7777-000000000008', 'MRN-2025-0008', 'Hassan Al-Mahmoud',  '19811217-8901', '1981-12-17', 'male',   'B-',
   '+46 70 111 0008', 'hassan.almahmoud@example.se', 'Drottninggatan 5, Stockholm', NULL, NULL, '2025-11-20 13:00:00+02');

INSERT INTO doctors (id, full_name, license_number, specialty_id, years_of_experience,
                     consultation_fee, phone, email, bio, accepts_new_patients, is_active, created_at) VALUES
  ('88888888-8888-8888-8888-000000000001', 'Dr. Anna Lindberg',  'SE-123456',
   '44444444-4444-4444-4444-000000000001', 12,  500.00, '+46 70 222 0001', 'a.lindberg@stockholmhealthcenter.se',
   'General practitioner with twelve years of community medicine experience.', true,  true, '2025-09-01 09:00:00+02'),
  ('88888888-8888-8888-8888-000000000002', 'Dr. Karim Hassan',   'SE-123457',
   '44444444-4444-4444-4444-000000000002', 18, 1200.00, '+46 70 222 0002', 'k.hassan@stockholmhealthcenter.se',
   'Consultant cardiologist, focuses on hypertension and heart-failure care.', true,  true, '2025-09-01 09:00:00+02'),
  ('88888888-8888-8888-8888-000000000003', 'Dr. Elin Bergström', 'SE-123458',
   '44444444-4444-4444-4444-000000000003',  9,  700.00, '+46 70 222 0003', 'e.bergstrom@stockholmhealthcenter.se',
   'Pediatrician with a special interest in childhood asthma and allergies.', true,  true, '2025-09-01 09:00:00+02'),
  ('88888888-8888-8888-8888-000000000004', 'Dr. Yusuf Ahmadi',   'SE-123459',
   '44444444-4444-4444-4444-000000000004', 15,  900.00, '+46 70 222 0004', 'y.ahmadi@stockholmhealthcenter.se',
   'Dermatologist; skin-cancer screening and minor surgical procedures.', false, true, '2025-09-01 09:00:00+02'),
  ('88888888-8888-8888-8888-000000000005', 'Dr. Magnus Holmberg','SE-123460',
   '44444444-4444-4444-4444-000000000005', 22, 1800.00, '+46 70 222 0005', 'm.holmberg@stockholmhealthcenter.se',
   'Internal medicine consultant; diabetes and metabolic health.', true,  true, '2025-09-01 09:00:00+02');

-- -----------------------------------------------------------------------------
-- Scheduling: weekly schedules + time off
-- -----------------------------------------------------------------------------
--
-- weekday: 0 = Sunday, 6 = Saturday (Postgres `extract(dow ...)` convention).
-- Doctors work mornings 09:00-13:00 + afternoons 14:00-17:00 with a lunch
-- break. Spread across both branches.

INSERT INTO doctor_schedules (id, doctor_id, branch_id, weekday, start_time, end_time, slot_minutes) VALUES
  -- Dr. Lindberg (GP): Mon–Thu at Östermalm
  ('aaaaaaaa-aaaa-aaaa-aaaa-000000000001', '88888888-8888-8888-8888-000000000001', '22222222-2222-2222-2222-000000000001', 1, '09:00', '13:00', 30),
  ('aaaaaaaa-aaaa-aaaa-aaaa-000000000002', '88888888-8888-8888-8888-000000000001', '22222222-2222-2222-2222-000000000001', 1, '14:00', '17:00', 30),
  ('aaaaaaaa-aaaa-aaaa-aaaa-000000000003', '88888888-8888-8888-8888-000000000001', '22222222-2222-2222-2222-000000000001', 2, '09:00', '13:00', 30),
  ('aaaaaaaa-aaaa-aaaa-aaaa-000000000004', '88888888-8888-8888-8888-000000000001', '22222222-2222-2222-2222-000000000001', 3, '09:00', '13:00', 30),
  ('aaaaaaaa-aaaa-aaaa-aaaa-000000000005', '88888888-8888-8888-8888-000000000001', '22222222-2222-2222-2222-000000000001', 4, '09:00', '13:00', 30),
  -- Dr. Hassan (Cardiology): Tue + Thu at Östermalm
  ('aaaaaaaa-aaaa-aaaa-aaaa-000000000006', '88888888-8888-8888-8888-000000000002', '22222222-2222-2222-2222-000000000001', 2, '09:00', '13:00', 45),
  ('aaaaaaaa-aaaa-aaaa-aaaa-000000000007', '88888888-8888-8888-8888-000000000002', '22222222-2222-2222-2222-000000000001', 4, '14:00', '17:00', 45),
  -- Dr. Bergström (Peds): Mon, Wed, Fri at Söder
  ('aaaaaaaa-aaaa-aaaa-aaaa-000000000008', '88888888-8888-8888-8888-000000000003', '22222222-2222-2222-2222-000000000002', 1, '09:00', '13:00', 30),
  ('aaaaaaaa-aaaa-aaaa-aaaa-000000000009', '88888888-8888-8888-8888-000000000003', '22222222-2222-2222-2222-000000000002', 3, '09:00', '13:00', 30),
  ('aaaaaaaa-aaaa-aaaa-aaaa-00000000000a', '88888888-8888-8888-8888-000000000003', '22222222-2222-2222-2222-000000000002', 5, '14:00', '17:00', 30),
  -- Dr. Ahmadi (Derm): Wed + Fri at Östermalm
  ('aaaaaaaa-aaaa-aaaa-aaaa-00000000000b', '88888888-8888-8888-8888-000000000004', '22222222-2222-2222-2222-000000000001', 3, '14:00', '17:00', 30),
  ('aaaaaaaa-aaaa-aaaa-aaaa-00000000000c', '88888888-8888-8888-8888-000000000004', '22222222-2222-2222-2222-000000000001', 5, '09:00', '13:00', 30),
  -- Dr. Holmberg (IM): Mon + Thu at Söder
  ('aaaaaaaa-aaaa-aaaa-aaaa-00000000000d', '88888888-8888-8888-8888-000000000005', '22222222-2222-2222-2222-000000000002', 1, '09:00', '13:00', 60),
  ('aaaaaaaa-aaaa-aaaa-aaaa-00000000000e', '88888888-8888-8888-8888-000000000005', '22222222-2222-2222-2222-000000000002', 4, '09:00', '13:00', 60);

-- Dr. Hassan vacation: 2026-06-01 → 2026-06-14
INSERT INTO doctor_time_off (id, doctor_id, starts_at, ends_at, reason, status, created_at) VALUES
  ('bbbbbbbb-bbbb-bbbb-bbbb-000000000001',
   '88888888-8888-8888-8888-000000000002',
   '2026-06-01 00:00:00+02',
   '2026-06-15 00:00:00+02',
   'Annual leave',
   'approved',
   '2026-04-15 09:00:00+02');

-- -----------------------------------------------------------------------------
-- Appointments — 12 rows, mixed statuses, all respect no_doctor_overlap.
-- -----------------------------------------------------------------------------
--
-- Same doctor never has overlapping non-cancelled slots. Slot widths chosen
-- to match each doctor's `slot_minutes` for realism.

INSERT INTO appointments (id, appointment_number, patient_id, doctor_id, branch_id, room_id, service_id,
                          scheduled_start, scheduled_end, visit_type, status,
                          chief_complaint, notes, checked_in_at, completed_at,
                          booking_channel, cancellation_reason, created_at, updated_at) VALUES

  -- COMPLETED — 2026-05-04 (past), Anna Lindqvist with Dr. Lindberg (GP)
  ('99999999-9999-9999-9999-000000000001', 'APT-2026-0001',
   '77777777-7777-7777-7777-000000000001', '88888888-8888-8888-8888-000000000001', '22222222-2222-2222-2222-000000000001',
   '33333333-3333-3333-3333-000000000001', '55555555-5555-5555-5555-000000000001',
   '2026-05-04 09:00:00+02', '2026-05-04 09:30:00+02', 'in_person', 'completed',
   'Sore throat, persistent cough for 5 days.', 'Patient afebrile; throat erythematous.',
   '2026-05-04 08:55:00+02', '2026-05-04 09:28:00+02',
   'phone', NULL, '2026-04-30 11:00:00+02', '2026-05-04 09:28:00+02'),

  -- COMPLETED — 2026-05-11 (past), Erik Johansson with Dr. Hassan (Cardiology)
  ('99999999-9999-9999-9999-000000000002', 'APT-2026-0002',
   '77777777-7777-7777-7777-000000000002', '88888888-8888-8888-8888-000000000002', '22222222-2222-2222-2222-000000000001',
   '33333333-3333-3333-3333-000000000002', '55555555-5555-5555-5555-000000000002',
   '2026-05-12 10:00:00+02', '2026-05-12 10:45:00+02', 'in_person', 'completed',
   'Hypertension follow-up; medication review.', 'BP elevated; medication titrated.',
   '2026-05-12 09:55:00+02', '2026-05-12 10:42:00+02',
   'referral', NULL, '2026-04-20 14:00:00+02', '2026-05-12 10:42:00+02'),

  -- COMPLETED — 2026-05-15 (past), Omar Saleh with Dr. Holmberg (IM, annual check)
  ('99999999-9999-9999-9999-000000000003', 'APT-2026-0003',
   '77777777-7777-7777-7777-000000000006', '88888888-8888-8888-8888-000000000005', '22222222-2222-2222-2222-000000000002',
   '33333333-3333-3333-3333-000000000004', '55555555-5555-5555-5555-000000000008',
   '2026-05-14 09:00:00+02', '2026-05-14 10:00:00+02', 'in_person', 'completed',
   'Annual health check; diabetes monitoring.', 'HbA1c stable; renal function normal.',
   '2026-05-14 08:50:00+02', '2026-05-14 09:58:00+02',
   'web', NULL, '2026-04-25 10:00:00+02', '2026-05-14 09:58:00+02'),

  -- CONFIRMED — Tue 2026-05-26, Fatima Karim (35y adult) with Dr. Lindberg (GP)
  -- for her asthma review. Originally seeded against Dr. Bergström (Peds)
  -- which was a clinical mismatch — a 35-year-old wouldn't normally see a
  -- paediatrician. Now routed to GP, the correct primary-care path for
  -- routine adult asthma management. The 09:30 slot fits Dr. Lindberg's
  -- Tuesday 09:00–13:00 schedule and doesn't overlap her existing
  -- 10:00–10:30 slot with Hassan Al-Mahmoud.
  ('99999999-9999-9999-9999-000000000004', 'APT-2026-0004',
   '77777777-7777-7777-7777-000000000003', '88888888-8888-8888-8888-000000000001', '22222222-2222-2222-2222-000000000001',
   '33333333-3333-3333-3333-000000000001', '55555555-5555-5555-5555-000000000001',
   '2026-05-26 09:30:00+02', '2026-05-26 10:00:00+02', 'in_person', 'confirmed',
   'Asthma review; needs new prescription.', NULL,
   NULL, NULL,
   'mobile_app', NULL, '2026-05-15 11:00:00+02', '2026-05-21 09:00:00+02'),

  -- CHECKED_IN — today, Lukas Bergman with Dr. Bergström (Peds vaccination)
  ('99999999-9999-9999-9999-000000000005', 'APT-2026-0005',
   '77777777-7777-7777-7777-000000000004', '88888888-8888-8888-8888-000000000003', '22222222-2222-2222-2222-000000000002',
   '33333333-3333-3333-3333-000000000005', '55555555-5555-5555-5555-000000000007',
   '2026-05-22 10:30:00+02', '2026-05-22 10:45:00+02', 'in_person', 'checked_in',
   'MMR booster vaccination.', NULL,
   '2026-05-22 10:25:00+02', NULL,
   'phone', NULL, '2026-05-15 12:00:00+02', '2026-05-22 10:25:00+02'),

  -- CONFIRMED — this week, Sofia Andersson with Dr. Lindberg (GP)
  ('99999999-9999-9999-9999-000000000006', 'APT-2026-0006',
   '77777777-7777-7777-7777-000000000005', '88888888-8888-8888-8888-000000000001', '22222222-2222-2222-2222-000000000001',
   '33333333-3333-3333-3333-000000000001', '55555555-5555-5555-5555-000000000001',
   '2026-05-25 09:30:00+02', '2026-05-25 10:00:00+02', 'in_person', 'confirmed',
   'Recurrent migraine; refill request.', NULL,
   NULL, NULL,
   'web', NULL, '2026-05-18 09:00:00+02', '2026-05-20 16:00:00+02'),

  -- CONFIRMED — this week, Hassan Al-Mahmoud with Dr. Lindberg (GP)
  ('99999999-9999-9999-9999-000000000007', 'APT-2026-0007',
   '77777777-7777-7777-7777-000000000008', '88888888-8888-8888-8888-000000000001', '22222222-2222-2222-2222-000000000001',
   '33333333-3333-3333-3333-000000000001', '55555555-5555-5555-5555-000000000001',
   '2026-05-26 10:00:00+02', '2026-05-26 10:30:00+02', 'in_person', 'confirmed',
   'Annual physical examination.', NULL,
   NULL, NULL,
   'mobile_app', NULL, '2026-05-19 14:00:00+02', '2026-05-21 10:00:00+02'),

  -- CONFIRMED — this week, Maja Karlsson with Dr. Bergström (Peds)
  ('99999999-9999-9999-9999-000000000008', 'APT-2026-0008',
   '77777777-7777-7777-7777-000000000007', '88888888-8888-8888-8888-000000000003', '22222222-2222-2222-2222-000000000002',
   '33333333-3333-3333-3333-000000000005', '55555555-5555-5555-5555-000000000004',
   '2026-05-27 09:30:00+02', '2026-05-27 10:00:00+02', 'in_person', 'confirmed',
   'Recurrent abdominal pain.', NULL,
   NULL, NULL,
   'phone', NULL, '2026-05-18 11:00:00+02', '2026-05-20 12:00:00+02'),

  -- BOOKED — next week, Anna Lindqvist with Dr. Ahmadi (Derm)
  ('99999999-9999-9999-9999-000000000009', 'APT-2026-0009',
   '77777777-7777-7777-7777-000000000001', '88888888-8888-8888-8888-000000000004', '22222222-2222-2222-2222-000000000001',
   '33333333-3333-3333-3333-000000000003', '55555555-5555-5555-5555-000000000005',
   '2026-06-03 14:30:00+02', '2026-06-03 15:00:00+02', 'in_person', 'booked',
   'Persistent skin rash on forearm.', NULL,
   NULL, NULL,
   'web', NULL, '2026-05-21 09:00:00+02', '2026-05-21 09:00:00+02'),

  -- BOOKED — next week, Erik Johansson with Dr. Holmberg (IM)
  ('99999999-9999-9999-9999-00000000000a', 'APT-2026-0010',
   '77777777-7777-7777-7777-000000000002', '88888888-8888-8888-8888-000000000005', '22222222-2222-2222-2222-000000000002',
   '33333333-3333-3333-3333-000000000004', '55555555-5555-5555-5555-000000000008',
   '2026-06-04 09:00:00+02', '2026-06-04 10:00:00+02', 'in_person', 'booked',
   'Lipid panel review.', NULL,
   NULL, NULL,
   'web', NULL, '2026-05-21 10:00:00+02', '2026-05-21 10:00:00+02'),

  -- CANCELLED — Sofia Andersson cancelled last week
  ('99999999-9999-9999-9999-00000000000b', 'APT-2026-0011',
   '77777777-7777-7777-7777-000000000005', '88888888-8888-8888-8888-000000000001', '22222222-2222-2222-2222-000000000001',
   '33333333-3333-3333-3333-000000000001', '55555555-5555-5555-5555-000000000001',
   '2026-05-19 09:30:00+02', '2026-05-19 10:00:00+02', 'in_person', 'cancelled',
   'GP follow-up.', NULL,
   NULL, NULL,
   'web', 'Patient ill and rescheduled.', '2026-05-12 09:00:00+02', '2026-05-18 16:00:00+02'),

  -- NO_SHOW — Hassan Al-Mahmoud missed his vaccination
  ('99999999-9999-9999-9999-00000000000c', 'APT-2026-0012',
   '77777777-7777-7777-7777-000000000008', '88888888-8888-8888-8888-000000000003', '22222222-2222-2222-2222-000000000002',
   '33333333-3333-3333-3333-000000000005', '55555555-5555-5555-5555-000000000007',
   '2026-05-18 09:00:00+02', '2026-05-18 09:15:00+02', 'in_person', 'no_show',
   'Flu vaccination.', NULL,
   NULL, NULL,
   'phone', NULL, '2026-05-10 11:00:00+02', '2026-05-18 09:30:00+02');

-- -----------------------------------------------------------------------------
-- Appointment status history — transition trail for completed appointments
-- -----------------------------------------------------------------------------

INSERT INTO appointment_status_history (id, appointment_id, from_status, to_status, changed_by, reason, changed_at) VALUES
  ('cccccccc-cccc-cccc-cccc-000000000001', '99999999-9999-9999-9999-000000000001', NULL,         'booked',      NULL, 'Initial booking',  '2026-04-30 11:00:00+02'),
  ('cccccccc-cccc-cccc-cccc-000000000002', '99999999-9999-9999-9999-000000000001', 'booked',     'confirmed',   NULL, 'Confirmed by SMS', '2026-05-03 09:00:00+02'),
  ('cccccccc-cccc-cccc-cccc-000000000003', '99999999-9999-9999-9999-000000000001', 'confirmed',  'checked_in',  NULL, 'Walked in',        '2026-05-04 08:55:00+02'),
  ('cccccccc-cccc-cccc-cccc-000000000004', '99999999-9999-9999-9999-000000000001', 'checked_in', 'in_progress', NULL, 'Doctor seeing',    '2026-05-04 09:00:00+02'),
  ('cccccccc-cccc-cccc-cccc-000000000005', '99999999-9999-9999-9999-000000000001', 'in_progress','completed',   NULL, 'Visit complete',   '2026-05-04 09:28:00+02');

-- -----------------------------------------------------------------------------
-- Appointment reminders — pending sends for confirmed future appointments
-- -----------------------------------------------------------------------------

INSERT INTO appointment_reminders (id, appointment_id, channel, send_at, status, sent_at) VALUES
  ('dddddddd-dddd-dddd-dddd-000000000001', '99999999-9999-9999-9999-000000000006', 'sms',   '2026-05-24 09:30:00+02', 'pending', NULL),
  ('dddddddd-dddd-dddd-dddd-000000000002', '99999999-9999-9999-9999-000000000007', 'sms',   '2026-05-25 10:00:00+02', 'pending', NULL),
  ('dddddddd-dddd-dddd-dddd-000000000003', '99999999-9999-9999-9999-000000000008', 'email', '2026-05-26 09:30:00+02', 'pending', NULL),
  ('dddddddd-dddd-dddd-dddd-000000000004', '99999999-9999-9999-9999-000000000001', 'sms',   '2026-05-03 09:00:00+02', 'sent',    '2026-05-03 09:00:12+02');

-- -----------------------------------------------------------------------------
-- Clinical: medical records + diagnoses + prescriptions + lab orders
-- -----------------------------------------------------------------------------
--
-- One record per completed appointment. Diagnoses use real ICD-10 codes;
-- prescriptions tie back to the medications table.

INSERT INTO medical_records (id, appointment_id, patient_id, doctor_id,
                             subjective, objective, assessment, plan,
                             bp_systolic, bp_diastolic, temperature, pulse_rate,
                             weight_kg, height_cm, recorded_at) VALUES
  -- Anna's URI visit
  ('eeeeeeee-eeee-eeee-eeee-000000000001',
   '99999999-9999-9999-9999-000000000001',
   '77777777-7777-7777-7777-000000000001',
   '88888888-8888-8888-8888-000000000001',
   'Sore throat × 5 days, dry cough, mild fatigue. No fever at home.',
   'Throat: erythema, no exudate. Cervical lymph nodes: mildly tender. Lungs clear.',
   'Acute upper respiratory infection, likely viral.',
   'Symptomatic care; paracetamol PRN; return if symptoms persist > 7 days or fever develops.',
   118, 78, 36.70, 72, 62.50, 168.00,
   '2026-05-04 09:25:00+02'),
  -- Erik's hypertension follow-up
  ('eeeeeeee-eeee-eeee-eeee-000000000002',
   '99999999-9999-9999-9999-000000000002',
   '77777777-7777-7777-7777-000000000002',
   '88888888-8888-8888-8888-000000000002',
   'Hypertension diagnosed 6 months ago. Current lisinopril 10 mg. Some headaches.',
   'BP 158/96 mmHg (repeat 154/94). HR 78. No peripheral oedema.',
   'Essential hypertension, not fully controlled on current dose.',
   'Increase lisinopril to 20 mg daily. Home BP monitoring. Review in 6 weeks.',
   158, 96, 36.50, 78, 88.40, 178.00,
   '2026-05-12 10:40:00+02'),
  -- Omar's diabetes annual check
  ('eeeeeeee-eeee-eeee-eeee-000000000003',
   '99999999-9999-9999-9999-000000000003',
   '77777777-7777-7777-7777-000000000006',
   '88888888-8888-8888-8888-000000000005',
   'Type 2 diabetes, on metformin. Feels well. No hypoglycaemic episodes.',
   'BP 142/86. HbA1c last week 58 mmol/mol (target < 53). BMI 28.2.',
   'Type 2 diabetes — modest deterioration of glycaemic control. Hypertension on borderline control.',
   'Lifestyle reinforcement. Add lisinopril 10 mg for BP. Repeat HbA1c in 3 months.',
   142, 86, 36.60, 74, 84.20, 172.00,
   '2026-05-14 09:55:00+02');

INSERT INTO diagnoses (id, medical_record_id, icd10_code, description, is_primary) VALUES
  ('ffffffff-ffff-ffff-ffff-000000000001', 'eeeeeeee-eeee-eeee-eeee-000000000001', 'J06.9', 'Acute upper respiratory infection, unspecified', true),
  ('ffffffff-ffff-ffff-ffff-000000000002', 'eeeeeeee-eeee-eeee-eeee-000000000002', 'I10',   'Essential (primary) hypertension',                true),
  ('ffffffff-ffff-ffff-ffff-000000000003', 'eeeeeeee-eeee-eeee-eeee-000000000003', 'E11.9', 'Type 2 diabetes mellitus without complications',  true),
  ('ffffffff-ffff-ffff-ffff-000000000004', 'eeeeeeee-eeee-eeee-eeee-000000000003', 'I10',   'Essential (primary) hypertension',                false),
  ('ffffffff-ffff-ffff-ffff-000000000005', 'eeeeeeee-eeee-eeee-eeee-000000000001', 'R05',   'Cough',                                           false);

INSERT INTO prescriptions (id, medical_record_id, prescribed_by, medication_id,
                           dosage, frequency, duration_days, instructions,
                           is_dispensed, issued_at) VALUES
  ('a1a1a1a1-a1a1-a1a1-a1a1-000000000001', 'eeeeeeee-eeee-eeee-eeee-000000000001',
   '88888888-8888-8888-8888-000000000001', '66666666-6666-6666-6666-000000000001',
   '1 tablet', 'Every 6 hours as needed', 5, 'Take with water. Maximum 4 tablets per 24 hours.', true, '2026-05-04 09:28:00+02'),
  ('a1a1a1a1-a1a1-a1a1-a1a1-000000000002', 'eeeeeeee-eeee-eeee-eeee-000000000002',
   '88888888-8888-8888-8888-000000000002', '66666666-6666-6666-6666-000000000005',
   '20 mg',    'Once daily, morning',     90, 'Continue indefinitely. Monitor BP weekly.',          true, '2026-05-12 10:42:00+02'),
  ('a1a1a1a1-a1a1-a1a1-a1a1-000000000003', 'eeeeeeee-eeee-eeee-eeee-000000000003',
   '88888888-8888-8888-8888-000000000005', '66666666-6666-6666-6666-000000000004',
   '500 mg',   'Twice daily with meals',  90, 'Continue current dose; review with HbA1c at 3 months.', true, '2026-05-14 09:58:00+02'),
  ('a1a1a1a1-a1a1-a1a1-a1a1-000000000004', 'eeeeeeee-eeee-eeee-eeee-000000000003',
   '88888888-8888-8888-8888-000000000005', '66666666-6666-6666-6666-000000000005',
   '10 mg',    'Once daily, morning',     90, 'New medication. Monitor for cough or dizziness.',     false, '2026-05-14 09:58:00+02');

INSERT INTO lab_orders (id, medical_record_id, test_code, test_name, status,
                        result_value, reference_range, ordered_at, resulted_at) VALUES
  ('a2a2a2a2-a2a2-a2a2-a2a2-000000000001', 'eeeeeeee-eeee-eeee-eeee-000000000002',
   'LIPID',    'Lipid panel',                'completed',
   'Total chol 5.8 mmol/L, LDL 3.7, HDL 1.2', '< 5.0 / < 3.0 / > 1.0',
   '2026-05-12 10:42:00+02', '2026-05-14 11:00:00+02'),
  ('a2a2a2a2-a2a2-a2a2-a2a2-000000000002', 'eeeeeeee-eeee-eeee-eeee-000000000003',
   'HBA1C',    'Glycated haemoglobin (HbA1c)','completed',
   '58 mmol/mol',                              '< 53 mmol/mol',
   '2026-05-14 09:58:00+02', '2026-05-15 09:00:00+02'),
  ('a2a2a2a2-a2a2-a2a2-a2a2-000000000003', 'eeeeeeee-eeee-eeee-eeee-000000000003',
   'CREAT',    'Serum creatinine',           'processing',
   NULL,                                       '60-110 µmol/L',
   '2026-05-14 09:58:00+02', NULL);

-- -----------------------------------------------------------------------------
-- Billing: insurance policies + invoices + payments
-- -----------------------------------------------------------------------------

INSERT INTO insurance_policies (id, patient_id, provider_name, policy_number, holder_name,
                                coverage_percent, valid_from, valid_to) VALUES
  ('b1b1b1b1-b1b1-b1b1-b1b1-000000000001',
   '77777777-7777-7777-7777-000000000002', 'Folksam',         'FK-2025-0042', 'Erik Johansson',
   80.00, '2025-01-01', '2026-12-31'),
  ('b1b1b1b1-b1b1-b1b1-b1b1-000000000002',
   '77777777-7777-7777-7777-000000000006', 'If Försäkring',   'IF-2025-1187', 'Omar Saleh',
   70.00, '2025-06-01', '2026-12-31');

INSERT INTO invoices (id, invoice_number, patient_id, appointment_id, insurance_policy_id,
                      subtotal, discount_amount, tax_amount, total_amount, amount_paid,
                      status, due_date, issued_at, created_at) VALUES
  -- Anna's GP visit — paid in full (no insurance)
  ('b2b2b2b2-b2b2-b2b2-b2b2-000000000001', 'INV-2026-0001',
   '77777777-7777-7777-7777-000000000001', '99999999-9999-9999-9999-000000000001', NULL,
    500.00,  0.00,  0.00,  500.00,  500.00, 'paid',    '2026-05-18',
    '2026-05-04 10:00:00+02', '2026-05-04 09:30:00+02'),

  -- Erik's cardiology — partial (insurance covers 80%, patient pays remainder; here patient half-paid)
  ('b2b2b2b2-b2b2-b2b2-b2b2-000000000002', 'INV-2026-0002',
   '77777777-7777-7777-7777-000000000002', '99999999-9999-9999-9999-000000000002', 'b1b1b1b1-b1b1-b1b1-b1b1-000000000001',
   1200.00,  0.00,  0.00, 1200.00,  240.00, 'partial', '2026-05-26',
   '2026-05-12 11:00:00+02', '2026-05-12 10:45:00+02'),

  -- Omar's annual check — issued, awaiting first payment
  ('b2b2b2b2-b2b2-b2b2-b2b2-000000000003', 'INV-2026-0003',
   '77777777-7777-7777-7777-000000000006', '99999999-9999-9999-9999-000000000003', 'b1b1b1b1-b1b1-b1b1-b1b1-000000000002',
   1800.00,  0.00,  0.00, 1800.00,    0.00, 'issued',  '2026-05-28',
   '2026-05-14 10:30:00+02', '2026-05-14 10:00:00+02');

INSERT INTO payments (id, invoice_id, amount, payment_method, transaction_ref, status, paid_at) VALUES
  -- Anna paid by card same day
  ('b3b3b3b3-b3b3-b3b3-b3b3-000000000001',
   'b2b2b2b2-b2b2-b2b2-b2b2-000000000001',  500.00, 'card', 'TXN-CARD-58271', 'completed', '2026-05-04 09:35:00+02'),
  -- Erik mobile-pay copay
  ('b3b3b3b3-b3b3-b3b3-b3b3-000000000002',
   'b2b2b2b2-b2b2-b2b2-b2b2-000000000002',  240.00, 'mobile_pay', 'SWISH-92281', 'completed', '2026-05-13 14:20:00+02');

COMMIT;
