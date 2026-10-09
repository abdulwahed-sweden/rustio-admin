# rustio-admin composition unification study

A design-only study of how rustio-admin adopts RustIO's admin composition
("Composition 3.1") while keeping every component that makes it rustio-admin.
It contains documents and self-contained HTML boards. **It contains no runtime
change**: no Rust, no template, no production CSS or JS, no token file, no
emitter change, no migration, no test.

Branch: `design/composition-unification-study`. Nothing here is merged, and
nothing here is an implementation. The last document, `MIGRATION.md`, says
how an implementation would be sequenced if the proposal is accepted.

## Version

| | Commit | Date | Role |
|---|---|---|---|
| This study | v2 | 2026-10-09 | supersedes v1 on the points listed in `CRITIQUE.md` §G |
| rustio-admin | `d4b61fa` (`main`) | 2026-09-17 | the product under study; unchanged since v1 |
| RustIO `main` | `8a6da60` | 2026-10-08 | pre-composition baseline; Composition 3.1 is not on `main` |
| RustIO PR #5 `fix/admin-ui-system-redesign` | `884bf3b` | 2026-10-08 | layered implementation (`admin-composition.css`) |
| RustIO PR #6 `claude/awesome-cerf-7tx0ee` | `2887db4` | 2026-10-09 | package implementation in `admin.css` + templates |
| v1 of this study | `v1/` | 2026-10-08 | written against RustIO PR #6 at `298780e`, design package only |

**Assumption that changed between v1 and v2:** v1 assumed one RustIO target.
There are now two open implementations that disagree on the page head and on
the list toolbar. v2 names which one it follows for each component and why
(`CRITIQUE.md` §B, §G). If RustIO resolves the fork differently, only the
Shell and Data table boards move.

## Reading order

1. `CRITIQUE.md` — written first, before any v2 board: what in v1 remains
   correct (A), what is outdated because RustIO changed (B), rustio-admin's
   strong (C) and weak (D) components, what must not change (E), the token
   conclusions that stand (F), and what v1 itself gets wrong (G).
2. `boards/index.html` — the system in one page; then `Comparison.html`,
   which shows five pages in all three states: CURRENT · ORIGINAL STUDY ·
   UPDATED PROPOSAL.
3. `STUDY.md` — the revised study and its conclusions.
4. `CURRENT-VS-PROPOSAL.md` — per surface, what rustio-admin does today and
   what the proposal does, with the class of each change.
5. `TOKEN-MAPPING.md` — the token strategy and the full mapping.
6. `COMPONENT-MATRIX.md` — every component, classified KEEP / ALIGN WITH
   RUSTIO / IMPROVE RUSTIO-ADMIN / PRODUCT-SPECIFIC — KEEP UNIQUE.
7. `MIGRATION.md` — Phases 0–7, if and when the proposal is accepted.

## What is where

| Path | Holds |
|---|---|
| `boards/` | Fourteen boards (v2). Each is one self-contained file; product renders sit inside same-origin iframes so the stylesheets' real media queries drive the narrow states |
| `build/ra_mock_v2.css` | The mock stylesheet the UPDATED PROPOSAL frames render against: canonical tokens with `--rio-*` aliases, rustio-admin's own class names styled in the 3.1 grammar. A design artefact, not product CSS |
| `v1/` | Version 1 of the study, verbatim: `STUDY.md`, ten boards, its mock stylesheet. Preserved for the record; superseded where `CRITIQUE.md` §G says so |
| `reference/` | Snapshots of the RustIO stylesheets the study compares against, by commit (`reference/README.md`) |

The CURRENT frames on the boards render rustio-admin's real template markup
(`list.html`, `form.html`, `users_list.html`, `account_sessions.html`,
`index.html`, `user_view.html`) against the real 30-fragment stylesheet at
`d4b61fa`, concatenated in manifest order. They are the product as it is, not a
drawing of it.

## Boards

| Board | Shows |
|---|---|
| `index.html` | What changed since v1, the four classes, the ten strongest improvements, what stays unique, the primary-colour decision |
| `Shell.html` | Rail, two-row header, workspace, footer — CURRENT / v1 / v2 at 1440; the measures table |
| `Dashboard.html` | Site administration at 1440 (three states) and 390 |
| `Data-Table.html` | Products: three states; with a selection; at 900 with the sticky actions column |
| `View-Modes.html` | The adaptive view region in List, Cards, Compact |
| `Users-Groups.html` | Users (three states), the group editor |
| `Permissions-Matrix.html` | The matrix with row and column toggles, members, one action bar; at 390 |
| `API-Surface.html` | API cards, v1 vs v2 |
| `Account-Sessions.html` | Active sessions (three states), user detail Overview and Sessions tabs |
| `Form.html` | Edit product (three states), validation state, 390 |
| `History.html` | The audit log |
| `Dense-Orders.html` | A real-world dense page: nine columns, three active filters, two timestamps; Table and Compact; at 1100 |
| `Narrow.html` | Six pages at 390 |
| `Comparison.html` | Five pages in all three states, and a table of what the states say |

## Verification

Every v2 page was rendered with Playwright/Chromium at 1440 and 390 and
checked for document-level horizontal overflow (none). Two overflow sources
found on the way are fixed in the mock stylesheet and recorded for the product
in `CURRENT-VS-PROPOSAL.md`: an absolutely positioned `.rio-sr-only` label
escaping the table scroller, and a fieldset's default `min-inline-size:
min-content`. rustio-admin itself was not built or run; the CURRENT frames are
static renders of its templates' markup, which is why they are labelled by
commit.
