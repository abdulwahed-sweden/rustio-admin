---
artifact: DESIGN_REASONING
layer: reasoning
status: active
updated: 2026-06-07
---

# Design Reasoning — the pass before the spec

> **Brief → Reasoning → Architecture → Spec → Generated.** Every token in
> `rustio.design.toml` traces to an entry here. ADR-style; newest on top.
> Drive new entries with `/design-reason`.

## R-007 · Migrate the theme into rustio-design's declarative stack

- **Date:** 2026-06-07
- **Status:** accepted
- **Serves:** Brief → "must never feel like framework default"; maintainability.
- **Context:** the navy identity, amber accent, and type scale lived as two
  hand-coded `<style>` blocks inline in `templates/admin/_theme.html`. Scattered
  design CSS drifts and breaks silently.
- **Options considered:**
  1. Keep the inline `<style>` blocks — zero migration, but no validation, no
     drift detection, design intent undocumented.
  2. Move everything into `rustio.design.toml` + `design/DESIGN_*.md` — one
     source of truth, validated, with a reasoning trail.
- **Decision:** option 2. Tokens → spec; `accent2` + chrome rules → `[custom_css]`;
  type scale → `[typography]`; intent/structure → this `design/` memory.
- **Rationale:** single source of truth, WCAG + token-name validation, and an
  auditable *why*. `_theme.html` is reduced to the AdminTheme passthrough.
- **Rejected because:** inline CSS gave no guard rails and no memory.
- **Spec impact:** entire `[colors]`/`[typography]`/`[radius]`/`[custom_css]`.
- **Architecture impact:** none (presentation only).

## R-006 · Accept `text-subtle` at 4.43:1 (with caveat)

- **Date:** 2026-06-07
- **Status:** accepted
- **Serves:** Brief → "Accessibility floor: WCAG AA".
- **Context:** `rustio-design build` flagged `text-subtle = #6b7894` at 4.43:1 on
  white — just under the 4.5:1 AA threshold for normal text.
- **Options considered:**
  1. Keep #6b7894 (preserves the exact hand-tuned look).
  2. Darken to ≥ 4.5:1 (e.g. #64748b-ish) for strict AA.
- **Decision:** keep #6b7894 for now; track the caveat.
- **Rationale:** the token is used only for genuinely tertiary metadata, not body
  copy; the visual tuning is intentional. Logged rather than silently shipped.
- **Rejected because:** darkening now would shift the established look before the
  team confirms the trade-off.
- **Spec impact:** `[colors] text-subtle` (unchanged, tracked as D-006).

## R-005 · Sharpen corners to a premium SaaS radius

- **Date:** 2026-06-07
- **Status:** accepted
- **Serves:** Brief → "premium, quietly editorial".
- **Context:** the framework default radius (10/6/12px) reads a touch soft for the
  premium tone.
- **Options considered:**
  1. Keep the framework default (10/6/12).
  2. Sharpen to 7/5/9px.
  3. Square corners (0) — too austere for "premium, calm".
- **Decision:** 7/5/9px.
- **Rationale:** crisper without being hard; reads premium across cards, inputs,
  badges admin-wide from one edit.
- **Rejected because:** default felt generic; square felt cold.
- **Spec impact:** `[radius] default/sm/lg`.

## R-004 · Raise the type scale (≥ 16px readable)

- **Date:** 2026-05 (original shop theme)
- **Status:** accepted
- **Serves:** Brief → "readable first; never fatiguing".
- **Context:** the stock 14/15/16px tiers read small on laptops during long shifts.
- **Decision:** primary UI text 16–17px; 14px reserved for true metadata.
- **Rationale:** legibility over a ten-hour shift is the success condition.
- **Rejected because:** denser scales saved space at the cost of fatigue.
- **Spec impact:** `[typography] fs-*`.

## R-003 · Warm amber as the secondary accent

- **Date:** 2026-05 (original shop theme)
- **Status:** accepted
- **Serves:** Brief → "measured delight without competing with navy".
- **Context:** navy alone is authoritative but austere.
- **Decision:** amber (`#e0912b` / ink `#b06d0e` / soft `#fbeed9`) for icons,
  secondary links, hovers — never primary actions.
- **Rationale:** the classic navy companion; joyful but premium, and it never
  steals the navy primary's job.
- **Rejected because:** a single-accent palette felt flat.
- **Spec impact:** `[custom_css]` `--rio-accent2*` (not a framework token).

## R-002 · Dark navy chrome for sidebar + topbar

- **Date:** 2026-05 (original shop theme)
- **Status:** accepted
- **Serves:** Brief → "premium, trustworthy".
- **Context:** the framework ships light chrome; the shop signature is a navy frame.
- **Decision:** dark navy sidebar/footer/topbar with light text and a steel-blue
  accent lifted so active items read on navy.
- **Rationale:** the navy frame is the shop's recognisable identity.
- **Rejected because:** light chrome read as generic.
- **Spec impact:** `[custom_css]` scoped `.rio-sidebar` / `.rio-topbar` rules.

## R-001 · Deep Navy as the primary identity

- **Date:** 2026-05 (original shop theme)
- **Status:** accepted
- **Serves:** Brief → "premium, calm, trustworthy"; "never framework default".
- **Context:** the framework default accent is a teal; shop needs its own identity.
- **Options considered:**
  1. Framework teal default — zero effort, no identity.
  2. A curated preset (ocean/forest/sunset) — quick, but shared.
  3. Deep Navy `#1e3a5f` from the ui/ design system.
- **Decision:** Deep Navy `#1e3a5f`.
- **Rationale:** carries authority and trust; distinct; pairs with amber.
- **Rejected because:** teal/presets gave no ownership of the brand.
- **Spec impact:** `[colors]` brand/accent/surface/status block.
