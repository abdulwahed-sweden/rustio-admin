# POLISH-HANDOFF.md

Final visual polish pass — `feat/composition-completion` @ `e83e0c6`.

**Provenance.** This is NOT a recovered copy of a `POLISH.md`. No such file
exists in this repo, on any branch, in git history, or on disk, and no polish
audit was run in this session — so the "7 medium / 15 small" tally cannot be
reproduced. What follows is built from the fifteen defects named in the
request, each resolved to its real file, selector and correction by reading
the current source. Nothing here is inferred from a page I was not pointed at.

**Gap.** Four items in the agreed priority order were named without a defect
description and could not be reconstructed. They are listed in §4 and are
blocked pending one line each.

**Constraints honoured by every correction below:** no token values, colours,
type scale, radii, spacing scale, shell, navigation, routes, handlers, RBAC
or behaviour. Existing approved classes only.

---

## 1 · Issues — fully specified

### P-01 · Notifications — inconsistent link colour · MEDIUM
- **Page** `/admin/notifications`
- **Defect** The message link is a bare `<a>` with no class, so it inherits
  the global anchor colour instead of the console's link treatment. Every
  other in-table link on the page uses an approved class; this one does not.
- **Correction** Add `class="rio-cell-primary"` to the anchor.
- **File** `crates/rustio-admin-assets/assets/templates/admin/notifications.html:36`
- **Selector** `.rio-cell-primary`

### P-02 · Account sessions — "other devices" count floats in the band · MEDIUM
- **Page** `/admin/account/sessions`
- **Defect** `<span class="rio-meta">{{ other_count }}</span>` prints a bare
  numeral between the band title and its action, reading as a stray digit.
- **Correction** Give the count its noun, matching the facts line above it:
  `{{ other_count }} session{% if other_count != 1 %}s{% endif %}`.
- **File** `.../admin/account_sessions.html:48`
- **Selector** `.rio-board-title .rio-meta`

### P-03 · Cmd-K palette — "CUSTOMERS2" runs together · MEDIUM
- **Page** command palette (all pages)
- **Defect** The group label and its result count are adjacent inline spans
  with no separator, so `CUSTOMERS` + `2` renders as `CUSTOMERS2`.
- **Correction** Put a gap between the label text and the count:
  `.rio-search-palette__group-label { display: flex; align-items: baseline;
  gap: var(--rio-space-8); }`. No markup change.
- **File** `crates/rustio-admin/assets/static/admin/components/navigation.css:221`
- **Selector** `.rio-search-palette__group-label`

### P-04 · Search page — scope row too tight · MEDIUM
- **Page** `/admin/search`
- **Defect** `.rio-scope` sits directly under the find row with no separating
  space, so the scope links read as part of the form.
- **Correction** Add `margin-block: var(--rio-space-16) var(--rio-space-20);`
  to `.rio-scope`, and `gap: var(--rio-space-4) var(--rio-space-16);` between
  its links.
- **File** `crates/rustio-admin/assets/static/admin/pages/list.css` — `.rio-scope` block
- **Selector** `.rio-scope`

### P-05 · Health — code chips break badly · MEDIUM
- **Page** `/admin/health`
- **Defect** `<code class="rio-code">rustio-admin doctor email --to …</code>`
  in the facts `<dd>` wraps mid-token because the chip has no break control,
  splitting a command across two lines at an arbitrary character.
- **Correction** `.rio-facts dd > code.rio-code { white-space: nowrap;
  overflow-wrap: normal; }` and let the `<dd>` scroll rather than the chip
  break.
- **File** `crates/rustio-admin/assets/static/admin/pages/tools.css` — HEALTH block
- **Selector** `.rio-facts dd > code.rio-code`

### P-06 · Schema — "Customers customers", weak hierarchy · MEDIUM
- **Page** `/admin/dev/schema`
- **Defect** `<h2>{{ m.display_name }} <code>{{ m.admin_name }}</code></h2>`
  prints the display name and the slug at the same weight, reading as the
  word twice ("Customers customers").
- **Correction** Demote the slug out of the heading into the band's meta
  slot, beside the field count: heading becomes `<h2>{{ m.display_name }}</h2>`
  and the meta carries `<code>{{ m.admin_name }}</code> · N fields`.
- **File** `.../admin/schema.html:29-30`
- **Selector** `.rio-board-title h2`, `.rio-board-title .rio-meta`

