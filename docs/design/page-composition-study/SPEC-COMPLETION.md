# Spec — the completion pass

Every redesigned page: what is structurally wrong today, the new composition,
the Composition 3.1 patterns it reuses, the components it adds, how it
reflows at 390, and what the implementation must know. Classes as in the
first study: **CSS ONLY · TEMPLATE + CSS · REQUIRES WIRING · UNCHANGED.**

The shared vocabulary every page below is built from (all shipped on `main`):
`.rio-crumbs` + `.rio-masthead-top` (the unboxed head) · `.rio-dash-facts`
(the facts line) · `.rio-board` + `.rio-board-title` (a surface with a band)
· `.rio-dtable.rio-dtable--ops` (the operational table) · `.rio-find` (the
find row) · `.rio-form-card` + `.rio-fieldset` (a card with bands, the action
bar as its foot) · `.rio-facts` (the facts panel) · `.rio-empty` (the empty
state) · `.rio-pill`, `.rio-mark`, `.rio-action-link`, `.rio-code`.

The stylesheet layer is `build/composition-completion.css`: 290 lines, every
value a `--rio-*` token, no new colour, size, radius, shadow or spacing step.

---

## Audit log — `/admin/history` · `log_entries.html`

**Current problems.** A six-column table (When · Action · Record · By ·
Summary · IP). Field diffs render inside the Summary cell, so an edited row
is three rows tall. The relative time and the IP each own a column but
carry little. The only filter is the By link; there is no search, no action
/ model / period filter, no pagination, and none of the identity the
`rustio_admin_actions` row already has (exact timestamp, correlation id).

**New composition.**

```
crumbs · title
find row:  search · Action · Actor · Model · Period · count
board
  day band (sticky)            2026-10-10 · 3 events
  row  time · pill · Record #id  summary: fields … · actor · ›
  row (open)
       diff (old → new per field)   | When  exact timestamp (relative)
                                     | Actor email (user #n)
                                     | From  ip
                                     | Request correlation id
                                     | Event type · record history
  foot: Showing 1–9 of 9 · 50 per page · pagination
```

- One `<details>` per event; `<summary>` is the row, the body is the event.
  No script. The first row may open by default on the implementation's
  choice; the boards show it open to demonstrate the body.
- Row grid `52px · max-content · 1fr · max-content · 20px`, 48px closed.
- Clock time on the row (the day is the band); the exact ISO timestamp and
  the relative form in the body.

**Reused.** find row, active-filter line, board + sticky day band (the
existing `.rio-history-date-divider` becomes `.rio-audit-day`), pill
vocabulary, `.rio-history-diff` markup, ops foot.

**Added.** `.rio-audit`, `.rio-audit-day`, `.rio-audit-row` (details),
`.rio-audit-when/-what/-sum/-who/-chev`, `.rio-audit-detail`,
`.rio-audit-kv`, `.rio-audit-empty-diff`.

**Narrow.** Two-line summary (time · pill · actor / record · summary); the
body stacks diff over identity; filters are one scrolling row (list rule).

**Implementation.** TEMPLATE + CSS for the rows, bands, detail and foot.
REQUIRES WIRING: `?q=`, `?action=`, `?model=`, `?from=/to=` and pagination on
the history handler; `time_hm` and `correlation_id` on `HistoryEntryCtx`
(both derivable from `AdminAction`). The actor filter (`?user_id=`) exists.
Keep `page_title`, `user_filter_label`, `history_entries`, the pill class
names and the `_csrf`-free GET.

## Record history — `/admin/<model>/<id>/history` · `object_history.html`

Same row anatomy as the audit log without the find row: a facts line
(events · created by · last change), *Open record* as the head action,
*Full audit log →* in the foot. Head moves to shape B. TEMPLATE + CSS; the
facts need `created_at` / `created_by` / `last_change` on
`ObjectHistoryCtx` (REQUIRES WIRING, three fields derivable from the entries).

## API surface — `/admin/apis` · `apis_index.html`

**Current problems.** Each model section repeats the identical five endpoint
rows; the head offers three equal buttons; no authentication or permission
facts although the handlers enforce both; no way to jump to a model.

**New composition.**

```
crumbs · title · [Open playground]      lead: the JSON negotiation rule
facts: 4 models · 20 endpoints · session cookie, _csrf on POST · openapi.json · rustio-sdk.ts
board  The endpoint pattern — method · path pattern · does · needs (permission) · returns
board  Models — model · base path · fields · singular · Browse / Try    (= table of contents)
board  <Model> — paths in the band · field · type · required/optional     (× N)
```

