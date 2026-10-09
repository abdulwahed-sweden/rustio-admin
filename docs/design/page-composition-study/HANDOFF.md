# Handoff — where each change would live

Nothing here is implemented. This maps every composition in `SPEC.md` onto
the template and CSS fragment that owns it today, so an implementation pass
knows where to go. `build/composition.css` is the proposal as one layer; in
production each rule moves into the fragment named here, and the
`@import` list in `admin/admin.css` and `ADMIN_CSS` in `src/admin/routes.rs`
stay in lock-step (guarded by `css_lockstep_tests`).

## List page

| Change | Template | CSS fragment | Notes |
|---|---|---|---|
| Drop the generic lead on lists | `list.html`, `users_list.html`, `groups_list.html` | `components/page-header.css` | remove `<p class="rio-masthead-desc">` from list heads |
| Find row: up to 4 filters inline | `list.html` (`loop.index <= 2` → `<= 4`; `filters\|length > 2` → `> 4`) | `pages/list.css` (`.rio-find`, `.rio-find-filters`, `.rio-find-count`) | the `rio-cmdbar` class retires |
| One reset | `list.html` | — | Reset only when `search_query and active_filter_count == 0`; the pill line keeps *Clear all* |
| No head strip in Table mode | `list.html` | `pages/list.css` | render `.rio-board-head` only `if mode_links or adaptive`; Sort / Direction dropdowns inside it only `if adaptive` |
| Selection strip in the head slot | `list.html` | `components/data.css` (`.rio-bulkbar` → `.rio-board-sel`, groups) | two loops over `bulk_action_buttons`: `not btn.destructive`, then `btn.destructive`; `data-rio-bulk*` hooks unchanged |
| Foot: count · per page · pagination | `list.html` | `components/data.css` (`.rio-board-foot--ops`) | move the per-page dropdown markup from the head to the foot; render the foot whenever rows exist |
| Checkbox column 40px | — | `components/data.css` (`.rio-dtable--ops .col-check`) | — |
| Identity mono, hugging | `list.html` (`rio-cell-fit` on the first column when `column_hugs`) | `components/data.css` | the users list sets it statically |
| Choice values humanised as a badge | `list.html` (`elif f.kind == "select"` branch: `<span class="rio-pill rio-pill--neutral">{{ row[f.name]\|replace("_", " ") }}</span>`) | — | verify the kind name the context emits for `choices` fields |
| Booleans as marks | `list.html` (`checkbox` branch) | `components/data.css` (`.rio-mark`, `.rio-mark--off`) | `rio-pill--on/off` stay for pages that know *true* is good (users Active) |
| Colour by meaning | — | — | REQUIRES WIRING: the view layer's `badge` semantic via a saved ViewSpec; not on the generic table |
| Empty states | `list.html` | `components/data.css` | two branches: `search_query or active_filter_count` → filtered copy; else fresh copy with a secondary Add |
| Narrow | — | `pages/list.css`, `components/page-header.css` (`@media (max-width: 760px)`) | small head controls, full-width search, scrolling filter row, wrapping foot |

## View modes

| Change | Template | CSS fragment |
|---|---|---|
| Head strip only with `mode_links` or `adaptive` | `list.html` | `pages/list.css` (`.rio-board-head--views`) |
| Foot in adaptive modes | `list.html` | `components/data.css` |
| Layouts | `_list_adaptive.html`, `view_layer/_cell.html` | `components/adaptive-views.css` — unchanged |

## Form

