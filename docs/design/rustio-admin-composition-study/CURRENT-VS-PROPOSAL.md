# Current vs proposal (v3)

Per surface: rustio-admin at `d4b61fa`, the v3 proposal, the class, and
where the change would live. "ALIGN" means RustIO `main @ 00b933c`. Nothing
in the proposal column has been implemented.

## Shell

| | Current | Proposal | Class |
|---|---|---|---|
| Utility row | 52px; 38px ⌘K pill; 40px account pill | 56px; 32px triggers, radius 8; 24px avatar | ALIGN |
| Module row | 48px, 15/500, 18px icons | 40px on `--surface-soft`, 14/600, 16px icons | ALIGN |
| Rail | 232px; 40px items; tint + 3px bar | 240px; 38px items; tint only; captions 13px mono | ALIGN |
| Measures | wide 1480 / standard 1120 / narrow 920 / form 672; centred, 32px gutter | wide 1600 / 1120 / 920 / form 728; `--gutter` 32 · 24 · 16 | KEEP (mechanism) · ALIGN (wide, gutter) · INTENTIONAL DIVERGENCE (form, see Forms) |
| Footer | full-width chrome | same; faces on the ramp; the time in the family format | KEEP · ALIGN |
| Env pill, bell, Docs, account menu, ⌘K | present | present | KEEP UNIQUE |
| AdminTheme override | one hex into six `--rio-rust*` names; focus untouched | hover / active shades from `rio-theme`; focus still untouched | IMPROVE · INTENTIONAL DIVERGENCE (`main` lets `rustio.design.json` set `--focus`) |

## Page head

| | Current | Proposal | Class |
|---|---|---|---|
| Container | white card, radius, shadow | none — `main`'s `.page-head` grid | ALIGN |
| Title | 36 / 800 | 24 / 800 | ALIGN |
| Action | far right inside the card, 44px | one gap after the title, `justify-self: start`, 38px; primary nearest the title | ALIGN |
| Lead | 17px | 15px, max 680px, beneath title and action | ALIGN |
| Breadcrumb | 15px | 13px | ALIGN |
| Two markup shapes | both | both stay; CSS covers both | KEEP |
| v2's band with a rule | — | removed | DROP |

## List page

| | Current | Proposal | Class |
|---|---|---|---|
| Search | 44px hero pill, reveal button, glow | 38 × 340 field, 3px offset focus ring (`main` `.find-search`) | ALIGN |
| Filter triggers | `rio-btn--lg` 44px | `rio-btn--sm` 31px | ALIGN |
| Filter panels | chips, FK autocomplete, multi-select, date range | unchanged | KEEP UNIQUE |
| Sort / direction / rows per page | on the command bar | on the data-surface head (`main` `.data-head`) | ALIGN |
| Result count | absent | after the last control (`main` `.find-count`) | ALIGN |
| Active-filter pills | present | present, 13/600 | KEEP UNIQUE · ALIGN |
| View-mode switch | present | on the surface head, 31px segments | KEEP UNIQUE · ALIGN |
| Bulk bar | present | strip on `--blue-soft`; selected rows tinted | KEEP UNIQUE · ALIGN |
| Rows | 56–64px | 48px, 16px padding (`main` `--td-h`) | ALIGN |
| Head | 15px uppercase | 40px, 13px mono (`main` `--th-h`) | ALIGN |
| Row actions | icon-only, hover-revealed | quiet text, always visible (`main` `.button-quiet`) | ALIGN + IMPROVE |
| Identity hug | users grid only | `.rio-cell-fit` where every value is one token (`main` `.cell-fit` / `column_hugs`) | ALIGN |
| Cells by kind | kinds emitted, no rules | date / datetime / boolean nowrap; tabular digits; numeric right | IMPROVE |
| Actions column on scroll | lost first | sticky inside `.rio-tablewrap`, every width | IMPROVE |
| Compact | tighter rows | 40px lines, no head (`main` `.record-compact-item`) | ALIGN |
| Empty state | padded, icon tile | flat, inside the surface (`main`) | ALIGN |

## View modes

| | Current | Proposal | Class |
|---|---|---|---|
| List | rows | one surface, 64px rows, badge beside the primary (`main` `.record-list-*`) | ALIGN |
| Cards | shadowed, radius 12 | flat, radius 14, 16px (`main` `.data--cards`) | ALIGN |
| Card identity | wraps freely | 12rem floor, `break-word` | IMPROVE |
| `.av-badge` | own face | the one badge face | ALIGN |

## Forms

| | Current | Proposal | Class |
|---|---|---|---|
| Measure | `rio-page--form` 672, centred | 728, centred. `main` renders a 728 card left-aligned in the 1120 column beside a System aside for readonly fields; rustio-admin has no aside and keeps its centred measure, matching the card width | INTENTIONAL DIVERGENCE |
| Fieldset | legend on border | mono band inside the card; `min-width: 0` | ALIGN · IMPROVE |
| Inputs | 46px | 38px | ALIGN |
| Boolean | inline | 38px bordered row (`main` `.field-boolean`) | ALIGN |
| Action bar | three save variants + text actions | unchanged order, 38px | KEEP UNIQUE · ALIGN |
| Validation | alert + per-field | flex alert; red line + message | ALIGN |
| Inline related sections | present | compact table, quiet actions | KEEP UNIQUE · ALIGN |

## Users, groups, permissions

| | Current | Proposal | Class |
|---|---|---|---|
| Users grid | the reference table | unchanged structure; 48px; faces | KEEP · ALIGN |
| Monogram tile | 44px square | removed | ALIGN |
| Role chip | own face | the one badge face | ALIGN |
| Permission matrix | present | 40px rows, boxes on the primary | KEEP UNIQUE · ALIGN |
| Group editor action bar | present | one bar, Delete as red text at the far end | ALIGN |

## Account

| | Current | Proposal | Class |
|---|---|---|---|
| Active sessions | five stacked cards | one list surface | ALIGN |
| User detail tabs | present | 38px, 2px underline | ALIGN |
| Detail list | present | 140px mono label column | ALIGN |
| Sessions tab table | present | nowrap date cells, UA ellipsis 28ch | ALIGN |

## Timestamps

| Page | Current | Proposal |
|---|---|---|
| Model list cells | date only (PR #154) | `YYYY-MM-DD HH:MM UTC` |
| Users list, sessions | `%Y-%m-%d %H:%M` | `YYYY-MM-DD HH:MM UTC` |
| User detail, footer | `%Y-%m-%d %H:%M UTC` | unchanged |
| History "when", "last seen", "expires" | relative | relative |

Class: IMPROVE RUSTIO-ADMIN. The format is the one rustio-admin's user
detail already uses and the one `main` uses on its history and audit pages;
`main` has no cell-level rule, so this is not an alignment. The only Rust
change in the proposal (Phase 5).

## Product pages that only take scale and faces

API surface, history, health, DB browser, feature flags, notifications, docs
viewer, view designer, branding, MFA, auth. KEEP UNIQUE; ALIGN on type,
controls, radii, badges and empty states.

## Found while verifying the boards (product-relevant, IMPROVE)

- `.rio-sr-only` is `position: absolute`; inside an unpositioned
  `.rio-tablewrap` it widens the document at narrow widths. Fix:
  `.rio-tablewrap { position: relative }`.
- `<fieldset>` defaults to `min-inline-size: min-content`; with the matrix
  inside, the group editor widens the page at 390. Fix: `.rio-fieldset {
  min-width: 0 }`.
- `tokens/compat.css` says "the current Teal palette". Comment only.
