# RustIO Admin — APIs Page, Docs Page, Templates Scaffolding Plan

> Saved 2026-05-07. Source: design conversation with maintainer.
> This document is a design, not a coding plan. Approve, modify, or
> reject before any code moves.

## 1. Goals

Three intersecting features that ship together as **v0.2.0**:

1. **Auto-generated REST API** for every registered `ModelAdmin`.
   JSON CRUD endpoints mirror the admin URL structure (`/api/<name>`,
   `/api/<name>/:id`). Opt-in per-project via `Admin::with_api()`.

2. **`/admin/apis` page** — self-documenting reference for the
   auto-generated API. Lists every endpoint, parameter,
   request/response schema. Renders an OpenAPI 3.1 document that's
   also exposed at `/api/openapi.json` for external tools (Swagger
   UI, Postman, Stoplight).

3. **`/admin/docs` page** — markdown-rendered project documentation.
   Reads `*.md` from a project-local `docs/` directory, renders
   navigable pages with sidebar nav. Lets a project ship its own
   help / runbook / settings pages alongside the admin UI.

Plus a fourth piece tying it together:

4. **Templates + docs scaffolding in `rustio startproject`** — a new
   project skeleton ships with `templates/admin/_base.html` (and
   optionally page-level overrides) plus `docs/` (starter markdown)
   so users can customise from day one without copying files from
   upstream.

## 2. Non-goals

- ✗ GraphQL (REST only).
- ✗ Real-time / SSE / WebSocket endpoints.
- ✗ Bulk endpoints (`PATCH /api/posts` for batch).
- ✗ File upload / multipart endpoints.
- ✗ Custom URL routing per Model — URL is always `/api/<admin_name>`.
- ✗ Per-field permissions. The unit of permission stays the model.
- ✗ Auto-generated client SDKs.
- ✗ Webhook delivery on data changes.
- ✗ Markdown extensions beyond CommonMark + GFM tables / strikethrough
  / task lists.
- ✗ Live "Try it" UI in the apis page (read-only docs in v0.2;
  reconsider for v0.3).
- ✗ Syntax highlighting in code blocks (Geist Mono mono-color in v0.2).

## 3. Phases

Each phase is independently mergeable; B depends on A; C depends on B;
D depends on C; **E and F are independent** of A–D and can land in
parallel.

### Phase A — Read-only API (GET only)

**Goal**: every project that opts in via `Admin::with_api()` gets
`GET /api/<name>` (list, with filter/sort/pagination via the same
query params the admin list page accepts) and `GET /api/<name>/:id`
(retrieve).

**Files** (under `crates/rustio-admin/src/admin/`):
- `api/mod.rs` (new) — module entry + re-exports.
- `api/handlers.rs` (new, ~250 LOC) — `list`, `retrieve` handlers
  wrapping `ConcreteOps::list` + `Db::find_by_id`.
- `api/serialize.rs` (new, ~150 LOC) — Model → JSON serialization.
- `api/routes.rs` (new, ~80 LOC) — `register_api_routes` mounting
  `/api/<admin_name>` + `/api/<admin_name>/:id`.
- `types.rs` — `Admin::with_api(self) -> Self` flag + getter.
- `routes.rs` — call `register_api_routes` when the flag is set.

**Auth/perm**:
- Reuse session cookie auth (browser-friendly).
- Permission gate is `view_<model>` for both list and retrieve.
- API tokens deferred to v0.3 (see §6.B).

**Wire format**:
- Response (list): `{ "data": [...], "total": N, "page": N, "per_page": N }`
- Response (retrieve): `{ "data": {...} }`
- Errors: `{ "error": { "code": "not_found", "message": "..." } }`
  with appropriate HTTP status (404, 401, 403).

**Acceptance**:
- `cargo run` against the `examples/minimal` skeleton; `curl -b
  cookies http://127.0.0.1:8000/api/posts` returns valid JSON.
- `?published=true&sort=-created_at&page=2&per_page=10` works
  identically to the admin list page.
- Anonymous request → 401. Staff user without `view_post` → 403.

**LOC budget**: ~500 framework + ~150 tests.

### Phase B — Mutating API (POST / PATCH / DELETE)

**Goal**: full CRUD, gated by per-model permissions
(`add_<model>`, `change_<model>`, `delete_<model>`).

