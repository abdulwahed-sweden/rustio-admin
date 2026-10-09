# Critique — the v1 study against the current repositories

Written before any v2 board was drawn. Baselines:

| Repository | Commit | Date | Note |
|---|---|---|---|
| rustio-admin | `d4b61fa` (`main`) | 2026-09-17 | **Unchanged since v1.** Last merge: PR #154, "render list timestamps as dates, not wire format" |
| RustIO `main` | `8a6da60` | 2026-10-08 | Still the pre-composition restore. Composition 3.1 is **not on `main`** |
| RustIO PR #5 `fix/admin-ui-system-redesign` | `884bf3b` | 2026-10-08 18:49 → 22:52 | A layered implementation: `admin-composition.css` appended to `admin.css` by `build.rs`; keeps the `.ops*` / `.record-list-side` / `.table-compact` vocabulary and refines it |
| RustIO PR #6 `claude/awesome-cerf-7tx0ee` | `2887db4` | 2026-10-09 02:14 → 14:22 | The v1 design package landed step by step into `admin.css` and the templates: `.find` / `.data` / `.button-quiet` vocabulary, plus `ColumnView.hug` |

v1 of this study was written against RustIO at `298780e` on PR #6 — the design package *before* either implementation existed. Both implementations are later than v1, both are open, neither is merged, and **they disagree with each other on two components** (page head, list toolbar). That fork is the single most important fact for this update: "current RustIO" is not one thing yet.

The items you listed (consistent primary blue and focus, removal of divergent theme injection, timestamp formatting, filter width, disclosure affordance, card-title wrapping, geometry and responsive verification) are PR #5's commits `91044df…884bf3b`. This critique treats **PR #5 as the verified composition baseline** where it made a measured correction, and **PR #6 as the baseline for the vocabulary it alone implements** (find row, data surface, quiet actions, flat empty states, profile, sign-in). Where the two disagree, the decision and the reason are stated below and carried into v2.

---

## A · What from v1 remains correct

