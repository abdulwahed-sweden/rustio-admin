# RustIO Admin Professional Redesign Audit

> Inspection-only report. **No code, templates, CSS, or Rust were changed** to
> produce this document; nothing here has been implemented. It maps the real
> `rustio-admin` surface (56 templates, ~84 routes, 53 render-context structs,
> one `admin.js`, a 28-fragment token CSS bundle) and proposes a presentation-
> layer redesign that preserves every internal contract.

---

## 1. Executive Summary

`rustio-admin` already ships a **coherent, token-driven design system** — this
is not a greenfield UI. The chrome is a hyper-served, server-rendered
"RustIO Console": a dark-graphite sidebar + light workspace, an Inter/SF-mono
type stack, a Primer/Carbon-style multi-file CSS bundle (`tokens → base →
layout → components → pages → print`), and a single progressive-enhancement
`admin.js`. Light and dark are **token-only** (re-derived in
`tokens/colors.css`); there is no per-component dark CSS and no build step.

What this means for a redesign: the foundations are sound, so the work is
**refinement, not reinvention** — tightening hierarchy, spacing rhythm, table
readability, form density, empty/`DangerZone` states, and the newer
adaptive-view + view-designer surfaces (which shipped functional but visually
plain). The highest-value, lowest-risk move is a **token + component polish
pass** that touches only `tokens/*.css` and `components/*.css`, leaving every
template's markup, variables, routes, and JS hooks untouched.

The single biggest hazard is `list.html` and its `admin.js`-driven behaviors
(bulk select, FK autocomplete, ⌘K palette, sort/filter/pagination, the
adaptive `{% if adaptive %}` branch). Those must be treated as **read-only
contracts** during visual work.

---

## 2. Design Principle

> **Preserve internal contracts. Redesign only the presentation layer.**

Concretely, for this redesign a change is **in-bounds** only if it touches:

- token values in `assets/static/admin/tokens/*.css`,
- component/page rules in `assets/static/admin/components/*.css` and
  `pages/*.css`,
- or *purely visual* markup (class names, wrapper elements, ordering of
  presentational nodes) **without** removing or renaming any:
  - route path or HTTP method,
  - `{{ context_variable }}` consumed by a template,
  - `name="…"` form field, `action="…"`, or `method=`,
  - `data-rio-*` attribute, `id`, or class that `admin.js` queries,
  - permission/role-gated branch (`{% if not read_only %}`, role-dependent UI),
  - CSRF `_csrf` hidden input, or the bulk `_ids` hidden input.

Runtime behavior (data loading in `handlers.rs`, guards in `routes.rs`,
`ViewSpec` persistence in `view_specs.rs`, the adaptive renderer in
`view_layer/render.rs`) is **out of scope** and must not move.

---

## 3. Page Inventory

Templates live under `crates/rustio-admin/assets/templates/admin/`. Routes are
registered in `src/admin/routes.rs` with `role_guard(…, Role::X)` /
`perm_guard(…, "…")`. Priority = redesign value; Risk = chance of breaking a
contract.

