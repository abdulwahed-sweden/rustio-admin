# rustio-admin composition unification study

A design-only study of how rustio-admin adopts RustIO's admin composition
("Composition 3.1") while keeping every component that makes it rustio-admin.
Documents and self-contained HTML boards. **No runtime change**: no Rust, no
template, no production CSS or JS, no token file, no emitter change, no
migration, no test.

Branch: `design/composition-unification-study`. Nothing here is merged, and
nothing here is an implementation. `MIGRATION.md` says how an implementation
would be sequenced if the proposal is accepted.

## Version — v3

| | Commit | Date | Role |
|---|---|---|---|
| This study | v3 | 2026-10-09 | re-based on RustIO `main`; supersedes v2 on the points in `CRITIQUE.md` §H |
| rustio-admin | `d4b61fa` (`main`) | 2026-09-17 | the product under study; unchanged since v1 |
| **RustIO `main`** | **`00b933c`** | 2026-10-09 | **authoritative.** PR #6 squash-merged: "style(admin): recompose the admin — Composition 3.1 (#6)". Its `admin.css` is byte-identical to the PR #6 snapshot v2 compared against |
| RustIO PR #5 `fix/admin-ui-system-redesign` | `884bf3b` | 2026-10-08 | **unmerged, not authoritative.** Kept in `reference/` for the record only |
| v2 of this study | `v2/` | 2026-10-09 | written while PR #5 and PR #6 were both open; picked between them per component |
| v1 of this study | `v1/` | 2026-10-08 | written against the design package alone |

**Assumption that changed between v2 and v3:** v2 treated RustIO as a fork
and followed PR #5 where it judged it a measured correction. There is no
fork now. "ALIGN WITH RUSTIO" means `main @ 00b933c` and nothing else. Every
idea v2 took from PR #5 alone is reclassified in `CRITIQUE.md` §H as DROP,
IMPROVE RUSTIO-ADMIN or INTENTIONAL DIVERGENCE.

## Reading order

1. `CRITIQUE.md` — §A–G as in v2 (what remains correct, what is outdated,
   strong and weak components, what must not change, token conclusions, what
   the earlier study got wrong), plus **§H: the PR #5 reclassification**.
2. `boards/index.html` — the system in one page; then `Comparison.html`,
   five pages in four states: CURRENT · v1 · v2 · v3.
3. `STUDY.md` — the revised study and its conclusions.
4. `CURRENT-VS-PROPOSAL.md` — per surface, today vs proposal, with the class.
5. `TOKEN-MAPPING.md` — the token strategy and the full mapping.
6. `COMPONENT-MATRIX.md` — every component classified.
7. `MIGRATION.md` — Phases 0–7.

## Classes

| Class | Meaning |
|---|---|
| KEEP | right in rustio-admin today; untouched |
| ALIGN WITH RUSTIO | takes what RustIO `main @ 00b933c` does; markup and behaviour stay |
| IMPROVE RUSTIO-ADMIN | a rustio-admin-only quality fix, independent of RustIO |
| PRODUCT-SPECIFIC — KEEP UNIQUE | a capability RustIO does not have; keeps its composition, takes scale and faces |
| DROP | a PR #5-only idea v2 carried; not on `main`; removed |
| INTENTIONAL DIVERGENCE | a deliberate difference from `main`, with the reason stated |

## What is where

| Path | Holds |
|---|---|
| `boards/` | Fourteen boards (v3). Self-contained; product renders in same-origin iframes |
| `build/ra_mock_v3.css` | The mock stylesheet the UPDATED PROPOSAL frames render against. A design artefact, not product CSS |
| `v2/` | Version 2 verbatim: its seven documents, fourteen boards and mock stylesheet |
| `v1/` | Version 1 verbatim: `STUDY.md`, ten boards, its mock stylesheet |
| `reference/` | Snapshots of the RustIO stylesheets by commit (`reference/README.md`): `main-00b933c-admin.css` (authoritative), plus the PR #5 / PR #6 files v2 used |

The CURRENT frames on the boards render rustio-admin's real template markup
against the real 30-fragment stylesheet at `d4b61fa`, concatenated in
manifest order. They are the product as it is.

## Boards

| Board | Shows |
|---|---|
| `index.html` | What changed v2 → v3, the six classes, the ten strongest improvements, what stays unique, the primary-colour decision |
| `Shell.html` | CURRENT / v2 / v3 at 1440; the measures table with classes |
| `Dashboard.html` | Three states at 1440; 390 |
| `Data-Table.html` | Products: three states; with a selection; at 900 |
| `View-Modes.html` | List, Cards, Compact as `main` renders them |
| `Users-Groups.html` | Users (three states), the group editor |
| `Permissions-Matrix.html` | The matrix, members, one action bar; 390 |
| `API-Surface.html` | API cards, v2 vs v3 |
| `Account-Sessions.html` | Active sessions (three states), user detail Overview and Sessions |
| `Form.html` | Edit product (three states), validation, 390 — the one intentional divergence |
| `History.html` | The audit log |
| `Dense-Orders.html` | Nine columns, three filters, two timestamps; Table and `main`'s Compact; 1100 |
| `Narrow.html` | Six pages at 390 |
| `Comparison.html` | Five pages in four states and a table of what each state says |

## Verification

Every v3 page was rendered with Playwright/Chromium at 1440 and 390 and
checked for document-level horizontal overflow (none). rustio-admin itself
was not built or run; the CURRENT frames are static renders of its
templates' markup, labelled by commit. RustIO `main` was not built either;
its `admin.css` was read from the commit.
