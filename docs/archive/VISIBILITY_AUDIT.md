# VISIBILITY_AUDIT — rustio-admin 0.8.0

A read-only survey of every framework surface, what it's wired to, and where the wiring is broken. **No code is changed in this document.** Recovery sequencing follows in a separate post.

The framework's R0–R4 architecture is sound. The visible weaknesses are integration and surfacing, not capability.

---

## 0. Executive summary

| # | Finding | Severity |
|---|---|---|
| **F1** | Audit log writes **four different `model_name` conventions** for the same logical model (`"User"`, `"user"`, `"users"`, `"rustio_users"`). The History page renders these as URL slugs. Three of the four 404. | 🔴 Admin integrity |
| **F2** | Built-in `User` / `Group` / History live in a **hardcoded `_sidebar.html` block**, parallel to (and inconsistent with) the dynamic `entries[]` loop for project models. Two label-source paths, two visual treatments. | 🟡 Navigation integrity |
| **F3** | `ModelAdmin` trait has **no label/name method**. All display strings come from the `RustioAdmin` derive macro at compile time. No project-side override. `Case` → `"Cases"` is fine; `CaseAction` → `"Case actions"` is bearable; `Disclosure` → `"Disclosures"` is barely legible. | 🟡 UX polish |
| **F4** | Generated projects ship a **3-middleware chain** (`logger → security_headers → csrf_protect`). The R0-locked correct chain is **4** — `correlation_id` is silently dropped. Every audit row from a scaffolded project lacks a `correlation_id`, breaking the cross-request pivot that R5+ depends on. | 🔴 Generated-project completeness |
| **F5** | `RUSTIO_SECRET_KEY` is **not in the scaffold's `.env.example`**. R3 (MFA) needs it; R4 (emergency export signing) needs it. A fresh project that enrols its first MFA user 500s on the AES-GCM init. | 🔴 Generated-project completeness |
| **F6** | `startapp` generates **an empty `impl ModelAdmin`**. Operators never discover `list_display`, `search_fields`, `list_filter`, `ordering`, `list_per_page`, `bulk_actions` without reading either the demo Post in `startproject` or one of lursystem's `models/*.rs`. | 🟡 Discoverability |
| **F7** | Search / filter / sort / bulk-actions surfaces are **only reachable inside list pages**. There is no top-bar global search, no sidebar hint that filters exist for a particular model, no documentation in the generated README. Built but invisible. | 🟡 Discoverability |
| **F8** | `Admin::mailer / password_policy / require_mfa / recovery_policy` builder methods are **callable but invisible in scaffold**. A first-time operator has no way to discover they exist short of reading source. | 🟡 Discoverability |
| **F9** | Custom-route patterns (`/admin/cases/:id/work`, `/report/new`) are **first-class in production deployments** (lursystem) but the scaffold leaves zero documentation of how to wire one. Operators must read another project to learn the pattern. | 🟡 Generated-project completeness |
| **F10** | The R4 emergency CLI subcommands (published in `rustio-admin-cli@0.8.0`) are **not in the generated `README.md`**. Operators discover them only via `rustio --help`. | 🟢 UX polish |

**Total findings: 10 — 3 critical (🔴), 6 medium (🟡), 1 polish (🟢).**

The audit log integrity finding (F1) is the immediate blocker. It is also the root cause of the user-reported 404 in screenshot 1 (`/admin/rustio_users/2/edit → "no admin model: rustio_users"`).

---

## 1. Inventory — every existing framework surface

### 1.1 Static framework routes (anonymous-reachable)

| Route | Method | Purpose | Visibility |
|---|---|---|---|
| `/static/admin.css`, `admin.js`, `fonts/*` | GET | bundled assets | invisible (asset load) |
| `/admin/login` | GET/POST | login | linked from top-right "Log in" |
| `/admin/forgot-password` (+ `/sent`) | GET/POST | R1 self-recovery | linked from login |
| `/admin/reset-password/:token` | GET/POST | R1 token consume | URL-only |

### 1.2 Authenticated framework routes

