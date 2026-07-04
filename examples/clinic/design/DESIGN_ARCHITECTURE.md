---
artifact: DESIGN_ARCHITECTURE
layer: what
status: active
source: reverse-engineered from examples/clinic (read-only analysis — no redesign)
updated: 2026-06-07
---

# Design Architecture — WHAT (Clinic)

> The structure of the system as built: entities, relationships, screens, and the
> information hierarchy a design must respect. Extracted from the models,
> migrations, and ModelAdmin config — not invented.

## Information Architecture

**The Patient is the spine.** Every other record exists only in relation to a
patient. Three business capabilities (Rust crates) group the four tables:

- **Patients** — `Patient`, `Vitals` (clinical records of a person)
- **Scheduling** — `Appointment` (booked visits)
- **Billing** — `Invoice` (money owed)

```text
Patient (hub)
  ├─ Vitals        (patient_id → patients.id, ON DELETE CASCADE)
  ├─ Appointment   (patient_id → patients.id, ON DELETE CASCADE)
  └─ Invoice       (patient_id → patients.id, ON DELETE CASCADE)
```

**Priority order (by daily use + business weight):** Patients first (the front
door), then Appointments (the day's work), then Invoices (the money), with Vitals
as a per-patient clinical sub-record.

## Primary Entities (exact, from code + schema)

### Patient — `patients` (the only searchable model)
| Field | Type | Notes |
|---|---|---|
| id | i64 (BIGSERIAL) | PK |
| full_name | TEXT NOT NULL | the human key |
| email | TEXT NOT NULL ('') | PII; no format validation |
| phone | TEXT NOT NULL ('') | PII; no format validation |
| created_at | TIMESTAMPTZ now() | |
- **List:** id, full_name, email, phone, created_at · **Search (FTS):** full_name,
  email, phone (Postgres `tsvector`, GIN-indexed) · **Order:** newest first.
- The **only** model with search enabled.

### Vitals — `vitals` (clinical measurements)
| Field | Type | Notes |
|---|---|---|
| id | i64 | PK |
| patient_id | i64 FK→patients (CASCADE) | shown as a raw number, not a name |
| heart_rate | i64 (default 0) | |
| notes | TEXT | free text |
| created_at | TIMESTAMPTZ now() | |
- **List:** id, patient_id, heart_rate, notes, created_at · **Filter:** patient_id
  · **Order:** newest first.

### Appointment — `appointments` (booked visits)
| Field | Type | Notes |
|---|---|---|
| id | i64 | PK |
| patient_id | i64 FK→patients (CASCADE) | |
| scheduled_at | TIMESTAMPTZ NOT NULL | **no default — must be set** |
| reason | TEXT | searchable |
| status | TEXT (default 'scheduled') | free text: scheduled / completed / cancelled |
| created_at | TIMESTAMPTZ now() | |
- **List:** id, patient_id, scheduled_at, reason, status · **Filter:** status ·
  **Search:** reason · **Order:** scheduled_at descending.

### Invoice — `invoices` (money owed)
| Field | Type | Notes |
|---|---|---|
| id | i64 | PK |
| patient_id | i64 FK→patients (CASCADE) | |
| amount_cents | i64 (default 0) | **whole cents** — format for display |
| status | TEXT (default 'unpaid') | free text: unpaid / paid |
| created_at | TIMESTAMPTZ now() | |
- **List:** id, patient_id, amount_cents, status, created_at · **Filter:** status ·
  **Order:** newest first.

## Main Workflows

1. **Register a patient** — create `Patient` (name, email, phone).
2. **Find a patient** — full-text search the patient list (the primary entry point).
3. **Record vitals** — add `Vitals` for a patient (heart_rate + notes).
4. **Book & run appointments** — create `Appointment` (patient, time, reason);
   move `status` scheduled → completed / cancelled; work the list filtered by status.
5. **Bill & collect** — raise `Invoice` (amount in cents); move `status`
   unpaid → paid; work the list filtered by status.
6. **Administer** — manage users/groups/permissions; review the audit history.

## Screens That Matter Most (ranked)

1. **Patient list** — the front door; the only FTS surface; newest-first. Most used.
2. **Patient record (detail/edit)** — identity + the gateway to that patient's
   vitals, appointments, and invoices.
3. **Appointments list** — status-filtered, time-ordered: the day's operational view.
4. **Invoices list** — status-filtered: the money view; amounts need humanising.
5. **Vitals list** — reached per-patient (filter by patient_id).
6. **Auth & audit** — Users / Groups / History (administrator surface).

## Information Hierarchy (what the eye needs)

- **Identity leads:** a record's human key is the patient's *name*; contact
  (email/phone) is secondary; timestamps are tertiary.
- **Status is the primary scannable signal** on appointments and invoices —
  it drives the list filters and should read at a glance.
- **The foreign key is a number, not a name.** `patient_id` is a bare `i64` on
  every child list/detail (relation widgets are an opt-in the models don't take).
  The patient's name — the natural human anchor — is not surfaced on child records.
  (Stated as an observed information-hierarchy gap, not a redesign.)
- **Money is stored as integer cents** and currently displays as raw cents.

## Resolved Context (gap-closing pass — 2026-06-07)

Clarifications that earlier blocked confident design (facts from source + approved
decisions; see `DESIGN_REASONING.md` / `DESIGN_DECISIONS.md`):

- **Detail composition (OBS-08):** the patient record page is the patient's *own
  fields*; vitals/appointments/invoices are reached via their own lists filtered by
  patient — not aggregated inline. Child create/edit shows `patient_id` as a plain
  number (no picker). The biggest legibility opportunity.
- **No provider entity (OBS-09):** appointments are clinic-level; design cannot show
  "which clinician." Single-provider is assumed (D-04).
- **Permissions (OBS-10 → D-01):** per-model perms are seeded; assignment is the
  administrator's. **Design per-role scope:** Reception → patients + scheduling;
  Clinical → patients + vitals + scheduling; Billing → invoices + patient (read);
  Administrator → all + user/group management.
- **Status lifecycle (D-02):** Appointment scheduled → completed/cancelled, with
  reschedule; Invoice unpaid → paid, with paid → unpaid for corrections. Present
  known states; allow reversal; tolerate unexpected free-text values.
- **Money (D-03):** format as USD/en-US `$1,234.56` — tabular, right-aligned.
- **Deployment (D-04):** desktop-first, single-site, single-provider, modest scale;
  optimise for desktop density + keyboard. Tablet/multi-provider are future needs.
