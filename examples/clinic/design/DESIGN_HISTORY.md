---
artifact: DESIGN_HISTORY
layer: memory
status: active
updated: 2026-06-07
---

# Design History — the evolution (Clinic)

> How the clinic's design understanding evolved, and why. Reverse-chronological.

## 2026-06-07 — v1 design direction + Tier 1 slice (D-05)

Claude Design proposed a v1 direction from the context; reviewed and accepted as
**R-05 / D-05**. Implemented **Tier 1 only** — the changes that fit the existing
safe design seam (tokens + validated `custom_css` + the `[navigation]` layer), with
**no new model semantics**: capability-grouped navigation (Vitals hidden),
search-first emphasis, status colour-coding (cell-level), emerald/slate kept within
existing token values. Provider/calendar/reporting/portal, the aggregate patient
profile, the patient picker, and relation-aware FK rendering are explicitly out of
scope; true status pills, USD money formatting, per-column PII de-emphasis, and
role-scoped nav are recorded as needing later model/framework support.

**Browser review found one regression and fixed it:** enabling
`RUSTIO_TEMPLATE_DIR=templates` (to serve the generated `_sidebar.html`) also
activated the clinic's pre-existing *empty* `templates/admin/patients/list.html`
stub, which blanked the patient list. Removed the stub (its own comment sanctioned
this) so the framework default renders. Verified: grouped sidebar, status
colour-coding, and the emphasised patient search all render correctly.

## 2026-06-07 — Gap-closing reasoning pass

A no-source validation (Claude Design reading only the design artifacts) confirmed
the stack transfers domain, entities, workflows, priorities, risks, and a11y — but
flagged gaps it could not resolve from static artifacts. This pass closed them:

- **Facts recovered from source** and recorded as observations: detail pages show
  the record's own fields (children via filtered lists; FK is a number input) —
  OBS-08; no provider/clinician entity — OBS-09; permissions are seeded per-model
  but assigned by an administrator — OBS-10.
- **Four decisions** made for design purposes (no code change), each reviewed and
  approved: role-scoped permissions (**D-01**), status lifecycle forward + with
  corrections (**D-02**), money as USD/en-US (**D-03**), and a desktop-first,
  single-provider, modest-scale posture (**D-04**).

`DESIGN_CONTEXT.md`'s "what Claude Design needs to decide" section is now resolved;
`DESIGN_ARCHITECTURE.md` carries the clarified context.

## 2026-06-07 — Initial extraction

The clinic was reverse-engineered into the design-memory stack (Brief, Reasoning
OBS-01..07, Architecture, and a consolidated Context) for Claude Design —
read-only analysis, no redesign.
