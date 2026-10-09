# Study v2 — rustio-admin × RustIO composition

Supersedes `v1/STUDY.md` on the points in `CRITIQUE.md` §G. Baselines are in
`README.md`. The critique came first; this document is the conclusion.

## Final answer

**Unchanged: yes.** rustio-admin adopts RustIO's composition, scale and faces
without losing a component, a route or a behaviour. The palette already
matches value for value on both sides; the gap is scale (16px / 44px against
15px / 38px), boxing (a white card around every page head), density (56–64px
rows) and three small systems that grew separately (badges, row actions,
timestamp formats). All of it is CSS and template arrangement.

Two things changed since v1, and both make the proposal *smaller*:

- The page head becomes a **band with a rule** (title left, actions right),
  which is rustio-admin's current composition minus the white box — not the
  action-beside-title arrangement v1 asked for.
- The form keeps **its own measure** (728, centred), which is what
  rustio-admin's `rio-page--form` already does at 672 — not the 1120 column
  v1 asked for.

One blocker remains and it is a document: `docs/design/VISUAL-CONTRACT.md`
v2.1 mandates 44px controls and forbids text under 14px and any 36/38px
control, the exact values RustIO is built on. It is already stale on its own
terms (a copper `#B84318` accent and a dark theme that `d4b61fa` no longer
has), so amending it is Phase 0 of `MIGRATION.md`, not a cost of this
proposal.

## 1 · What RustIO is now

```
RustIO main 8a6da60            the pre-composition admin (restore)
   │
   ├─ PR #5  884bf3b            admin.css + admin-composition.css (layered)
   │                            page head = band · ops panel kept · compact tokens
   │                            timestamp format · filter width · sticky actions
   │                            card identity floor · focus ≠ blue · one blue
   │
   └─ PR #6  2887db4            the design package landed in admin.css
                                page head = action beside title · find row
                                data surface · quiet actions · ColumnView.hug
```

Neither is merged. Where they agree (geometry, quiet actions, dot badges, flat
tiles, one form action area, List and Compact as surfaces, no token changes,
one blue, focus distinct), this study treats the agreement as settled. Where
they disagree:

| Component | PR #5 | PR #6 | v2 follows | Why |
|---|---|---|---|---|
| Page head | band with rule, action right | action beside title | **PR #5** | PR #5 built the beside-title version, measured it at 2560 and reversed it with a stated reason; it is also the smaller change for rustio-admin |
| List toolbar | the two-row `.ops` panel, refined | find row + data-surface head | **PR #6** | rustio-admin's command bar is already unboxed on the page ground; there is nothing to box. The data-surface head is where view modes, sort and rows-per-page belong because they control the surface |

## 2 · Architecture comparison (unchanged from v1)

```
RustIO                                   rustio-admin
masthead 56                              utility 52 + module row 48
rail 240: models + System pages          rail 232: destinations within the active module
one page = one measure, --gutter 32      page_measure: wide 1480 / standard 1120 / narrow 920 / form 672
                                         centred, calc(100% - 64px)       ← same geometry
footer inside the measure                footer full-width chrome
one CSS file (Tailwind pass)             30 fragments, no build step
```

