# RustIO Admin Visual Contract

Version: 3.0
Status: Mandatory — the source of truth for all content-area token **values**
(colors, type scale, fonts, spacing, control heights, row heights, RTL).

Changelog:
- **3.0** — **Composition 3.1.** Adopts the shared RustIO family scale and
  retires the v2.x assumptions that contradicted the shipped admin. Repealed:
  the burnt-copper accent (`#B84318`), the 44px control floor, the "36/38px
  control heights are forbidden" clause, the "no content-area UI text below
  14px" floor, the 36px page title, the 16px body, and the mandatory dark
  theme. Recorded: 15px body, 13px mono micro-labels, 24px titles, 38/31/32px
  controls, 40/48px rows, primary `#1F5797`, focus `#2F7BD6` as a distinct
  semantic role, a light-only system, the shared semantic token vocabulary
  with permanent `--rio-*` aliases, and the two intentional divergences from
  RustIO `main` (§15). Derived from
  `docs/design/rustio-admin-composition-study/` (v3, frozen) against RustIO
  `main @ 00b933c`. See §16 for what is recorded here versus what has landed
  in the stylesheets.
- **2.1** — §3 split into three section-pattern cases (form / list-table / single-group),
  derived from the §0 references after the §13 walkthrough surfaced that "legend-on-border
  everywhere" contradicted feature_flags.png and the simple-create cards; §13 checklist
  line updated to "section pattern matches its case"; encoded two label facts the
  references settle (lowercase status pills; "Current password").
- **2.0** — initial contract.

This contract is the single owner of the concrete visual numbers. Doctrine docs
(`DESIGN_DOCTRINE.md`, `DESIGN_SYSTEM.md`, …) own *principles and architecture*
and point here for values — one source of truth per fact. When this contract and
an older stylesheet/doc disagree, this contract wins. When both are silent, ask.

Companion contracts: `TOKENS-EMIT-SPEC.md` owns the `tokens.css` emission
contract for generators; `REMEDIATION_V2.md` is a **historical** record of the
v2.x rollout and is not normative for v3.

## 0. Reference

The authoritative visual reference is **RustIO `main @ 00b933c`**
(`rustio-core/assets/static/admin.css`) — the "Composition 3.1" recomposition.
rustio-admin and RustIO are one family: they share the semantic palette, the
type ladder, the control and row scale, the radii and the component faces. They
do **not** share markup, class names, or capabilities. `main`'s selectors are
cited throughout as *the rule being matched*; they are never imported, aliased,
or renamed. rustio-admin keeps `.rio-*`.

Where this contract cites `main`, the citation is the alignment evidence. Where
it departs from `main`, the departure is recorded in §15 with its reason — a
decision, not drift.

The v2.x reference screenshots (nine light pages plus a dark mirror) that seeded
this contract were removed from the repository; the measured values below are the
standalone source of truth. Recover an image from git history if ever needed:
`git log --all -- docs/visual-reference/`. For the record the nine pages were
`admin_reset_password · feature_flags · form · group_edit · group_new ·
lock_user · password_change · user_new · products_list`. Their **dark** mirrors
are historical only — see §12.

If a surface being changed has no counterpart on `main`, it is product-specific:
match the closest pattern in this contract, take the scale and the faces, and
keep the composition (§15.3).

## 1. Canonical tokens (light, the only theme)

The shared semantic core. Canonical names carry the values; every `--rio-*` name
is a **permanent alias** onto them, because `AdminTheme`, `rio-theme` and
downstream projects already target the `--rio-*` contract and renaming them would
break every branding override for no visual gain.

```
canonical (value)   --page: #F1F1EE;
   ↓ alias
product name        --rio-bg: var(--page);
   ↓ consumed by    components, AdminTheme, rio-theme, downstream projects
```

### 1.1 Surfaces, lines, rail