**Reused.** masthead CTA, facts line, board + band, ops table, verb pills
(`.rio-api-verb--*`), `.rio-mark` / `.rio-mark--off`, action links.

**Added.** `.rio-ref-table`, `.rio-ref-models`, `.rio-ref-schema`,
`.rio-ref-fields`, `.rio-ref-paths`, `.rio-ref-perm`.

**Narrow.** Bands wrap meta under the title; tables scroll inside the
surface; paths wrap at the slash.

**Implementation.** TEMPLATE + CSS. The pattern band is static markup (five
rows) parameterised by nothing — the permission string pattern is
`{model}.{verb}_{singular}` as `handlers.rs` builds it. `ApiEntryCtx` already
carries everything else. The old `.rio-api-card*`, `.rio-api-grid`,
`.rio-api-section*` and `.rio-api-ep-list` rules retire.

## API playground — `/admin/apis/playground` · `apis_playground.html`

Two surfaces on the standard measure: a `.rio-form-card` with bands Request
(model, endpoint, row id) · Query (`q`, sort, direction, page, per page) ·
Body, Send and the live URL in the foot; a Response board (status line
HTTP · ms · bytes · type, then a `.rio-code` well), sticky beside the form.
Below 1000 the response stacks under the form. TEMPLATE + CSS; every element
id and the script are unchanged; `data-show-for` keeps toggling fields.

## Health — `/admin/health` · `health.html`

**Current problems.** The verdict is a small pill beside a count inside a
card; each row repeats a pill; no time; no next step; the probe route and
the CLI are not on the page.

**New composition.**

```
crumbs · title · [Re-run checks]
status band    ● 1 check failing, 1 warning
               4 checks · as of <server_now> · the same probes rustio-admin doctor runs
board Checks (failing first)    |  facts panel Diagnostics
  FAILING  RUSTIO_SECRET_KEY  message      |  Probe  GET /admin/healthz
  WARNING  Active administrator …         |  CLI    rustio-admin doctor
  PASSING  Postgres reachable …           |  Email  rustio-admin doctor email --to …
  PASSING  Auth tables present …          |  Audit  Recent authority events
```

**Reused.** masthead CTA, board + band, `.rio-facts` panel, `.rio-code`.

**Added.** `.rio-status` (+ `--warn`, `--error`), `.rio-status-dot/-title/
-meta`, `.rio-status-layout`, `.rio-check-rows`, `.rio-check-row` (+ state
modifiers), `.rio-check-state/-label/-msg`.

**Narrow.** Band first; state beside the name, message under it; the panel
stacks below.

**Implementation.** TEMPLATE + CSS. Order the checks in the template
(`error`, `warn`, `ok`) or in `health_ctx`; `server_now` is already on the
base context. *Re-run checks* is a link to the page (every GET re-probes).
`/admin/healthz` exists. The old `.rio-health__*` rules retire.

## Docs — `/admin/docs` · `docs_index.html` and `/admin/docs/<slug>` · `doc_page.html`

**Current problems.** The index is three hover-lifting glyph cards. The
document is prose in a card with a shadow, a 12rem TOC and a back link in the
card foot; no navigation between documents; dark code wells from another
system; headings cannot be linked.

**New composition — index.** Facts line (documents · rendered server-side ·
version) and one board, *Documents*, as a numbered reading list: title ·
path · Read.

**New composition — document (wide measure).**

```
crumbs · title
[docs nav 12rem] [prose ≤ 68ch, unboxed] [on-page nav 12rem, sticky]
                 foot: ← Framework docs · Next: … →
```

**Reused.** facts line, board + band, `.rio-doc-prose` typography,
`.rio-doc-toc` + `initDocToc`, `.rio-code` well, secondary button, action link.

**Added.** `.rio-docs` (grid), `.rio-docs-nav`, `.rio-docs-list`,
`.rio-docs-item*`, `.rio-docs-toc`, `.rio-docs-foot`, `.rio-docs-anchor`; a
re-skin of `pre`, `blockquote`, `thead` inside `.rio-docs .rio-doc-prose`.

**Narrow.** Under 1100 the on-page nav drops (as today); under 900 the docs
nav becomes one scrolling row above the prose; the foot wraps.