| # | Page / group | Route(s) | Template | Purpose | Priority | Risk |
|---|---|---|---|---|---|---|
| Shell | App shell | (all) | `_base.html` + `_sidebar.html` + `_topbar.html` + `_theme.html` | Layout frame, nav, theme toggle | **High** | **High** |
| 1 | Dashboard | `GET /admin` | `index.html` (`DashboardCtx`) | Landing: counts, recent actions | High | Low |
| 2 | Model list | `GET /admin/:admin_name` | `list.html` (`ListCtx`) | The core data board | **High** | **Very High** |
| 3 | Adaptive list region | (inside list) | `_list_adaptive.html` + `view_layer/_row.html` + `_cell.html` | Card/list/compact rows | High | Med |
| 4 | Create / Edit | `GET/POST /admin/:m/new`, `…/:id/edit` | `form.html` (`FormCtx`) + `includes/_form_field.html`, `_field_errors.html` | Model create/update | **High** | High |
| 5 | Detail | `GET /admin/:m/:id` | (renders `form.html` read-context / detail) | Read view of a record | Med | Med |
| 6 | Delete confirm | `GET/POST /admin/:m/:id/delete` | `confirm_delete.html` (`ConfirmDeleteCtx`) | Cascade-aware delete | Med | Med |
| 7 | Object history | `GET /admin/:m/:id/history` | `object_history.html` (`ObjectHistoryCtx`) | Per-record audit | Med | Low |
| 8 | Bulk delete confirm | `POST /admin/:m/bulk_delete` | `bulk_confirm_delete.html` (`BulkConfirmDeleteCtx`) | Confirm N deletes | Med | High |
| 9 | Bulk action confirm | `POST /admin/:m/bulk/:action` | `bulk_confirm_action.html` (`BulkConfirmActionCtx`) | Confirm custom bulk | Med | High |
| 10 | CSV import result | `POST /admin/:m/import.csv` | `csv_import_result.html` (`CsvImportResultCtx`) | Import outcome | Low | Low |
| 11 | View designer index | `GET /admin/dev/view-designer` | `view_designer.html` (`ViewDesignerIndexCtx`) | Model picker | Med | Low |
| 12 | View designer editor | `GET … /:admin_name`, `POST …/save` | `view_designer_model.html` (`ViewDesignerCtx`) | Edit a `ViewSpec` | Med | **High** |
| 13 | Login | `GET/POST /admin/login` | `login.html` (`LoginCtx`) | Auth entry | High | High |
| 14 | Forgot password | `GET/POST /admin/forgot-password`, `…/sent` | `forgot_password.html`, `forgot_password_sent.html` | Recovery request | Med | High |
| 15 | Reset password | `GET/POST /admin/reset-password/:token` | `reset_password.html` | Token reset | Med | High |
| 16 | Must-change / Reauth | `…/must-change-password`, `…/reauth`, `…/password_change` | `must_change_password.html`, `reauth.html`, `password_change.html` | Forced/step-up flows | Med | High |
| 17 | MFA family | `…/account/mfa/*`, `…/mfa/verify` | `mfa_verify.html`, `mfa_enroll*.html`, `mfa_disable.html`, `mfa_regenerate*.html` | TOTP lifecycle | Med | **High** |
| 18 | Users list | `GET /admin/users` | `users_list.html` | Operator roster | High | Med |
| 19 | User view | `GET /admin/users/:id` | `user_view.html` | Operator detail | Med | Med |
| 20 | User new/edit/delete | `…/users/new|/:id/edit|/:id/delete` | `user_new.html`, `user_edit.html`, `user_confirm_delete.html` | User CRUD | Med | High |
| 21 | User admin actions | `…/:id/lock|/reset-password|/revoke-sessions|/unlock` | `lock_user.html`, `admin_reset_password.html`, `confirm_admin_action.html` | Lock/reset/revoke | Med | **High** |
| 22 | Groups list/CRUD | `GET /admin/groups`, `…/new|/:id/edit|/:id/delete` | `groups_list.html`, `group_new.html`, `group_edit.html`, `group_confirm_delete.html` | Roles + **permission grid** | High | **High** |
| 23 | Account sessions | `GET /admin/account/sessions` | `account_sessions.html` (`AccountSessionsCtx`) | Self session mgmt | Med | High |
| 24 | Notifications | `GET /admin/notifications` | `notifications.html` (`NotificationsCtx`) | Operator inbox | Low | Low |
| 25 | History / audit | `GET /admin/history` | `log_entries.html` (`LogEntriesCtx`) | Global audit log | Med | Low |
| 26 | DB browser (dev) | `GET /admin/db` | `db_browser.html` | Schema explorer | Med | Low |
| 27 | Health (dev) | `GET /admin/health` | `health.html` (`HealthCtx`) | Diagnostics | Low | Low |
| 28 | Feature flags | `GET/POST /admin/feature_flags`, `…/:key/toggle` | `feature_flags.html` (`FeatureFlagsCtx`) | Flag toggles | Low | Low |
| 29 | Docs | `GET /admin/docs`, `…/:slug` | `docs_index.html`, `doc_page.html` | Embedded docs | Low | Low |
| 30 | APIs | `GET /admin/apis`, `…/playground` | `apis_index.html`, `apis_playground.html` | OpenAPI/SDK explorer | Low | Med |
| 31 | Errors | (any) | `error.html` (`ErrorCtx`), `forbidden.html` (`ForbiddenCtx`) | 4xx/5xx + 403 | Low | Low |

---

## 4. Internal Data Contracts To Preserve

These are the verified contracts a visual redesign must not disturb. Names are
from the real templates / `render.rs` structs / `admin.js`.