| Route | Method | Purpose | Surface | Visibility |
|---|---|---|---|---|
| `/admin` | GET | dashboard | landing | top-right brand link |
| `/admin/history` | GET | global admin actions audit | sidebar Auth → History | sidebar (hardcoded) |
| `/admin/account/sessions` (+ 3 revoke routes) | GET/POST | session management | top-right "System" menu? | **unclear — not in scope of the surveyed templates** |
| `/admin/password_change` | GET/POST | self password change | top-right "Change password" link (screenshot 2) | top-bar link |
| `/admin/reauth` | GET/POST | re-auth wall (R2) | redirect target | not user-clickable |
| `/admin/must-change-password` | GET/POST | forced rotation interstitial (R2) | redirect target | not user-clickable |
| `/admin/mfa/verify` | GET/POST | login-time TOTP gate (R3) | redirect target | not user-clickable |
| `/admin/account/mfa/{enroll,regenerate-codes,disable}` | GET/POST | MFA self-service (R3) | **not surfaced in any navigation** | URL-only |
| `/admin/logout` | POST | logout | top-right "Log out" | top-bar |

### 1.3 Built-in admin model routes (User + Group, slug-keyed)

| Slug | Routes | Source | Visibility |
|---|---|---|---|
| `users` (User admin) | `list / new / :id / :id/edit / :id/delete / :id/reset-password / :id/lock / :id/unlock / :id/revoke-sessions` (R2 added the last four) | `src/admin/builtin.rs` (`core=true`) | **hardcoded in `_sidebar.html`** as `<a href="/admin/users">Users</a>` |
| `groups` (Group admin) | `list / new / :id/edit / :id/delete` | `src/admin/builtin.rs` (`core=true`) | **hardcoded in `_sidebar.html`** as `<a href="/admin/groups">Groups</a>` |

### 1.4 Per-model dynamic CRUD routes (project-registered)

Pattern: `/admin/:admin_name/...` where `admin_name` is `M::ADMIN_NAME` (derived by the `RustioAdmin` macro as `plural_snake(struct_name)`).

| Sub-path | Method | Handler | Permission required | Status |
|---|---|---|---|---|
| `:admin_name` | GET | `list_model` | `<admin>.view_<singular>` | ✅ wired, search/filter/sort honoured |
| `:admin_name/new` | GET/POST | `show_new_form / do_create` | `<admin>.add_<singular>` | ✅ |
| `:admin_name/:id/edit` | GET/POST | `show_edit_form / do_update` | `<admin>.change_<singular>` | ✅ |
| `:admin_name/:id/delete` | GET/POST | `show_delete_confirm / do_delete` | `<admin>.delete_<singular>` | ✅ |
| `:admin_name/:id/history` | GET | `show_object_history` | `<admin>.view_<singular>` | ✅ |
| `:admin_name/bulk_delete` | POST | `handle_bulk_delete` | `<admin>.delete_<singular>` | ✅ |
| `:admin_name/bulk/:action` | POST | `handle_bulk_action` | `<admin>.change_<singular>` | ⚠️ requires `ModelAdmin::bulk_actions()` override; no UI hint that bulk actions are possible |

The dynamic router emits **`Error::NotFound(format!("no admin model: {admin_name}"))`** when `admin.find(admin_name)` returns `None` (routes.rs:290-297, handlers.rs:50-58). This is the **F1 root error**.

### 1.5 R4 emergency CLI surface (no HTTP routes — shell-only)

| Subcommand | Function | Status |
|---|---|---|
| `rustio user reset-password / unlock / disable-mfa / promote / emergency-access` | CLI-only, shipped in `rustio-admin-cli@0.8.0` | ✅ working; not mentioned in generated README |

### 1.6 CLI subcommands

| Command | Mentioned in scaffold README? | Mentioned in scaffold help output? |
|---|---|---|
| `startproject` / `startapp` | ✅ | ✅ |
| `migrate apply / status` | ✅ | ✅ |
| `user create / list / role / delete` | ✅ (create + role only) | ✅ |
| `user reset-password / unlock / disable-mfa / promote / emergency-access` | ❌ | ✅ |
| `group create / list / add-user` | ✅ (create only) | ✅ |
| `perm grant-user / grant-group / list` | ✅ (grant-group only) | ✅ |
| `doctor` | ✅ | ✅ |

### 1.7 Embedded templates (after the 0.7.1 patch)

