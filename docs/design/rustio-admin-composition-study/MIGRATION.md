# Migration proposal — Phases 0–7

A proposal, not a plan in motion. Nothing below has started. Each phase
leaves the admin shippable; each is one or a few commits; the order is the
order of dependency, not of visibility. The first thing it changes is a
document.

```
0  documents            contract, doctrine, compat header, emit spec — the decision
1  tokens               canonical names + --rio-* aliases, values
2  type scale           the --rio-text-* values
3  control heights      buttons, inputs, rail, utility, rows
4  page head + list     band with rule · find row · data-surface head
5  row-level rules      badges · row actions · cells by kind · sticky actions · timestamps
6  product pages        one page per commit
7  emitter + tests      rio-theme, emit spec, contract test suites
```

## Phase 0 — Documents (decision)

- Amend `docs/design/VISUAL-CONTRACT.md` to `d4b61fa` (blue accent, no dark
  theme) **and** to the 3.1 scale: 38 / 31px controls, 13px mono labels, 24px
  titles, 48px rows. This is the decision the whole proposal depends on; it
  is made here, in writing, before any CSS moves.
- Amend `DESIGN_DOCTRINE.md` (slate / dark chrome) and the `compat.css`
  header ("Teal"). Replace `docs/assets/admin-shop.png` when Phase 4 lands.
- Record in this study's `README.md` which RustIO branch the family follows
  for the page head and the toolbar once RustIO resolves PR #5 / PR #6.

Risk: none to code. Cost: the one real argument, held early.

## Phase 1 — Tokens

- In `tokens/colors.css`, `spacing.css`, `radius.css`, `shadows.css`: declare
  the canonical names with the 3.1 values and redefine every `--rio-*` name
  as `var(--canonical)`. Add `--gutter`, `--form-measure`, `--th-h`,
  `--td-h`, `--th-h-compact`, `--td-h-compact`, `--ctl`, `--ctl-sm`,
  `--ctl-utility`.
- Value changes land here: the ink ramp (no pure black), `--red`, the soft
  fills as opaque colours, `--rio-page-wide` 1600, `--rio-page-form` 728.
- `--rio-accent-focus` keeps its value and stays distinct from `--blue`.
- `AdminTheme` keeps working unchanged: it sets `--rio-rust*`, which are now
  aliases that the components still read.

Verification: every page renders; only colours and two measures changed.

## Phase 2 — Type scale

- Re-point the `--rio-text-*` values to the 3.1 scale. Base 15. Titles 24.
  Mono labels 13.
- Look at every page before Phase 3. This is the phase where perceived
  hierarchy changes most; it lands alone so it can be judged alone.

## Phase 3 — Control heights and rows

- Replace the ~12 hard-coded 44 / 46px rules with `var(--ctl)`; `--sm`
  variants with `var(--ctl-sm)`; the utility-row controls with
  `var(--ctl-utility)`.
- Rows `--td-h` / `--th-h`; Compact `--td-h-compact` / `--th-h-compact`.
- Rail 240, items 38, no left bar; utility 56, module row 40.
- Textareas at a reading height; the boolean field as a 38px bordered row.

Verification: no control under 31px; no text smaller at 390 than at 1920.

## Phase 4 — Page head and list page

- `components/page-header.css`: remove the card from both markup shapes;
  band with a rule; form pages without the rule. CSS-only; both shapes stay.
- `list.html` + `pages/list.css`: search 38 × 340; filter triggers
  `rio-btn--sm`; the result count after the last control; Sort / direction /
  rows-per-page and the view-mode switch move to a `.rio-board-head` on the
  surface; bulk bar as a strip. Every hidden input, `_csrf`, `sort` / `dir` /
  `per_page` carry-over, `data-rio-*` hook and route stays byte-identical.
- Empty states flat, inside the surface.

Verification: the Data table and Comparison boards, at 1440, 900, 390.

## Phase 5 — Row-level rules

- One badge face for `.rio-pill`, `.rio-role`, `.av-badge`.
- `_row_actions.html`: text Edit / Delete, quiet, always visible; the users
  grid already is the reference.
- `components/data.css`: `rio-td--date|datetime|boolean` nowrap; tabular
  digits; numeric right; `.rio-cell-fit` for hugging identity columns;
  actions column sticky inside `.rio-tablewrap`; `.rio-tablewrap { position:
  relative }`; `.rio-fieldset { min-width: 0 }`.
- Timestamps: one formatter for cells (`YYYY-MM-DD HH:MM UTC`) used by the
  model list, users, sessions and the footer; relative time stays for
  History "when", "last seen", "expires". **The only Rust change in the
  proposal**, with its tests updated in the same commit (the list-timestamp
  test from PR #154 pins the current date-only output and must be
  retargeted, not skipped).

Verification: the Dense: Orders board; no cell wraps a date; the actions
column is reachable at every width.

## Phase 6 — Product pages, one per commit

Sessions (five cards → one surface) · user detail (tabs, dl) · group editor
and permission matrix · API surface (identity floor) · dashboard (flat tiles,
no monogram tiles) · history · health · DB browser · feature flags ·
notifications · docs viewer · view designer · branding · MFA and auth pages.
Each takes scale and faces only; each keeps its markup and behaviour.

## Phase 7 — Emitter, spec, contract tests

- `rio-theme`: emit canonical names with `--rio-*` aliases; keep the hover /
  active shades it computes and let `_theme.html` consume them instead of
  writing one hex six times (the AdminTheme IMPROVE). Dark blocks: keep
  emitting only what the runtime accepts.
- `docs/design/TOKENS-EMIT-SPEC.md`: the alias pass.
- Contract test suites: update the assertions that pin 44px controls, 14px
  floors, the boxed page header and the date-only list cell. Retarget; never
  skip or `#[ignore]`.

## What this migration must not do

Add product capability, remove a component, change a route or a handler,
touch `AdminTheme`'s surface, touch the Postgres-only / no-build-step rules,
or make the admin depend on RustIO's build (the Tailwind pass stays RustIO's).

## Dependencies outside this repository

- RustIO's PR #5 / PR #6 resolution (page head, toolbar). Two boards depend
  on it; Phase 4 waits for it or follows this study's pick and says so.
- Nothing else. The token values are already shared; no RustIO artefact is
  imported.