### 4.1 Shell (`_base.html`, `_sidebar.html`, `_topbar.html`, `_theme.html`)
- **Blocks:** `title`, `extra_head`, `sidebar`, `topbar`, `content` — every
  page extends `_base.html` and fills `content`. Renaming a block breaks all
  pages.
- **`BaseContext` (flattened into every page):** `identity`, `csrf_token`,
  `site_title`, `site_header`, `index_title`, `footer_copyright`, `app_name`,
  `app_tagline`, `show_powered_by`, `framework_version`, `environment_label`,
  `unread_count`. Referenced as bare `{{ csrf_token }}`, `{{ app_name }}`, etc.
- **JS hooks (shell):** `data-rio-sidebar-toggle` (`initSidebar`),
  `data-rio-search-trigger` + `data-rio-search-palette` /
  `-dialog` / `-input` / `-results` (`initSearchPalette`, ⌘K → `GET /admin/_search`),
  `data-rio-dropdown` + `.rio-dropdown-toggle` / `.rio-dropdown-panel`
  (`initDropdowns`), and `initConsole` (theme toggle writing
  `[data-theme]`, collapsible rail). The `icon()` minijinja global is used
  throughout.

### 4.2 List (`list.html`, `ListCtx`)
- **Context:** `fields[]` (`name`, `kind`, `label`, `sort_active`, `sort_link`),
  `rows[]` (`id`, flattened `values` → `row[f.name]`, `links[f.name]`,
  `highlights[f.name]`), `search_query`, `filters[]`, `active_filter_pills[]`,
  `clear_all_filters_link`, `csv_export_url`, sort (`sort_fields`,
  `default_sort_link`, `active_sort_field/_dir`, `sort_dir_toggle_link`),
  per-page (`per_page_options`, `current_per_page_label`,
  `active_per_page_override`), pagination (`page`, `total_pages`, `total_rows`,
  `prev_page_link`, `next_page_link`, `page_items[]`), `bulk_actions_enabled`,
  `bulk_action_buttons[]` (`form_action`, `label`, `destructive`),
  `saved_filters[]`, `current_query_string`, `flash`, **`read_only`**, and the
  adaptive additions **`adaptive`** (Optional `RenderedView`) + **`mode_links[]`**.
- **Forms / actions:** bulk `<form … action="/admin/{{admin_name}}/bulk_delete"
  data-rio-bulk>` with hidden `_csrf` and `_ids`; per-row edit/delete links
  `"/admin/{{admin_name}}/{{row.id}}/edit|delete"`; saved-filter
  `POST /admin/:m/saved_filters` (+ `…/:id/delete`).
- **JS hooks:** `data-rio-bulk`, `-all`, `-row`, `-ids`, `-count`, `-clear`
  (`initBulkSelect`, CSV `_ids`); `data-rio-row-actions` (`initRowActions`
  kebab `<details>`); `data-rio-fk-autocomplete` + `-field-name` / `-id` /
  `-lookup-url` / `-results` / `-search` (`initFkAutocomplete`,
  `GET /admin/_lookup/:admin_name`) in filter widgets.
- **Critical branch:** `{% if adaptive %}{% include "admin/_list_adaptive.html" %}{% else %}<legacy table>{% endif %}`
  and `{% if not read_only and not adaptive %}` gating the bulk form. **Do not
  reorder or collapse these.**

### 4.3 Adaptive (`_list_adaptive.html`, `view_layer/_row.html`, `_cell.html`)
- **Context:** `adaptive.mode`, `adaptive.rows[]` (`id`, `cells[]`); each cell is
  serde-tagged — templates switch on `cell.kind` (`primary`/`secondary`/`badge`/
  `timestamp`/`composed`) and never on roles. Row link
  `"/admin/{{admin_name}}/{{row.id}}/edit"`.
- **Hard rule:** `Hidden` fields never reach this context — markup must not
  attempt to surface "hidden" data.

### 4.4 Form (`form.html`, `FormCtx`, `includes/_form_field.html`)
- **Action:** `{% if mode == 'new' %}/admin/{{admin_name}}/new{% else %}/admin/{{admin_name}}/{{object_id}}/edit{% endif %}`, `method="post"`, hidden `_csrf`.
- **Field names** are emitted from the model's `AdminField.name` — every
  `name="…"` is a data contract with `Model::from_form`. Field rendering goes
  through `includes/_form_field.html` (+ `_field_errors.html`); FK fields carry
  the `data-rio-fk-*` autocomplete hooks. Inline formsets via `FormInlineCtx` /
  `FormInlineRowCtx`.