35 templates resolved via `Templates::new(None)`. Confirmed by the `every_handler_rendered_template_resolves` test landed in `b2513e7`. **No orphan templates** detected on disk; **no embedded-but-unrendered templates** detected.

---

## 2. Mapping — feature → route → registration → template → visibility

| Feature | Route | Registration | Template | Visibility |
|---|---|---|---|---|
| Dashboard | `GET /admin` | `register_admin_routes` | `admin/index.html` | ✅ top-bar brand, default landing |
| **History (global audit)** | `GET /admin/history` | `register_admin_routes` | `admin/log_entries.html` | ✅ **but its rendered links 404** (F1) |
| Object history | `GET /admin/:model/:id/history` | dynamic | `admin/object_history.html` | ✅ |
| Built-in User CRUD | `/admin/users/...` (slug `users`) | `builtin.rs` (`core=true`) | `users_*.html` | ✅ sidebar Auth section (**hardcoded**) |
| Built-in Group CRUD | `/admin/groups/...` (slug `groups`) | `builtin.rs` (`core=true`) | `groups_*.html` + `group_*.html` | ✅ sidebar Auth section (**hardcoded**) |
| Project model CRUD | `/admin/:slug/...` (slug from macro) | `Admin::model::<M>()` | `admin/list.html`, `form.html`, `confirm_delete.html` | ✅ sidebar Models section (dynamic) |
| Search | `?q=...` on list page | `ModelAdmin::search_fields()` | `admin/list.html` (conditional `{% if has_search %}`) | ⚠️ **hidden** — no surface above the page itself |
| Filter | `?filter_<col>=<val>` on list page | `ModelAdmin::list_filter()` | `admin/list.html` (conditional render) | ⚠️ **hidden** |
| Sort | `?sort=<col>` on list page | `ModelAdmin::ordering()` + URL | column header click | ✅ |
| Bulk delete | `POST /admin/:slug/bulk_delete` | always wired for any admin | confirm template | ⚠️ checkbox + button on list page only |
| Bulk actions | `POST /admin/:slug/bulk/:action` | `ModelAdmin::bulk_actions()` | confirm template | ⚠️ project-defined; framework provides no example |
| R1 self-recovery | `/admin/forgot-password*` | `register_admin_routes` | `admin/forgot_password*.html`, `reset_password.html` | ✅ linked from login |
| R2 re-auth wall | `/admin/reauth` | `register_admin_routes` | `admin/reauth.html` (embedded since 0.7.1) | ✅ redirect target |
| R2 admin reset | `/admin/users/:id/reset-password` | builtin | `admin/admin_reset_password.html` (0.7.1) | ✅ User edit page action |
| R2 lock / unlock | `/admin/users/:id/lock`, `/unlock` | builtin | `admin/lock_user.html` (0.7.1) | ✅ User edit page action |
| R2 forced password change | `/admin/must-change-password` | `register_admin_routes` | `admin/must_change_password.html` (0.7.1) | ✅ redirect target |
| R3 MFA enrol | `/admin/account/mfa/enroll` | `register_admin_routes` | `admin/mfa_enroll*.html` (0.7.1) | ⚠️ **no link in chrome** — must know URL |
| R3 MFA verify | `/admin/mfa/verify` | `register_admin_routes` | `admin/mfa_verify.html` (0.7.1) | ✅ redirect target |
| R3 MFA regenerate codes | `/admin/account/mfa/regenerate-codes` | `register_admin_routes` | `admin/mfa_regenerate*.html` (0.7.1) | ⚠️ **no link in chrome** |
| R3 MFA disable | `/admin/account/mfa/disable` | `register_admin_routes` | `admin/mfa_disable.html` (0.7.1) | ⚠️ **no link in chrome** |
| R4 emergency recovery | none (CLI-only) | n/a | n/a | ⚠️ not in generated README |
| Session management | `/admin/account/sessions` | `register_admin_routes` | `admin/account_sessions.html` | ⚠️ surface possibly via "System" top-right; unclear |

---

## 3. Detected issues

### 3.1 🔴 F1 — Audit log `model_name` is not the admin slug

**The most damaging finding of the audit.**