**Files**:
- `api/handlers.rs` — add `create`, `update`, `destroy`.
- `api/serialize.rs` — request body deserialization +
  field-level validation (mirrors the admin form's `validate` step).
- `api/routes.rs` — wire POST/PATCH/DELETE routes.

**Wire format**:
- POST/PATCH body: `{ "data": { "title": "...", ... } }` (envelope
  mirrors the response).
- POST → `201 Created` + body `{ "data": {...inserted-row...} }`.
- PATCH → `200 OK` + the updated row.
- DELETE → `204 No Content`.
- 422 on validation failure with
  `{ "error": { "code": "validation", "fields": { "title": ["required"] } } }`.

**CSRF**:
- Browser cookie POSTs require the `X-CSRF-Token` header set to the
  session's CSRF token (already issued).
- Documented in the `/admin/apis` page; bypassed only when an API
  token is used (Phase D / v0.3 work).

**Audit log**:
- Every mutating call writes to `rustio_admin_actions` with
  `user_email`, `model_name`, `object_id`, `label` ("Created" /
  "Updated" / "Deleted") so it surfaces on `/admin/history` and the
  dashboard's recent activity.

**Acceptance**:
- `curl -X POST -H 'Cookie: ...' -H 'X-CSRF-Token: ...' -d
  '{"data":{"title":"Hi"}}' /api/posts` → 201, row appears in admin's
  recent activity timeline.
- Validation failures → 422 with field errors matching what the admin
  form would report.

**LOC budget**: ~400 framework + ~200 tests.

### Phase C — OpenAPI 3.1 schema + `/api/openapi.json`

**Goal**: machine-readable API description at `/api/openapi.json`.
Built once at registration time from the registered `AdminEntry`s.

**Files**:
- `api/openapi.rs` (new, ~300 LOC) — `OpenApiBuilder` consumes
  `&[AdminEntry]` and produces a `serde_json::Value`. Schema for each
  model derived from `Model::COLUMNS` + an extension trait
  `JsonSchema` (default impl returns `{ "type": "object" }`).
- `api/routes.rs` — register `/api/openapi.json`.

**OpenAPI doc contents**:
- `info.title` + `info.version` from `Admin::site_branding`.
- `paths` for every model: list / retrieve / create / update /
  destroy (5 endpoints × N models).
- `components.schemas` per model: column → JSON Schema type
  (text/email/integer/timestamp/boolean), `required` array, primary
  key marker.
- `components.securitySchemes`: `cookieAuth` (session cookie) +
  `csrfHeader` (X-CSRF-Token). `bearerAuth` added in v0.3.

**Acceptance**:
- `curl /api/openapi.json` validates against the OpenAPI 3.1 schema
  (`swagger-cli validate` in CI).
- Pasting the JSON into Swagger UI / Stoplight renders correctly.

**LOC budget**: ~300 framework + ~100 tests.

### Phase D — `/admin/apis` page

**Goal**: a polished documentation page in the admin chrome that
renders the OpenAPI doc as structured per-model sections. Like
ReDoc, but hand-written and self-hosted (no client-side JS
framework, no third-party dependency).

**Files**:
- `assets/templates/admin/apis.html` (new, ~150 LOC).
- `assets/templates/admin/apis/_endpoint.html` (new, ~80 LOC) —
  partial for one endpoint card.
- `assets/templates/admin/apis/_schema.html` (new, ~60 LOC) —
  partial for a schema box.
- `admin/handlers.rs` — `apis` handler that fetches the OpenAPI doc
  + builds the template context.
- `admin/routes.rs` — `GET /admin/apis`.
- `_sidebar.html` — add an "APIs" link below the Models block (gated
  on `admin.has_api()`).

**Page structure**:
- Header: API base URL, auth requirements (cookie + CSRF; bearer in
  v0.3).
- Per-model section: 5 endpoints collapsed by default, expandable.
- Per-endpoint card: HTTP method badge, path, params table, request
  schema, response schema, runnable `curl` example.
- Right-side jump-to-model navigation.

**Permission gate**: visible to anyone with admin access (Staff or
above).

**LOC budget**: ~250 framework + ~290 templates + ~100 tests.

### Phase E — `/admin/docs` page (markdown viewer)

**Goal**: render `*.md` files from `<project>/docs/` as navigable
pages within the admin chrome. Independent of A–D — can ship as a
v0.1.2 release ahead of the API work if you want a smaller
incremental drop.

**Files**:
- `Cargo.toml` (rustio-admin) — add `pulldown-cmark = "0.10"`
  (~150 KB compressed transitive footprint).
- `admin/docs.rs` (new, ~200 LOC) — disk discovery, frontmatter
  parsing, markdown → HTML.
- `assets/templates/admin/docs.html` + `docs/_page.html` (~150 LOC).
- `_sidebar.html` — "Docs" link (gated on `admin.has_docs()`).
- `admin/handlers.rs` — `docs_index`, `docs_page` handlers.
- `admin/routes.rs` — `GET /admin/docs`, `GET /admin/docs/<page>`.

**Discovery rules**:
- Project sets `RUSTIO_DOCS_DIR` (default `./docs/`).
- Each `*.md` becomes a page; sub-folders become categories.
- File frontmatter (`---\ntitle: ...\norder: 1\n---`) controls
  display order + page title; missing frontmatter falls back to the
  filename.

**Markdown features**:
- CommonMark + GFM tables + strikethrough + task lists.
- Code blocks: `<pre>` + Geist Mono (no syntax highlighting in v0.2).
- Raw HTML stripped (sanitise for safety).

**Acceptance**:
- `docs/getting-started.md` shows up at `/admin/docs/getting-started`.
- Sidebar lists every doc grouped by sub-folder.
- `Admin::with_docs()` toggles whether the link appears at all.

**LOC budget**: ~350 framework + ~150 templates + ~100 tests.

### Phase F — Templates + docs scaffolding in `rustio startproject`

**Goal**: `rustio startproject myapp` produces a project that
already has `templates/admin/_base.html` (and optionally page
overrides) plus `docs/` (starter markdown) so users can customise
from day one without copying files from upstream.

**Files** (all under `crates/rustio-admin-cli/templates/project/`):
- New: `templates/admin/_base.html` (verbatim copy of the
  framework's bundled default).
- New: `templates/admin/{login,index,list,form}.html` (gated on
  `--with-overrides` flag).
- New: `docs/index.md` (starter), `docs/getting-started.md`.
- Update: `.env.example` adds `RUSTIO_TEMPLATE_DIR=templates`.
- Update: `README.md.tmpl` — section explaining the templates and
  docs layout.

**CLI changes** (in `crates/rustio-admin-cli/src/scaffold.rs`):
- `rustio startproject <name>` — default; ships `_base.html` + `docs/`
  only.
- `rustio startproject <name> --with-overrides` — also ships the
  four page-level overrides.
- New: `rustio templates publish` — for existing projects that want
  to opt in after the fact. Pulls the framework's current bundled
  templates into `<project>/templates/admin/`. Prompts before
  overwriting; `--force` skips the prompt.

**Anti-drift CI guard**: a test in `rustio-admin-cli` reads each
file from the framework's bundled `assets/templates/admin/*.html`
and the matching CLI scaffold copy and asserts byte equality. If
they drift, CI fails until they're synchronised. Keeps the two
sources of truth from diverging silently.

**Acceptance**:
- Fresh `rustio startproject foo` + `cd foo && cargo run` boots an
  admin that uses Geist + crimson, with the user's own
  `_base.html` controlling the layout.
- Editing `templates/admin/_base.html` and refreshing the browser
  shows the change without rebuilding.
- `rustio templates publish` in an existing project writes 5 files;
  re-running prompts before overwriting.

**LOC budget**: ~150 CLI + ~600 templates copied verbatim from the
framework.

## 4. Cross-cutting design notes

### A. JSON shape: `display_values()` vs. typed JSON

`Model::display_values()` already exists for HTML rendering — every
column gets a string representation. Reusing it for JSON would unify
serialization, but produces wrong types for booleans, integers,
timestamps (consumers expect typed JSON, not stringified values).

**Recommendation**: emit a separate `JsonSerializable` impl from the
existing `RustioAdmin` derive macro. The macro inspects struct fields
and emits:

| Rust type | JSON value |
|---|---|
| `i64`, `i32`, `i16`, `f32`, `f64` | `Value::Number` |
| `String`, `&str` | `Value::String` |
| `bool` | `Value::Bool` |
| `chrono::DateTime<Utc>` | `Value::String` (RFC 3339) |
| `Option<T>` | `Value::Null` if `None`, else recurse |
| `Vec<T>` | `Value::Array` |

No new project-side trait to implement — projects keep writing the
same `impl Model for X` they already write.

### B. API tokens vs. cookie auth

Phase A ships with cookie auth only (browser-friendly). Programmatic
clients (CI, scripts, third-party services) need bearer tokens.

**Recommendation**: defer API tokens to **v0.3**. The `/admin/apis`
page in v0.2 documents only cookie + CSRF auth. When tokens land in
v0.3, the OpenAPI doc gains `bearerAuth` automatically and the apis
page picks up the new section.

If the user wants tokens in v0.2, add a 7th phase (~250 LOC):
`auth/api_tokens.rs`, `rustio_api_tokens` table, `Admin::with_api_tokens()`,
and a `/admin/api_tokens` CRUD page.

### C. URL conflict: `/admin/apis` vs. `/api`

`/api/*` is the auto-generated namespace; `/admin/apis` is the docs
page. Distinct top-level paths, no risk of collision unless a project
manually mounts handlers at either.

`Admin::with_api()` warns at boot if the project's router already has
a `/api/*` route registered (avoids silent shadowing).

### D. Templates scaffolding: source-of-truth strategy

The CLI's `crates/rustio-admin-cli/templates/project/templates/admin/`
holds byte-for-byte copies of the framework's bundled templates from
`crates/rustio-admin/assets/templates/admin/`. Two mitigations against
drift:

1. **CI test** asserts byte equality (described in Phase F).
2. **Sync script** `xtask sync-templates` (or a Makefile target)
   that copies the framework templates into the CLI scaffold dir,
   so a single command keeps them aligned.

### E. Docs page rendering: `pulldown-cmark` vs. `comrak`

| | pulldown-cmark | comrak |
|---|---|---|
| Compressed footprint | ~150 KB | ~600 KB |
| CommonMark | ✓ | ✓ |
| GFM tables / strikethrough / task lists | ✓ | ✓ |
| Footnotes | ✗ | ✓ |
| Math (KaTeX) | ✗ | ✓ |
| Active maintenance | active | active |

**Recommendation**: `pulldown-cmark` for v0.2. If projects need
footnotes / math later, swap to `comrak` in a minor.

## 5. Dependencies + ordering

```text
Phase A (read API)
   ↓
Phase B (mutating API) — depends on A's serialize.rs
   ↓
Phase C (OpenAPI schema) — depends on B (so the schema covers
                            POST/PATCH/DELETE bodies)
   ↓
Phase D (apis page UI) — depends on C
                                     ─── independent ───
                                                          ↓
Phase E (docs page) — independent of A-D, can ship in parallel
                                                          ↓
                                     Phase F (scaffolding) —
                                     uses templates from A-D
                                     and docs/ from E
```

A → D is one coherent thread that ships as v0.2.0.
E can ship earlier as **v0.1.2** if you want a smaller release.
F is non-blocking and ships any time after E.

## 6. Open questions

1. **Opt-in vs. opt-out for the API**: ship with `Admin::with_api()`
   (default-off, explicit opt-in) or always-on for any registered
   model? **Recommendation**: opt-in. Existing projects upgrading
   from v0.1 must not accidentally expose data via JSON.

2. **Docs URL design**: file-tree mirror (`docs/getting-started.md`
   → `/admin/docs/getting-started`) or front-matter slug
   (`slug: getting-started`)? **Recommendation**: file-tree by
   default, frontmatter slug as override.

3. **`rustio templates publish`** behaviour when `templates/admin/`
   already has overrides: prompt + `--force`, or fail safely?
   **Recommendation**: prompt by default, `--force` to skip prompt
   (mirrors `git checkout` ergonomics).

4. **Should the API surface respect `readonly_fields` from
   `ModelAdmin`** (e.g. POST/PATCH ignore `created_at` if it's
   read-only)? **Recommendation**: yes — `readonly_fields` is a
   model-wide constraint, not UI-only.

5. **Should the API expose an "admin lookup" endpoint** (i.e.
   `GET /api/<name>?fields=id,display` for typeahead pickers in
   admin forms)? **Recommendation**: defer. Phase A's list endpoint
   already handles this if the consumer projects the columns
   client-side.

6. **API response envelope**: `{ "data": ..., "total": N }` vs. just
   the array? **Recommendation**: envelope. Lets us add metadata
   (cursors, links, deprecation warnings) without breaking the
   contract.

## 7. v0.2.0 release shape

If all five phases (A–E) ship together:

| Crate | Bump | What changed |
|---|---|---|
| `rustio-admin` | 0.1.x → 0.2.0 | New `Admin::with_api()`, `with_docs()`; new `/api/*`, `/admin/apis`, `/admin/docs` routes. `pulldown-cmark` dep. |
| `rustio-admin-macros` | 0.1.x → 0.2.0 | `RustioAdmin` derive emits a `JsonSerializable` impl. |
| `rustio-admin-cli` | 0.1.x → 0.2.0 | `startproject --with-overrides`, new `templates publish` subcommand, scaffolds `templates/admin/_base.html` + `docs/` by default. |

Public API surface is **purely additive** — projects upgrading from
v0.1 don't need code changes. Minor bump (`0.1` → `0.2`) is justified
because:

- Dependency footprint grows (`pulldown-cmark`).
- Project layout for new projects changes (templates/, docs/).
- The OpenAPI schema becomes a public contract that v0.3+ must respect.

## 8. Estimated total effort

| | LOC |
|---|---:|
| Code (framework + CLI) | ~2300 |
| Templates (apis + docs + scaffolds) | ~1050 |
| Tests | ~700 |

Roughly half the size of v0.1.0's strategic-reset rollout. At the
same pace, **v0.2.0 ≈ 1 week of focused work**, deliverable in
phase-sized commits.

A smaller alternative: ship just **Phase E + F** as **v0.1.2** (the
docs page + scaffolding), defer A–D to v0.2.0. v0.1.2 lands in
~2 days; v0.2.0 follows once the API surface is shaped.