### P-07 · Admin reset — option controls inconsistent with Lock account · MEDIUM
- **Page** `/admin/users/<id>/reset-password`
- **Defect** The Mode radios are bare `<label class="rio-radio">` siblings.
  Lock account wraps the same control in `<div class="rio-radio-list">`, which
  supplies the grid and row gap — so the two pages render the identical
  control at different spacing.
- **Correction** Wrap the two `Mode` radios in `<div class="rio-radio-list">`,
  exactly as `lock_user.html:29` does. No CSS change.
- **File** `.../admin/admin_reset_password.html:70-77`
- **Selector** `.rio-form-card .rio-radio-list`

### P-08 · Record history — facts split awkwardly · MEDIUM
- **Page** `/admin/<model>/<id>/history`
- **Defect** The facts line emits "created by X" and "on Y" as two separate
  `<span>` items, so the console's `·` separator lands between a clause and
  its own object — "created by sara · on 2026-10-09".
- **Correction** Merge them into one span:
  `<span>created by <strong>{{ created_by }}</strong> on <strong>{{ created_at }}</strong></span>`,
  guarded so a missing `created_by` still prints the date alone.
- **File** `.../admin/object_history.html:23-24`
- **Selector** `.rio-dash-facts > span`

### P-09 · Feature flags — raw ISO timestamp · SMALL
- **Page** `/admin/feature_flags`
- **Defect** The Updated column prints `updated_iso` verbatim
  (`2026-10-09T14:22:51.118374+00:00`), the wire format, in a cell the rest
  of the console renders as a date.
- **Correction** Render the existing human field the other tables use. If the
  context carries only `updated_iso`, add the formatted sibling in
  `render.rs` the same way `HistoryEntryCtx.time_hm` is derived — presentation
  only, no behaviour change.
- **File** `.../admin/feature_flags.html:29` (+ `src/admin/render.rs`, flags ctx)
- **Selector** `.rio-td--datetime`

### P-10 · Branding — determinism badge wraps to a second line · SMALL
- **Page** `/admin/dev/branding`
- **Defect** `<span class="rio-vd-flag">deterministic · no AI at runtime</span>`
  sits inside `.rio-masthead-desc`, whose 680px cap pushes the badge onto its
  own line below the lead.
- **Correction** `.rio-vd-flag { white-space: nowrap; }` and move the badge
  out of the `<p>` to sit after it.
- **File** `.../admin/branding.html:15`
- **Selector** `.rio-vd-flag`

### P-11 · View designer — same badge, same wrap · SMALL
- **Page** `/admin/dev/view-designer/<model>`
- **Defect** Identical to P-10.
- **Correction** Identical to P-10; the `.rio-vd-flag` rule fixes both.
- **File** `.../admin/view_designer_model.html:13`
- **Selector** `.rio-vd-flag`

### P-12 · View designer — three row heights in one row · SMALL
- **Page** `/admin/dev/view-designer/<model>`
- **Defect** A field row puts three controls side by side at three heights:
  `.rio-vd-role` is a `.rio-input` (38px), `.rio-vd-toggle__face` is padding-
  derived (~27px), `.rio-iconbtn--sm` is 28px. The row reads as unaligned.
- **Correction** Normalise the two small controls to the approved small-control
  height: `.rio-vd-toggle__face { min-block-size: var(--rio-ctl-sm); display:
  inline-flex; align-items: center; }` and `.rio-vd-reorder .rio-iconbtn--sm
  { block-size: var(--rio-ctl-sm); }`. Uses the existing 31px token; no new value.
- **File** `crates/rustio-admin/assets/static/admin/pages/view-designer.css:74,82`
- **Selector** `.rio-vd-toggle__face`, `.rio-vd-reorder .rio-iconbtn--sm`

### P-13 · Add user — Cancel / Create order · SMALL
- **Page** `/admin/users/new`
- **Defect** `.rio-action-bar-end` pushes Cancel to the trailing edge while
  the primary sits at the leading edge, inverting the order every other form
  card in the console uses (primary first, quiet escape beside it).
- **Correction** Drop the `.rio-action-bar-end` wrapper so the foot reads
  `Create user` then `Cancel`, matching `user_edit.html`.
- **File** `.../admin/user_new.html:20-25`
- **Selector** `.rio-form-actions`, `.rio-action-bar-end`

