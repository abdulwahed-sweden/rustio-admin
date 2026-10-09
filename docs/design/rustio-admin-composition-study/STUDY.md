# Study v3 — rustio-admin × RustIO composition (`main @ 00b933c`)

Supersedes `v2/STUDY.md` on the points in `CRITIQUE.md` §G–H. Baselines in
`README.md`.

## Final answer

**Unchanged: yes.** rustio-admin adopts RustIO's composition, scale and faces
without losing a component, a route or a behaviour. The palette already
matches value for value; `main` changed no token value. The gap is scale
(16px / 44px against 15px / 38px), boxing (a white card around every page
head), density (56–64px rows) and three small systems that grew separately
(badges, row actions, timestamp formats). All of it is CSS and template
arrangement, plus one Rust change (the timestamp formatters).

With `main` authoritative the proposal is **smaller than v2**: the page head
returns to v1's arrangement (which is what `main` built), Compact is a list
not a token pair, and the form keeps rustio-admin's own mechanism. There is
exactly one intentional divergence from `main` in composition (the form
measure) and one in behaviour (`AdminTheme` never retargets focus); both are
rustio-admin's current state, kept on purpose.

One blocker remains, and it is a document: `docs/design/VISUAL-CONTRACT.md`
v2.1 (44px controls, no text under 14px). Amending it is Phase 0 of
`MIGRATION.md`.

## 1 · What RustIO is now

```
RustIO main 00b933c   ← PR #6 squash-merged (Composition 3.1)
   page head: grid, action beside the title, lead beneath
   find row (.find) + data surface (.data / .data-head)
   quiet row actions (.button-quiet / .row-actions)
   badges without border; flat stat tiles; flat empty states
   List / Cards / Compact rebuilt (.record-list-*, .data--cards, .record-compact-item)
   form: one action area, .form-layout (card 728 + System aside) in --content 1120
   .cell-fit for hugging identity columns (column_hugs)
   --gutter 32 / 24 / 16; --content 1120; --content-wide 1600
   shell.rs injects --blue / --blue-dark / --focus from rustio.design.json (every shell page)
   session.rs injects --blue / --focus (sign-out page)

PR #5 884bf3b         ← unmerged; not authoritative (CRITIQUE.md §H)
```

## 2 · Architecture comparison (unchanged)

```
RustIO main                              rustio-admin
masthead 56                              utility 52 + module row 48
rail 240                                 rail 232
.main: --gutter 32, --content 1120/1600  page_measure: wide 1480 / standard 1120 / narrow 920 / form 672, centred
footer inside the measure                footer full-width chrome
one admin.css (Tailwind pass)            30 fragments, no build step
```

Both shells are sound; rustio-admin's three-measure system is the richer
one. Nothing in the shell architecture moves.

## 3 · Compatibility by surface

| Surface | Class | v3 note |
|---|---|---|
| Shell | ALIGN | header 56 + 40; rail 240 / 38px items / no bar |
| Measures, gutter | KEEP · ALIGN | mechanism kept; wide 1480 → 1600; `--gutter` 24 step |
| Topbar: ⌘K, account menu, notifications, env pill | KEEP UNIQUE | controls at 32px |
| Page head | ALIGN | boxed card → `main`'s grid, action beside the 24px title, lead beneath |
| Find row | ALIGN | `main`'s `.find`: search 38 × 340; triggers 31; count after the last control |
| Data surface head | ALIGN | `main`'s `.data-head` |
| Tables | ALIGN + IMPROVE | 48px rows, 40px mono heads, identity hug (`.cell-fit`), numeric / tabular / nowrap cell behaviour (`.cell-num`, `.record-*-time`, `.table-audit`) — ALIGN; applying it automatically by `rio-td--kind`, and the sticky actions column with its companion rules — IMPROVE |
| Row actions | ALIGN + IMPROVE | quiet text, always visible |
| Badges | ALIGN | one status face (dot + word + soft fill); a second, bordered count face as `main`'s `.module-link .badge` — rustio-admin's `.rio-dropdown-badge` and tab counts |
| Forms | ALIGN · INTENTIONAL DIVERGENCE | bands, 38px controls, boolean row (ALIGN); centred 728 measure on `--rio-page-form`, against `main`'s left-aligned `.form-layout--single` (DIVERGENCE) |
| Cards | ALIGN + IMPROVE | flat, radius 14 (ALIGN); identity floor (IMPROVE) |
| Compact | ALIGN | 40px lines, no head |
| Sessions, account detail | ALIGN | one surface; 140px label column |
| Permission matrix, API, history, health, DB browser, flags, docs, view designer | KEEP UNIQUE | scale and faces only |
| Timestamps | IMPROVE | one format in cells; the model list already has it; two call sites (users list, account sessions) change |
| AdminTheme override | IMPROVE · INTENTIONAL DIVERGENCE | hover shade (IMPROVE); never retargets focus (DIVERGENCE from `main`, where `shell.rs` and `session.rs` inject `--focus` from `rustio.design.json`) |
| Empty states | ALIGN | flat, inside the surface |
| Responsive | KEEP | same reflow |
| Type scale + control heights | the one D | 16/44 contract → 15/38 |

