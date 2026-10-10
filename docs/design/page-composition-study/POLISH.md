# Polish pass — `feat/composition-completion`

A visual quality pass over the implemented pages, not a design change. Every
page on the branch (42 renders: the whole admin surface with fixshop data,
at 1440 and 390, against the branch's own bundle) was inspected for small
composition defects. Pages that need nothing are not listed; that is most of
them — dashboard, every list, the record form, users, groups, the group
editor, docs index and document, confirmations, MFA complete / regenerate /
disable, change password, edit user, error pages are clean.

Severity: **medium** = visible on first glance or breaks a composition rule;
**small** = noticed on a second look. Nothing here touches tokens, routes,
handlers, hooks or behaviour. Where a fix is a template change it keeps
every `name`, `id`, `data-rio-*` and form action.

## Defects

| # | Page | Defect | Correction | Where | Sev |
|---|---|---|---|---|---|
| 1 | Audit log, Search | The search icon sits **outside** the field, to its left, and the input starts 18px right of the title's left edge. `.rio-search` lays the icon out as a flex sibling while the input already reserves 36px of padding for it. | Position the icon inside the field: `.rio-search > svg { position: absolute; inset-inline-start: 12px; inline-size: 16px; block-size: 16px; color: var(--rio-text-mute); pointer-events: none; }` | `components/forms.css` `.rio-search` | medium |
| 2 | Audit log (1440) | The find row wraps: two native date inputs at 40px tall and ~160px wide plus Apply push *Showing 1–9 of 9* onto a second line. The dates are also 8px taller than the sm filter buttons beside them. | `.rio-input--date { block-size: var(--rio-ctl-sm); inline-size: 8.5rem; padding-block: 0; }` — the row then holds search · Action · Model · from · to · Apply · count at 1440. | `pages/tools.css` | medium |
| 3 | Lock account | **Broken markup**: the Duration `<div class="rio-radio-list">` is never closed and a stray `</div>` sits inside the Reason band (line 76), so the textarea and the action bar render *outside* the card. | Close the list after the last radio (`</div>` before `{% if "duration" in field_errors %}`); delete the stray `</div>` after the Reason label; wrap label + textarea in `<div class="rio-field">`. | `lock_user.html` | medium |
| 4 | MFA enrolment | A card inside a card: `<section class="rio-card">` wraps the two label/value rows **and** a `.rio-form-card` form, so the sunken foot is inset inside a padded box and the 6-digit input stretches to full width. | Drop the outer section. Make the form the card with two bands: band 1 the two rows (link as a secondary sm button, key as `.rio-code-block`), band 2 the code field in a `.rio-field` with `max-inline-size: 12rem` and mono. Foot unchanged. | `mfa_enroll.html` | medium |
| 5 | CSV import result, Admin reset (success) | `.rio-alert` is `display: flex` for icon + body, so a `<strong>` followed by `<p>`s becomes three columns; at 390 the warning is three 100px columns. | Wrap the alert's content in one `<div>` (the pattern every other alert uses), or add `.rio-alert > :where(strong, p) { display: block }` guarded by a `.rio-alert-body` wrapper. Template fix preferred. | `csv_import_result.html` line 30, `admin_reset_password.html` line 19 | medium |
| 6 | API playground | The Response board is an empty 44px strip when idle; the Send foot is inset inside the form's padding instead of bleeding like every other card foot. | Template: make the `<form>` itself the `.rio-form-card`, fields inside `<fieldset class="rio-fieldset"><div class="rio-fieldset-grid">…`, and `.rio-form-actions` as the last child. Give the pre an idle line (`Send a request to see the response here.`, the script clears it on send) and `.rio-playground__body { min-block-size: 160px; }`. | `apis_playground.html`, `pages/tools.css` | medium |
| 7 | Schema (390) | The page is 474px wide at 390: the facts panel's `pre` sets a min-content width and the stacked grid column is `1fr`, not `minmax(0, 1fr)`. | `.rio-dev-layout { grid-template-columns: minmax(0, 1fr) 320px; }` and in the ≤1100 query `grid-template-columns: minmax(0, 1fr);` plus `.rio-facts .rio-code { overflow-x: auto; }`. | `pages/tools.css` | medium |
| 8 | Notifications | Message weight and colour mix two signals: unread rows are bold **and** blue, read rows with a URL are blue, the last row is black. Link colour reads as "has URL", not state. | `.rio-notif-msg a { color: var(--rio-text-hi); } .rio-notif-msg a:hover { color: var(--rio-rust); }` — ink by default, unread carries the weight and the dot. | `pages/tools.css` | small |
| 9 | Sessions | In the *Other devices* band the count `2` floats mid-band beside the button (`.rio-meta` has `margin-inline-start: auto`); at 390 it drops to its own line under the title. | Put the count in the title: `<h2>Other devices <span class="rio-meta">{{ other_count }}</span></h2>`, and add `.rio-board-title h2 .rio-meta { margin: 0 0 0 var(--rio-space-6); font-weight: var(--rio-weight-regular); }`. | `account_sessions.html` line 47, `components/data.css` | small |
| 10 | ⌘K palette | Group label reads `CUSTOMERS2` — the count span is appended with no gap. | `.rio-search-palette__group-label .rio-meta { margin-inline-start: var(--rio-space-6); text-transform: none; letter-spacing: 0; font-weight: var(--rio-weight-regular); }` | `components/navigation.css` | small |
| 11 | Search page | Scope row sits with no air between the find row and the facts line (4px / 8px); at 390 the chips wrap into two ragged lines. | `.rio-scope { margin-block: var(--rio-space-12) var(--rio-space-16); }` and at ≤760 `flex-wrap: nowrap; overflow-x: auto; scrollbar-width: none;` with `.rio-scope a { white-space: nowrap; }`. | `pages/list.css` | small |
| 12 | Health | Diagnostics panel: inline `code.rio-code` chips break mid-chip across lines (`rustio-admin doctor email --to …` becomes two chips). | `.rio-facts dd > code { display: inline-block; max-inline-size: 100%; white-space: normal; overflow-wrap: anywhere; }` — one chip that wraps inside itself. | `pages/tools.css` | small |
| 13 | Schema | Band titles read `Customers customers` — the slug `<code>` inside `h2` has the title's weight and size. | `.rio-board-title h2 code { font-size: var(--rio-text-13); font-weight: var(--rio-weight-regular); color: var(--rio-text-mute); background: none; padding: 0; margin-inline-start: var(--rio-space-4); }` (Database uses the same h2 code pattern and benefits). | `components/data.css` | small |
| 14 | Admin reset password | Mode options render as two bordered option cards while the sibling Lock page uses the flat radio list; the second card's description wraps under the radio with a leading dash. The intro sentence sits loose on the page ground between head and card. | Wrap the radios in `<div class="rio-radio-list">`; move the intro into `.rio-masthead-desc`; wrap the Reason label + textarea in `.rio-field`. | `admin_reset_password.html` | small |
| 15 | Record history | Facts read `created by anna · on 2026-09-30` as two separate facts. | One span: `created <strong>{{ created_at }}</strong> by <strong>{{ created_by }}</strong>`. | `object_history.html` lines 23–24 | small |
| 16 | Feature flags | Updated column shows the raw ISO string (`2026-10-08T14:20:00Z`) in a 12px mono cell. | Render the date part only, or the shared timestamp format the lists use; `rio-td--datetime` is already on the cell. | `feature_flags.html` line 29 (format in `feature_flags_ctx`) | small |
| 17 | Branding, Composition editor | The green `deterministic · no AI at runtime` flag breaks onto its own line under the lead; the editor also parks a `Primary developer` chip in the CTA slot, where an action belongs. | Move the sentence into the facts line as text (View designer index already does this) and drop the chip, or render it as `.rio-meta` after the title. | `branding.html` line 15, `view_designer_model.html` lines 13, 17 | small |
| 18 | Composition editor | Field rows: three control heights (select 40, Filter toggle 32, priority 40) and the ↑↓ buttons stacked vertically; at 390 the arrows fall onto two lines of their own. Composition slots keep an old legend-on-border fieldset with 4px between a label and the previous input. | Set `.rio-vd-role, .rio-vd-prio { block-size: var(--rio-ctl-sm); }`, `.rio-vd-reorder { flex-direction: row; }`; give `.rio-vd-slot` the band treatment (`border: 0; padding: 0; legend as .rio-fieldset-legend`) and `.rio-vd-slot .rio-form-field { margin-block-start: var(--rio-space-12); }`. | `pages/view-designer.css`, `view_designer_model.html` | small |
| 19 | Add user | The foot puts Cancel *before* Create user — every other form puts the primary first and the links at the end. | Order: primary button, then `<span class="rio-action-bar-end"><a … class="rio-action-link">Cancel</a></span>`. | `user_new.html` | small |
| 20 | CSV import result | Legend `2 · MODEL STRUCT (SRC/CUSTOMERS.RS)` uppercases a file path. | Keep `2 · Model struct` in the legend; the path goes in the `.rio-imp-note` under the code. | `csv_import_result.html` line 43 | small |
| 21 | Admin reset (success) | 48px of dead space between the paragraph and the rule above Done / Back (`rio-stack-16` plus the actions' own margin). | Remove `rio-stack-16` from the actions div, or set `.rio-form-actions.rio-stack-16 { margin-block-start: var(--rio-space-16); }`. | `admin_reset_password.html` line 46 | small |
| 22 | API playground | Labels `Search ( ?q= )` / `Sort field ( ?sort= )` carry spaced parentheses around a code chip. | `Search <code>?q=</code>` — drop the parentheses. | `apis_playground.html` lines 48, 53 | small |

## Verified as already correct

- Health rows order failing → warning → passing in `health_ctx` (`render.rs`
  sorts); the mock in this pass fed unsorted checks, the product does not.
- The 390 footer overflow from the earlier inventory is fixed on this branch;
  every page except Schema (#7) measures 390 wide.
- Title bands wrap their meta and action under the title at 390; audit rows,
  session rows and search rows stack as specified.

## Patch guidance — CSS, grouped by fragment

```css
/* components/forms.css — #1 */
.rio-search > svg { position: absolute; inset-inline-start: 12px; inline-size: 16px; block-size: 16px; color: var(--rio-text-mute); pointer-events: none; }

/* components/data.css — #9, #13 */
.rio-board-title h2 .rio-meta { margin: 0 0 0 var(--rio-space-6); font-weight: var(--rio-weight-regular); }
.rio-board-title h2 code { font-size: var(--rio-text-13); font-weight: var(--rio-weight-regular); color: var(--rio-text-mute); background: none; padding: 0; margin-inline-start: var(--rio-space-4); }

/* components/navigation.css — #10 */
.rio-search-palette__group-label .rio-meta { margin-inline-start: var(--rio-space-6); text-transform: none; letter-spacing: 0; font-weight: var(--rio-weight-regular); }

/* pages/list.css — #11 */
.rio-scope { margin-block: var(--rio-space-12) var(--rio-space-16); }
@media (max-width: 760px) { .rio-scope { flex-wrap: nowrap; overflow-x: auto; scrollbar-width: none; } .rio-scope a { white-space: nowrap; } }

/* pages/tools.css — #2, #6, #7, #8, #12 */
.rio-input--date { block-size: var(--rio-ctl-sm); inline-size: 8.5rem; padding-block: 0; }
.rio-playground__body { min-block-size: 160px; }
.rio-dev-layout { grid-template-columns: minmax(0, 1fr) 320px; }
@media (max-width: 1100px) { .rio-dev-layout { grid-template-columns: minmax(0, 1fr); } }
.rio-facts .rio-code { overflow-x: auto; }
.rio-notif-msg a { color: var(--rio-text-hi); }
.rio-notif-msg a:hover { color: var(--rio-rust); }
.rio-facts dd > code { display: inline-block; max-inline-size: 100%; white-space: normal; overflow-wrap: anywhere; }

/* pages/view-designer.css — #18 */
.rio-vd-role, .rio-vd-prio { block-size: var(--rio-ctl-sm); }
.rio-vd-reorder { flex-direction: row; }
.rio-vd-slot { border: 0; padding: 0; margin-block-end: var(--rio-space-16); }
.rio-vd-slot > legend { padding: 0; margin-block-end: var(--rio-space-8); font-family: var(--rio-font-mono); font-weight: var(--rio-weight-bold); letter-spacing: var(--rio-tracking-caps); color: var(--rio-text-mono); }
.rio-vd-slot .rio-form-field + .rio-form-field { margin-block-start: var(--rio-space-12); }
```

Template-only items: #3, #4, #5, #6 (structure), #14, #15, #16, #17, #19,
#20, #21, #22.

## Handoff — order

1. **#3 Lock account markup** — a rendering bug, first.
2. **#1 search icon** and **#2 date inputs** — the audit log and search
   page heads, the two most-seen new pages.
3. **#4 MFA card-in-card**, **#5 alert bodies**, **#6 playground** — the
   three composition-rule breaks.
4. **#7 schema overflow**.
5. The small items, in file order; most are one line.

Gates after each group: `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace --all-targets`
(`contract_assets::the_page_header_halves_are_a_matched_pair` and
`css_lockstep_tests` are the ones these files touch), then render the page
at 1440 and 390.

What this pass does not do: no new component, no new token, no layout
replaced, no page concept revisited. Twenty-two corrections; the direction
is unchanged.
