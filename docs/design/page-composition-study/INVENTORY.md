# Inventory — every admin page, classified

Completion pass of the page composition study. Baseline is **`origin/main @
d495e80`**, which carries the shipped Composition 3.1 implementation
(`#156`). Every template under
`crates/rustio-admin-assets/assets/templates/admin/` was read and every routed
page was rendered with `examples/fixshop` data against the real CSS bundle at
1440 and 390 before it was classified.

Classes:

- **A — already Composition 3.1.** Shipped on `main`; nothing to do.
- **B — intentionally special, visually complete.** The page's composition
  *is* a contract of its own (the signed-out shell, the confirmation surface,
  the error state) and that contract is already on the frozen theme. Applied
  strictly: a page is B only when it already renders the confirmation, error
  or signed-out composition — never because it merely works.
- **C — still old, needs redesign.** Carries the old boxed page header,
  a card grid, stat tiles, a legend-on-border fieldset, a page-local style
  block, or a composition that hides its own information.

Priority: **P1** the six mandatory pages · **P2** utility pages an operator
meets weekly · **P3** account flows and developer pages · **P4** developer
tools used rarely, where the change is a head swap and bands.

## Totals

| | Count |
|---|---|
| Templates reviewed | **58** (49 pages, 9 shell partials) |
| A — already Composition 3.1 | **7** |
| B — special but complete | **15** |
| C — redesigned in this pass | **27** templates **+ 1 new page** (search) |
| C with a board | 17 renders on 9 boards |
| C spec only | 11 (branding, view designer ×2, four MFA completion/confirm pages, admin reset, change password, add user, add group) |

## Pages

