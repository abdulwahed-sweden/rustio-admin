# Migration proposal — Phases 0–7 (v3)

A proposal, not a plan in motion. Nothing below has started. "ALIGN" means
RustIO `main @ 00b933c`. Each phase leaves the admin shippable.

```
0  documents            contract, doctrine, compat header, emit spec — the decision
1  tokens               canonical names + --rio-* aliases, values, --gutter
2  type scale           the --rio-text-* values
3  control heights      buttons, inputs, rail, utility, rows
4  page head + list     main's .page-head grid · .find · .data-head · Compact as lines
5  row-level rules      badges · row actions · hug · cells by kind · sticky · timestamps
6  product pages        one page per commit
7  emitter + tests      rio-theme, emit spec, contract test suites
```

## Phase 0 — Documents (decision)

- Amend `docs/design/VISUAL-CONTRACT.md` to `d4b61fa` (blue accent, no dark
  theme) **and** to the shared scale: 38 / 31px controls, 13px mono labels,
  24px titles, 48px rows. The whole proposal depends on this one decision.
- Amend `DESIGN_DOCTRINE.md` (slate / dark) and the `compat.css` header
  ("Teal"). Replace `docs/assets/admin-shop.png` when Phase 4 lands.
- Record the two intentional divergences from `main` (centred form measure;
  `AdminTheme` never retargets focus) in the contract so they are decisions,
  not drift.

## Phase 1 — Tokens

- Declare canonical names with `main`'s values in `tokens/`; redefine every
  `--rio-*` as `var(--canonical)`. Add `--gutter`, `--th-h`, `--td-h`,
  `--ctl`, `--ctl-sm`, `--ctl-utility`. **No compact tokens, no
  `--form-measure`** (neither is on `main`).
- Value changes: ink ramp, `--red`, opaque soft fills, `--rio-page-wide`
  1600, `--rio-page-form` 728.
- `--rio-accent-focus` keeps its value; `AdminTheme` keeps working.

## Phase 2 — Type scale

- Re-point `--rio-text-*` to the shared scale. Base 15, titles 24, mono 13.
  Land alone; look before Phase 3.

## Phase 3 — Control heights and rows

- Replace the hard-coded 44 / 46px rules with `var(--ctl)` and `--ctl-sm`;
  utility-row controls with `--ctl-utility`. Rows `--td-h` / `--th-h`.
- Rail 240, items 38, no bar; utility 56, module row 40.
- Textareas at a reading height; boolean as a 38px bordered row.

## Phase 4 — Page head and list page

- `components/page-header.css`: remove the card from both markup shapes;
  `main`'s grid — `"title action" / "lead lead"`, `display: contents` on the
  wrapper, action `justify-self: start`, lead beneath. One column below
  760; action full width below 480. CSS-only.
- `list.html` + `pages/list.css`: search 38 × 340; triggers `rio-btn--sm`;
  result count after the last control; Sort / direction / rows-per-page and
  the view-mode switch move to a `.rio-board-head`; bulk bar as a strip.
  Every hidden input, `_csrf`, `sort` / `dir` / `per_page` carry-over,
  `data-rio-*` hook and route stays byte-identical.
- `components/adaptive-views.css`: Compact = `av-list--compact` rows at
  `--th-h`, no head; Cards flat with `auto-fill minmax(300px, 1fr)`; List as
  one surface.
- Empty states flat, inside the surface.

## Phase 5 — Row-level rules

- One badge face for `.rio-pill`, `.rio-role`, `.av-badge`.
- `_row_actions.html`: text Edit / Delete, quiet, always visible.
- `components/data.css`: `.rio-cell-fit` (ALIGN, `main`'s `.cell-fit`);
  `rio-td--date|datetime|boolean` nowrap, tabular digits, numeric right
  (IMPROVE); actions column sticky inside `.rio-tablewrap`; `.rio-tablewrap
  { position: relative }`; `.rio-fieldset { min-width: 0 }` (IMPROVE).
- Which columns hug is a `ConcreteOps` decision later (as `main` decides it
  in `column_hugs`); Phase 5 adds the class and applies it where the users
  grid already fixes its identity track.
- Timestamps: one cell formatter (`YYYY-MM-DD HH:MM UTC`) for the model
  list, users, sessions and the footer; relative time stays for History
  "when", "last seen", "expires". **The only Rust change**, with the PR #154
  list-timestamp test retargeted in the same commit — never skipped.

## Phase 6 — Product pages, one per commit

Sessions · user detail · group editor and matrix · API surface (identity
floor) · dashboard · history · health · DB browser · feature flags ·
notifications · docs viewer · view designer · branding · MFA and auth.
Each takes scale and faces only.

## Phase 7 — Emitter, spec, contract tests

- `rio-theme`: emit canonical names with `--rio-*` aliases; let
  `_theme.html` consume the computed hover / active shades instead of one
  hex six times. `--rio-accent-focus` stays untouched by the override.
- `docs/design/TOKENS-EMIT-SPEC.md`: the alias pass.
- Contract tests: retarget assertions that pin 44px controls, 14px floors,
  the boxed page header, the date-only list cell. Never skip or `#[ignore]`.

## What this migration must not do

Add product capability, remove a component, change a route or a handler,
touch `AdminTheme`'s surface, touch the Postgres-only / no-build-step rules,
or make the admin depend on RustIO's build.

## Dependencies outside this repository

None blocking. `main @ 00b933c` is the reference; if `main` later moves the
page head or the form layout, Phase 4 follows it. No RustIO artefact is
imported.