| Canonical | Value | `--rio-*` alias |
|---|---|---|
| `--page` | `#F1F1EE` | `--rio-bg` |
| `--surface` | `#FFFFFF` | `--rio-surface` |
| `--surface-soft` | `#F7F9FC` | `--rio-sunken` — also the row-hover ground |
| `--surface-head` | `#F1F4F8` | `--rio-raised` — table heads, card heads |
| `--border` | `#D5DCE5` | `--rio-line` |
| `--border-strong` | `#B9C5D3` | `--rio-line-strong` |
| `--rail-bg` | `#F4F6F9` | `--rio-rail-bg` |
| `--rail-line` | `#D3DBE5` | `--rio-rail-line` |

Never pure white as a page ground; never pure black as ink.

### 1.2 Ink ramp

| Canonical | Value | Use |
|---|---|---|
| `--ink` | `#171B22` | titles, cells, labels, values |
| `--ink-body` | `#202733` | body, lead, column heads |
| `--ink-2` | `#3F4A59` | breadcrumb, counts, in-cell detail, page-head lead |
| `--ink-mono` | `#334052` | mono micro-labels (table heads, fieldset bands, `dl` labels, eyebrows, stat labels, rail section labels) |
| `--ink-hint` | `#4C5868` | field hints, help text, placeholders |
| `--ink-3` | `#596779` | tertiary technical metadata **only** (footer meta, record ids, counts) |

`--rio-text-faint` splits across `--ink-hint` and `--ink-3` by use-site. Neither
may go lighter than these values: Composition 3.1 forbids pale operational text.

### 1.3 Accent and focus

| Canonical | Value | `--rio-*` alias |
|---|---|---|
| `--blue` | `#1F5797` | `--rio-rust`, `--rio-rust-solid` |
| `--blue-dark` | `#174578` | `--rio-rust-hover` |
| `--blue-soft` | `#DFEAFB` | `--rio-rust-tint` |
| `--blue-line` | `#B7CDEA` | `--rio-rust-tint-2` |
| `--focus` | `#2F7BD6` | `--rio-accent-focus` |

**The primary is `#1F5797`** — RustIO Blue, a restrained professional blue. It is
the family's shared default and replaces the v2.x burnt-copper rust (`#B84318`)
on every normal primary action. The `--rio-rust*` names are kept deliberately as
the established token contract (§1 preamble); the *name* is historical, the
*value* is blue.

**Focus is a distinct semantic role, not a shade of the primary.** `--focus`
(`#2F7BD6`) is deliberately not `--blue`: sharing the primary-button colour made
a focused control read as a button. `#2F7BD6` is ~1.72× lighter than `--blue` and
clears 3:1 against every surface the ring can sit beside. The ring is a **solid
3px outline at a 2px offset** (`outline: 3px solid var(--focus);
outline-offset: 2px`) — the v2.x 4px translucent glow
(`--rio-accent-ring`) is retired, having reached only ~1.54:1 as a 28% wash.

The accent is reserved for affordances — primary buttons, focus, active state,
links, dots, tints — and is never flood-filled across page chrome.

### 1.4 Status

| Canonical | Value | Use |
|---|---|---|
| `--green` / `--green-soft` / `--green-line` | `#19724B` / `#E8F6EE` / `#B4DBC5` | positive state ("Yes", "Active", allowed) |
| `--amber` / `--amber-soft` | `#935B0A` / `#FFF4DF` | warning |
| `--red` / `--red-soft` / `--red-line` | `#A23F3A` / `#FFF0EE` / `#E3B0AB` | destructive, invalid, denied |
| `--grey` / `--grey-soft` / `--grey-line` | `#4C5868` / `#F2F4F7` / `#CFD6DF` | neutral / off state ("No", "Inactive") |

Soft fills are **opaque**, not alpha washes. Red is reserved for genuinely
destructive actions and invalid state. The v2.x `--rio-danger` `#B42318` moves to
`#A23F3A`.

### 1.5 Control boundaries