Both shells are sound. rustio-admin's three-measure system is better
engineered than RustIO's two measures and RustIO has been converging on it
(PR #5 added a form measure). Nothing in the shell architecture moves.

## 3 · Compatibility by surface

| Surface | Class | v2 note |
|---|---|---|
| Shell | ALIGN | header 56 + 40; rail 240 / 38px items / no bar |
| Measures, gutter | KEEP | already the principle; wide 1480 → 1600, form 672 → 728 |
| Topbar: ⌘K, account menu, notifications, env pill | KEEP UNIQUE | controls at 32px |
| Page head | ALIGN | boxed card → band with rule; 36 → 24px title; form pages no rule |
| Find row | ALIGN | hero search 44 → 38 × 340; triggers 44 → 31; count after last control |
| Data surface head | ALIGN | view modes · sort statement · tools on `--surface-head` |
| Tables | ALIGN + IMPROVE | 48px rows, 40px mono heads; cells by kind; identity hug; sticky actions |
| Row actions | ALIGN + IMPROVE | hover-revealed icons → quiet text, always visible |
| Badges | ALIGN | pills + roles + av-badges → one dot badge |
| Forms | ALIGN | bands, 38px controls, own 728 measure; save variants KEEP UNIQUE |
| Cards | ALIGN | flat, radius 14, identity floor 12rem |
| Sessions | ALIGN | five cards → one list surface |
| Account detail | ALIGN | tabs at 38px; dl on a 140px label column |
| Permission matrix | KEEP UNIQUE | 40px rows, bands, one action bar |
| API surface, history, health, DB browser, flags, docs, view designer | KEEP UNIQUE | scale and faces only |
| Timestamps | IMPROVE | one family format in cells |
| AdminTheme override | IMPROVE | keep hover shade |
| Empty states | ALIGN | flat, inside the surface |
| Responsive | KEEP | same reflow; gutters 24 / 16 |
| Type scale + control heights | the one D | 16/44 contract → 15/38 |

## 4 · Token strategy

**Shared semantic core + product-specific tokens.** Canonical names carry
values; every `--rio-*` name becomes an alias of a canonical name and is kept
permanently, because `AdminTheme` overrides and downstream projects target
them. Tokens with no RustIO equivalent (`--rio-overlay`, `--rio-shadow-lg`,
`--rio-syntax-*`, `--rio-hit-*`, `--rio-topbar-h`, `--rio-modulebar-h`,
`--rio-on-solid`, `--rio-rust-active`) stay as product tokens. Two canonical
names gain product aliases the other way round: RustIO's `--gutter` and
`--form-measure` (PR #5 / #6) map onto `--rio-shell-pad-x` and
`--rio-page-form`. Full table: `TOKEN-MAPPING.md`. `tokens/compat.css` is
already this shape.

## 5 · Primary colour

**Keep `#1F5797`.** Re-evaluated: RustIO PR #5 removed the only page that
injected another blue and re-pinned the scaffold to the contract blue;
rustio-admin carries the same value under `--rio-rust`. The family blue is
settled on both sides. `--focus #2F7BD6` stays distinct from the primary
(PR #5 restates why: a ring sharing the primary fill makes a focused control
read as a button) and `AdminTheme` already leaves it alone. The one recorded
alternative (`#0F5FA8`, v1 §4) is not pursued.

## 6 · The improvements, classified

1. **Page head band** — ALIGN. The largest visible change for the smallest
   CSS: drop the card, keep the arrangement, add the rule.
2. **Scale** — ALIGN. 15px base, 38 / 31px controls, 48px rows, 40px heads,
   24px titles. One typography file and about a dozen hard-coded heights.
3. **Quiet row actions, always visible** — ALIGN + IMPROVE. Removes hidden
   authorised controls and unlabeled glyphs on every list but Users.
4. **One badge system** — ALIGN. `.rio-pill`, `.rio-role`, `.av-badge` share
   one face: dot + word + soft fill, 13/600, radius 6.
5. **Find row + data-surface head** — ALIGN. Finding controls above the
   surface, presentation controls on it.
6. **One timestamp format** — ALIGN + IMPROVE. `YYYY-MM-DD HH:MM UTC` in
   cells; relative time only in History "when", "last seen", "expires".
7. **Cells by kind** — ALIGN + IMPROVE. `rio-td--date|datetime|boolean`
   nowrap, tabular digits, identity hug, sticky actions column at every width.
8. **Sessions and account detail as surfaces** — ALIGN.
9. **AdminTheme hover shade** — IMPROVE. The override should emit the shades
   `rio-theme` already computes instead of one hex six times.
10. **Flat empty states, no monogram tiles, form on its own measure** — ALIGN.

## 7 · What stays unique

The two-row header and module navigation, contextual rail, ⌘K palette,
account menu, notifications, filter dropdown panels (chips, FK autocomplete,
multi-select, date range), active-filter pills, bulk selection and bulk
actions, the three save variants, inline related sections, the permission
matrix, the session list, the API surface, audit history with diffs,
branding, health, DB browser, feature flags, docs viewer, view designer,
`AdminTheme`, `rio-theme`, the full-width footer, the `page_measure`
mechanism. Each keeps its markup and behaviour and takes only scale and faces.

## 8 · Difficulty

Medium, unchanged. Tokens and faces: easy. Type scale: easy mechanically,
medium visually (45 pages re-scale at once). Control heights: medium.
Page head: easy now (the band is the current arrangement unboxed; two markup
shapes, CSS-only). List page: medium. Timestamps: small Rust change in the
formatters, touched page by page. Product pages: medium, one at a time.
Emitter and docs: medium.

## 9 · Risks

1. The contract conflict must be decided in Phase 0, not discovered in Phase 2.
2. `--rio-*` names must stay as aliases permanently (AdminTheme, downstream).
3. RustIO's PR #5 / PR #6 fork: two boards depend on which lands. The
   proposal picks and says why; a different outcome moves the page head or the
   toolbar, nothing else.
4. 36 → 24px titles change perceived hierarchy everywhere; land the scale
   alone and look before continuing.
5. Timestamps are a Rust change (the formatters) and a tested one; it is the
   only phase that touches `.rs` files other than tests of CSS output.
6. `rio-theme` emits legacy names and dark blocks; the emitter and
   `TOKENS-EMIT-SPEC.md` need the same pass or the override drifts.
7. Documentation drift is real today (`VISUAL-CONTRACT.md` copper / dark,
   `DESIGN_DOCTRINE.md` slate / dark, the `compat.css` "Teal" header,
   `docs/assets/admin-shop.png`); unification without fixing it will drift
   again. Documented here, not edited.

## 10 · Recommended order

`MIGRATION.md`, Phases 0–7. Phase 0 is documents only; Phases 1–3 are tokens
and scale; Phase 4 is the page head and list page; Phase 5 the row-level
rules (badges, actions, cells, timestamps); Phase 6 the product pages one by
one; Phase 7 the emitter, spec and contract tests. Each phase leaves the
admin shippable.