### 4.5 Auth & step-up
- `login.html`: `POST /admin/login`, fields `email`, `password`, `_csrf`
  (rendered via `_form_field` sections). `mfa_verify.html`: `POST /admin/mfa/verify`.
  `reset_password.html`: `POST /admin/reset-password/{{token}}`.
  `password_change.html` / `reauth.html` / `must_change_password.html` each POST
  to their own route. **All carry `_csrf`; uniform-response surfaces (recovery)
  must keep their single response shape — do not add field-level "user not
  found" hints.**

### 4.6 Users / Groups / Permissions
- **Permission grid** (`group_edit.html`, user pages): `data-rio-perm-checkbox`,
  `data-rio-perm-row`, `data-rio-perm-row-all` drive the JS row/column toggles —
  every checkbox `name`/`value` is read server-side. Admin actions POST to
  `/admin/users/{{id}}/lock|reset-password|revoke-sessions|unlock` and
  `/admin/groups/{{id}}/edit|delete`, each with `_csrf`. Last-developer guards
  and disabled states are permission-dependent UI.

### 4.7 View designer (`view_designer_model.html`, `ViewDesignerCtx`)
- **Save:** `POST /admin/dev/view-designer/{{admin_name}}/save`, `_csrf`.
- **Field names (must match `handlers::spec_from_form`):** `role__<name>`,
  `priority__<name>`, `filter__<name>`, `default_mode`, `mode_allowed__<slug>`,
  and per slot `comp<i>_primary` / `comp<i>_label` / `comp<i>_style` /
  `comp<i>_sec__<name>`. Selects rely on `{slug==current} selected` and the
  disabled default-mode checkbox. **Renaming any of these silently drops data
  on save.**

---

## 5. Page-by-Page Redesign Recommendations

### Page: App shell — `_base.html` / `_sidebar.html` / `_topbar.html`
- **Current issues:** solid graphite rail is good; the topbar mixes search,
  env-badge, bell, theme toggle, Docs, and the account menu with uneven
  spacing; sidebar section eyebrows (`MODELS` / `ACCESS` / `DEVELOPER`) are
  understated; active-item affordance is subtle.
- **Redesign:** tighten topbar to a 3-zone grid (brand/search · status ·
  account) on a consistent baseline; give the sidebar a clearer active pill and
  a slightly stronger section label; unify the focus ring (teal) across all
  interactive chrome. Pure token + `layout/console.css` / `components/navigation.css`.
- **Preserve:** the five blocks; `data-rio-sidebar-toggle`,
  `data-rio-search-*`, `data-rio-dropdown`, the theme-toggle `[data-theme]`
  write; `icon()` calls; the `unread_count` bell.
- **New layout:** unchanged structure; refined spacing/elevation only.
- **Risk:** **High** — every page depends on this. Token-only changes are safe;
  do not touch the include graph.

### Page: Dashboard — `index.html` (`DashboardCtx`)
- **Issues:** count tiles + recent-actions list feel utilitarian; weak visual
  rhythm between sections.
- **Redesign:** a clean metric-tile grid (`StatusBadge`-style accents),
  consistent `PageHeader`, a refined `RecentActionCtx` timeline.
- **Preserve:** `DashboardCtx` fields, links to models, `RecentActionCtx` rows.
- **Risk:** Low.

### Page: Model list — `list.html` (`ListCtx`) **(flagship, highest risk)**
- **Issues:** dense toolbar (search · filters · sort · per-page) wraps awkwardly
  at mid widths; table hairlines and row hover are slightly heavy; the bulk bar,
  active-filter pills, and pagination are visually disconnected; the new
  `ViewModeSwitcher` (`rio-view-modes`) reads as plain text tabs.
- **Redesign:** harmonize the toolbar into one `DataToolbar` row with
  consistent control heights; calmer table (lighter `--rio-line`, clearer
  `sort_active` markers, comfortable row padding, sticky header optional);
  promote the bulk bar and pagination to first-class `components/data.css`
  patterns; style `rio-view-mode` as a real segmented control.