## 4 · Token strategy

Shared semantic core + product-specific tokens. Canonical names carry the
values; every `--rio-*` name is a permanent alias. `main` contributes
`--gutter`, `--masthead`, `--ctl` / `--ctl-sm` / `--ctl-util`, `--th-h` /
`--td-h`, the ink ramp with `--ink-mono`, the grey / green / red / amber
pairs with their `-line` variants, `--field-line`, `--filter-line` /
`--filter-fill` and the two shadows. It has no form-measure token and no
compact tokens; rustio-admin's form width stays its own `--rio-page-form`.
Class names are not aliased: `.rio-*` stays. Full table: `TOKEN-MAPPING.md`.

## 5 · Primary colour

**Keep `#1F5797`** — `main`'s default `--blue` and rustio-admin's
`--rio-rust`. Per-project override is each product's own mechanism
(`rustio.design.json` on `main`, which sets primary *and* focus; `AdminTheme`
in rustio-admin, which sets the primary and leaves focus alone). The shared
default is the family resemblance; the override mechanisms stay distinct.

## 6 · The improvements, classified

1. Page head: `main`'s grid, unboxed — ALIGN.
2. Scale 15 / 38 / 31 / 48 — ALIGN.
3. Quiet row actions, always visible — ALIGN + IMPROVE.
4. One status-badge face, plus the bordered count face — ALIGN.
5. Find row + data-surface head — ALIGN.
6. One timestamp format in cells: two remaining call sites — IMPROVE.
7. Numeric / tabular / nowrap cell behaviour and identity hug (ALIGN); automatic application by cell kind and the sticky actions column (IMPROVE).
8. Sessions and account detail as surfaces — ALIGN.
9. AdminTheme hover shade — IMPROVE.
10. Flat empty states, no monogram tiles, Compact as 40px lines — ALIGN.

Dropped from v2: the page-head band, the compact tokens, the filter width
and chevron notes, the "RustIO settled one blue" evidence.

Intentional divergences: the centred 728 form measure; `AdminTheme` never
retargeting focus.

## 7 · What stays unique

Unchanged from v2 (`COMPONENT-MATRIX.md`).

## 8 · Difficulty

Medium. Smaller than v2: the page head is CSS-only on both markup shapes
and is the v1 grid; Compact is a list rule; no new tokens beyond `--gutter`.
Timestamps remain the one Rust change.

## 9 · Risks

1. The contract conflict, decided in Phase 0.
2. `--rio-*` names stay as aliases permanently.
3. 36 → 24px titles; land the scale alone and look.
4. Timestamps: two one-line Rust changes (users list in `admin/builtin.rs`,
   account sessions in `admin/render.rs`). The model-list rewrite and its
   test (`a_timestamp_cell_reads_as_a_date_not_a_wire_format`) are already
   right and stay as they are.
5. `rio-theme` emitter and `TOKENS-EMIT-SPEC.md` need the alias pass.
6. Documentation drift (`VISUAL-CONTRACT.md`, `DESIGN_DOCTRINE.md`,
   `compat.css` header, `admin-shop.png`); documented, not edited.
7. If `main` later changes the page head or the form layout, the
   corresponding board moves; nothing else depends on it.

## 10 · Recommended order

`MIGRATION.md`, Phases 0–7.
