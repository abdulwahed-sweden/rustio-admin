# rustio-admin × RustIO Composition 3.1 — design study

Read-only. Inspected `abdulwahed-sweden/rustio-admin` at `d4b61fa` (HEAD of
`main`, 0.33.1 unreleased) against `rustio` `docs/design/admin-composition-3.1`.
No file in either repository was changed. Boards: `boards/index.html`.

## Final answer

**Yes — rustio-admin can adopt Composition 3.1 visually without losing its
identity or functionality.** The palette already matches value for value;
every richer component (module row, ⌘K palette, filter dropdowns, bulk bar,
permission matrix, sessions, API cards, history diffs, three save variants)
keeps its markup and behaviour and only takes the shared scale, faces and
composition. One thing blocks it, and it is a document: rustio-admin's
`VISUAL-CONTRACT.md` v2.1 mandates 44px controls and forbids text under 14px
and any 36/38px control — the exact values RustIO 3.1 is built on. The
contract is already stale (it describes a copper accent, a teal predecessor and
a dark theme that HEAD no longer has), so amending it is step 0, not a cost of
this proposal.

## 1 · Architecture comparison

```
RustIO                                   rustio-admin
masthead 56 (brand · identity · log out) utility 52 (env · ⌘K · bell · Docs · account menu)
                                         + module row 48 (Dashboard · Models · Admin · Security · System)
rail 240: models + System pages          rail 232: destinations within the active module
main: one page = one measure             main: page_measure block → wide 1480 / standard 1120 / narrow 920 / form 672
  centred, gutter 32 (--gutter)            centred, calc(100% - 64px)  ← same geometry
footer inside the measure                footer full-width chrome with system links + identity + time
one CSS file, Tailwind pass              26 CSS fragments concatenated, no build step
templates: 18                            templates: 58 (45 pages)
```