- **Preserve (critical):** the `{% if adaptive %}…{% else %}<table>…{% endif %}`
  branch byte-for-byte in behavior; the `not read_only` / `not adaptive` gates;
  every `data-rio-bulk*`, `data-rio-row-actions`, `data-rio-fk-*` hook;
  `row.id` checkbox `value`, `row.links[f.name]`, `row.highlights[f.name]|safe`,
  `f.sort_link` / `f.sort_active`; the bulk form `_csrf` + `_ids`.
- **Risk:** **Very High** — visual-only via `pages/list.css` + `components/data.css`;
  no markup reordering inside the table loop.

### Page: Adaptive list/cards/compact — `_list_adaptive.html` + `view_layer/*`
- **Issues:** cards/list/compact shipped with `components/adaptive-views.css`
  but are minimal; cards lack an image/affordance slot; compact density could be
  tighter; badges and timestamps need clearer type roles.
- **Redesign:** richer `AdaptiveCards` (title + meta + badge zone, hover
  elevation already present), denser `AdaptiveCompact`, clearer `AdaptiveList`
  separation. Token + `components/adaptive-views.css` only.
- **Preserve:** `cell.kind` switching, `row.id` link, no-hidden-fields rule.
- **Risk:** Med.

### Page: Create/Edit — `form.html` (`FormCtx`)
- **Issues:** long single-column forms; fieldset grouping and help/error text
  hierarchy could be stronger; FK autocomplete results panel styling is plain.
- **Redesign:** a `FormPanel` with section headers (from `Fieldset`), a clear
  label/control/help rhythm, refined `_field_errors.html`, a tidy FK results
  dropdown, and a sticky action bar (Save / Cancel) — without moving inputs.
- **Preserve:** `action`/`method`, every `name="…"`, `_csrf`, FK `data-rio-fk-*`,
  inline formset structure, error context.
- **Risk:** High (field names are contracts).

### Page: Detail / Delete / History — `form.html` (read), `confirm_delete.html`, `object_history.html`
- **Redesign:** a `DetailPanel` (label/value rows), a `DangerZone`-styled delete
  confirm that surfaces cascade lists clearly, and a readable per-object audit
  timeline (`HistoryEntryCtx` / `HistoryChangeCtx`).
- **Preserve:** delete `POST` action + `_csrf`; cascade context; history fields.
- **Risk:** Med.

### Page: Bulk confirms / CSV — `bulk_confirm_delete.html`, `bulk_confirm_action.html`, `csv_import_result.html`
- **Redesign:** consistent confirm-dialog pattern (count summary, item preview,
  `DangerZone` accent for destructive). 
- **Preserve:** the re-POST of selected ids + `action_name`, `_csrf`. **Risk: High**
  (bulk id round-trip).

### Page: View designer — `view_designer.html`, `view_designer_model.html`
- **Issues:** functional but plain; the long stacked sections (default mode /
  allowed modes / fields table / 3 composition slots) need stronger grouping;
  the live preview needs a framed "device" feel; the composition slots are
  visually repetitive.
- **Redesign:** a two-column editor (controls left, **`ViewSpecInspector`** +
  live preview right), card-framed composition slots, a `DeveloperNotice`
  banner. Token + a small designer-scoped page CSS.
- **Preserve:** all `role__/priority__/filter__/default_mode/mode_allowed__/comp<i>_*`
  names; `selected`/`checked` logic; the preview include chain.
- **Risk:** **High** (save-form field names).

### Page: Auth & MFA — `login.html`, `forgot_password*`, `reset_password.html`, `mfa_*`, `reauth.html`, `must_change_password.html`, `password_change.html`
- **Issues:** auth cards are clean but generic; MFA enroll (QR + codes) and
  backup-code pages need a more deliberate "security" tone.
- **Redesign:** a unified `auth-card` (centered, brand wordmark from `app_name`,
  calm graphite), clearer MFA enroll steps and backup-code presentation.
- **Preserve:** every POST action + field names + `_csrf`; **uniform recovery
  response shape** (no field-level account-existence hints); MFA step semantics.
- **Risk:** **High** (security surfaces).

### Page: Users / Groups / Permissions — `users_list.html`, `user_view.html`, `group_edit.html`, etc.
- **Issues:** the **permission grid** is the densest UI in the product and the
  hardest to scan; user/group lists are plain tables; admin-action confirms vary
  in tone.