| Change | Template | CSS fragment | Notes |
|---|---|---|---|
| Standard measure with an aside | `form.html` (`page_measure` block: `rio-page--standard` if `has_readonly or inlines` else `rio-page--form`) | `layout/console.css` | `has_readonly` is a context field to add if absent: any field with `disabled` |
| Layout grid | `form.html` | `pages/form.css` (`.rio-form-layout`, `--single`, `.rio-form-main`, `.rio-form-aside`) | single column ≤ 1040 |
| One card, action bar as foot | `form.html` | `pages/form.css` (`.rio-form-card`, band legend, `.rio-form-actions` inside the card) | `.rio-fieldset` loses its own border/shadow inside the card |
| Read-only facts panel | `form.html` | `pages/form.css` (`.rio-facts`) | partition `section.fields` by `field.disabled`; render disabled ones as `<dl>`; no `<input>` for them (stored values are re-injected on save — confirm in `handlers.rs` before removing the inputs; otherwise keep them `hidden`) |
| Related sections in the aside | `form.html` (inline loop moves into the aside; `rio-iconbtn` → quiet text actions via the `row_actions` macro) | `pages/form.css` (`.rio-related*`) | — |
| Inputs sized by content | — | `components/forms.css` | `max-inline-size: 240px` on date / datetime-local / number inside the card |
| Legend inside a card | — | `pages/form.css` | the band rule is a `box-shadow`, the legend floats into the flow (`float: left; inline-size: 100%`) and the grid clears it |

## Dashboard

| Change | Template | CSS fragment |
|---|---|---|
| Facts line | `index.html` (replace `.rio-ledger`) | `pages/dashboard.css` (`.rio-dash-facts`) |
| Title band on the surface | `index.html` | `components/data.css` (`.rio-board-title`) |
| App label once / as a column | `index.html` (`apps\|length > 1`) | `pages/dashboard.css` |
| Docs quiet, Audit log secondary | `index.html` | — |

## Users · Groups

| Change | Template | CSS fragment |
|---|---|---|
| Users as the generic table | `users_list.html` | `pages/list.css` (retire `.rio-dtable--users` grid rules; add `.rio-dtable--people`) |
| Group editor on the standard measure, one card | `group_edit.html` (`page_measure` → standard; wrap in `.rio-form-layout--single`; `rio-form-card`) | `pages/form.css`, `pages/permissions.css` (flat matrix, head on raised) |
| Extras open as a grid | `group_edit.html` (`<details open class="rio-perm-extras rio-perm-extras--open">`) | `pages/permissions.css` |

## Sessions · API · History

| Change | Template | CSS fragment |
|---|---|---|
| Session rows | `account_sessions.html` | `pages/account.css` (`.rio-sess-rows`, `.rio-sess-row*`; retire `.rio-sess-card*`) |
| API sections | `apis_index.html` | `pages/tools.css` (`.rio-api-section*`; retire `.rio-api-card*`, `.rio-api-grid`) |
| History | `log_entries.html` (drop `.rio-avatar`) | `pages/detail.css` (`.rio-table--audit`) |

## Order to land it in

1. **Jobs / list page** — the direction is judged here. One commit for the
   find row and foot, one for the strips, one for the cells, one for narrow.
2. **View modes** — the head-strip condition.
3. **Form** — layout, card foot, facts, related sections.
4. **Dashboard, Users, Groups** — one commit each.
5. **Sessions, API, History** — one commit each.

Each commit leaves the admin shippable; none touches a route, a handler, a
hidden input or a `data-rio-*` hook.

## Tests that touch this

- `tests/contract_assets.rs::the_page_header_halves_are_a_matched_pair`
  — the head markup pair stays; still holds.
- `css_lockstep_tests::import_manifest_matches_concat_bundle` — holds as
  long as no fragment is added; if `composition.css` were kept as its own
  fragment, both lists change in the same commit.
- `contract_assets::every_rio_alias_resolves_to_a_declared_canonical_token`
  — unaffected: no token is added or renamed.
- The list-timestamp and view-layer tests are unaffected: no cell value
  changes; the humanised status is a template filter on display.

## Risks

- **Removing the disabled inputs** for read-only fields depends on the
  save path re-injecting stored values; if it does not for some field type,
  keep hidden inputs. Verify before Phase 3.
- **The 4-filter rule** changes a count-dependent branch; pages with five or
  more filters keep "More filters".
- **The humanised status** is display-only; search and filters still use the
  stored value.
- **Users grid retirement** removes a special case other pages may rely on
  for the ellipsised email; the hugging identity cell with `title` covers it.