**Implementation.** TEMPLATE + CSS for both. REQUIRES WIRING (small):
`DocPageCtx` gains `docs` (the `EMBEDDED_DOCS` list for the nav) and
`prev` / `next`. Section anchors are CSS on the ids `initDocToc` already
sets; a server-side id would make them work without JS. The old
`.rio-doc-card*`, `.rio-doc-grid`, `.rio-docpage*`, `.rio-doc-foot/-back`
rules retire; `.rio-doc-prose` and `.rio-doc-toc*` stay.

## Sessions — `/admin/account/sessions` · `account_sessions.html`

**Current problems.** This device and the others are rows of one list; four
unlabelled facts in one grey line per row; the two bulk actions are equal
buttons under the list, the destructive one second.

**New composition (standard measure).**

```
crumbs · title
facts: 3 sessions · 2 other devices · 2 addresses
board This device   [Trusted]            sign out from the account menu
      DEVICE · IP · SIGNED IN · LAST ACTIVE · EXPIRES   (labelled facts)
board Other devices · 2   [Sign out of every other device]
      device · trust · ip · signed in · last active · expires · Revoke
danger section  Sign out everywhere — one sentence of consequence — [Sign out everywhere]
```

**Reused.** facts line, board + band with a band action, ops table, pills
(`--success` for trusted, `--warning` for new device), action link in danger,
danger button.

**Added.** `.rio-sess-facts`, `.rio-sess-table` + cell classes
(`.rio-sess-device`, `.rio-sess-col-trust`, `.rio-ip`, `.rio-sess-time`),
`.rio-sess-danger`, `.rio-board-title-action` (shared).

**Narrow.** The table becomes stacked rows (device · trust / ip / three
labelled times, Revoke at the edge) via `data-label`; the facts band wraps to
two columns; the danger section stacks.

**Implementation.** TEMPLATE + CSS. The three POST routes and `_csrf` are
unchanged; the empty state for *no other devices* lives in the band; the
band action is omitted when there are no other devices. The old
`.rio-sess-card*`, `.rio-sess-row*`, `.rio-sess-rows`, `.rio-sess-bulk` rules
retire.

## Search — the `⌘K` palette (`_base.html`, `admin.js`) and `/admin/search` (new)

**Definitions.**

| Surface | Job | Results | Enter |
|---|---|---|---|
| `⌘K` palette | jump to a record (and, later, a page) | ≤ 5 per model, ≤ 20, grouped, edit-gated | opens the highlighted result |
| `/admin/search?q=` | see and compare every match | grouped per model with counts and context, view-gated, scoped | submits the form |
| `/admin/<model>?q=` | filter one model | that model's table with filters, sort, pages, bulk | submits the find row |

The palette does not grow into a page; the page does not become a palette;
the page hands off to the list page per model (*Search inside Jobs →* carries
the term).

**Palette changes.** Group labels carry a count (*Customers · 2*); each
result says what Enter does (*↵ open* on the highlighted one, *edit* on the
rest); a foot with the keys and *All results for “tom” →* into the page;
placeholder *Jump to a record…*. TEMPLATE + CSS + a few lines in
`render()` of `initSearchPalette`. Debounce, minimum length, grouping order,
keyboard handling, focus trap and the `change` gate are unchanged.

**Search page (standard measure, `rio-c-list`).**

```
crumbs · title
find row:  [term] [Search]   All · Customers 2 · Jobs 3 · Quotes · Job events
facts: 5 results in 2 models for “baker” · searches the fields each model declares searchable
board Customers · 2 of 2 · Search inside Customers →
      label (match marked) · context cells · Edit / History
board Jobs · 3 of 3 · Search inside Jobs →
```

States: **no query** — what the page searches, the models as entry points,
the `⌘K` hint; **no results** — the term quoted, what to try, a way into each
model's filtered list.

**Reused.** find row, facts line, board + band + band action, ops table,
`.rio-cell-primary` / `.rio-cell-meta`, action links, `.rio-empty`, `.rio-kbd`.

**Added.** `.rio-find--page`, `.rio-scope`, `.rio-search-group`,
`.rio-search-hint`, `.rio-search-empty-models`, palette:
`.rio-search-palette__foot`, `__result-hint`.

**Narrow.** The field spans, Search beneath it, the scope row scrolls,
result rows stack label over context with the actions at the edge.