- **Redesign:** a high-readability `PermissionGrid` (sticky row/col headers,
  zebra-free calm rows, teal "granted" affordance, the existing row/col toggles
  kept), consistent roster tables, unified admin-action confirm dialogs.
- **Preserve:** `data-rio-perm-checkbox/-row/-row-all`, every checkbox
  `name`/`value`, last-developer disabled states, all action POSTs + `_csrf`.
- **Risk:** **High** (grid JS + auth actions).

### Page: Account sessions / Notifications — `account_sessions.html`, `notifications.html`
- **Redesign:** a session list with device/IP/last-seen and clear revoke
  affordances; a calmer notifications inbox with read/unread states feeding
  `unread_count`.
- **Preserve:** revoke POSTs (`…/sessions/:id/revoke`, `revoke-all`,
  `revoke-others`), `mark_all_read` POST, `_csrf`. **Risk: High** (session revoke).

### Page: Developer/tools — `db_browser.html`, `health.html`, `feature_flags.html`, `docs_*`, `apis_*`
- **Issues:** these are powerful but visually inconsistent with the main app;
  `db_browser` and `apis_playground` read as raw output.
- **Redesign:** a shared **`DeveloperNotice`** header + `code/spec inspector`
  styling (reuse `components/code.css`), consistent tool-page chrome
  (`pages/tools.css`). Feature-flags + health become tidy status panels.
- **Preserve:** flag toggle POST + `:key`, `_csrf`; OpenAPI/SDK links; health
  context. **Risk: Low–Med.**

### Page: Errors — `error.html`, `forbidden.html`
- **Redesign:** dignified empty-state-style pages with a clear status code,
  message, and a path back. **Preserve** `ErrorCtx` / `ForbiddenCtx` fields and
  the `min.label()` role hint on 403. **Risk: Low.**

---

## 6. Global UI System Recommendations

Direction: **serious, Rust-native, calm-enterprise.** Dark-graphite foundation
(already the sidebar), light precise workspace, strong type hierarchy, generous
but disciplined spacing, **teal/blue accent used only for meaning** (active,
focus, primary action, "granted") — never decoration. No playful SaaS gradients.
All of the below is achievable in `tokens/*` + `components/*` + `pages/*`.

- **Layout shell** — keep the rail + workspace; tighten the 3-zone topbar,
  unify focus rings, standardize page max-width and gutters.
- **Sidebar** — clearer active pill, stronger section eyebrows, consistent icon
  alignment.
- **Top bar** — baseline-aligned controls; env badge, ⌘K, bell, theme, account
  on one rhythm.
- **Cards** — single `.rio-card` elevation language (`--rio-shadow-sm` +
  `--rio-highlight-top`), consistent radius (`--rio-radius-lg`) and padding.
- **Tables** — lighter hairlines, comfortable density, unmistakable sort state,
  sticky header option, calm hover; this is the readability win.
- **Adaptive views** — segmented `ViewModeSwitcher`; richer cards; denser
  compact; clear type roles (primary/secondary/timestamp/badge).
- **Forms** — one label/control/help/error rhythm; clear required/disabled/
  invalid states; tidy FK autocomplete panel.
- **Buttons** — finalize the `components/buttons.css` ladder (primary / secondary
  / subtle / ghost / danger) with consistent sizes and one focus ring.
- **Badges** — token tints only (`--rio-success/-warn/-danger/-rust` + tints),
  consolidate `rio-badge` / `rio-pill` / `av-badge` into one semantic scale.
- **Filters** — unify the chip / multi-select / date-range / FK-autocomplete
  widgets into one filter language.
- **Modals/dialogs** — a single confirm-dialog pattern (used by delete, bulk,
  admin actions) with a `DangerZone` accent.
- **Empty states** — one `EmptyState` (icon + title + lead + primary action),
  applied to lists, search, notifications, audit.
- **Alerts** — one `flash` / banner language (info/success/warn/danger) reused
  everywhere (`components/feedback.css`).
- **Code / spec inspectors** — reuse `components/code.css` for `db_browser`,
  `apis_playground`, and the designer's `ViewSpecInspector`.
- **Developer-only pages** — a shared `DeveloperNotice` marker so `/admin/db`,
  `/admin/health`, `/admin/dev/view-designer` read as a coherent dev surface.

---

## 7. Component Map

(Component = a recurring presentational pattern; most already exist implicitly
in templates/CSS — this names them for consolidation.)