| Canonical | Value | Use |
|---|---|---|
| `--field-line` | `#848F9E` | input / select / textarea border at rest (deepened for SC 1.4.11) |
| `--filter-line` / `--filter-fill` | `#7890AF` / `#F4F8FD` | the active-filter trigger treatment |

Fields stay clearly outlined rather than melting into the card.

### 1.6 Elevation

| Canonical | Use |
|---|---|
| `--shadow-sm` | `0 1px 2px rgba(22,32,56,.06)` — reserved for cards **in a grid** |
| `--shadow` | the one deep shadow, for the auth card only (`--rio-shadow-xl`) |

**Flat by default.** A white card on the `--page` ground with a 1px `--border` is
already separated; a shadow under it communicated no hierarchy. `--rio-shadow-md`,
`--rio-shadow-card` and `--rio-shadow-inset` are retired. `--rio-shadow-lg` stays
a product token for transient overlays (dropdown panels, popovers) — `main` has
no equivalent.

### 1.7 Geometry

| Canonical | Value |
|---|---|
| `--s1 … --s6` | 4 · 8 · 12 · 16 · 24 · 32 px |
| `--radius` | 14px — cards |
| `--radius-btn` | 8px — buttons and controls |
| `--radius-sm` | 6px — badges, small buttons, rail items, page links |
| `--gutter` | 32px · 24px ≤ 760 · 16px ≤ 480 |
| `--content` / `--content-wide` | 1120 / 1600px |
| `--masthead` | 56px |

`--radius-md` 9 / `--radius-lg` 12 / `--radius-xl` 16 / `--radius-pill` collapse
onto `--radius` 14 and `--radius-sm` 6. Off-scale spacing steps
(`--rio-space-2/6/20/40/48/64/80/96`) are legacy and are audited out as components
move onto `--s1 … --s6`.

## 2. Typography

```css
--rio-font-sans: "Inter", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
--rio-font-mono: "JetBrains Mono", ui-monospace, "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
```

**Inter** is the single Latin face for body **and** titles (no serif). Per-script
Arabic fallbacks (Noto Naskh display, Tajawal body/mono) are appended. Latin faces
are self-hosted from the binary — no CDN round-trip.

### 2.1 The ladder

Eight steps: **12 · 13 · 14 · 15 · 16 · 18 · 24 · 33** px.

| Role | Size / weight | Notes |
|---|---|---|
| Body, table cells, inputs, labels, field values | **15** / 500 | line-height 1.4 in cells, 1.55 in prose |
| Buttons | 14 / 680 | small buttons 13 / 650 |
| Mono micro-labels | **13** / 700 | uppercase, `letter-spacing: .08em`, `--ink-mono` — table heads, fieldset bands, `dl` labels, eyebrows, stat labels, rail sections, card heads |
| Breadcrumb | 13 / 600 | `--ink-2`; current segment `--ink` / 700 |
| Badges | 13 / 600 | |
| Secondary detail, counts, hints | 14 / 500–600 | |
| Page title (h1) | **24** / 800 | `letter-spacing: -.02em`, line-height 1.2 |
| h2 | 18 / 700 | `-.01em`, 1.3 |
| h3 | 16 / 700 | `-.01em`, 1.35 |
| Page-head lead | **15** | `max-width: 680px`, `--ink-2` |
| Display | 33 / 800 | the ladder's top step, `-.025em`. Reserved; rustio-admin's pages use the 24px title (§2.2) |

**There is no 14px floor.** The v2.x rule "no content-area UI text below 14px; no
11/12/13px anywhere" is **repealed**: 13px is the mono micro-label size across the
whole family and 12px is the ladder's bottom step. The floor that replaces it is a
*contrast and role* floor, not a size floor — operational text may not be pale
(§1.2), and 13px is reserved for the mono/micro roles named above rather than for
prose.

The names `--rio-text-12 … --rio-text-64` describe a pixel value that their
current values no longer match; Phase 2 re-points them onto this ladder
(§16).

### 2.2 Page header pattern

