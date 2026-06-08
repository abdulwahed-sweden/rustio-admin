---
artifact: DESIGN_BRIEF
layer: why
status: active
owner: shop team
updated: 2026-06-07
---

# Design Brief — WHY

> The north star for the shop admin. Claude Design reasons from this before
> touching architecture or tokens. Shared source of truth for humans, Claude
> Design, and Claude Code.

## Business Context

- **Product / domain:** back-office admin for a small e-commerce store — catalogue,
  customers, orders, payments. Built on `rustio-admin` (Postgres, security-first).
- **Primary operators (who uses it for hours):** shop staff processing orders and
  curating the catalogue; a store owner reviewing sales. Not end shoppers.
- **Business goals this admin advances:** process orders quickly and confidently;
  keep the catalogue clean; make customer and payment history legible at a glance.
- **Hard constraints:** Postgres-only; RBAC + audit trail + optional MFA (framework);
  WCAG AA; light-only (no dark mode); single binary, no front-end build step.
- **Brand voice:** premium, calm, trustworthy, quietly editorial.

## Design Intent

- **It should feel:** calm and premium across a long order-processing shift —
  authoritative without shouting, fast to scan, never fatiguing.
- **It must never feel:** neon, toy-like, cramped, or "framework default."
- **The success moment we optimise for:** an operator opens an order, understands
  its state and line items instantly, and acts — with no hunting.
- **Non-goals:** executive dashboards/BI, dark mode, marketing flair, theming
  knobs for end users.

## Visual Direction

- **Density:** balanced — comfortable for hours, not sparse.
- **Tone:** premium / editorial, not corporate-generic.
- **Colour story (D-010 — RustIO Patina):** one accent only — Patina `#0E6B5B`,
  the calm teal-green verdigris copper forms as it oxidizes — on warm-stone
  neutrals and light warm-paper chrome. No second hue. (Superseded the earlier
  deep-navy + amber story.)
- **Typographic voice:** readable first — primary UI text ≥ 16px, 14px reserved
  strictly for genuine metadata.
- **References / anti-references:** premium operational SaaS; *anti*: neon accents,
  OLED-black, 13px body text.
- **Accessibility floor:** WCAG AA; 16px minimum body. (Known caveat: `text-subtle`
  sits at 4.43:1 on white — see D-006.)