Both shells are structurally sound. rustio-admin's three-measure system and
32px gutter are already the 3.1 principle ("centred inside the workspace after
the rail, not glued to it"). Nothing in the shell architecture needs to move.

## 2 · Visual compatibility

| Surface | A/B/C/D | Note |
|---|---|---|
| Shell | B | header 56 + 40 instead of 52 + 48; rail 240; rail head level with the utility row |
| Sidebar | A/B | already light with blue-soft active; items 38px, drop the 3px bar |
| Topbar | C (+B faces) | module row, palette trigger and account menu are rustio-admin IA; controls at 32px |
| Page width | A | same mechanism; wide 1480→1600 is one number |
| Page head | B | unboxed; 24px title; action one gap after it |
| Tables | B | 48px rows, 40px mono heads, 16px cells; text actions always visible |
| Forms | B/C | 38px controls, band headings; the three save variants stay |
| Cards | B/C | same card types, flat, radius 14, 16px padding |
| Badges | B | pills → dot badges; lowercase values kept where settled |
| Buttons | B/C | 38/31px; ghost→secondary, subtle→quiet; solid danger stays for one-action pages |
| Search | C/B | palette stays; list search → 340px find field |
| Filters | C/B | dropdown panels stay; triggers become 31px small controls on the find row |
| Footer | C | content stays; ink and type on the 3.1 ramp |
| Account controls | C | dropdown stays |
| Empty states | B | flat inside the surface, 32px padding, no icon tile |
| Responsive | A | rail strip at 1040, local scrolling; gutters 24/16 |
| Type scale + control heights | **D** | 16px/44px contract vs 15px/38px — the one unification decision |

## 3 · Token mapping

See `boards/index.html` → D for the full table with notes. Summary:

- Surfaces, lines, rail, blue, blue-soft, blue-line, focus: **exact** value
  matches. Rename by alias only.
- Ink: `--rio-text-hi #000000 → --ink #171B22`, `--rio-text #0B0C0E →
  --ink-body #202733`, `--rio-text-mute #30343A → --ink-2 #3F4A59`,
  `--rio-text-faint → --ink-hint` (hints, placeholders) and `--ink-3`
  (tertiary metadata only). Value changes; 3.1 never uses pure black.
- Status: success/warn near-exact; danger `#B42318 → --red #A23F3A`; alpha
  tints → opaque soft fills.
- No safe equivalent (keep as product tokens): `--rio-rust-active`,
  `--rio-on-solid`, `--rio-overlay`, `--rio-shadow-lg` (popovers, palette),
  `--rio-syntax-*`, `--rio-hit-*`, `--rio-topbar-h`, `--rio-modulebar-h`.
- Retire: `--rio-rust-ring` (glow → solid 3px ring), `--rio-rust-solid*`
  (two accent roles → one), `--rio-radius-md/lg/xl/pill` → `--radius 14`,
  `--rio-shadow-md/card/inset/xl` (cards go flat).
- Spacing: `--rio-space-{4,8,12,16,24,32}` map 1:1; 2, 6, 20, 40, 48, 64, 80,
  96 are off-scale and need an audit (20 and 48 are the common ones).
- Typography: `--rio-text-12…64` names say px, values do not
  (`--rio-text-12` renders 15px). Re-pointing the values re-scales every
  component from one file; the hard-coded control heights do not.

Shape: canonical names carry values, `--rio-*` names are aliases — the
two-layer shape `tokens/compat.css` already has. `AdminTheme` overrides and
every downstream `var(--rio-rust)` keep working.

## 4 · Primary colour — one proposal

**Keep `#1F5797`.** rustio-admin already uses RustIO's blue; sharing it is the
family resemblance. The one recorded alternative, `#0F5FA8` (hover `#0B4A84`,
soft `#DCEBFB`, line `#A9CBEF`, focus `#2D7FDC`), measures white-on-fill 6.5:1
(current 7.3), on the page 5.8:1 (6.5), ring on blue-soft 3.3:1 (3.5): all pass,
all slightly lower; marginally livelier, marginally less serious; status
separation unchanged since green/amber/red do not move. Not worth it for an
all-day admin. If ever adopted, adopt in RustIO first and inherit through the
aliases.

## 5 · Boards

`boards/index.html` → Shell · Dashboard · Data table · Users & groups · API
surface · Account & sessions · Form · History (dense) · Narrow. The mockups
render rustio-admin's own templates' markup (`.rio-*` classes) against a mock
stylesheet (`build/ra_mock.css`) that expresses the 3.1 grammar. It is a
design artefact, not product CSS.

## 6 · Converge / 7 · Remain

Converge: page head, find row + data-surface head, table geometry and row
actions, badges, buttons/inputs, cards, fieldset bands, empty states, ink and
shadows, rail item geometry.

Remain: two-row header and module row, ⌘K palette, account menu, filter
dropdown panels, sort/direction/rows-per-page, bulk selection, three save
variants and the History/Delete/Cancel text actions, inline related sections,
permission matrix, session list, API cards, history diffs, health, DB browser,
feature flags, notifications, docs viewer, view designer, full-width footer,
`AdminTheme` surface, `rio-theme` emission.

## 8 · Difficulty: medium

Tokens and faces: easy. Type scale: easy mechanically, medium visually (45
pages re-scale at once). Control heights: medium (~12 hard-coded rules).
Page head: medium (two markup shapes, 39 templates, CSS-only if wrappers stay).
List page composition: medium. Product pages: medium, page by page. Emitter and
docs: medium.

## 9 · Risks

1. The contract conflict (44px / 14px floor vs 38px / 13px) must be decided
   before step 1, not discovered during it.
2. Downstream projects override `--rio-*` via `AdminTheme`; the names must stay
   as aliases permanently.
3. `rio-theme` emits legacy names and a dark block the runtime already warns
   about; the emitter and `TOKENS-EMIT-SPEC.md` need the same pass.
4. 36px → 24px titles change perceived hierarchy everywhere; land the scale
   alone and look before continuing.
5. Text row actions widen the actions column; the users grid already does
   this and reads fine.
6. Documentation drift is real today (`VISUAL-CONTRACT.md`, `DESIGN_DOCTRINE.md`,
   `compat.css` header, `docs/assets/admin-shop.png`); unification without
   fixing it will drift again.

## 10 · Recommended order

0. Amend the contract docs to HEAD and to the 3.1 scale.
1. Token values + canonical names with `--rio-*` aliases.
2. Type scale values.
3. Control heights (buttons, inputs, rail, utility).
4. Shell heights and rail head.
5. Page head, both markup shapes.
6. List page: find row + data-surface head.
7. Row actions as quiet text.
8. Badges, pills, role chips.
9. Forms: bands, action bar, checkbox control.
10. Product pages one by one.
11. `rio-theme` emitter + emit spec.
12. Contract test suites.