### P-14 · CSV import result — file path upper-cased in a legend · SMALL
- **Page** CSV import result
- **Defect** `.rio-fieldset-legend` is `text-transform: uppercase`, so the
  legend `2 · Model struct (src/{{ admin_name }}.rs)` renders the path as
  `SRC/CUSTOMERS.RS` — a path that does not exist at that casing.
- **Correction** Take the path out of the legend and put it in the band body,
  or exempt it: `.rio-fieldset-legend code { text-transform: none; }` with the
  path wrapped in `<code>`.
- **File** `.../admin/csv_import_result.html:43`
- **Selector** `.rio-fieldset-legend`

### P-15 · Admin reset — dead space before Done · SMALL
- **Page** `/admin/users/<id>/reset-password` (result state)
- **Defect** The action row carries `rio-stack-16` on top of
  `.rio-form-card .rio-form-actions`' own padding, opening a gap above Done
  that no other terminal action row has.
- **Correction** Remove `rio-stack-16` from the class list.
- **File** `.../admin/admin_reset_password.html:46`
- **Selector** `.rio-form-actions.rio-stack-16`

### P-16 · API playground — parentheses around `?q=` · SMALL
- **Page** `/admin/apis/playground`
- **Defect** `<span>Search (<code>?q=</code>)</span>` — the parentheses are
  punctuation around a code chip that already reads as an aside.
- **Correction** `<span>Search <code>?q=</code></span>`.
- **File** `.../admin/apis_playground.html:48`
- **Selector** —

---

## 2 · Grouped CSS changes

**`components/navigation.css`**
- P-03 `.rio-search-palette__group-label` → flex + `gap: var(--rio-space-8)`

**`pages/list.css`**
- P-04 `.rio-scope` → `margin-block`, link `gap`

**`pages/tools.css`**
- P-05 `.rio-facts dd > code.rio-code` → `white-space: nowrap`
- P-14 `.rio-fieldset-legend code` → `text-transform: none`

**`pages/view-designer.css`**
- P-10/P-11 `.rio-vd-flag` → `white-space: nowrap`
- P-12 `.rio-vd-toggle__face`, `.rio-vd-reorder .rio-iconbtn--sm` → `var(--rio-ctl-sm)`

No other fragment is touched. No token is added, renamed or revalued.

---

## 3 · Grouped template changes

| File | Issues |
|---|---|
| `notifications.html` | P-01 |
| `account_sessions.html` | P-02 |
| `search.html` | P-04 (CSS-only; no change if `.rio-scope` already emitted) |
| `schema.html` | P-06 |
| `admin_reset_password.html` | P-07, P-15 |
| `object_history.html` | P-08 |
| `feature_flags.html` | P-09 |
| `branding.html` | P-10 |
| `view_designer_model.html` | P-11 |
| `user_new.html` | P-13 |
| `csv_import_result.html` | P-14 |
| `apis_playground.html` | P-16 |

`src/admin/render.rs` is touched only for P-09, and only to derive a display
string beside the existing ISO field — no query, route or handler change.

---

## 4 · Blocked — defect text not recoverable

These four are named in the agreed priority order but carry no description I
can reconstruct. Each needs one line from you before it can be specified; I
will not invent a defect for a page I was not shown.

| # | Item | What is missing |
|---|---|---|
| B-1 | **Lock account defect** (priority 1) | `lock_user.html` reads clean against the approved composition. Which element, and what is wrong with it? |
| B-2 | **Search icon / date-field issues** (priority 2) | Which page — `/admin/search`, the audit find row, or both? Icon alignment, or sizing? |
| B-3 | **Three composition breaks** (priority 3) | Which three pages, and which composition rule each breaks. |
| B-4 | **Schema overflow** (priority 4) | Distinct from P-06. Which element overflows, and at what width? |

---

## 5 · Implementation order

1. **B-1** Lock account — *blocked*
2. **B-2** search icon / date fields — *blocked*
3. **B-3** three composition breaks — *blocked*
4. **B-4** Schema overflow — *blocked*; P-06 (Schema hierarchy) can land now
5. **Remaining** — P-01 … P-16 in the order listed above

Unblocked work is P-01 through P-16: five CSS edits across four fragments and
twelve template edits. Landing them does not depend on B-1 … B-4.

## 6 · Verification

One `cargo fmt --all --check`; one clippy pass over `rustio-admin` and
`rustio-admin-assets`; `cargo test -p rustio-admin --all-targets` (the CSS
lock-step and asset contracts are the ones that can catch these); a short
render smoke of the twelve affected pages at 1440 and 390.