**Implementation.** REQUIRES WIRING: route `GET /admin/search` (Staff gate,
mounted before `/admin/:admin_name`), a handler that reuses the
`search_models` loop with a per-model limit of 25, gated on `view` (the page
links to view-gated routes), returning label, the row's list cells, counts
per model and the total; a `search.html` template; `/` focusing the field
(optional JS). The palette changes are template + CSS + JS only.

## Notifications — `/admin/notifications` · `notifications.html`

Head to shape B with *Mark all read* as the action; facts line (unread ·
total · provenance); one board *Inbox* of rows — unread dot · message (link
when `url`) · when. Unread rows semibold. Empty state in the board. TEMPLATE
+ CSS; the POST is unchanged.

## Feature flags — `/admin/feature_flags` · `feature_flags.html`

Head to shape B; facts line (flags · on · how to read · cache note); board
*Flags* as an ops table — key · on/off mark · description · updated ·
Enable/Disable; the register form as a `.rio-form-card` band beneath with
*Add flag* in its foot. TEMPLATE + CSS; both POST routes and the key pattern
unchanged.

## Database — `/admin/db` · `db_browser.html` and Schema — `/admin/dev/schema` · `schema.html`

Facts line instead of the stat card; a jump row of table / model names; one
board per table — name and counts in the band, the columns as an ops table,
a one-line foot listing foreign keys both ways. Schema adds a `.rio-facts`
panel *Change the schema* beside the boards with the CLI commands in a code
well. TEMPLATE + CSS; the old `.rio-db-*` rules retire.

## CSV import result — `csv_import_result.html`

Remove the inline `<style>` and the `.imp-*` system. Head to shape B with
*Back to <model>*; facts line (inserted of total · failed · columns skipped);
a warning alert naming the skipped columns; the three fix steps as bands of
a form card, each a `.rio-code` well with a hint; the row detail as an ops
table (row · status mark · detail). TEMPLATE + CSS; generation unchanged.

## Account & security flows

All take the record composition already shipped for `form.html` and
`group_edit.html`: head shape B, `.rio-form-layout--single` ›
`.rio-form-card`, bands, the action bar as the card's foot.

- **Two-factor enrolment** `mfa_enroll.html` — bands *1 · Pair your app*
  (authenticator link as a secondary button, manual key as `.rio-code-block`,
  as label/value rows) and *2 · Confirm* (a six-digit mono input); foot
  Verify and enable · Cancel.
- **Lock account** `lock_user.html` — bands Duration (a radio *list*, not
  option cards) and Reason; foot Lock account (danger) · Cancel.
- **Edit user** `user_edit.html` — bands Identity · Group memberships; foot
  Save · Delete user · Back. **Add user**, **Add group**, **Change password**
  take the same shape (one band each).
- **Reset password (admin)** `admin_reset_password.html` — bands Mode ·
  Reason; the success state keeps its alert and shows the temporary password
  in a `.rio-code-block` inside a facts panel.
- **MFA enrolment complete, regenerate (+ complete), disable** — head swap to
  shape B only; their `.rio-confirm` / backup-code surfaces are already right.

TEMPLATE + CSS throughout; field names, hidden inputs, validation, the
last-developer guard and every POST are unchanged. Added:
`.rio-kv-rows`, `.rio-kv-row`, `.rio-input--code`, `.rio-radio-list`.

## Branding · View designer (spec only)

- **Branding** `branding.html` — head shape B; one form card with bands
  *Brand colour* (picker, hex, presets as a row) and *Live preview* (the
  sample controls); *Bake it* as a facts panel with the three commands in a
  code well. All `data-rio-brand*` hooks unchanged.
- **View designer index** `view_designer.html` — one board: model · slug ·
  saved/draft pill · Design.
- **Composition editor** `view_designer_model.html` — keep the Studio Phase 1
  structure. Changes: one Save (the form's foot; the masthead button goes),
  *Live preview* as a board with a band, *Modes* / *Fields* / *Compositions*
  as bands of the form card, the generated ViewSpec as a `.rio-code` well in
  an aside under the preview. Every input name and `data-rio-vd*` hook stays.

## What this pass does not do

No new colour, type size, control height, radius, shadow or spacing step.
No change to the shell, rail, header rows, module navigation, `⌘K` trigger,
account menu or footer (the footer overflow at 390 is a one-rule fix
recorded in `INVENTORY.md`). No route, handler, permission, hidden input or
`data-rio-*` hook moves; wiring is listed, not performed.