1. **The gap is composition, scale and density — not colour.** Confirmed again: rustio-admin's `tokens/colors.css` carries RustIO's values verbatim (`#F1F1EE`, `#1F5797`, `#174578`, `#DFEAFB`, `#B7CDEA`, `#2F7BD6`, `#D5DCE5`, `#B9C5D3`, `#F4F6F9`). Both RustIO PRs leave every token value untouched; PR #5 even removed the one page that injected a different blue. The palette is settled on both sides.
2. **The primary stays `#1F5797`.** PR #5's `e307f31` removed the sign-out page's `--blue: #2B54E0` injection and re-pinned the scaffold's `rustio.design.json` to the contract blue — RustIO moved *toward* one shared blue, not away from it. No evidence for a change.
3. **`--focus` must stay distinct from `--blue`.** PR #5 restates it ("a ring sharing the primary-button fill makes a focused control read as a button"). rustio-admin already honours this: `_theme.html` never touches `--rio-accent-focus`. Keep.
4. **Workspace geometry.** Both PRs centre the surface in the workspace beside the rail with a 32px minimum gutter (`--gutter` in PR #6; `2 × --s6` in PR #5), capped at `--content` / `--content-wide` = 1120 / 1600. rustio-admin's `.rio-ws-inner` (`calc(100% - 64px)`, `margin-inline: auto`, three measures) is the same geometry already. **A — already compatible**, as v1 said.
5. **Token strategy: canonical names + `--rio-*` aliases.** Still the safest shape; `compat.css` already is that shape. No new evidence against it.
6. **Quiet row actions, unframed badges, flat tiles and empty states, one form action area, List as one surface, Compact as a denser surface.** All four implementations (v1, PR #5, PR #6, rustio-admin's own users grid) converge on these.
7. **The contract conflict.** rustio-admin's `VISUAL-CONTRACT.md` v2.1 (44px controls, no text under 14px, 36/38px forbidden) still contradicts RustIO (38 / 31px controls, 13px mono labels). Unchanged; still the one real blocker; still a document.

## B · What is outdated because RustIO changed

1. **Page head — "action one gap after the title" is superseded.** v1 proposed it (and PR #6 built it). PR #5's `87995b3` tested it and reversed it with a measured reason: a blue fill parked against a bold 24px title reads as part of the heading; thrown to the far edge with nothing joining them, the two read as unrelated. The fix is *structure*: the head is a **band** — title and breadcrumb at one end, the page's primary action at the other, a 1px rule beneath spanning the content measure, the action landing on the same right edge as the data surface below. Single-card form pages drop the rule. **v2 adopts the band.** It also happens to be rustio-admin's current composition minus the white box, which is a smaller change than v1 asked for.
2. **Form measure — "use the 1120 column for two-column forms" is withdrawn.** v1 widened rustio-admin's forms to 1120 on the argument that 672 is too narrow for a two-column grid at 38px controls. PR #5's `ee950bb` settled the RustIO form at its own measure, centred (`--form-measure: 728px`, `.content--form`), because a 728 card at the left of an 1120 column "sat centred in nothing". rustio-admin's `rio-page--form` (672, centred) is the same answer already. **v2 keeps the form at its own measure, aligned to 728, centred.** Two short fields per row fit; textareas span.
3. **Timestamps.** PR #5's `3d82af9` fixed RustIO cells to the audit log's `YYYY-MM-DD HH:MM UTC` so a product never disagrees with itself. rustio-admin currently disagrees with itself: model list cells render date only (PR #154), users and sessions render `HH:MM` without the zone, user detail and the footer render `HH:MM UTC`, history renders relative time. v1 did not look at this. **v2 adds one family rule: `YYYY-MM-DD HH:MM UTC` in every cell; relative time only where it is the point (history "when", "last seen").**
4. **Table cells by role, not index.** PR #5 put `data-role` on cells and set `white-space: nowrap` on timestamp and badge cells plus tabular digits everywhere, because a 1558px table still wrapped a timestamp and pushed rows to 64px. rustio-admin already emits `rio-td--{{ f.kind }}` — the same information by kind. v1 missed it; **v2 uses it** (nowrap for date, datetime, boolean kinds).
5. **Row actions stay reachable on constrained widths.** PR #5 makes the actions column sticky at 761–1100px inside the scrolling table. v1 had nothing for this; rustio-admin's boards scroll inside `.rio-tablewrap`, where the action column is the first thing to disappear. **v2 adopts the sticky column.**
6. **Compact row geometry is a token, not a number.** PR #5 added `--th-h-compact: 32px` / `--td-h-compact: 38px` and made vertical padding the lever (cell `height` is only a minimum). v1 left Compact at a 40px line. **v2 adopts 38px rows, 32px head for rustio-admin's `.rio-table--compact`.**
7. **Card titles must not break mid-string.** PR #5's `884bf3b`: identity gets a 12rem floor, the state group wraps beneath, no `margin-left: auto` on the state group. v1's cards had the badge inline after the title with `overflow-wrap: anywhere` — the exact failure PR #5 removed. **v2 corrects the card head.**
8. **A filter control is a control, not a paragraph.** PR #5 caps filter selects at 220px. rustio-admin's filters are dropdown triggers, so no change; but the FK autocomplete and multi-select panels inside them should keep a 240px floor and no auto width — v2 states it.
9. **Disclosure needs an affordance.** PR #5 restored the chevron on `<summary>` elements that had lost it to `display: flex`. rustio-admin's `.rio-perm-extras summary` and `.ve-values` equivalents keep the native marker today; v2 keeps it and says so, so a future flex summary does not lose it.
10. **Identity column hugging.** PR #6's `ColumnView.hug` puts `.cell-fit` on an identity column whose every value is one unbreakable token (`INV-2026-0045`, an email). rustio-admin's users grid already fixes its identity track at `minmax(420px, 1fr)` with ellipsis; the generic list does not. v2 notes it as a behaviour rustio-admin's `ConcreteOps` list could decide the same way; it is Rust, so it is a later phase, not a CSS item.
11. **`reference/admin.css` in the RustIO package is a snapshot, not the shipped file**, and the shipped file on PR #6 differs (one `.layout-switch` block, not two; `.detail-single` gone). v1 called it "the proposal is the stylesheet". The snapshots this study keeps under `reference/rustio/` are PR #5's `admin.css` + `admin-composition.css` and PR #6's `admin.css`, by commit, so the comparison is against what actually renders.

## C · rustio-admin components that are already strong and should remain

- The **three-measure page system** (`page_measure` block → wide / standard / narrow / form) with centring and a 32px gutter. Better engineered than RustIO's two measures; RustIO is converging on it.
- The **two-row header**: utility strip + module row, with the rail carrying the destinations inside the active module. It is justified by the IA (five modules × many destinations) and is the main thing that makes rustio-admin read as the larger product.
- The **⌘K palette** and its topbar trigger.
- The **filter dropdown panels** (chips, FK autocomplete, multi-select, date range), *More filters* with a count, *Reset*, and the **active-filter pills** with per-pill removal — richer than RustIO's selects and already well composed.
- The **bulk selection** flow: checkbox column → bulk bar → bulk actions.
- The **users grid** (`.rio-dtable--users`): one x-origin, explicit tracks, always-visible actions, ellipsised identity. It is the best table in either product and the model for the generic list.
- The **form action bar** order (primary → save variants → auto gap → History · Delete · Cancel as text).
- **Inline related sections**, the **permission matrix**, the **session list**, the **API cards**, the **history diffs with date dividers**, the **docs viewer with sticky TOC**.
- The **full-width footer** with system links and identity.
- `AdminTheme` as a patch layer that never touches focus.

## D · Components that are visually weak and worth improving

1. **The boxed page head** (`page-header.css`): a white card with a 36px/800 title and shadow above every page, then another card below it. Two boxes for one page. → unboxed band with a rule.
2. **Row actions**: icon-only, revealed on hover (`opacity: 0`) on every list except users. Hidden authorised controls, unlabeled glyphs. → quiet text actions, always visible.
3. **Pills**: radius-999, 15px/700, lowercase, five variants plus `.rio-badge` plus `.rio-role` — three badge systems. → one badge: dot + word + soft fill, 13/600.
4. **Scale**: 16px base, 17px lead, 36px titles, 44px buttons, 46px inputs, 56–64px rows. Readable, but every surface is 15–25% taller than it needs to be, and the product's richness turns into height. → RustIO scale.
5. **The hero search** (44px, pill, reveal-on-hover "Search" button, 4px glow) and `--lg` dropdown triggers (44px): the toolbar is the tallest element on the list page. → 38px search, 31px triggers.
6. **Monogram tiles** in users, dashboard and groups rows (44px blue squares with an initial): decorative identity. → removed; identity is the link.
7. **Session cards**: five stacked cards with glyph tiles and footers for three facts each. → one list surface.
8. **Empty states**: 48px padding, a 48px icon tile, a 20px title. → flat, inside the surface.
9. **Timestamp formats** disagree page to page (see B.3).
10. **AdminTheme accent override flattens hover and active**: `_theme.html` writes the same hex into `--rio-rust`, `-hover`, `-active`, `-solid`, `-solid-hover`, `-solid-active`. An overridden primary has no hover state. `rio-theme` already computes the shades; the override could consume them. Behaviour preserved, quality improved.

## E · What should NOT be changed

The two-row header, module navigation, contextual rail, ⌘K search, account menu, notifications, API surface, permissions, sessions, audit/history, filter dropdown panels, bulk actions, the three save variants, inline related sections, branding, health, DB browser, feature flags, docs viewer, view designer, `AdminTheme` behaviour, `rio-theme` behaviour, the `page_measure` mechanism, the full-width footer, Postgres-only / no-build-step rules, every route and handler. The product-specific capability set is the identity.

## F · Token conclusions that remain valid

All of v1's mapping (`TOKEN-MAPPING.md`). Two refinements from the current repositories:

- RustIO now has **`--gutter`** (PR #6) and **`--form-measure`, `--th-h-compact`, `--td-h-compact`** (PR #5). rustio-admin's equivalents are `--rio-shell-pad-x` (already 32 / 16), `--rio-page-form` (672 → 728) and `--rio-row-h` family. The mapping table gains these four rows.
- RustIO's `admin.css` still has **no overlay, popover or large-shadow token**; rustio-admin keeps `--rio-overlay`, `--rio-shadow-lg`, `--rio-shadow-xl` as product tokens. Confirmed.

## G · What should change in the old study itself

v1 is preserved verbatim under `v1/`. It is superseded on these points:

| v1 said | v2 says | Why |
|---|---|---|
| Primary action one gap after the title | Head is a band: title left, action right, rule beneath; form pages drop the rule | PR #5 `87995b3`, measured at 2560 |
| Forms use the 1120 column | Forms sit on their own measure, 728, centred | PR #5 `ee950bb`; rustio-admin already does this |
| Card badge inline after the title, `overflow-wrap: anywhere` | Identity floor 12rem, badges wrap beneath, `break-word` | PR #5 `884bf3b` |
| Compact = 40px lines, no head | Compact = 38px rows, 32px head, kept as a table | PR #5 `57a48d8` |
| (not covered) | One timestamp format family-wide | PR #5 `3d82af9`; rustio-admin disagrees with itself |
| (not covered) | Sticky actions column 761–1100 | PR #5 `62165e3` |
| (not covered) | Nowrap by cell kind, tabular digits | PR #5 `62165e3` |
| "RustIO's reference/admin.css is the proposal" | Compare against the shipped files by commit | PR #6 `2887db4` |
| Shell header 56 + 40 | Unchanged | — |
| Rail 240, items 38, no bar | Unchanged | — |
| Find row + data-surface head | Unchanged — rustio-admin's command bar is already unboxed; the data-surface head is where view modes, sort and rows-per-page go | PR #6 built this vocabulary; PR #5 kept a boxed two-row panel. For rustio-admin the unboxed row is the current state, so there is nothing to box |

One dependency to record: **RustIO must resolve PR #5 vs PR #6** before "align with RustIO" has one meaning for the page head and the list toolbar. This study's v2 picks the band head (PR #5) and the find-row + data-surface vocabulary (PR #6) and says why; if RustIO decides differently, only those two boards move.
