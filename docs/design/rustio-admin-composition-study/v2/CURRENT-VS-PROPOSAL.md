# Current vs proposal

Per surface: what rustio-admin at `d4b61fa` does, what v2 proposes, the class
of the change, and the files the change would live in. Nothing in the
right-hand column has been implemented. Classes: KEEP · ALIGN WITH RUSTIO ·
IMPROVE RUSTIO-ADMIN · PRODUCT-SPECIFIC — KEEP UNIQUE.

## Shell

| | Current | Proposal | Class | Would live in |
|---|---|---|---|---|
| Utility row | 52px; ⌘K trigger 38px pill on `--rio-sunken`; account trigger 40px pill, 30px avatar | 56px; triggers 32px, radius 8, field line; 24px avatar | ALIGN | `layout/console.css` |
| Module row | 48px, 15/500 items, 18px icons | 40px on `--surface-soft`, 14/600, 16px icons | ALIGN | `layout/console.css` |
| Rail | 232px; 40px items; active = tint + 3px left bar | 240px; 38px items; tint only; head level with the utility row; captions 13px mono | ALIGN | `layout/console.css` |
| Measures | wide 1480 / standard 1120 / narrow 920 / form 672; `calc(100% - 64px)` centred | wide 1600 / 1120 / 920 / form 728; `--gutter` 32 · 24 ≤ 760 · 16 ≤ 480 | KEEP (mechanism) · ALIGN (two numbers) | `layout/console.css`, `tokens/spacing.css` |
| Footer | full-width chrome, links + identity + time | same; type and ink on the 3.1 ramp; the time in the family format | KEEP · ALIGN (faces) | `layout/console.css` |
| Env pill, bell, Docs, account menu, ⌘K | present | present | KEEP UNIQUE | — |
| AdminTheme override | `_theme.html` sets `--rio-rust`, `-hover`, `-active`, `-solid`, `-solid-hover`, `-solid-active` to one hex | emit the hover / active shades `rio-theme` computes; never touch `--rio-accent-focus` (already true) | IMPROVE | `_theme.html` (template), `rio-theme` |

## Page head

| | Current | Proposal | Class |
|---|---|---|---|
| Container | `.rio-page-header` / `.rio-masthead-top`: white card, radius, shadow, padding 24–32 | no card; a band with a 1px rule beneath spanning the measure; form pages drop the rule | ALIGN |
| Title | 36px / 800 | 24px / 800 | ALIGN |
| Lead | 17px | 15px, max 680px | ALIGN |
| Actions | far right inside the card, 44px buttons | far right on the band, 38px; the primary last (as `list.html` already orders them) | ALIGN |
| Breadcrumb | 15px, `·` separators | 13px, same separators | ALIGN |
| Two markup shapes (`.rio-page-header` and `.rio-crumbs` + `.rio-masthead-top`) | both exist | both stay; CSS covers both | KEEP |

## List page

| | Current | Proposal | Class |
|---|---|---|---|
| Search | `.rio-search--lg.rio-search--hero`: 44px pill, reveal-on-hover Search button, 4px glow | 38 × 340px field, radius 8, 3px offset focus ring | ALIGN |
| Filter triggers | `rio-btn--lg` 44px | `rio-btn--sm` 31px; active = three-signal treatment (fill, line, weight) | ALIGN |
| Filter panels | chips, FK autocomplete, multi-select, date range | unchanged; inputs inside keep a 240px floor, never auto width | KEEP UNIQUE |
| Sort / direction / rows per page | on the command bar, 44px | on the data-surface head, 31px, pushed right | ALIGN |
| Result count | not on the bar | after the last control, 14/600 with the number 700 | ALIGN |
| Active-filter pills | present, below the bar | present; same position; 13/600 | KEEP UNIQUE · ALIGN (faces) |
| View-mode switch | present when a ViewSpec exists | on the data-surface head, 31px segments | KEEP UNIQUE · ALIGN |
| Bulk bar | present on selection | a strip between head and rows on `--blue-soft`; selected rows tinted, including the sticky actions cell | KEEP UNIQUE · ALIGN |
| Rows | 56–64px, 22px padding | 48px, 16px padding; Compact 38px with a 32px head | ALIGN |
| Head | 15px, uppercase | 40px, 13px mono, uppercase on `--surface-head` | ALIGN |
| Row actions | `.rio-row-actions` icon-only, `opacity: 0` until hover (Users excepted) | quiet text Edit / Delete, always visible, 31px, Delete red at rest | ALIGN + IMPROVE |
| Cells by kind | `rio-td--{{ f.kind }}` emitted; no kind-specific rules | date / datetime / boolean nowrap; tabular digits; numeric right-aligned; identity columns hug | ALIGN + IMPROVE |
| Actions column on scroll | lost first | sticky at the right edge inside `.rio-tablewrap` at every width | IMPROVE |
| Empty state | `.rio-empty-state`: 48px padding, icon tile, 20px title | flat, inside the surface, 32px, no tile | ALIGN |
| Pagination | present | as shipped, on the surface foot | KEEP |

