# Handoff — completion pass

For Claude Code, implementing on a branch cut from `main`. The source of
truth for every composition is `SPEC-COMPLETION.md` and the boards under
`boards/completion/`; the stylesheet layer is `build/composition-completion.css`
(every rule, grouped by page, tokens only). Where a board and the shipped
Jobs / Form / Dashboard composition could disagree, the shipped composition
wins.

## Rules that hold for every page

- **Head shape B everywhere.** Replace `<header class="rio-page-header">` +
  `.rio-breadcrumbs` + `.rio-page-header__lead` + `.rio-page-actions` with
  the `.rio-crumbs` nav followed by `.rio-masthead-top` (title + optional
  `.rio-masthead-desc`, actions in `.rio-masthead-cta`).
  `contract_assets::the_page_header_halves_are_a_matched_pair` must keep
  passing: both halves, once each.
- **Facts line** is `<p class="rio-dash-facts">` (already in
  `pages/dashboard.css`); promote the rule to `components/data.css` since it
  is no longer dashboard-only.
- **A band with an action**: add `.rio-board-title-action` to
  `components/data.css` next to `.rio-board-title` (the shared block at the
  top of the layer), plus the narrow wrap rule for `.rio-board-title`.
- **No runtime hook moves.** Every `name=`, hidden input, `_csrf`, `id`,
  `data-rio-*` attribute, form `action` and route in the templates below is
  preserved verbatim. The boards were built from the real templates with
  the hooks in place.
- **Retire what the composition replaces** in the same commit (listed per
  page) so the bundle does not carry two systems.
- Add every new fragment or rule to the `admin.css` `@import` list and the
  `ADMIN_CSS` concat in the same order; `css_lockstep_tests` enforces it.

## Per page

| Page | Template(s) | CSS fragment to change | Composition change | Retire | Wiring |
|---|---|---|---|---|---|
| Audit log | `log_entries.html` | `pages/tools.css` ← the `AUDIT LOG` block | rows in `<details>` under sticky day bands; find row; ops foot | `.rio-table--audit`, `.rio-hist-when`, `.rio-hist-ip`, `.rio-table-scroll`, `.rio-by`, `.rio-avatar` (tools.css) | `?q= ?action= ?model= ?from= ?to=`, pagination; `time_hm`, `correlation_id` on `HistoryEntryCtx` |
| Record history | `object_history.html` | same block | same rows; facts line; foot link | — | `created_at`, `created_by`, `last_change` on `ObjectHistoryCtx` |
| API surface | `apis_index.html` | `pages/tools.css` ← `API REFERENCE` block | pattern band · models table · schema bands | `.rio-api-grid`, `.rio-api-card*`, `.rio-api-endpoints`, `.rio-api-fields*`, `.rio-api-ftable*`, `.rio-api-sections`, `.rio-api-section*`, `.rio-api-ep-list` (keep `.rio-api-verb*`, `.rio-api-path`) | none |
| Playground | `apis_playground.html` | `pages/tools.css` ← `playground` part of `DEVELOPER PAGES` | form card + response board | `.rio-playground*` | none |
| Health | `health.html` | `pages/tools.css` ← `HEALTH` block | status band; check rows failing first; diagnostics panel | `.rio-health__*` | none (`server_now` exists; order checks in `health_ctx`) |
| Docs index | `docs_index.html` | `pages/tools.css` ← `DOCS` block (list part) | facts line; numbered reading list | `.rio-doc-grid`, `.rio-doc-card*` | none |
| Document | `doc_page.html` | `pages/tools.css` ← `DOCS` block | three-column reading grid; nav; anchors; foot | `.rio-docpage`, `.rio-docpage-card`, `.rio-doc-foot`, `.rio-doc-back` (keep `.rio-doc-prose`, `.rio-doc-toc*`) | `docs`, `prev`, `next` on `DocPageCtx` |
| Sessions | `account_sessions.html` | `pages/account.css` ← `SESSIONS` block | this-device band; others table with band action; danger section | `.rio-sess-card*`, `.rio-sess-row*`, `.rio-sess-rows`, `.rio-sess-bulk*`, `.rio-sess-trust`, `.rio-sess-you`, `.rio-sess-glyph`, `.rio-sess-list`, `.rio-sess-meta`, `.rio-sess-note`, `.rio-sess-foot`, `.rio-sess-title`, `.rio-sess-top` | none |
| Search page | new `search.html` | `pages/list.css` ← `SEARCH` block (page part) | find row with scope; facts; one board per model; three states | — | route + handler (below) |
| Palette | `_base.html`, `admin.js` `initSearchPalette.render()` | `components/navigation.css` ← palette part | counts on groups; hint per result; foot | — | none |
| Notifications | `notifications.html` | `pages/tools.css` ← `NOTIFICATIONS · FLAGS` | head B; facts; rows | `.rio-section*` uses on this page | none |
| Feature flags | `feature_flags.html` | same | head B; facts; ops table; register band | — | none |
| Database | `db_browser.html` | `pages/tools.css` ← `DEVELOPER PAGES` | facts; jump row; table bands; FK foot | `.rio-db-stats`, `.rio-db-stat`, `.rio-db-table*`, `.rio-db-fks*`, `.rio-db-fk-list` | none |
| Schema | `schema.html` | same | same + facts panel | same | none |
| CSV import | `csv_import_result.html` | same (`csv import` part) | drop the inline `<style>`; facts; alert; form card; ops table | the whole `.imp-*` block | none |
| MFA enrol | `mfa_enroll.html` | `pages/form.css` ← `ACCOUNT FLOWS` | form card, two bands | `.rio-form-row` uses here | none |
| Lock account | `lock_user.html` | same | form card; radio list | — | none |
| Edit / add user, add group | `user_edit.html`, `user_new.html`, `group_new.html` | same | one form card, bands, foot | `.rio-fieldset.rio-board` pairing | none |
| Change password, admin reset | `password_change.html`, `admin_reset_password.html` | same | head B; form card | `.rio-card--callout` (reset) | none |
| MFA complete / regenerate / disable | four templates | — | head B only | — | none |
| Branding, view designer ×2 | three templates | `pages/view-designer.css` | bands; one Save; table index | `.rio-vd-picker*` | none |

