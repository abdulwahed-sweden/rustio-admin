---
artifact: DESIGN_HISTORY
layer: memory
status: active
updated: 2026-06-07
---

# Design History — the evolution

> How the shop's design changed over time, and *why*. Reverse-chronological.
> Token churn lives in git; this is the human-readable arc tying changes to the
> decisions (`D-NNN`) that drove them.

## 2026-06-07 — Adopt the rustio-design stack

The hand-coded navy/amber theme moved out of `templates/admin/_theme.html`'s
inline `<style>` blocks and into the declarative design stack: tokens in
`rustio.design.toml`, intent/structure in `design/DESIGN_*.md`. The look is
preserved; what changed is that it is now validated, drift-protected, and has an
auditable *why* (D-007). The output directory moved from `design/` to
`generated/` so `design/` could hold the design memory. Corners were sharpened to
7/5/9px (D-005), and the validator surfaced a borderline tertiary-text contrast,
now tracked rather than silently shipped (D-006).

Verified in a real browser (system Chrome) after migration — login, dashboard,
and the Products list render the navy chrome, amber `accent2`, and sharpened
corners correctly. Visual record: `design/screenshots/` (also attached to PR #48).

## 2026-05 — The "Deep Navy" identity

The shop established its visual identity on `rustio-admin`: Deep Navy primary
(D-001) with dark navy chrome (D-002), a warm amber secondary for measured
delight (D-003), and a raised type scale for long-shift legibility (D-004).
