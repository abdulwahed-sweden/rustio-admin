# Component matrix (v3)

Every rustio-admin component, classified against RustIO `main @ 00b933c`.

| Class | Meaning |
|---|---|
| **KEEP** | right today; untouched |
| **ALIGN** | takes what `main` does; markup and behaviour stay |
| **IMPROVE** | a rustio-admin-only quality fix, independent of RustIO |
| **UNIQUE** | PRODUCT-SPECIFIC — KEEP UNIQUE; takes scale and faces only |
| **DROP** | a PR #5-only idea v2 carried; removed |
| **DIVERGENCE** | INTENTIONAL DIVERGENCE from `main`, reason stated |

| Component | Class | Board | Files (would) |
|---|---|---|---|
| Two-row header | UNIQUE · ALIGN (56 + 40) | Shell | `layout/console.css` |
| Module navigation | UNIQUE · ALIGN (14/600) | Shell | `layout/console.css` |
| Contextual rail | ALIGN (240, 38px, no bar) | Shell | `layout/console.css` |
| Rail head | ALIGN | Shell | `layout/console.css` |
| ⌘K palette + trigger | UNIQUE · ALIGN (32px) | Shell | `components/navigation.css` |
| Account menu | UNIQUE · ALIGN (32px) | Shell | `components/navigation.css` |
| Notifications, env pill, Docs link | UNIQUE · ALIGN (faces) | Shell | `components/feedback.css` |
| `page_measure` block | KEEP | Shell | — |
| Workspace gutter | KEEP · ALIGN (24 step) | Shell · Narrow | `tokens/spacing.css` |
| Footer | UNIQUE · ALIGN (faces, timestamp) | Shell | `layout/console.css` |
| Page head (both markup shapes) | ALIGN (`main` `.page-head` grid) | Shell · Data table · Form | `components/page-header.css` |
| Page-head band with rule (v2) | DROP | — | — |
| Breadcrumb | ALIGN (13px) | every board | `components/page-header.css` |
| Hero search | ALIGN (`main` `.find-search`) | Data table | `pages/list.css` |
| Filter triggers | ALIGN (31px) | Data table | `components/buttons.css` |
| Filter panels | UNIQUE · ALIGN (faces) | Data table | `components/navigation.css` |
| More filters, Reset | UNIQUE · ALIGN | Data table | — |
| Result count | ALIGN (`main` `.find-count`) | Data table | `list.html`, `pages/list.css` |
| Active-filter pills | UNIQUE · ALIGN | Data table · History · Dense | `pages/list.css` |
| Sort / direction / rows per page | UNIQUE · ALIGN (moved to `.data-head`) | Data table · Dense | `list.html`, `pages/list.css` |
| View-mode switch, Set as default, Edit view | UNIQUE · ALIGN | View modes | `list.html`, `pages/list.css` |
| Bulk selection + bar + actions | UNIQUE · ALIGN | Data table | `pages/list.css` |
| Table head | ALIGN (`--th-h`, mono) | Data table | `components/data.css` |
| Table rows | ALIGN (`--td-h`) | Data table | `components/data.css` |
| Compact | ALIGN (`main` `.record-compact-item`: 40px lines, no head) | View modes · Dense | `components/adaptive-views.css` |
| Compact tokens 32 / 38 (v2) | DROP | — | — |
| Identity hug (`.rio-cell-fit`) | ALIGN (`main` `.cell-fit` / `column_hugs`) | Dense | `components/data.css`; later a `ConcreteOps` decision |
| Numeric / tabular / nowrap cell behaviour | ALIGN (`main` `.cell-num`, `.record-*-time`, `.table-audit`, `.actions`) | Dense | `components/data.css` |
| Cell rules applied automatically by `rio-td--{{ f.kind }}` | IMPROVE | Dense | `components/data.css` |
| Row actions | ALIGN + IMPROVE (quiet text, always visible) | Data table · Users | `_row_actions.html`; the `opacity: 0` / hover-reveal rules live in `layout/console.css` and the users-grid override in `pages/list.css` |
| Sticky actions column, with companion surface / head / hover / selected / shadow / stacking rules | IMPROVE | Data table · Dense · Narrow | `components/data.css` (the sticky `thead` it must stack above is there too) and `pages/list.css` (`.rio-tablewrap`) |
| `.rio-tablewrap { position: relative }` | IMPROVE | Narrow | `pages/list.css` (where `.rio-tablewrap` is defined; `pages/detail.css` carries a second copy) |
| Users grid | KEEP · ALIGN (faces) | Users & groups | `pages/list.css` |
| Monogram tiles | ALIGN (removed) | Dashboard · Users | templates |
| Pills, role chip, `.av-badge` | ALIGN (the status-badge face) | every list | `components/data.css` (`.rio-pill`), `pages/list.css` (`.rio-role`), `components/adaptive-views.css` (`.av-badge`) |
| Count badges (`.rio-dropdown-badge`, tab counts) | ALIGN (the second, bordered count face — `main` `.module-link .badge`) | Data table · Account | `pages/list.css`, `pages/detail.css` |
| Adaptive List / Cards (cards keep `main`'s `min(100%, 300px)` track guard) | ALIGN | View modes | `components/adaptive-views.css` |
| Card identity floor 12rem | IMPROVE | View modes · API | `adaptive-views.css`, `pages/tools.css` |
| Empty states | ALIGN (flat) | — | `pages/states.css` |
| Pagination | KEEP | Data table | — |
| Ledger (dashboard) | ALIGN (flat tiles) | Dashboard | `pages/dashboard.css` |
| Models board (dashboard) | ALIGN | Dashboard | `index.html`, `pages/dashboard.css` |
| Fieldset | ALIGN (band) · IMPROVE (`min-width: 0`) | Form · Permissions | `pages/form.css` (`.rio-fieldset` box rules; `base/base.css` only resets the element) |
| Inputs, selects, textareas | ALIGN (38px) | Form | `components/forms.css` |
| Boolean field | ALIGN (`main` `.field-boolean`) | Form | `components/forms.css` |
| Form action bar | UNIQUE · ALIGN (faces) | Form | `components/forms.css` |
| Form measure (centred, 728, `--rio-page-form`) | DIVERGENCE (`main`: left-aligned `.form-layout` / `.form-layout--single`) | Form | `tokens/spacing.css` |
| Validation alert | ALIGN (flex) | Form | `components/feedback.css` |
| Inline related sections | UNIQUE · ALIGN | Form | `pages/form.css` |
| Permission matrix | UNIQUE · ALIGN (faces) | Permissions | `pages/permissions.css` |
| Group editor | ALIGN (one action bar) | Permissions | `group_edit.html`, `pages/form.css` |
| Session cards | ALIGN (one list surface) | Account | `pages/account.css`, `account_sessions.html` |
| Sign out everywhere (solid danger) | KEEP | Account | — |
| User detail tabs, dl, timeline, sessions tab | ALIGN | Account | `pages/detail.css` |
| API surface cards | UNIQUE · ALIGN · IMPROVE (identity floor) | API surface | `pages/tools.css` |
| History | UNIQUE · ALIGN | History | `pages/detail.css` |
| Health, DB browser, flags, notifications, docs, view designer, branding, MFA, auth | UNIQUE · ALIGN (scale, faces) | — | `pages/*.css` |
| Timestamps in cells | IMPROVE (one format; the model list already has it) | Dense · Users · Account | `admin/builtin.rs` (users list `created_at`), `admin/render.rs` (`AccountSessionRowCtx.created_at`) |
| AdminTheme override | IMPROVE (hover shade) · DIVERGENCE (never retargets focus; `main`'s `shell.rs` and `session.rs` inject `--focus` from design.json) | Shell | `_theme.html`, `rio-theme` |
| `rio-theme` emitter | KEEP (behaviour) · ALIGN (alias names) | — | `crates/rio-theme`, `TOKENS-EMIT-SPEC.md` |
| Filter select width, summary chevron (PR #5) | DROP | — | — |
| Stale docs (`VISUAL-CONTRACT.md`, `DESIGN_DOCTRINE.md`, `compat.css` header, `admin-shop.png`) | IMPROVE (documented, not edited) | — | docs |