## The search page — wiring

```
GET /admin/search?q=<term>&scope=<admin_name|all>
  gate: Role::Staff (role_guard), mounted before /admin/:admin_name
  per registered non-core model with search_fields:
     skip unless check_permission(<model>.view_<singular>)
     entry.ops.list(ListOpts { search, search_index_column, limit: 25 })
     total via the count the list page already computes
  ctx: query, scope, groups: [{ display_name, admin_name, shown, total,
       rows: [{ id, label, cells: [...list cells...], edit_url, history_url }] }],
       result_count, model_count
  template: search.html (standard measure, rio-c-list)
```

The palette's *All results for “…” →* and the no-query / no-results states
need nothing beyond this. `/` to focus the field is optional JS in `admin.js`.

## Intentionally unchanged

Dashboard, list (all modes), form, users, groups, group editor, user detail;
login, forgot / sent / reset password, MFA verify, re-auth, must-change-
password; every confirmation page; error and forbidden. The shell: rail,
header rows, module navigation, `⌘K` trigger, account menu, footer (apart
from the one-rule overflow fix). Tokens, `AdminTheme`, `rio-theme`.

## Order to land it in

1. Shared: `.rio-board-title-action`, the title-band wrap at 760, promote
   `.rio-dash-facts`; the footer overflow rule.
2. Audit log + record history (the row anatomy is reused by both).
3. Sessions.
4. Health.
5. API surface + playground.
6. Docs index + document.
7. Palette changes; then the search route, handler, template.
8. Notifications, feature flags, CSV import.
9. Database, schema.
10. Account flows (MFA enrol, lock, edit/add user, add group, passwords,
    the four head swaps).
11. Branding, view designer.

Each step: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
-- -D warnings`, `cargo test --workspace --all-targets`, and
`cargo check --manifest-path examples/fixshop/Cargo.toml`; render the page
at 1440 and 390 with real data before moving on.

## Tests that touch this

- `contract_assets::the_page_header_halves_are_a_matched_pair` — every head
  swap.
- `css_lockstep_tests::import_manifest_matches_concat_bundle` — any new
  fragment.
- `contract_assets::every_rio_alias_resolves_to_a_declared_canonical_token`
  — the layer uses `--rio-*` names only; no new token is introduced.
- Template inventory contract (`embedded_template_names`) — `search.html`
  is a new embedded template.
- `render.rs` unit tests around `map_audit_actions` when `time_hm` /
  `correlation_id` are added; `account_sessions_ctx` is unchanged.
- `handlers.rs` search tests — the new handler shares the loop with
  `search_models`; factor the per-model query into one function and test
  the `view` vs `change` gate once each.

## Risks

- `<details>` in a table-less list: the audit rows are not a `<table>`, so
  the sticky actions-column rule does not apply; nothing else in
  `components/data.css` targets them.
- The `.rio-doc-prose` rules still use compat aliases (`--rio-fs-*`,
  `--rio-s*`); they resolve today and are untouched here. Migrating them to
  canonical names is a separate, mechanical change.
- The search page's `view` gate differs from the palette's `change` gate on
  purpose; document it where `search_models` documents its own.