| Component | Used in | Data it needs | Behavior to preserve | Visual improvement |
|---|---|---|---|---|
| **AppShell** | `_base.html` | `BaseContext` + `block content` | 5 blocks, include graph | gutters, max-width, focus ring |
| **Sidebar** | `_sidebar.html` | `entries[]`, role-gated links, `icon()` | `data-rio-sidebar-toggle`, active state | active pill, section eyebrows |
| **HeaderBar** | `_topbar.html` | search, `unread_count`, identity, theme | `data-rio-search-*`, `-dropdown`, theme toggle | 3-zone baseline grid |
| **PageHeader** | most pages | `page_title`, breadcrumbs, primary action | — | consistent title/lead/action |
| **DataToolbar** | `list.html` | search/filter/sort/per-page + saved filters | every `data-rio-*` + links | one-row control rhythm |
| **LegacyTable** | `list.html` `{% else %}` | `fields[]`, `rows[]` (id/values/links/highlights), `read_only` | bulk/row/fk hooks, sort links, checkbox `value` | lighter lines, sort clarity |
| **AdaptiveList/Cards/Compact** | `_list_adaptive.html` + `view_layer/*` | `adaptive.mode`, `rows[].cells[]`, `row.id` | `cell.kind` switch, edit link, no-hidden | richer cards, density, type roles |
| **FormPanel** | `form.html` | `FormCtx`, `Fieldset`s, `_form_field` | action/method, field `name`s, `_csrf`, FK hooks | section rhythm, sticky actions |
| **DetailPanel** | detail render | record values | links | label/value rows |
| **StatusBadge** | lists, dashboard, health, flags | value + semantic | — | one semantic tint scale |
| **EmptyState** | lists, search, notifications, audit | icon/title/lead/action | — | single pattern |
| **DangerZone** | delete/bulk/admin confirms | item summary + POST | re-POST ids, `action_name`, `_csrf` | consistent destructive accent |
| **ViewModeSwitcher** | `list.html` | `mode_links[]` (`href`, `active`) | hrefs preserve query | segmented control |
| **ViewSpecInspector** | designer | `ViewSpec` / preview | preview include chain | code-styled spec view |
| **DeveloperNotice** | dev pages | static | — | shared dev-surface marker |

---

## 8. Redesign Priorities (phased)

### Phase 1 — Safe Visual Polish (token + component CSS only)
No markup changes. Refine `tokens/*` (spacing rhythm, line/shadow weights,
type scale, accent usage) and `components/*` (cards, buttons, badges, tables,
feedback). Re-verify light **and** dark via the token blocks only.
**Files:** `tokens/*.css`, `components/*.css`. **Behavior risk: none.**

### Phase 2 — Page Structure Polish (presentational markup, contracts frozen)
`PageHeader`, `DataToolbar`, `EmptyState`, `DangerZone`, `FormPanel` rhythm —
wrapper/class refinements that keep every variable, `name`, action, and hook.
**Files:** `pages/*.css` + minimal class/wrapper tweaks in stable templates
(dashboard, errors, confirms, account, notifications).

### Phase 3 — Adaptive View + Designer Polish
`components/adaptive-views.css` richer cards/compact; designer two-column layout
+ `ViewSpecInspector` (no form-field renames). **Files:**
`components/adaptive-views.css`, designer page CSS, `view_designer_model.html`
(visual wrappers only).

### Phase 4 — Component Consolidation
Extract repeated patterns into shared partials where the include graph already
supports it (e.g. a single confirm-dialog partial behind `confirm_delete` /
`bulk_confirm_*` / admin-action confirms; one `EmptyState` partial). Pure
refactor of presentation, contracts unchanged; guarded by the existing
`every_handler_rendered_template_resolves` test.

### Phase 5 — Optional Larger Redesign (only after tests)
Deeper layout moves (e.g. sticky list headers, list density modes) — only with
a visual-regression pass and the contracts in §4 re-audited.

---

## 9. Risk Analysis

