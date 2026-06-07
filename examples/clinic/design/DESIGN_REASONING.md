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
  emerald accent `#059669` set in `main.rs` (and, since D-05, in the rustio-design
  `[colors]` spec — formerly `static/tokens.css`).
- **Rationale:** security and theming are the framework's contract, not the
  clinic's to reinvent; the clinic only chooses a brand colour.
- **Design implication:** design within the token system and light-only palette;
  assume authority gating and an audit trail exist; do not invent a theming layer.

## OBS-08 · Detail = the record's own fields; children via filtered lists; FK is a number

- **Status:** accepted (observed)
- **Evidence:** the framework supports a relation widget (`AdminRelation`), but the
  clinic's models opt out (`relation: None`; vitals.rs: "relation widgets are an
  opt-in"). No inline configuration. `Vitals` carries `list_filter = [patient_id]`.
- **Design implication:** the patient detail page shows the **patient's own fields
  only** — it does *not* aggregate vitals/appointments/invoices inline today; those
  are reached via each model's list filtered by patient. On a child create/edit,
  `patient_id` is a **plain number input** (no patient picker). This is the largest
  legibility gap a design will feel; closing it is a design exploration, not a fact.

## OBS-09 · No provider / clinician / room entity

- **Status:** accepted (observed)
- **Evidence:** the four entities are Patient, Vitals, Appointment, Invoice. An
  `Appointment` is `patient + scheduled_at + reason + status` — there is no provider
  or room foreign key.
- **Design implication:** scheduling is **clinic-level, not provider-level**; a
  design cannot show "which clinician" because the data does not exist. Treated as
  intentional for a single-provider reference (see D-04).

## OBS-10 · Permissions are seeded per-model; assignment is operator policy

- **Status:** accepted (observed)
- **Evidence:** `seed_permissions` creates `{app}.{add,change,delete,view}_{singular}`
  for every model; it does **not** bind them to personas — an administrator assigns
  them via groups/users.
- **Design implication:** "who sees what" is a deployment policy, not a code fact —
  which is why it had to be decided for design purposes (see D-01).

---

# Design proposal — v1 direction (2026-06-07)

## R-05 · Adopt the v1 design direction; implement Tier 1 only → D-05

- **Status:** accepted (reviewed)
- **What the context says:** the patient is the spine (OBS-01); search is the one
  primary nav surface (OBS-02); status is the headline scannable signal (OBS-04);
  the FK is a bare number and detail shows only own fields (OBS-05/08); the frame is
  light-only `--rio-*` tokens, no build step, desktop-first single-provider
  (OBS-07, D-04), with role-scoped access (D-01), forward+correcting status (D-02),
  and USD money (D-03).
- **Proposed direction (Claude Design v1):** a calm, desktop-dense clinical admin
  that makes "find a patient and act" effortless — search-first patient list;
  the patient record as the human anchor (children aggregated); status read at a
  glance via a chip language; money humanised to USD; deletes that name their
  cascade; PII de-emphasised; navigation grouped by capability and scoped per role.
- **Alternatives rejected:**
  - *Dashboard-first / KPI home* — the context optimises for lookup, not metrics;
    reporting is an explicit non-goal.
  - *Calendar-centric scheduling* — there is no provider/room/time-slot model
    (OBS-09); a calendar would imply data that does not exist.
  - *Card/gallery layouts* — desktop-dense tables fit operator scanning (D-04).
  - *Big-bang implementation (all tiers at once)* — would force model/framework
    changes before the safe, reversible visual layer is proven.
- **Why Tier 1 is the right first slice:** it lives entirely in the **existing safe
  design seam** — tokens + validated `custom_css` (served via `RUSTIO_TOKENS_CSS`)
  and the `[navigation]` layer (served via `RUSTIO_TEMPLATE_DIR`). It introduces
  **no new model semantics**, is drift-detected, fully reversible, and
  independently shippable — so the governance loop (Context → Reasoning → Approved
  Slice → Safe Implementation → Browser Review) can be proven before any
  model/framework work is risked.
- **Tier 1 scope (this pass implements only this):**
  - Navigation **grouping** by capability (Patients / Scheduling / Billing;
    Vitals hidden, reached via a patient).
  - **Search-first** emphasis on the patient list (`.rio-search-bar`).
  - **Status colour-coding** via cell-level CSS on existing markup
    (`td.rio-td--text[title="…"]` for the known status values).
  - **Emerald/slate** refinement within the existing token values.
