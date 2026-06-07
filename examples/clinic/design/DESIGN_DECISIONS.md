---
artifact: DESIGN_DECISIONS
layer: memory
status: active
updated: 2026-06-07
---

# Design Decisions — the ledger (Clinic)

> Accepted design-memory decisions (approved outcomes of `DESIGN_REASONING.md`).
> These resolve the gaps the no-source validation surfaced. They are decisions for
> *design purposes* and do **not** change clinic code. Never delete a row —
> supersede it.

| ID    | Date       | Decision                                                          | Status   | Reasoning |
|-------|------------|------------------------------------------------------------------|----------|-----------|
| D-01  | 2026-06-07 | Role-scoped permissions (Reception/Clinical/Billing/Admin)       | accepted | R-01      |
| D-02  | 2026-06-07 | Status lifecycle: forward + corrections (reschedule; paid↔unpaid) | accepted | R-02      |
| D-03  | 2026-06-07 | Money displayed as USD, en-US ($1,234.56)                        | accepted | R-03      |
| D-04  | 2026-06-07 | Desktop-first, single-provider, modest-scale deployment posture  | accepted | R-04      |

_Observed facts that informed these (not decisions) live in `DESIGN_REASONING.md`
as OBS-08 (detail = own fields; FK is a number), OBS-09 (no provider entity),
OBS-10 (perms seeded, assignment is operator policy)._