The `rustio_admin_actions.model_name` column is supposed to be (in practice) the same value as the admin URL slug, so the History page can render `<a href="/admin/{model_name}/{object_id}">` and the dispatcher's `admin.find(model_name)` resolves it. In the current codebase **at least four different conventions are written to this column**:

| Writer location | Value | Where slug should match | Result |
|---|---|---|---|
| `builtin.rs:365` (User edit) | `"User"` (struct name) | `"users"` | `/admin/User/2/edit` → 404 |
| `builtin.rs:984` (User delete) | `"User"` | `"users"` | 404 |
| `builtin.rs:1316` (Group edit) | `"Group"` | `"groups"` | 404 |
| `handlers.rs:265` (auto-throttle lock) | `"user"` (lowercase struct) | `"users"` | `/admin/user/2/edit` → 404 |
| `admin_recovery_handlers.rs:566` | `"user"` | `"users"` | 404 |
| **R4 CLI handlers (lursystem session)** | `"rustio_users"` (SQL table name) | `"users"` | **screenshot 1 evidence** — 404 |
| Project-side audit emissions (e.g. lursystem case_actions / disclosures) | `"rustio_users"` etc. | n/a | History page either 404s or renders the row without a working link |

**Live evidence from lursystem's DB right now:**

```text
  model_name  | count
--------------+-------
 rustio_users |     9
 user         |     1
```

Both 404 against the real admin slug `users`. **Every History-page user-row link in lursystem is currently broken.** Some of those `rustio_users` rows are mine (R4 CLI), some predate this session.

**The root cause:** there is no single canonical mapping. Each writer chooses, by hand, what to put in `model_name`. The History renderer assumes the value is a URL slug.

**Why this is integration, not architecture**: the framework already has a single source of truth for the admin slug (`M::ADMIN_NAME`). The fix is to **enforce that `model_name` in `rustio_admin_actions` always equals the admin slug** — same value the dispatcher's `admin.find()` looks up.

### 3.2 🟡 F2 — Two parallel sidebar surfaces

`_sidebar.html` contains:

```html
{% if entries %}                          ← dynamic Models loop (project models)
  <h3>Models</h3>
  {% for entry in entries %}
    <a href="/admin/{{ entry.admin_name }}">{{ entry.display_name }}</a>
  {% endfor %}
{% endif %}

{% if identity and identity.is_admin %}   ← hardcoded Auth block
  <h3>Auth</h3>
  <a href="/admin/users">Users</a>
  <a href="/admin/groups">Groups</a>
  <a href="/admin/history">History</a>
{% endif %}
```