- **Intentionally OUT of scope (now and for Tier 1):** provider/room/calendar,
  reporting, patient portal, dark mode, an aggregate patient profile, the patient
  picker, and relation-aware FK (name-anchor) rendering. No new model semantics.
- **Requires later model/framework support (discovered while scoping the seam):**
  - *True status **pills*** need a wrapping element → a per-model list template
    (Tier 3). Tier 1 ships cell-level colour-coding instead.
  - *USD money **formatting*** (cents → `$1,234.56`) cannot be done in CSS and has
    no per-field hook → needs a formatting hook / template (later). Tier 1 leaves
    the value as-is.
  - *Per-column **PII de-emphasis*** — email/phone share `rio-td--text` with no
    column hook → needs a per-column class/template (later). Deferred.
  - *Role-**scoped** navigation* — the nav layer emits one static sidebar; per-role
    visibility needs framework support (later). Tier 1 ships grouping only.
  - *Cascade-naming **delete** copy* — the framework already lists cascades on the
    delete-confirm page (RelationRegistry); bespoke copy needs a template override
    (later). Not shipped to avoid a fragile override.
  - *Patient anchor / picker / aggregate profile* — Tier 2 / Tier 3.

# Forward decisions — gap-closing pass (2026-06-07)

> These resolve the open questions the no-source validation surfaced, so Claude
> Design can proceed confidently. They are **design-memory decisions for a
> reference app — they do NOT change clinic code.** Approved by review. Each maps
> to a row in `DESIGN_DECISIONS.md`.

## R-01 · Role-scoped permission policy → D-01

- **Status:** accepted
- **Context:** OBS-10 — code seeds perms but assigns none; a designer needs to know
  who sees what.
- **Options:** role-scoped least-privilege · flat (all staff see all) · leave
  operator-configured.
- **Decision:** **role-scoped** — Reception: patients + scheduling · Clinical:
  patients + vitals + scheduling · Billing: invoices + patient (read) ·
  Administrator: everything + user/group management.
- **Rationale:** PHI and money warrant separation; realistic least privilege; gives
  each role a scoped navigation/landing.
- **Rejected because:** flat exposes sensitive data to all; operator-undecided
  blocks the design.
- **Design implication:** design **per-role views/nav scope** — not every user sees
  all four models; Reception lands on patients, Billing on invoices.

## R-02 · Status lifecycle: forward with corrections → D-02

- **Status:** accepted
- **Context:** OBS-04 — status is free-text with no enforced transitions.
- **Decision:** Appointment `scheduled → completed | cancelled`, **reschedule
  allowed** (back to scheduled / new time). Invoice `unpaid → paid`, with
  **`paid → unpaid` permitted** for corrections/refunds. Present the known states;
  allow reversal.
- **Rationale:** clinics reschedule and correct billing; a one-way model forces
  workarounds.
- **Rejected because:** strictly one-way has no correction path; fully unconstrained
  gives no affordance guidance.
- **Design implication:** status controls offer the known transitions including
  reversal, while still tolerating unexpected stored values (free-text).

## R-03 · Money display: USD, en-US → D-03

- **Status:** accepted
- **Context:** OBS-03 — amount is bare integer cents, no currency stored.
- **Decision:** format money as **$1,234.56 (USD, en-US)** everywhere it appears.
- **Rationale:** a concrete, legible default for the reference; raw cents must never
  be shown.
- **Rejected because:** EUR is not this reference; fully agnostic defers a needed
  concrete format.
- **Design implication:** a money-format slot (cents → USD), right-aligned and
  tabular for scanning.

## R-04 · Deployment posture: desktop-first, single-provider, modest scale → D-04

- **Status:** accepted
- **Context:** scale/device are not in the code; OBS-09 (no provider entity).
- **Decision:** design for a **single-site, single-provider** clinic; staff work at
  a **reception/back-office desktop**; hundreds–low-thousands of patients. Treat the
  absence of a provider entity as intentional here.
- **Rationale:** matches the modeled data and the back-office nature of the tool.
- **Rejected because:** tablet/exam-room responsive has no evidence and adds scope;
  unspecified blocks density decisions.
- **Design implication:** optimise for **desktop density and keyboard**;
  multi-provider and tablet use are explicit *future* needs, not designed for now.