## View modes (adaptive list)

| | Current | Proposal | Class |
|---|---|---|---|
| List | rows with 16px cells | one surface, 64px minimum rows, badge beside the primary | ALIGN |
| Cards | shadowed cards, radius 12 | flat, radius 14, 16px padding, 12rem identity floor, `break-word` | ALIGN |
| Compact | tighter rows | the `--td-h-compact` token; same component | ALIGN |
| Badges (`.av-badge--*`) | own face | the one badge face | ALIGN |

## Forms

| | Current | Proposal | Class |
|---|---|---|---|
| Measure | `rio-page--form` 672, centred | 728, centred (`--form-measure`) | ALIGN (one number) |
| Fieldset | legend on the border, 20px, radius 16 | mono band heading inside the card; `min-width: 0` on the fieldset | ALIGN · IMPROVE (the min-width) |
| Inputs | 46px | 38px; textareas at a reading height | ALIGN |
| Boolean | checkbox + label inline | 38px bordered row holding the box and its label | ALIGN |
| Action bar | Save · Save and continue · Save and add another · History · Delete · Cancel | unchanged order; 38px; text actions at the far end | KEEP UNIQUE · ALIGN (faces) |
| Validation | alert + per-field error | alert as a flex row; per-field red line + message | ALIGN |
| Inline related sections | present | present; compact table inside, quiet actions, foot action | KEEP UNIQUE · ALIGN (faces) |

## Users, groups, permissions

| | Current | Proposal | Class |
|---|---|---|---|
| Users grid | explicit tracks, always-visible actions, ellipsised identity | the model for every list | KEEP |
| Monogram tile | 44px blue square with an initial | removed; identity is the link | ALIGN |
| Role chip | `.rio-role` own face | the one badge face (mono text kept) | ALIGN |
| Permission matrix | present | 40px rows, 18px boxes on the primary, row and column All, granted count in the foot | KEEP UNIQUE · ALIGN (faces) |
| Group editor action bar | present | one bar; Delete group as red text at the far end | ALIGN |

## Account

| | Current | Proposal | Class |
|---|---|---|---|
| Active sessions | five stacked `.rio-sess-card`s with a glyph tile and a footer each | one list surface, one session per row, the current tinted, Revoke small secondary, bulk actions beneath | ALIGN |
| User detail tabs | `.rio-tabs` | 38px, 2px underline on the primary | ALIGN |
| Detail list | `.rio-dl` | 140px mono label column, hairlines | ALIGN |
| Sessions tab table | present | nowrap date cells, UA ellipsised at 28ch | ALIGN |

## Timestamps (family rule)

| Page | Current | Proposal |
|---|---|---|
| Model list cells | date only (`PR #154`) | `YYYY-MM-DD HH:MM UTC` |
| Users list, sessions | `%Y-%m-%d %H:%M` (no zone) | `YYYY-MM-DD HH:MM UTC` |
| User detail, footer | `%Y-%m-%d %H:%M UTC` | unchanged |
| History "when", "last seen", "expires" | relative | relative — the point of those cells |

Class: IMPROVE (rustio-admin disagrees with itself) and ALIGN (RustIO PR #5
settled on this format). This is a Rust change in the formatters and the
only one in the proposal; it belongs to Phase 5 of `MIGRATION.md`.

## Product pages that only take scale and faces

API surface, history, health, DB browser, feature flags, notifications, docs
viewer, view designer, branding, MFA flows, auth pages. KEEP UNIQUE; ALIGN on
type, controls, radii, badges and empty states only.

## Found while verifying the boards (product-relevant)

- `.rio-sr-only` is `position: absolute`. Inside a `.rio-tablewrap` that is
  not positioned, the label in the actions head is laid out at its static
  position beyond the scroller and widens the **document** at narrow widths.
  Fix: `.rio-tablewrap { position: relative }`. IMPROVE.
- A `<fieldset>` defaults to `min-inline-size: min-content`; with the matrix
  inside it, the group editor widens the page at 390. Fix:
  `.rio-fieldset { min-width: 0 }`. IMPROVE.
- `tokens/compat.css` says it maps legacy names onto "the current Teal
  palette"; the palette is blue. Comment only. Documented, not edited.
