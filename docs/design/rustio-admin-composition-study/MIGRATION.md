# Migration proposal — Phases 0–7 (v3)

A proposal, not a plan in motion. Nothing below has started. "ALIGN" means
RustIO `main @ 00b933c`. Each phase leaves the admin shippable.

```
0  documents            contract, doctrine, compat header, emit spec — the decision
1  tokens               canonical names + --rio-* aliases, values, --gutter
2  type scale           the --rio-text-* values
3  control heights      buttons, inputs, rail, utility, rows
4a page head           main's .page-head grid; the page-header invariant test reviewed
4b list page            .find · .data-head · Compact as lines · empty states
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

- Declare the full canonical set from `TOKEN-MAPPING.md` → "The canonical
  set Phase 1 declares" in `tokens/`, with `main`'s values: surfaces, lines,
  rail, the ink ramp including `--ink-mono`, the accent and `--focus`, the
  green / amber / red / grey pairs with `--green-line`, `--red-line`,
  `--grey-line`, `--field-line`, `--filter-line`, `--filter-fill`,
  `--shadow`, `--shadow-sm`, `--masthead`, `--ctl`, `--ctl-sm`,
  `--ctl-util`, `--th-h`, `--td-h`, `--gutter`, `--content`,
  `--content-wide`, the radii and `--s1 … --s6`. Redefine every `--rio-*` as
  `var(--canonical)`; split `--rio-text-faint` consumers between
  `--ink-hint` and `--ink-3` by use-site. **No compact tokens, no
  form-measure token** (neither is on `main`); `--rio-page-form` stays a
  product token.
- If the canonical declarations go into a new fragment (e.g.
  `tokens/canonical.css`) rather than into the existing token files, the
  `@import` list in `admin/admin.css` and the `ADMIN_CSS` `concat!` block in
  `src/admin/routes.rs` must be updated together, in the same commit —
  `every_css_import_resolves_to_a_file_on_disk` in `contract_assets.rs`
  guards the first; the lock-step with the second is by hand.
- Value changes: ink ramp, `--red`, opaque soft fills, `--rio-page-wide`
  1600, `--rio-page-form` 728.
- `--rio-accent-focus` keeps its value; `AdminTheme` keeps working.

## Phase 2 — Type scale

- Re-point `--rio-text-*` to the shared scale. Base 15, titles 24, mono 13.
  Land alone; look before Phase 3.

## Phase 3 — Control heights and rows

- Replace the hard-coded 44 / 46px rules with `var(--ctl)` and `--ctl-sm`;
  utility-row controls with `--ctl-util`. Rows `--td-h` / `--th-h`.
- Rail 240, items 38, no bar; utility 56, module row 40.
- Textareas at a reading height; boolean as a 38px bordered row.

## Phase 4a — Page head

- `components/page-header.css`: remove the card from both markup shapes;
  `main`'s grid — `"title action" / "lead lead"`, `display: contents` on the
  wrapper, action `justify-self: start`, lead beneath. One column below
  760; action full width below 480. CSS-only; the dashboard keeps the
  standard head (its two actions rule out `main`'s `.page-head--hero`).
- The one existing page-header contract test,
  `the_page_header_halves_are_a_matched_pair` in
  `tests/contract_assets.rs`, asserts that `.rio-crumbs` and
  `.rio-masthead-top` appear together in every template because
  `components/page-header.css` styles them as one capped box. The markup
  pair stays, so the assertion should still hold; its doc comment and the
  CSS comment it cites describe the box and are updated **in this phase**,
  and the test is re-run before the commit. It is not retargeted and not
  skipped.

## Phase 4b — List page

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

- One status-badge face for `.rio-pill` (`components/data.css`),
  `.rio-role` (`pages/list.css`) and `.av-badge`
  (`components/adaptive-views.css`); the count badges
  (`.rio-dropdown-badge`, tab counts) take the second, bordered face.
- `_row_actions.html`: text Edit / Delete, quiet, always visible. The
  `opacity: 0` / hover-reveal rules to remove are in `layout/console.css`;
  the users-grid override that already forces them visible is in
  `pages/list.css`.
- `components/data.css`: `.rio-cell-fit` (ALIGN, `main`'s `.cell-fit`);
  numeric right with tabular digits and time cells nowrap (ALIGN, the
  behaviour of `main`'s `.cell-num` / `.record-*-time` / `.table-audit`),
  applied automatically from `rio-td--date|datetime|boolean|integer|decimal`
  (IMPROVE).
- Sticky actions column (IMPROVE), as one rule plus its companions, or it
  is wrong: `th.col-act, td.col-act { position: sticky; right: 0 }`; each
  cell carries its own surface — `td` `--surface`, `th` `--surface-head`,
  `tr:hover td` `--surface-soft`, `tr[aria-selected] td` and its hover
  `--blue-soft`; `box-shadow: -1px 0 0 var(--border)` as the left edge
  line; `z-index: 1` on cells and `2` on the head cell so the corner stacks
  above the sticky `thead` already in `components/data.css`; and
  `.rio-tablewrap { position: relative }` in `pages/list.css` so the
  absolutely positioned `.rio-sr-only` label in the actions head is clipped
  by the scroller.
- `.rio-fieldset { min-width: 0 }` in `pages/form.css` (IMPROVE).
- Which columns hug is a `ConcreteOps` decision later (as `main` decides it
  in `column_hugs`); Phase 5 adds the class and applies it where the users
  grid already fixes its identity track.
- Timestamps: the model list already renders `YYYY-MM-DD HH:MM UTC`
  (`humanise_timestamp_cell`, PR #154) and user detail and the footer print
  the same; nothing there changes and its test
  (`a_timestamp_cell_reads_as_a_date_not_a_wire_format`) stays as it is.
  Exactly two call sites are brought to the format: the users list
  `created_at` in `admin/builtin.rs` and `AccountSessionRowCtx.created_at`
  in `admin/render.rs` — each a one-token change from `%Y-%m-%d %H:%M` to
  `%Y-%m-%d %H:%M UTC`. Relative time stays for History "when", "last seen",
  "expires". **The only Rust change in the proposal.**

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
- Contract tests: **no existing test pins 44px controls, a 14px floor, the
  boxed page header or a date-only list cell** — those values live only in
  `VISUAL-CONTRACT.md` (Phase 0) and in CSS. The CSS-facing tests that do
  exist are in `tests/contract_assets.rs`: the embedded-template inventory,
  the override ladder, `every_css_import_resolves_to_a_file_on_disk`, the
  RTL icon flip, the JS bundle, and the page-header pair (Phase 4a). Phase 7
  *adds* assertions for the new state if wanted — that the canonical tokens
  are declared and that every `--rio-*` alias resolves — rather than
  retargeting anything. Never skip or `#[ignore]`.

## What this migration must not do

Add product capability, remove a component, change a route or a handler,
touch `AdminTheme`'s surface, touch the Postgres-only / no-build-step rules,
or make the admin depend on RustIO's build.

## Dependencies outside this repository

None blocking. `main @ 00b933c` is the reference; if `main` later moves the
page head or the form layout, Phase 4 follows it. No RustIO artefact is
imported.
