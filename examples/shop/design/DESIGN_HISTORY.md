---
artifact: DESIGN_HISTORY
layer: memory
status: active
updated: 2026-06-29
---

# Design History — the evolution

> How the shop's design changed over time, and *why*. Reverse-chronological.
> Token churn lives in git; this is the human-readable arc tying changes to the
> decisions (`D-NNN`) that drove them.

## 2026-06-29 — Drop the dead chrome remap (framework rail migration)

The Patina adoption (D-010) re-themed the framework's chrome to light warm-stone
via `custom_css` overrides on `.rio-sidebar` / `.rio-footer` / `.rio-topbar*` /
`.rio-sidebar-toggle` / `.rio-search-trigger*`. The framework has since migrated
its chrome to the dark `.rio-rail` command rail and the `.rio-ws*` workspace
header (the rail is intentionally dark in both themes), so every one of those
selectors now matches nothing — the overrides were dead.

**Action:** removed the chrome-remap block from `rustio.design.toml`'s
`[custom_css]` (and regenerated `generated/tokens.css` + the manifest hashes to
match). The Patina palette, radius, and type still ship via the token `:root`
block; the one-accent fold (`--rio-accent2*`), tabular figures, and the order/
payment status colour-coding (D-009, `td.rio-td--text[title=…]` — still emitted
by the list table) are unaffected. The light-chrome intent (D-010) is **deferred**
for the rail: re-add a remap targeting `.rio-rail`/`.rio-ws*` if a light rail is
wanted later.

Same pass also retired the **navigation grouping (D-008)**: it grouped the nav
into Catalogue / Customers / Sales and hid the join/child models, emitting
`templates/admin/_sidebar.html`. That generated override was already dropped
(repo commit `e3c8c3e`) because it used pre-rail `.rio-sidebar` markup, but the
`[navigation]` block and the manifest entry for the file were left dangling
(so `rustio-design check` referenced a missing file). Removed the `[navigation]`
block from `rustio.design.toml` (kept as a documented deferral) and the
`_sidebar.html` line from the manifest. The dark `.rio-rail` rail auto-groups nav
from the registry; custom group labels + hiding are deferred until a rail-aware
generator lands. `RUSTIO_TEMPLATE_DIR` stays — the product/order/customer
`list.html` view overrides still use it.

## 2026-06-08 — Adopt the RustIO Patina design system (D-010)

Re-skinned shop from the navy+amber identity to the official **RustIO Patina**
design system (a Claude Design handoff bundle whose admin kit *is* "Shop admin"):
one accent only — **Patina `#0E6B5B`** (teal-green verdigris) — on **warm-stone**
neutrals, **light warm-paper chrome** (sidebar flipped from dark navy to light),
radius 6/9/12, and the Hanken Grotesk / JetBrains Mono type families. The amber
secondary was folded into the single accent ("no second hue, ever"). Superseded
D-001/D-002/D-003/D-005; kept nav grouping (D-008), status colours (D-009,
recoloured), and the ≥16px sizes (D-004, a11y) while adopting the type families.
Light-only and system-font fallback — dark mode and the Google webfonts are out
by framework doctrine (recorded in R-010). Browser-verified.

## 2026-06-08 — Status colour-coding (D-009)

Coloured the order/payment status values via cell-level `custom_css`
(`td.rio-td--text[title="…"]`) — the same safe seam proven on the clinic:
paid/completed → success, shipped → navy accent, pending → warning, failed →
danger, cancelled/refunded → muted. No new model semantics; true pills remain
deferred. Browser-verified on the orders list.

## 2026-06-07 — Decide the navigation structure (D-008)

Reasoned through the sidebar (R-008): the admin still renders a flat 9-model list,
which hides the operator's domain model and lets join tables compete with primary
entities. Decided to group by domain — **Catalogue · Customers · Sales** — and show
primary entities only; `OrderItem`, `CartItem`, `ProductImage`, and `Address` are
reached through their parent (and by URL), not the nav. Recorded in
`DESIGN_ARCHITECTURE.md`. **Now rendered:** the `[navigation]` section compiles to
`templates/admin/_sidebar.html` (rustio-design#1) — the sidebar shows
Catalogue · Customers · Sales, served via `RUSTIO_TEMPLATE_DIR`. Browser-verified.

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