The page head is **not a card**. No white container, no radius, no shadow, no
rule beneath it. It is a grid:

```
"crumb  crumb"
"title  action"
"lead   lead"
```

- Breadcrumb on its own row (13 / 600).
- Title (24 / 800) and the primary action on one row: the action sits **one gap
  (`--s5`) after the title**, `justify-self: start` — beside the title, not pushed
  to the far edge.
- Lead beneath both, `max-width: 680px`, 15px, `--ink-2`.
- Below **760px**: one column, `"crumb" "title" "action" "lead"`.
- Below **480px**: the action goes full width.

Primary action nearest the title; secondary actions follow it. Matches `main`'s
`.page-head`. rustio-admin carries the breadcrumb in its own element
(`.rio-crumbs`) alongside `.rio-masthead-top`, so the crumb row sits outside the
grid rather than inside it — the arrangement is identical, the markup is
rustio-admin's.

`main`'s `.page-head--hero` (33px title, no action row) is **not** adopted:
rustio-admin's dashboard carries two actions, which the hero head does not seat.

## 3. Section patterns — three cases

The three cases stand. All three share the card shell: `--surface`, `--radius`
(14px), 1px `--border`, `--s5` (24px) padding, **flat** — no `--shadow-card`
(§1.6). A card in a grid may carry `--shadow-sm`.

### 3(a) Multi-section FORM pages → band inside the card