Implications:
- **Users / Groups** are admin models (registered as `core=true` entries in `admin.entries`), but the sidebar renders them via a parallel hardcoded path. The `core=true` filter in `dashboard_ctx` (render.rs:319-324) explicitly excludes them from the dynamic loop.
- This is the wiring decision that lets the Auth section use different labels and stable URLs, but it creates two ways to display the same fact ("Users is an admin model"). The labels can drift; the order is fixed; project-side customisation has two surfaces to chase.
- Adding a third built-in admin (say, R3's MFA backup-codes admin if it existed) would require another hardcoded template block. Not extensible.

### 3.3 🟡 F3 — `ModelAdmin` has no display-name override

The `RustioAdmin` derive macro generates `M::DISPLAY_NAME` at compile time as `humanise(plural_snake(struct_name))`. For `lursystem::Report` that's fine (`"Reports"`). For `lursystem::CaseAction` it produces `"Case actions"`. For `lursystem::Disclosure` it produces `"Disclosures"`.

**None of these can be overridden from the project.** There is no `ModelAdmin::display_name()` method (verified — agent survey 2 §3). A project that wants "Case Actions" or "Identity Disclosures" must either edit the macro or rename the struct.

This is hidden behind the otherwise-correct macro output, but it limits operator polish.

### 3.4 🔴 F4 — Scaffold's middleware chain is incomplete

Generated `src/main.rs.tmpl` (in `rustio-admin-cli/templates/`):

```rust
let router = Router::new()
    .middleware(middleware::logger)
    .middleware(middleware::security_headers)
    .middleware(middleware::csrf_protect);
```

The R0-canonical chain is (per `DESIGN_AUDIT.md` §11):

```rust
let router = Router::new()
    .middleware(middleware::logger)
    .middleware(middleware::correlation_id)   ← MISSING in scaffold
    .middleware(middleware::security_headers)
    .middleware(middleware::csrf_protect);
```

Lursystem wires the correct chain because the maintainer read DESIGN_AUDIT. A scaffolded project's audit rows lack `correlation_id`, and the History page's correlation pivot (which lursystem's Phase 5 audit relies on) is structurally broken.

### 3.5 🔴 F5 — Scaffold's `.env.example` omits `RUSTIO_SECRET_KEY`

Generated `.env.example.tmpl`:

```text
DATABASE_URL=postgres://...
RUSTIO_TEMPLATE_DIR=templates
RUST_LOG=info
# (no RUSTIO_SECRET_KEY)
```

The framework's `auth::mfa` module init (R3) refuses to operate without it; the `auth::emergency::emergency_access` function (R4) refuses to issue tokens without it; lursystem's compliance export (project-side, but a likely pattern) requires it for HMAC signing.

A first-time operator enrolling MFA in a scaffolded project gets a runtime 500 from the AES-GCM init. Discovering the cause requires reading `DESIGN_R3_MFA.md` §15.

### 3.6 🟡 F6 — `startapp` template is too minimal

The generated model file:

```rust
impl ModelAdmin for Foo {}
```

Empty. No defaults shown. An operator who wants search has to find an example elsewhere. The `startproject` Post template DOES show `list_display`, `search_fields`, `list_filter`, `ordering` — but a project that uses `startapp` to add models 2..N never sees these.

### 3.7 🟡 F7 — Search/filter/sort surfaces are reachable but undiscoverable

The framework HAS:
- column-header sort (clickable arrows on the list table)
- `?q=...` text search box (rendered if `search_fields` is non-empty)
- `?filter_<col>=<val>` filter chips (rendered if `list_filter` is non-empty)
- `?page=N` pagination
- bulk-action checkboxes + dropdown (rendered if `bulk_actions()` returns non-empty)

The framework provides **no signal that any of these exist** before the operator clicks into a list page. There is no:
- Global search box in the top bar
- Per-model "this list supports search / filter" hint in the sidebar
- Documentation in the generated README listing these capabilities

The "hidden search" complaint from the brief is structural. The feature works once you're inside the list; it just isn't discoverable from outside.

### 3.8 🟡 F8 — `Admin::*` builder methods are CLI-mentioned but scaffold-omitted

Methods callable on `Admin`:

| Method | Mentioned in scaffold | Documented anywhere reachable |
|---|---|---|
| `model::<T>()` | ✅ in `Admin::new().model::<Post>()` | ✅ macro-derived |
| `mailer(...)` | ❌ | DESIGN_RECOVERY.md §2 |
| `password_policy(...)` | ❌ | DESIGN_RECOVERY.md §11 |
| `require_mfa(MfaPolicy)` | ❌ | DESIGN_R3_MFA.md §6 |
| `recovery_policy(...)` | ❌ | DESIGN_RECOVERY.md §6 |
| `seed_permissions(&db)` | ✅ in scaffold `main.rs.tmpl` | ✅ |

Four of six advanced builders are invisible unless the operator reads DESIGN docs.

### 3.9 🟡 F9 — Custom-route mounting has no scaffold pattern

Lursystem mounts seven custom HTTP routes (Phase 2, 3, 4, 5, 6 surfaces). The scaffold has one — `GET /`. Adding a custom route requires understanding `Router::get(...)` + handler closure shape + `Db::clone()` + `req.param(...)` + auth via `auth_helper::require_role(...)`.

None of this is in the scaffold. The only reference implementation is lursystem itself, which a new operator may not have access to.

### 3.10 🟢 F10 — Emergency CLI not in generated README

`rustio user reset-password / unlock / disable-mfa / promote / emergency-access` shipped in `rustio-admin-cli@0.8.0`. The generated README lists `user create / list / role / delete` only. An operator who needs emergency recovery discovers it via `rustio --help`.

---

## 4. Concrete bug evidence

### 4.1 The user's screenshot 1 explained

`/admin/rustio_users/2/edit → 404 "no admin model: rustio_users"`

**Trace**:
1. Operator clicked an audit-log link rendered by the History page.
2. The audit row in question had `model_name = "rustio_users"` (written by my R4 CLI handlers — `crates/rustio-admin-cli/src/user.rs:373`).
3. History page renders `<a href="/admin/rustio_users/2/edit">`.
4. Router dispatches; `admin.find("rustio_users")` returns `None` (the real admin slug is `users`).
5. Handler emits `Error::NotFound("no admin model: rustio_users")`.
6. The 404 template renders.
7. **Crucially**: the 404 template (`admin/error.html`?) does NOT carry the sidebar. The operator sees a chromeless dead-end with no way to navigate except back-to-dashboard. (F2 + F1 compound here.)

### 4.2 The user's screenshot 2 explained

`/admin/history` page shows a column "MODEL" with values:
- `user #2` (from `handlers.rs:265` auto-throttle path)
- `rustio_users #2` ×8 (from R4 CLI handlers)

Both render as clickable blue links. Clicking either lands on the screenshot-1 404. Every operator hover on these links shows a URL that won't resolve.

The "Action" column shows opaque green pills labeled "Action". This is the action-type-pill style — but the actual event names (`emergency_recovery`, `forced_password_change_completed`, etc.) are hidden behind the visual styling. The pills don't include the typed event name in their text — they may be using CSS to truncate or the rendering may be feeding only `ActionType::Update` (the placeholder slot) instead of `AuditEvent::EmergencyRecovery.as_str()`.

This is a **separate visibility regression**: the History page is throwing away `AuditEvent::*` event-type information that's already in the audit row, in favour of a generic chip.

### 4.3 Sidebar "MODELS" section in screenshot 2

The screenshot resolution is low. The agent could not confirm whether the model labels are genuinely blank or just hard to read at small zoom. The framework's `DISPLAY_NAME` const is correctly generated and reaches the template. If the labels render blank in the user's browser, the next investigation step is browser DevTools — but the codebase audit found no framework gap.

---

## 5. Priority-ordered recovery roadmap

Following the brief's priority order: **admin integrity > navigation integrity > template embedding > generated-project completeness > discoverability > UX polish**.

### Phase A — Admin integrity (🔴 must fix first)

**A1. Canonicalise `model_name` to always be the admin slug.**
- Add a `pub fn audit_model_name<M: AdminModel>() -> &'static str { M::ADMIN_NAME }` helper.
- Audit every `LogEntry::new(.., .., model_name, ..)` site in the framework crate. Switch each to use the slug.
- Verify the CLI's R4 emergency emissions (`crates/rustio-admin-cli/src/user.rs:373`). Change `"rustio_users"` to `"users"`.
- Add a unit test that scans for `LogEntry::new(...)` literals with `model_name` arguments matching the framework's known mis-slugs (`"User"`, `"user"`, `"rustio_users"`, `"Group"`) and fails if any are found. Same shape as `emergency_recovery_is_cli_only`.
- Backfill: write a migration that updates pre-existing audit rows. Idempotent SQL: `UPDATE rustio_admin_actions SET model_name = 'users' WHERE model_name IN ('User','user','rustio_users');` and similar for Group.

**A2. Make the 404 page carry the sidebar.**
- The chromeless 404 page (screenshot 1) is a UX dead-end on its own and turns F1 into a navigational trap. The 404 template should extend `_base.html` (which renders the sidebar) instead of being a bare card.

### Phase B — Navigation integrity (🟡)

**B1. Surface MFA self-service on the top bar.**
- `/admin/account/mfa/enroll`, `/admin/account/mfa/regenerate-codes`, `/admin/account/mfa/disable` are reachable only by typing the URL. Add an MFA item to the "System" / account menu top-right.
- Pattern: alongside the existing "Change password" + "Log out" group.

**B2. Consider unifying the Auth-block sidebar rendering.**
- Either: keep the hardcoded block but template it as a list of `(label, href)` tuples so non-User/Group built-ins can extend cleanly.
- Or: surface `core=true` entries in the dynamic loop but in a separate "Auth" group keyed by `entry.category`. Add `ModelAdmin::sidebar_category() -> &'static str` (defaults to `"Models"`; framework overrides for User/Group to `"Auth"`).

**B3. Fix the History page's action-type column.**
- Render `action_type` (the full event name like `"emergency_recovery"`) in human form, not the placeholder pill. Read it via the `as_str()` mapping in `admin/audit.rs`.

### Phase C — Template embedding (now mostly clean post-0.7.1)

No further action needed — the `every_handler_rendered_template_resolves` test catches future drift.

### Phase D — Generated-project completeness (🔴)

**D1. Add `correlation_id` to scaffold's middleware chain.**
- Edit `crates/rustio-admin-cli/templates/project/src/main.rs.tmpl`.
- Add a one-line comment about the locked ordering pointing at DESIGN_AUDIT.md §11.

**D2. Add `RUSTIO_SECRET_KEY` to scaffold's `.env.example`.**
- Generation instructions (matching what lursystem's `.env.example` has): `openssl rand 32 | base64 | tr '+/' '-_' | tr -d '='`.

**D3. Add MFA + recovery callouts to the generated README.**
- "If you want MFA: `.require_mfa(MfaPolicy::Required)` on `Admin::new()`. Users enrol at /admin/account/mfa/enroll."
- "If you want shell-tier recovery when everything else fails: `rustio user reset-password / unlock / disable-mfa / promote / emergency-access`. See `rustio user --help`."

**D4. Add a "Custom routes" section to the README.**
- Two-paragraph mini-doc showing the lursystem pattern for a custom GET. Concrete example: a `/welcome` page that renders project HTML.

### Phase E — Discoverability (🟡)

**E1. `startapp` template should show a meaningful `ModelAdmin`.**
- Include `list_display`, `search_fields`, `list_filter`, `ordering` with TODO-style comments showing the syntax.
- The default `Foo` model can have `list_display = &["id", "name", "created_at"]` matching the migration's columns.

**E2. Per-model sidebar hover/tooltip.**
- Optional: when a model is registered with non-empty `search_fields` or `list_filter`, the sidebar entry shows a small icon hinting "this list is searchable" / "this list is filterable". Low-priority — but the framework's strengths become visible at zero operator effort.

**E3. `rustio doctor` should report `MfaPolicy`, `RecoveryPolicy`, mailer status.**
- These exist; surfacing them in doctor's output costs a few lines and answers "what is this project actually configured to do".

### Phase F — UX polish (🟢)

**F1. Add `#[rustio_admin(display_name = "...")]` attribute support to the derive macro.**
- Lets `Disclosure` register as `"Identity disclosures"` without renaming the struct.

**F2. Promote `model::<T>().with_label(...)` builder.**
- Same goal via the builder API; can ship alongside or instead of F1.

---

## 6. What this audit deliberately does NOT recommend

- **No rewrites.** Every finding lists a wiring or surfacing fix, never an architecture change.
- **No new admin surfaces.** The 22-doctrine framework is sound; the gap is discoverability, not capability.
- **No deprecations.** The hardcoded Auth-block sidebar stays in place until F2 can be done cleanly; tearing it out before the unified renderer is in place would be a regression.
- **No new tables or migrations except the model_name backfill (A1).** That migration is corrective, not architectural.

---

## 7. Sequencing summary

| Order | Fix | Why first |
|---|---|---|
| 1 | A1 — model_name canonicalisation + backfill | F1 is the only finding that produces user-visible 404s |
| 2 | A2 — 404 page gets sidebar | the 404 is no longer a dead end |
| 3 | B3 — History action-type column shows real event names | makes the audit log readable |
| 4 | D1 — scaffold middleware chain fix | every new project gets correlation_id |
| 5 | D2 — `RUSTIO_SECRET_KEY` in scaffold env | every new project that enrols MFA stops 500ing |
| 6 | D3 + D4 — README additions | discoverability for new projects |
| 7 | E1 — `startapp` template fleshed out | discoverability for new models |
| 8 | B1 — MFA self-service link in top bar | discoverability for existing operators |
| 9 | B2 — sidebar Auth-block unification | extensibility |
| 10 | F1/F2 — display_name override | polish |

Each step is independent. Each step can ship as a separate commit. Each step reduces hidden complexity (per the brief's rules).

---

*Audit complete. No code changed in this pass. Implementation sequencing pending operator review.*