| Route | Template | Class | Redesign | Priority | Reason |
|---|---|---|---|---|---|
| `/admin` | `index.html` | A | no | — | facts line + one surface; shipped in 3.1 |
| `/admin/<model>` (table, list, cards, compact) | `list.html`, `_list_adaptive.html`, `_row_actions.html`, `view_layer/*` | A | no | — | find row, selection strip, ops table and foot; the reference composition |
| `/admin/<model>/new`, `/<id>/edit` | `form.html`, `includes/_form_field.html` | A | no | — | record composition: card, bands, facts, related, foot |
| `/admin/users` | `users_list.html` | A | no | — | ops table, hugging identity |
| `/admin/groups` | `groups_list.html` | A | no | — | ops table |
| `/admin/groups/<id>/edit` | `group_edit.html` | A | no | — | one card, two bands, flat matrix, open extras |
| `/admin/users/<id>` | `user_view.html` | A | no | — | identity head, tabs, definition list, detail tables — the 3.1 detail contract |
| `/admin/login` | `login.html` | B | no | — | the signed-out auth card; `DESIGN_RECOVERY` uniform-response surface |
| `/admin/forgot-password`, `/sent`, `/reset-password/:token` | `forgot_password.html`, `forgot_password_sent.html`, `reset_password.html` | B | no | — | the `.rio-login` recovery flow with its aside; one contract, complete |
| `/admin/mfa/verify`, `/admin/reauth`, `/admin/must-change-password` | `mfa_verify.html`, `reauth.html`, `must_change_password.html` | B | no | — | step-up pages on the same `.rio-login` shell, sidebar suppressed by design |
| `/admin/<model>/<id>/delete` | `confirm_delete.html` | B | no | — | `.rio-confirm` on a board: icon, title, lead, cascade list, foot |
| `/admin/<model>/bulk_delete`, `/bulk/<action>` | `bulk_confirm_delete.html`, `bulk_confirm_action.html` | B | no | — | same confirmation contract |
| `/admin/users/<id>/delete`, `/admin/groups/<id>/delete` | `user_confirm_delete.html`, `group_confirm_delete.html` | B | no | — | same, with cascade impact |
| `/admin/users/<id>/unlock`, `/revoke-sessions` | `confirm_admin_action.html` | B | no | — | reason band + foot; already on crumbs + masthead |
| any 4xx/5xx | `error.html`, `forbidden.html` | B | no | — | the error state: status, heading, one way back |
| **`/admin/history`** | **`log_entries.html`** | **C** | **yes** | **P1** | six-column table, diffs balloon the summary cell, one filter, no event identity, no detail, no pagination |
| **`/admin/apis`** | **`apis_index.html`** | **C** | **yes** | **P1** | the same five endpoints repeated per model; three equal head buttons; no auth or permission facts; no table of contents |
| **`/admin/health`** | **`health.html`** | **C** | **yes** | **P1** | verdict is a pill in a card; a pill per row; no time; no next step |
| **`/admin/docs`** | **`docs_index.html`** | **C** | **yes** | **P1** | three hover-lifting glyph cards for three links |
| **`/admin/docs/<slug>`** | **`doc_page.html`** | **C** | **yes** | **P1** | prose boxed in a card; no docs navigation; dark code well from another system; no anchors |
| **`/admin/account/sessions`** | **`account_sessions.html`** | **C** | **yes** | **P1** | this device indistinguishable from the others; unlabelled meta line; two equal bulk buttons |
| **Search** — `⌘K` palette (`_base.html`) + **`/admin/search`** (new) | palette markup in `_base.html`; new `search.html` | **C** | **yes** | **P1** | a jump palette only; no result surface, no counts, no hand-off to list filtering; palette says nothing about what Enter does |
| `/admin/<model>/<id>/history` | `object_history.html` | C | yes | P2 | old `.rio-page-header`, striped table, same column problems as the audit log |
| `/admin/notifications` | `notifications.html` | C | yes | P2 | old header + section header + a card around a striped table with a *State* column that says *read* |
| `/admin/feature_flags` | `feature_flags.html` | C | yes | P2 | old header; two section heads; register form as a third surface |
| `/admin/apis/playground` | `apis_playground.html` | C | yes | P2 | old header; one card holding a form grid and the response |
| `/admin/<model>/import.csv` result | `csv_import_result.html` | C | yes | P2 | a page-local `.imp-*` design system in an inline `<style>` with hard-coded colours |
| `/admin/db` | `db_browser.html` | C | yes | P3 | stat card, a card per table, a two-column FK grid |
| `/admin/dev/schema` | `schema.html` | C | yes | P3 | same composition as `/admin/db`, plus a CLI section as a bare block |
| `/admin/account/mfa/enroll` | `mfa_enroll.html` | C | yes | P3 | old header; a bare card with label/value rows and the form inside it |
| `/admin/users/<id>/lock` | `lock_user.html` | C | yes | P3 | old header; six option cards for six radios; legend on the border |
| `/admin/users/<id>/edit` | `user_edit.html` | C | yes | P3 | two `.rio-fieldset.rio-board` boxes with legend on the border; actions outside any surface |
| `/admin/users/new`, `/admin/groups/new` | `user_new.html`, `group_new.html` | C | spec | P3 | same shape as edit user; same fix |
| `/admin/password_change` | `password_change.html` | C | spec | P3 | old header; legend on the border; the success state is already `.rio-confirm` |
| `/admin/users/<id>/reset-password` | `admin_reset_password.html` | C | spec | P3 | old header; a `.rio-card--callout` for the temp password; otherwise the lock-account shape |
| `/admin/account/mfa/enroll` complete, `/regenerate-codes` (+ complete), `/disable` | `mfa_enroll_complete.html`, `mfa_regenerate.html`, `mfa_regenerate_complete.html`, `mfa_disable.html` | C | spec | P4 | old `.rio-page-header` on an otherwise correct `.rio-confirm` / backup-codes surface — a head swap |
| `/admin/dev/branding` | `branding.html` | C | spec | P4 | two preview cards + a bake block; needs bands, not a redraw |
| `/admin/dev/view-designer` | `view_designer.html` | C | spec | P4 | a custom picker list where a table is the composition |
| `/admin/dev/view-designer/<model>` | `view_designer_model.html` | C | spec | P4 | Studio Phase 1 editor: two Save buttons, sections without bands, the generated spec as a terminal mock |

Shell partials, frozen and untouched: `_base.html`, `_topbar.html`,
`_sidebar.html`, `_theme.html`, `_list_adaptive.html`, `_row_actions.html`,
`includes/_form_field.html`, `view_layer/_cell.html`, `view_layer/_row.html`.

## Cross-cutting findings (not page composition, recorded for the handoff)

- **Footer overflow at 390.** `.rio-appfoot-meta` (`margin-inline-start:
  auto`, no wrap) is pushed 113px past the viewport on every console page at
  390 (`document.scrollWidth` 503). One rule in `components/navigation.css`:
  let `.rio-appfoot-inner` wrap and give the meta `flex-basis: 100%` under
  760.
- **Two header contracts.** Twelve templates still use shape A
  (`.rio-page-header` + `.rio-breadcrumbs`). `components/page-header.css`
  renders both shapes the same, so the swap is cosmetic in the bundle, but
  every C page above moves to shape B (`.rio-crumbs` + `.rio-masthead-top`) so
  there is one head markup left when this pass lands.
- **`.rio-sess-trust` collision.** `pages/account.css` still styles the old
  session-card trust pill under that name; the redesign names its column
  `.rio-sess-col-trust`. The old `.rio-sess-card*` rules can go with it.
