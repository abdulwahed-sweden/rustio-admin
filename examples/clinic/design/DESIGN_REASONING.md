---
artifact: DESIGN_REASONING
layer: reasoning
status: active
source: reverse-engineered from examples/clinic (read-only analysis — no redesign)
updated: 2026-06-07
---

# Design Reasoning — observed decisions (Clinic)

> **This documents reasoning already embodied in the system — reverse-engineered,
> not proposed.** No redesign. Each entry records a decision the code/schema makes,
> the rationale evidenced for it, and the *implication for any future design*. It
> exists so Claude Design inherits the system's intent and constraints instead of
> guessing at them. Status `accepted` = "this is how it is today."

## OBS-01 · Patient is the hub; everything cascades from it

- **Status:** accepted (observed)
- **Evidence:** `Vitals`, `Appointment`, `Invoice` each carry `patient_id`
  REFERENCES `patients(id)` **ON DELETE CASCADE** (migrations 0001–0003).
- **Rationale:** the clinic's whole information model is "a person and the things
  that happen to them." One root entity keeps the model coherent.
- **Design implication:** the patient record is the natural center of gravity —
  child records are most meaningful *in the context of a patient*. Deleting a
  patient destroys their entire clinical + financial history (no soft-delete);
  any delete affordance must treat this as high-consequence.

## OBS-02 · Search is opt-in; only Patient is searchable

- **Status:** accepted (observed)
- **Evidence:** only `Patient` implements `search_index_column()` → a generated
  Postgres `tsvector` over (full_name, email, phone), GIN-indexed; no other model
  enables search. Doctrine: "search is off until a model turns it on; never index
  a sensitive field."
- **Rationale:** fast, safe patient lookup is the primary task; indexing is a
  deliberate, auditable choice, not a default.
- **Design implication:** the patient list's search is *the* primary navigation
  affordance and should be prominent. Other lists rely on filters/ordering, not
  search. (Tension to respect, not fix: email/phone are PII yet are in the FTS
  vector — surfacing them in search results is an existing, intentional trade-off.)

## OBS-03 · Money is integer cents

- **Status:** accepted (observed)
- **Evidence:** `Invoice.amount_cents: i64`, comment "stored in whole cents to
  avoid floating-point money; format it for display in the UI layer."
- **Rationale:** correctness — no float rounding on money.
- **Design implication:** the UI is responsible for humanising money (e.g. cents →
  currency). Raw integer cents in a list is a presentation gap by design intent.

## OBS-04 · Status is free-text with safe defaults

- **Status:** accepted (observed)
- **Evidence:** `Appointment.status` TEXT default `'scheduled'`
  (scheduled/completed/cancelled); `Invoice.status` TEXT default `'unpaid'`
  (unpaid/paid). Both are `list_filter` fields and DB-indexed.
- **Rationale:** lightweight lifecycle without an enum migration; status is the
  main operational pivot, so it is filterable and indexed.
- **Design implication:** status is the headline scannable signal on these lists.
  Because it is free text (no DB enum), values are a convention, not a guarantee —
  a design should present the known states clearly while tolerating unexpected ones.

## OBS-05 · Capabilities are crates; the FK is a plain number

- **Status:** accepted (observed)
- **Evidence:** one crate per business capability (patients/scheduling/billing);
  `patient_id` is a bare `i64` column — "relation widgets are an opt-in the model
  declares explicitly when wanted," and these models do not.
- **Rationale:** structure tracks the business, not the schema; relations stay
  explicit and cheap.
- **Design implication:** today child records show `patient_id` as a number, not
  the patient's name. The information hierarchy *wants* a human anchor here; that
  the code leaves it numeric is an observed gap a design will feel immediately.

## OBS-06 · Newest-first, time-aware ordering

- **Status:** accepted (observed)
- **Evidence:** Patients/Vitals/Invoices order by `-created_at`; Appointments by
  `-scheduled_at`.
- **Rationale:** operators care about the most recent records and the upcoming/
  recent schedule.
- **Design implication:** recency is the default reading order; the schedule view
  is organised around appointment time, not record-creation time.

## OBS-07 · Inherited security & visual posture

- **Status:** accepted (observed)
- **Evidence:** framework 5-tier RBAC (Developer→…→User) + per-model permissions
  (`seed_permissions`) + audit-by-default; light-only theming via `--rio-*` tokens;
  emerald accent `#059669` set in `main.rs` and `static/tokens.css`.
- **Rationale:** security and theming are the framework's contract, not the
  clinic's to reinvent; the clinic only chooses a brand colour.
- **Design implication:** design within the token system and light-only palette;
  assume authority gating and an audit trail exist; do not invent a theming layer.