| Surface | Why risky | Guardrail |
|---|---|---|
| **`list.html` + legacy table** | the `{% if adaptive %}/{% else %}` branch, `not read_only`/`not adaptive` gates, and every `row.id`/`links`/`highlights`/`sort_link` are behavioral | CSS-only; never reorder the table loop; the existing render-both-branches test must stay green |
| **Bulk actions** | `admin.js` builds the `_ids` CSV from `data-rio-bulk-*`; the bulk bar visibility is JS-driven (`.is-active`/`.is-selected`) | keep all `data-rio-bulk*` attrs + the `<form data-rio-bulk>` wrapper + `_ids`/`_csrf` |
| **Search / filter / sort / pagination** | links are server-baked (`sort_link`, `clear_all_filters_link`, `prev/next`, `csv_export_url`); FK + ⌘K are JS | never rewrite these hrefs/hooks; restyle only |
| **Adaptive modes** | `cell.kind` switching + no-hidden-fields invariant | switch on data only; never surface hidden |
| **View designer save** | `spec_from_form` parses exact `role__/priority__/filter__/default_mode/mode_allowed__/comp<i>_*` names | freeze names; parser tests must stay green |
| **Auth / MFA / recovery** | CSRF, uniform-response recovery, step-up semantics, last-developer guards | keep field names + `_csrf`; no account-existence hints; preserve disabled/guard states |
| **Permission grid** | `data-rio-perm-*` JS + checkbox `name`/`value` map to grants | keep attrs + names; restyle the grid only |
| **Session revoke / admin actions** | destructive POSTs (`revoke-all`, `lock`, `reset-password`) | keep actions + `_csrf`; only re-skin the confirm |

CI guardrails that already protect a visual redesign: `cascade_lockstep.rs`
(the `@import` ↔ `ADMIN_CSS` lock-step), `every_handler_rendered_template_resolves`
and `every_embedded_template_loads` (template wiring), and the Tier-2 symbol
guard. Any new CSS fragment must be registered in **both**
`assets/static/admin/admin.css` and the `ADMIN_CSS` concat in `routes.rs`.

---

## 10. Recommended Next PR

**Title:** *"Professional visual polish — tokens & components, no behavior change"*

**Scope (Phase 1 only):**
- Refine `tokens/spacing.css`, `tokens/typography.css`, `tokens/colors.css`,
  `tokens/shadows.css` (rhythm, type scale, line/shadow weights, accent
  discipline) — light and dark re-derived in the token blocks only.
- Refine `components/data.css` (tables, cards, badges), `components/buttons.css`,
  `components/feedback.css` (alerts/flash), `components/navigation.css` (sidebar/
  topbar active + focus), and `components/adaptive-views.css` (switcher +
  cards/compact).

**Explicitly NOT in this PR:** any `.html` change, any route/handler change, any
new CSS fragment (so the lock-step list is untouched), any token *additions*
(value tweaks only; a *new* `--rio-*` token would need a CHANGELOG entry).

**Verification:** `cargo fmt --check`, `clippy -D warnings`, full suite +
`cascade_lockstep`, plus a manual light/dark pass on the list, form, dashboard,
designer, and auth pages (the live shop example is the fastest visual harness).

This keeps the first redesign PR **diff-minimal, contract-zero, and reversible.**

---

## 11. Do Not Change Yet

In the first redesign PR (and until each is individually re-audited), do **not**
touch:

- **Route handlers** — `src/admin/handlers.rs`, `builtin.rs` (no data-loading,
  context-building, or response changes).
- **Routing & guards** — `src/admin/routes.rs` (paths, methods, `role_guard` /
  `perm_guard`).
- **Permissions / roles** — any permission-dependent UI branch or role gate.
- **ViewSpec persistence** — `src/admin/view_specs.rs` and the
  `view_designer_model.html` save-form **field names**.
- **Legacy table logic** — the `list.html` `{% if adaptive %}/{% else %}` branch
  and `not read_only` / `not adaptive` gates.
- **Bulk actions** — `<form data-rio-bulk>`, `_ids`, `data-rio-bulk-*`, and
  `handle_bulk_delete` / `bulk/:action`.
- **Form field names** — every `name="…"` (model `from_form` + designer parser).
- **JavaScript hooks** — every `data-rio-*` attribute and the `id`/class
  selectors `admin.js` queries; `admin.js` itself.
- **Adaptive renderer logic** — `src/view_layer/render.rs`, `_cell.html` /
  `_row.html` `cell.kind` switching, the no-hidden-fields rule.
- **Security behavior** — `_csrf` inputs, uniform recovery responses, MFA/step-up
  flows, last-developer guards.

— End of audit. Nothing in this document has been implemented; it is a plan for
review only.
