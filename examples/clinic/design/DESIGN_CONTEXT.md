---
artifact: DESIGN_CONTEXT
layer: synthesis
status: active
source: reverse-engineered from examples/clinic (read-only analysis — no redesign)
updated: 2026-06-07
audience: Claude Design (assume it has NEVER seen the codebase)
---

# Design Context — Clinic

> A single, self-contained reduction of the clinic project into design terms.
> Read only this before proposing a design — it restates every fact you need, so
> you never have to open the source. **This describes the system as it is; it does
> not redesign it.**

---

## 1. What this system is (one paragraph)

The internal **back-office admin for an outpatient clinic**, built on
`rustio-admin` over Postgres. Clinic staff use it to manage **patients** and their
**vitals**, **schedule appointments**, and **bill invoices**. It is a
permission-gated, audited admin panel — not a patient-facing product. Four tables,
three business capabilities, one accent colour. The patient is the center of
everything.

## 2. Business domain

Outpatient clinical operations: people receiving care, the visits they book, the
measurements taken, and the money owed. Internal tool; PHI/PII and financial data.

## 3. User roles (inherited framework RBAC — no custom roles)

5-tier ladder, highest → lowest: **Developer → Administrator → Supervisor → Staff
→ User**, refined by **groups + per-model permissions** (view/add/change/delete).
Every authority change is **audited** (`/admin/history`, correlation IDs). Map real
staff onto these tiers:
- **Reception / front-desk →** Staff: register patients, book appointments.
- **Clinical staff →** Staff: record vitals, work appointments.
- **Billing staff →** Staff (scoped to billing perms): raise/settle invoices.
- **Administrator →** manage users, groups, permissions; oversight.

## 4. Primary entities (exact shape)

- **Patient** `patients` — `id, full_name, email, phone, created_at`. The hub and
  the **only searchable** model (full-text over name/email/phone). Order: newest.
- **Vitals** `vitals` — `id, patient_id→Patient (cascade), heart_rate, notes,
  created_at`. Filter by patient. Order: newest.
- **Appointment** `appointments` — `id, patient_id→Patient (cascade), scheduled_at
  (required), reason, status, created_at`. Status: scheduled/completed/cancelled.
  Filter by status; search reason; order by scheduled time (desc).
- **Invoice** `invoices` — `id, patient_id→Patient (cascade), amount_cents, status,
  created_at`. Amount is **whole cents**. Status: unpaid/paid. Filter by status.

Relationships: **all three child tables point at Patient with ON DELETE CASCADE.**
The FK is a bare number in the UI (no name lookup).

## 5. Main workflows

Register patient → find patient (search) → record vitals → book appointment and
advance its status (scheduled→completed/cancelled) → raise invoice and settle it
(unpaid→paid). Administrators manage access and review the audit log.

## 6. Screens that matter most (ranked)

1. **Patient list** — front door; full-text search; newest-first. *Most used.*
2. **Patient record** — identity + entry to that patient's vitals/appointments/invoices.
3. **Appointments list** — status-filtered, time-ordered: the day's work.
4. **Invoices list** — status-filtered: the money (amounts need humanising).
5. **Vitals list** — per-patient (filtered by patient).
6. **Auth & audit** — Users / Groups / History.

## 7. Operational priorities (optimise in this order)

1. **Find a patient fast** — the search-first patient list is the critical path.
2. **Read appointment status at a glance** — the operational pulse.
3. **Read billing status at a glance** — what's unpaid.
4. **Money is correct and legible** — cents, formatted.
5. **Actions are auditable and safe** — especially deletes and money changes.

## 8. Risk areas (design must respect these)

- **PHI / PII:** `full_name`, `email`, `phone` are sensitive *and surfaced* — in
  the patient list **and** in search results (they're in the FTS vector). Treat
  patient identity as confidential; be deliberate about where contact data appears.
- **Destructive cascade:** deleting a Patient **cascades** to all their vitals,
  appointments, and invoices — clinical and financial history gone, no soft-delete.
  Delete must read as high-consequence.
- **Money:** stored as integer cents; raw cents in a list is a misread risk —
  humanise to currency. Money-status changes are financially meaningful actions.
- **Free-text status:** `status` is not a DB enum — values are convention. Present
  the known states clearly but tolerate unexpected ones; don't assume exhaustivity.
- **No field validation:** `email`/`phone` are plain text with no format checks;
  `scheduled_at` is required with no default. The UI is the only guard rail.
- **Foreign key legibility:** child records show `patient_id` as a number; the
  human anchor (patient name) is absent on those surfaces.

## 9. Accessibility requirements

- **WCAG AA** is the floor; the framework's rio-theme is the contrast authority.
- **Light-only** (no dark mode); ≥16px readable type; keyboard-navigable.
- **Semantic status colour** for appointment/invoice states.
- Accent `#059669` (emerald-600) is borderline for *text* on white — the build
  reserves emerald-700 `#047857` for stronger contrast; respect that split.

## 10. Information hierarchy

Patient name is the human key (identity leads → contact → timestamps). **Status is
the headline signal** on appointments and invoices. Recency is the default reading
order. Money and the patient FK both need humanising at the presentation layer.

## 11. Hard constraints (the frame you design inside)

Postgres-only · security-first (RBAC + per-model perms + audit-by-default) ·
**no front-end build step** (hand-written CSS; theming via `--rio-*` tokens, an
optional generated `tokens.css` applied through `RUSTIO_TOKENS_CSS`) · light-only ·
append-only migrations · capability-per-business-domain (patients/scheduling/billing).

## 12. Current visual identity

Emerald accent `#059669` (hover `#047857`) on slate-50 canvas `#f8fafc`, white
surfaces, slate-900 text `#0f172a`, slate-600 muted, slate-200 borders. Calm,
clinical, restrained.

---

## Resolved context (gap-closing pass — 2026-06-07)

The open questions the no-source validation surfaced are now resolved — facts
recovered from source, and decisions approved by review (`DESIGN_DECISIONS.md`):

- **Detail composition (fact, OBS-08):** patient detail = the patient's own fields;
  children via filtered lists, not inline; child FK is a plain number input. → The
  prime design opportunity is making the patient the visible anchor on child records.
- **Permissions (D-01):** role-scoped — Reception (patients + scheduling), Clinical
  (patients + vitals + scheduling), Billing (invoices + patient read), Administrator
  (all + user/group mgmt). Design per-role scope and landing.
- **Status lifecycle (D-02):** forward + corrections — appointment reschedule
  allowed; invoice paid↔unpaid for corrections. Present known states, allow reversal.
- **Money (D-03):** USD/en-US `$1,234.56`, tabular/right-aligned.
- **Deployment (D-04):** desktop-first, single-site, single-provider, modest scale;
  no provider entity (OBS-09) — scheduling is clinic-level.

### Still genuinely open (no evidence; smaller residuals)

- Exact patient/appointment **volume** (affects pagination/density tuning).
- Whether **multi-provider / tablet** use ever becomes a requirement (currently out).
- Empty / error / loading **state copy** per screen.

### Design explorations these unlock (not commitments)

- Patient name as the anchor wherever `patient_id` appears; a patient picker on
  child create.
- Semantic status chips with the approved transitions.
- Cents → USD money formatting; unpaid emphasised.
- A delete confirmation that names the cascade.
- Search promoted as the primary navigation path; deliberate PII exposure.

Everything above is grounded in the actual models, migrations, and admin config
plus the reviewed decisions — no source reading required to act on it.