For forms with two or more named sections (`CONTENT`, `IDENTITY`, `MODE`,
`REASON`, `DURATION`, `PERMISSIONS`). The section label is a **mono band inside
the card** — 13 / 700 uppercase, `letter-spacing: .08em`, `--ink-mono`, on
`--surface-head` with a 1px `--border` beneath (`main`'s `.card-head`). Native
`<fieldset>`/`<legend>` markup is retained; the legend no longer cuts the card's
top border. Fieldsets carry `min-width: 0` so a wide child (the permission matrix)
cannot widen the page.

### 3(b) LIST / TABLE page sections → eyebrow heading

For content pages whose sections introduce a table or a sub-form (feature_flags'
`FLAGS` and `ADD`). The label is an **eyebrow + sub-heading stacked ABOVE the
card**, and the card itself is unlabeled:

- Eyebrow: 13 / 700, uppercase, `letter-spacing: .08em`, `--ink-mono`
  (`--blue` where it names the surface, as `main`'s `.eyebrow`). Margin-block-end `--s1`.
- Sub-heading: 18 / 700, `--ink`. Margin-block-end `--s4`.
- Then the card, unlabeled.

Do **not** convert these to a band.

### 3(c) Single-field-group cards on simple create forms → bare

When a create/edit form has exactly **one** field group and no second section
(`group_new`, `group_edit`'s name/description card, `password_change`), the card
carries no band and no eyebrow — fields sit directly inside it. A lone section
needs no name.

## 4. Labels, required markers, hints

Label 15 / 700 / `--ink`; required asterisk in `--red`; inline hint a
normal-weight parenthetical on the same line in `--ink-hint`. Gap label→control
`--s2`. Never let labels touch inputs.

Settled label fact: the password-change form's first field is **"Current
password"**, not "Old password".

## 5. Inputs, textareas, selects

**`--ctl` = 38px** min height. `--field-line` border, `--radius-btn` (8px), 15px
text. Focus: `--focus` border plus the solid 3px outline at 2px offset (§1.3) —
no inset shadow, no translucent ring. Mono placeholders for code identifiers
(slug, flag key). Textareas sit at a reading height rather than `--ctl`.

Booleans render as a **38px bordered row** (`main`'s `.field-boolean`): the
control and its label inside a `--field-line` box on `--surface`, hover
`--border-strong`, `focus-within` `--blue`. An 18px checkbox.

> **Repealed from v2.1:** "44px min height" and "36/38px control heights are
> forbidden". 38px is the family's standard control height. The v2.x rule was the
> single blocking conflict between this contract and the accepted direction, and
> it is withdrawn deliberately — see §16.

## 6. Radio rows

Full-width bordered rows, stacked, tappable end-to-end: `--s4` padding,
`--field-line` border, `--radius-btn` (8px), 18px `--blue` `accent-color` control
aligned to the first text line, `--s3` gap between rows. Primary phrase 700;
trailing description 400 in `--ink-body`. A radio row is a reading target rather
than a bare control, so it is not capped at `--ctl`.

## 7. Checkboxes

18px, **`--blue` `accent-color`** everywhere — including the permission grid.
(The v2.x rule said rust; the value is now blue. The rule is unchanged: one
accent, applied consistently, never native blue by accident.)

## 8. Buttons and the action bar

Three heights, all `--radius-btn` (8px):

| Token | Height | Use |
|---|---|---|
| `--ctl` | **38px** | standard button |
| `--ctl-sm` | **31px** | small action button, segmented-control segment, filter trigger, row action |
| `--ctl-util` | **32px** | compact masthead utility control (⌘K trigger, account menu, bell) |

Label 14 / 680; small buttons 13 / 650. Primary: `--blue` fill, white text, hover
`--blue-dark`. Secondary: `--surface`, `--ink`, `--border-strong` border. Danger
primary: solid `--red`. **Quiet** (`main`'s `.button-quiet`): transparent ground
and border, `--blue` text, hover `--blue-soft` — the face for row actions and
in-table affordances; its danger variant is `--red` text, hover `--red-soft`.

Destructive **secondary** is a red text action, never a solid red button beside
Save. History is a muted text action.

> **Repealed from v2.1:** "All buttons 44px / weight 700".

### 8.2 Action bar

One wrapping row with a hairline top border (`--border`), `--s2` margin-top,
`--s4` padding-top, `--s2` gap. Order: primary → secondary save variants → (auto
gap) → muted/destructive text actions + Cancel. No orphaned Cancel on its own
line. Below 480px the bar becomes a grid of full-width buttons.

## 9. Tables, rows, and badges

### 9.1 Rows

| Token | Height | Use |
|---|---|---|
| `--th-h` | **40px** | column head |
| `--td-h` | **48px** | data row minimum |

Head: `--surface-head` ground, 13 / 700 mono, uppercase, `letter-spacing: .08em`,
`--ink-body`, nowrap. Cells: 15 / 500, `--ink`, line-height 1.4, `--s4` padding.
Soft `--border` dividers, hover `--surface-soft`, **no zebra**.

The dense **Compact** view reuses `--th-h` (40px) as its line height and carries
**no column head** (`main`'s `.record-compact-item`). The **List** view is one
surface with 64px rows. There are no separate compact-row tokens.

### 9.2 Cell behaviour

- Identity columns whose every value is a single unbreakable token **hug** their
  content: `width: 1%; white-space: nowrap` (`main`'s `.cell-fit`, decided
  server-side as `main` decides it in `column_hugs`).
- Numeric cells are right-aligned with tabular digits (`main`'s `.cell-num`).
- Time cells are nowrap with tabular digits (`main`'s `.record-list-time`,
  `.table-audit`).
- The actions column is `width: 1%`, nowrap, right-aligned.

### 9.3 Badges — two faces, and only two

**Status face** (`main`'s `.badge`): a 6px dot in `currentColor`, a word, and an
opaque soft fill. `--radius-sm`, 13 / 600, `padding: --s1 --s2`, `border: 1px
solid transparent` — the border stays transparent so geometry is unchanged, but
**a status badge carries no coloured outline**. A coloured 1px line made every
status a small framed object and a table of twenty of them a wall of frames. Pairs:
`--green-soft`/`--green`, `--red-soft`/`--red`, `--blue-soft`/`--blue`,
`--grey-soft`/`--grey`, `--amber-soft`/`--amber`.

**Count face** (`main`'s `.module-link .badge`): bordered, `--surface` ground,
`--border` line, mono 13 / 700, `--ink-body`, **no dot**. For nav counts, tab
counts and dropdown counts — a quantity, not a state.

Status-pill text is **lowercase** (`enabled` / `disabled`) — set where the pill is
emitted, not by capitalizing the source string.

### 9.4 Row actions

Quiet text actions (§8), **always visible**. Not icon-only, and not revealed on
hover: a hover-revealed control is undiscoverable and unreachable by touch.

## 10. Layout primitives

The measure is chosen per page by the `page_measure` mechanism, which stays
rustio-admin's own:

| Measure | Width | Use |
|---|---|---|
| wide | **1600px** | lists, tables, reports, bulk surfaces |
| standard | **1120px** | default page measure |
| narrow | 920px | reading pages |
| form | **728px**, centred | forms, confirmations (§15.1) |

Geometry: `width: min(100% - 2 * var(--gutter), <measure>); margin: 0 auto` —
`--gutter` 32 / 24 / 16 (§1.7). Shell: masthead `--masthead` 56px, module row
40px, rail 240px with 38px items and no active bar (tint only), captions 13px
mono.

- Two-column field grid: `1fr 1fr`, gap `--s5`, stacks at 768px.
- Card grid: `repeat(auto-fill, minmax(min(100%, 300px), 1fr))`. The
  `min(100%, …)` guard is required — it is what keeps a card from overflowing
  below 300px.
- Inline code/kbd chip: `--rio-surface-tint` ground, mono, `--radius-sm`.
- Every horizontally scrollable region (tables, matrices, wide diagrams) scrolls
  inside its own container; the document itself never scrolls horizontally at any
  width.

The v2.x `.rio-form-shell` (880px) and `.rio-content-shell` (1040px) primitives
are superseded by `page_measure`.

## 11. Known defects and documented deferrals

1. Permission-grid checkboxes must be `--blue`, never native blue.
2. An action bar must be one wrapping row — no orphaned Cancel on a second line.
3. A radio control is centred on its first text line, not on the row.

### 11.1 Documented deferral — user_new "Active" checkbox

`user_new` pairs **Role | Active** (2-col) in the IDENTITY section. The user model
carries `is_active`, but `create_user` always inserts `is_active = TRUE` — a
functional Active toggle on the **create** form would need new create-path
behaviour (and a product decision on whether the admin should mint inactive
accounts). It is **deferred, not built**: the create form shows Role full-width
and omits Active. This is an adjudicated deviation, not an oversight. (An *edit*
form, where `is_active` is real existing state, may surface it later.)

## 12. Light only, and RTL

**The admin is light-only.** One calm, light surface everywhere: no dark theme, no
`@media (prefers-color-scheme: dark)` block, no `[data-theme]` block, no pre-paint
theme script. `:root` declares `color-scheme: light`.

> **Repealed from v2.1:** "Dark theme is **mandatory** and token-driven only",
> the slate dark ladder, and the dark half of the §13 checklist. The shipped
> stylesheets have been light-only since the blue-accent pass; v2.1 §12 described
> a theme that does not exist. This is a correction of the record, not a removal
> of a feature.

Consequences recorded here so they are not rediscovered: a generated `tokens.css`
override needs no dark blocks to compose correctly, and no component may carry
per-theme CSS. `TOKENS-EMIT-SPEC.md` still describes a dark-aware emission
contract; reconciling it is the emitter pass (§16), and until then **this
contract wins** on whether a dark theme exists.

RTL: all new CSS uses logical properties (`margin-inline-start`, `text-align:
start`). `letter-spacing` is neutralized to 0 for Arabic/RTL on
`.rio-fieldset > legend` and `.rio-table th` — a connected script breaks under
tracking. Directional icons flip exactly once.

## 13. Acceptance checklist

Verify each touched page **light/LTR**, then **light/RTL**, at 1440px and 390px:

- Page-head grid (§2.2) — no card, action beside the title, lead beneath, one
  column below 760.
- Section pattern matches its §3 case (band for multi-section forms,
  eyebrow-heading for list/table sections, bare for single-group create cards).
- Flat cards on the §1.1 surfaces, `--radius` 14, 1px `--border`.
- 38px controls (§5) with the solid 3px focus ring at 2px offset; 31px small controls;
  32px masthead utility controls.
- Blue `accent-color` on every checkbox and radio.
- The §8 button/action-bar taxonomy; quiet, always-visible row actions.
- §9 tables: 48px rows, 40px mono heads at 13px, 15px cells, no zebra, the two
  badge faces, lowercase status text.
- Mono micro-labels at 13px where §2.1 names them; no pale operational text.
- **No document-level horizontal overflow at any width.**
- Chrome (masthead, rail, footer) and every product capability unchanged.

## 14. Implementation approach

Update **tokens**, not scattered selectors. Map existing class names onto this
contract before inventing new ones. Hand-written CSS only — no build step, no
Tailwind, no PostCSS, no bundler, no JS framework, no second runtime.
Postgres-only, baked stylesheets. A new `--rio-*` token ships with a CHANGELOG
entry.

## 15. Intentional divergences from RustIO `main`

Recorded so they are decisions, not drift. Both are rustio-admin's existing
behaviour, kept on purpose.

### 15.1 The form measure is centred

`main` renders a 728px form card **left-aligned** inside the 1120px `--content`
column — `.form-layout` with a "System" aside for readonly fields, or
`.form-layout--single` without one. rustio-admin has no such aside and already
centres its form page on its own `page_measure`. It keeps the centred measure and
takes `main`'s **card width**: `--rio-page-form` moves 672 → **728px** (`main`'s
`680 + 2 × --s5`). `--rio-page-form` stays a product token; `main` has no
form-measure token to align with.

### 15.2 `AdminTheme` never retargets focus

On `main`, a project's `rustio.design.json` injects `--blue`, `--blue-dark` and
`--focus` (`shell.rs`, every shell page) or `--blue` and `--focus`
(`session.rs`, the sign-out page). rustio-admin's `AdminTheme` overrides the
primary and **never sets `--rio-accent-focus`**. That stays: focus is a distinct
semantic role (§1.3) with a verified contrast budget, and a project brand colour
is not required to clear it. The shared *default* is one blue with a distinct
focus; the override mechanisms stay distinct.

### 15.3 Product-specific components keep their composition

rustio-admin has capabilities `main` does not: the three-measure `page_measure`
system, the two-row header, the ⌘K palette, filter dropdown panels with
active-filter pills, bulk selection, the users grid, inline related sections, the
permission matrix, the session list, API cards, history diffs, the docs viewer,
the view designer, the full-width footer chrome, `AdminTheme` and `rio-theme`.

These take the **scale and the faces** from this contract — type, control heights,
row heights, radii, badges, empty states — and **keep their composition**. Aligning
them is not a licence to remove a capability, a route, a handler, or a
`data-rio-*` hook. Where this contract is silent on such a component, it stays as
it is.

## 16. What this contract records versus what has landed

This is version 3.0 of a **contract**, published ahead of the stylesheets it
governs. It is the accepted target, and it is binding on new and changed CSS from
now on. The runtime reaches it in phases
(`docs/design/rustio-admin-composition-study/MIGRATION.md`): tokens, then the type
scale, then control and row heights, then the page head and the list page, then
row-level rules, then the product pages, then the `rio-theme` emitter and
`TOKENS-EMIT-SPEC.md`.

Until a phase lands, the stylesheets will disagree with this document on the
values that phase owns. That is expected, and this contract is the authority on
the target. Three known pieces of drift are **recorded and not yet edited**: the
`tokens/compat.css` header comment still says "Teal"; `docs/assets/admin-shop.png`
shows the pre-recomposition admin; and `TOKENS-EMIT-SPEC.md` still requires dark
blocks (§12).
