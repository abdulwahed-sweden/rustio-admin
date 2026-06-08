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

## R-010 · Adopt the RustIO Patina design system (light) → D-010

- **Date:** 2026-06-08
- **Status:** accepted (reviewed)
- **Serves:** a Claude Design handoff bundle (`rustio-design-system`) whose admin
  UI kit is literally "Shop admin · Products." Adopting it gives shop the official
  RustIO visual language.
- **Context:** the bundle defines **one accent only — RustIO Patina `#0E6B5B`**
  (teal-green verdigris), **warm-stone** neutrals, a **light** warm-paper chrome
  (sidebar `background: var(--rio-surface)`, hairline border), radius 6/9/12, and
  Spectral/Hanken/JetBrains type. This replaces shop's navy+amber identity.
- **Options considered:**
  1. Keep navy+amber (status quo) — but the official system is Patina; the kit is
     shop.
  2. Adopt Patina **fully incl. dark mode + webfonts** — blocked: rustio-admin is
     light-only and forbids external fetch (no Google-Fonts `@import`).
  3. **Adopt Patina via the safe seam, light-only, system-font fallback** — chosen.
- **Decision:** map the Patina/warm-stone palette onto rustio-admin's runtime
  tokens (`[colors]`), radius → 6/9/12 (`[radius]`), adopt the **type families**
  (Hanken Grotesk / JetBrains Mono) via `[typography]`, and flip the chrome to
  **light warm-stone** via `[custom_css]` (re-asserting dark warm-stone text in the
  sidebar/topbar scope, since the framework's default chrome assumes dark).
- **Supersedes:** **D-001** (Deep Navy primary), **D-002** (dark navy chrome →
  now light warm-stone), **D-003** (amber `accent2` → folded into the one Patina
  accent: "no second hue, ever"), **D-005** (radius 7/5/9 → 6/9/12).
- **Retained:** **D-008** (nav grouping), **D-009** (status colour-coding — now
  recoloured to the system's desaturated success/warn/danger), and **D-004**
  (≥16px type sizes kept for accessibility; only the *families* change — the
  system's 14px product base is noted but not adopted, a deliberate a11y deviation).
- **Rejected because:** keeping navy ignores the official system; full adoption
  (dark mode, webfonts) breaks framework doctrine.
- **Spec impact:** `[colors]` (full repalette), `[radius]`, `[typography]`
  (families), `[custom_css]` (light chrome + accent2-fold + tabular numerals).
  **No new model semantics.**
- **Known limitations (recorded):** the Spectral/Hanken/JetBrains **webfonts are
  not loaded** (doctrine forbids external fetch); families fall back to system
  fonts until self-hosted faces are added to the framework (later). **Dark mode**
  is out (framework is light-only). The full kit (⌘K palette, dense-table chrome,
  dark toggle) remains Tier 2/3.

## R-009 · Status colour-coding for orders & payments → D-009

- **Date:** 2026-06-08
- **Status:** accepted
- **Serves:** the operator's need to read order/payment state at a glance — the
  same safe-seam pattern proven on the clinic (clinic D-05).
- **Context:** order status (`cancelled/paid/pending/shipped`) and payment status
  (`completed/failed/pending/refunded`) render as plain text in the lists; nothing
  signals state visually.
- **Options considered:**
  1. Plain text (status quo) — nothing to scan.
  2. Cell-level colour-coding via `custom_css` on the existing markup — safe seam.
  3. True pills — needs a per-model list template (wrapping element).
- **Decision:** option 2 — colour the known status values via
  `td.rio-td--text[title="…"]` (the framework mirrors a text cell's value into its
  `title`). Semantic mapping within the existing shop tokens:
  paid/completed → success · shipped → accent (navy) · pending → warning ·
  failed → danger · cancelled/refunded → muted.
- **Rationale:** highest-value, lowest-risk; fits the same safe seam as clinic; no
  new model semantics; drift-detected; reversible.
- **Rejected because:** plain text gives no signal; true pills need template work
  (deferred, as on the clinic).
- **Spec impact:** appended to `[custom_css]`. **No** new model semantics.

## R-008 · Group the sidebar by domain; bury junction models

- **Date:** 2026-06-07
- **Status:** accepted
- **Serves:** Brief → "open an order, understand it, act — no hunting"; Architecture
  → "Orders and Products one click away; line items / cart items / product images
  reached through their parent, not the nav."
- **Context:** the live admin renders a **flat list of all 9 models** in
  registration order (Product, Category, Customer, Order, OrderItem, Address,
  ProductImage, CartItem, Payment). Junction/secondary tables sit beside primary
  entities and compete for attention; the operator's domain mental model
  (Catalogue / Customers / Sales) is invisible. The existing
  `DESIGN_ARCHITECTURE.md` nav tree is also internally inconsistent: it shows
  *Addresses* as a visible nav item, yet the IA note says secondary records are
  reached through their parent.
- **Options considered:**
  1. **Status quo — flat 9-model list.** Zero work; but noisy, and four
     junction/secondary tables (Order items, Cart items, Product images,
     Addresses) clutter the sidebar and bury the three that matter.
  2. **Grouped, all 9 visible.** Domains become clear (Catalogue/Customers/Sales),
     but the sidebar still surfaces tables operators never navigate to directly.
  3. **Grouped, primary-only (proposed).** Three domains; only the primary
     entities in the sidebar; junction/secondary models reached inline through
     their parent (still reachable by URL), not listed in nav.
- **Decision:** option 3. Target sidebar:
  ```
  Dashboard
  Catalogue   → Products · Categories
  Customers   → Customers
  Sales       → Orders · Payments
  ```
  `OrderItem`, `CartItem`, `ProductImage`, **and `Address`** are removed from the
  sidebar and reached through their parent record. (This buries `Address`, which
  resolves the architecture inconsistency above — a deliberate change from the
  earlier tree that listed it.)
- **Rationale:** 5 focused destinations instead of 9, organised by the operator's
  mental model. Realises both the Brief ("no hunting") and the Architecture
  ("one click vs buried") with no loss of access — secondary records still open
  inline and by URL.
- **Rejected because:** status quo hides the domain model and dilutes the primary
  entities; all-9-grouped still puts join tables in the operator's path.
- **Spec impact:** **none** to `rustio.design.toml` — tokens are unchanged. This is
  a WHAT-layer (navigation) decision, not a HOW-layer one.
- **Architecture impact:** revise `DESIGN_ARCHITECTURE.md` → Navigation Structure
  to the tree above and mark `Address` as buried (reached via Customer).
- **Application note (seam not built yet):** navigation generation is tracked by
  rustio-design#1. On approval, this pass updates `DESIGN_ARCHITECTURE.md` +
  `DESIGN_DECISIONS.md` + `DESIGN_HISTORY.md` to record the decision; the *rendered*
  grouped sidebar lands when #1 picks a seam (proposed: a generated
  `_sidebar.html` override served via `RUSTIO_TEMPLATE_DIR`, the recompile-free
  parallel to `RUSTIO_TOKENS_CSS`).

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
