# Component matrix

Every rustio-admin component, classified. One row per component; the board
that shows it; the files its change would live in. Classes:

| Class | Meaning |
|---|---|
| **KEEP** | right today; untouched |
| **ALIGN** | ALIGN WITH RUSTIO: takes RustIO's composition, scale or face; markup and behaviour stay |
| **IMPROVE** | IMPROVE RUSTIO-ADMIN: a quality fix independent of RustIO |
| **UNIQUE** | PRODUCT-SPECIFIC — KEEP UNIQUE: no RustIO equivalent; takes scale and faces only |

| Component | Class | Board | Files (would) |
|---|---|---|---|
| Two-row header (utility + module row) | UNIQUE · ALIGN (heights 56 + 40) | Shell | `layout/console.css` |
| Module navigation | UNIQUE · ALIGN (14/600, 16px icons) | Shell | `layout/console.css` |
| Contextual rail | ALIGN (240, 38px items, no bar) | Shell | `layout/console.css` |
| Rail head (brand) | ALIGN (level with the utility row) | Shell | `layout/console.css` |
| ⌘K palette + trigger | UNIQUE · ALIGN (32px trigger) | Shell | `layout/console.css`, `components/navigation.css` |
| Account menu | UNIQUE · ALIGN (32px trigger, 24px avatar) | Shell | `components/navigation.css` |
| Notifications bell | UNIQUE | Shell | — |
| Environment pill | UNIQUE · ALIGN (env chip face) | Shell | `components/feedback.css` |
| Docs link | ALIGN (quiet small control) | Shell | `components/buttons.css` |
| `page_measure` block | KEEP | Shell | — |
| Workspace gutter | KEEP (principle) · ALIGN (24 step ≤ 760) | Shell · Narrow | `tokens/spacing.css` |
| Footer | UNIQUE · ALIGN (faces, timestamp) | Shell | `layout/console.css` |
| Page head (both markup shapes) | ALIGN (band with rule, 24px) | Shell · Data table · Form | `components/page-header.css` |
| Breadcrumb | ALIGN (13px) | every board | `components/page-header.css` |
| Hero search | ALIGN (38 × 340 field) | Data table | `components/forms.css`, `pages/list.css` |
| Filter dropdown triggers | ALIGN (31px) | Data table | `components/buttons.css` |
| Filter panels: chips, FK autocomplete, multi-select, date range | UNIQUE · ALIGN (faces; 240px floor) | Data table | `components/navigation.css` |
| More filters + count | UNIQUE | Data table | — |
| Reset | ALIGN (quiet small) | Data table | — |
| Result count | ALIGN (new on the find row) | Data table | `pages/list.css`, `list.html` |
| Active-filter pills | UNIQUE · ALIGN (13/600) | Data table · History · Dense | `pages/list.css` |
| Sort / direction / rows per page | UNIQUE · ALIGN (moved to the surface head, 31px) | Data table · Dense | `list.html`, `pages/list.css` |
| View-mode switch | UNIQUE · ALIGN (31px segments on the surface head) | View modes | `list.html`, `pages/list.css` |
| Set as default / Edit view | UNIQUE | View modes | — |
| Bulk selection + bulk bar + bulk actions | UNIQUE · ALIGN (strip on `--blue-soft`, quiet red destructive) | Data table | `pages/list.css` |
| Table head | ALIGN (40px, 13px mono, `--surface-head`) | Data table | `components/data.css` |
| Table rows | ALIGN (48px, 16px padding) | Data table | `components/data.css` |
| Compact table | ALIGN (32 / 38 tokens) | Dense · View modes | `components/data.css`, `tokens/spacing.css` |
| Cells by kind (`rio-td--*`) | ALIGN + IMPROVE (nowrap, tabular, numeric right) | Dense | `components/data.css` |
| Identity column hug | IMPROVE (CSS now; a `ConcreteOps` decision later) | Dense | `components/data.css` |
| Row actions | ALIGN + IMPROVE (quiet text, always visible) | Data table · Users | `_row_actions.html`, `components/data.css` |
| Sticky actions column | IMPROVE | Data table · Dense · Narrow | `components/data.css` |
| `.rio-tablewrap` | IMPROVE (`position: relative`) | Narrow | `components/data.css` |
| Users grid | KEEP · ALIGN (48px, faces) | Users & groups | `pages/list.css` |
| Monogram tiles | ALIGN (removed) | Dashboard · Users | `index.html`, `users_list.html`, `groups_list.html` |
| Pills (`.rio-pill`) | ALIGN (dot badge) | every list | `components/feedback.css` |
| Role chip (`.rio-role`) | ALIGN (same face, mono text) | Users | `components/feedback.css` |
| View-layer badges (`.av-badge`) | ALIGN (same face) | View modes | `components/adaptive-views.css` |
| Adaptive List / Cards / Compact | ALIGN | View modes | `components/adaptive-views.css` |
| Empty states (`.rio-empty`, `.rio-empty-state`) | ALIGN (flat, inside the surface) | — | `pages/states.css` |
| Pagination | KEEP | Data table | — |
| Ledger (dashboard) | ALIGN (flat tiles) | Dashboard | `pages/dashboard.css` |
| Models board (dashboard) | ALIGN (quiet Browse / Add, no tiles) | Dashboard | `index.html`, `pages/dashboard.css` |
| Fieldset (legend on border) | ALIGN (mono band) · IMPROVE (`min-width: 0`) | Form · Permissions | `components/forms.css` |
| Inputs, selects, textareas | ALIGN (38px) | Form | `components/forms.css` |
| Boolean field | ALIGN (38px bordered row) | Form | `components/forms.css` |
| Form action bar (three save variants + text actions) | UNIQUE · ALIGN (faces) | Form | `components/forms.css` |
| Validation alert | ALIGN (flex row) | Form | `components/feedback.css` |
| Inline related sections | UNIQUE · ALIGN (faces) | Form | `pages/form.css` |
| Form measure | ALIGN (728) | Form | `tokens/spacing.css` |
| Permission matrix | UNIQUE · ALIGN (40px rows, boxes on the primary) | Permissions matrix | `pages/permissions.css` |
| Group editor | ALIGN (one action bar) | Permissions matrix | `group_edit.html`, `pages/form.css` |
| Session cards | ALIGN (one list surface) | Account & sessions | `pages/account.css`, `account_sessions.html` |
| Sign out everywhere (solid danger) | KEEP | Account & sessions | — |
| User detail tabs | ALIGN (38px, underline) | Account & sessions | `pages/detail.css` |
| Detail list (`.rio-dl`) | ALIGN (140px label column) | Account & sessions | `pages/detail.css` |
| Timeline | ALIGN (faces) | Account & sessions | `pages/detail.css` |
| Sessions tab table | ALIGN (nowrap, UA ellipsis) | Account & sessions | `pages/detail.css` |
| API surface cards | UNIQUE · ALIGN (flat, identity floor) | API surface | `pages/tools.css` |
| History table, diffs, date dividers | UNIQUE · ALIGN (48px, badges) | History | `pages/detail.css` |
| Health, DB browser, feature flags, notifications, docs viewer, view designer, branding, MFA, auth | UNIQUE · ALIGN (scale and faces only) | — | respective `pages/*.css` |
| Timestamps in cells | IMPROVE + ALIGN (one format) | Dense · Users · Account | formatters (Rust), `account_sessions.html` |
| AdminTheme override | IMPROVE (hover shade) | Shell | `_theme.html`, `rio-theme` |
| `rio-theme` emitter | KEEP (behaviour) · ALIGN (alias names) | — | `crates/rio-theme`, `TOKENS-EMIT-SPEC.md` |
| `VISUAL-CONTRACT.md`, `DESIGN_DOCTRINE.md`, `compat.css` header, `admin-shop.png` | IMPROVE (stale; documented, not edited) | — | docs |
