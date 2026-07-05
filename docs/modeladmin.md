# `ModelAdmin` reference

`ModelAdmin` is the customisation surface for every model registered via `Admin::new().model::<M>()`. Every method on the trait has a default body, so the minimum you need is:

```rust
impl ModelAdmin for Task {}            // accept every default
```

Override only what you care about; the rest inherit framework defaults.

## A worked example

One model, the four hooks most projects reach for, and exactly what each one
produces. Say you scaffolded a `Task`:

```sh
rustio-admin startapp task \
  --field title:str \
  --field source_lang:str \
  --field target_lang:str \
  --field deadline:date \
  --field status:choice:available,in_progress,review,completed,reassigned
```

`startapp` writes the struct and `impl Model` for you. You add the
customisation:

```rust
impl ModelAdmin for Task {
    fn list_display()  -> &'static [&'static str] { &["title", "target_lang", "status", "deadline"] }
    fn list_filter()   -> &'static [&'static str] { &["status", "target_lang"] }
    fn search_fields() -> &'static [&'static str] { &["title", "source_lang"] }
    fn ordering()      -> &'static [&'static str] { &["-deadline"] }
}
```

`/admin/tasks` now renders:

```text
Tasks                                 search: title, source_lang   [ + Add task ]
  Status ▾   Target lang ▾            ← filter chips

  TITLE                       TARGET LANG   STATUS        DEADLINE
  Court hearing transcript    en            available     2026-07-15   ← newest first
  Medical discharge summary   ar            in_progress   2026-07-10
  Rental contract             en            review        2026-07-08
```

Line by line, that came from:

| Hook | What it did here |
|---|---|
| `list_display` | the four columns, in that order (`id` is always a link to the edit form, listed or not) |
| `list_filter` | the **Status ▾** / **Target lang ▾** chips — `status` is a `choice`, so a dropdown of its values; `target_lang` a dropdown of distinct values |
| `search_fields` | the search box scans `title` + `source_lang` (`?q=` → ILIKE) |
| `ordering` | `-deadline` → newest first |

Everything else — read-only fields, validation, child rows (`inlines`), form
sections (`fieldsets`), bulk actions — is opt-in, and each is shown on its own
below. Start with these four; add the rest when a page needs them.

## Why no blanket impl?

An earlier prototype shipped `impl<T: AdminModel> ModelAdmin for T {}` so every derived model would auto-pick-up defaults. That collides with Rust's coherence rules — without `feature(specialization)` (nightly-only), a blanket impl forbids per-type impls, which would block the project overrides this trait exists for. The opt-in `impl ModelAdmin for X {}` is the standard stable-Rust pattern (serde's `Serialize`, axum's `Handler`, std's various marker traits).

## The hooks

| Method | Default | Used by |
|---|---|---|
| `list_display`        | `&[]` (every column on `M::FIELDS`) | List page columns |
| `list_filter`         | `&[]` (none) | Filters dropdown chips |
| `search_fields`       | `&[]` (search box decorative) | `?q=term` ILIKE search |
| `search_index_column` | `None` (ILIKE path) | Opt the search box into Postgres full-text search |
| `ordering`            | `&["-id"]` (newest first) | List page `ORDER BY`; default for the Sort dropdown |
| `list_per_page`       | `50` | Default page size; user override via `?per_page=` |
| `readonly_fields`     | `&[]` | Disabled form inputs (value reloaded from DB on save) |
| `inlines`             | `&[]` (none) | Child rows rendered on the parent edit page |
| `fieldsets`           | `&[]` (heuristic grouping) | Explicit form section ordering |
| `validate`            | `Ok(())` | Project validation before a row is written |
| `bulk_actions`        | `&[]` (Delete only) | Extra buttons in the list-view bulk bar |

Most methods return `&'static` data, so the values are captured into `AdminEntry` once at registration and read straight from the entry on every request — no per-request virtual dispatch beyond the existing `dyn AdminOps`. The exceptions are `validate(&Self)`, which runs per submission against the candidate model, and `execute_bulk_action`, which dispatches a selected action.

---

### `list_display`

```rust
fn list_display() -> &'static [&'static str] { &[] }
```

Columns shown on the list page, in order. **Default** (`&[]`) means *every* field declared on `AdminModel::FIELDS`.

```rust
impl ModelAdmin for Course {
    fn list_display() -> &'static [&'static str] {
        &["code", "title", "credit_hours", "is_published"]
    }
}
```

The `id` column is always rendered as a clickable link to the edit form, regardless of whether it's listed.

### `list_filter`

```rust
fn list_filter() -> &'static [&'static str] { &[] }
```

Columns that surface as filter chips in the sidebar. The framework infers the chip widget from the column type (`bool` → Yes/No, `String` → dropdown of distinct values, etc.).

```rust
fn list_filter() -> &'static [&'static str] {
    &["status", "level", "is_published"]
}
```

Filter selections are SQL-pushed: `WHERE col::text = $1`. The `::text` cast keeps comparisons consistent with the values `display_values()` produces (so `is_active=true` matches both `true` and `'true'`-shaped storage).

### `search_fields`

```rust
fn search_fields() -> &'static [&'static str] { &[] }
```

Columns scanned by the list-page search box (`?q=term`). The framework emits a single ILIKE OR-chain across the listed columns:

```sql
WHERE (col1::text ILIKE '%term%' OR col2::text ILIKE '%term%' OR …)
```

Empty `search_fields` makes the search box decorative — ILIKE-ing every column would fight indexes.

```rust
fn search_fields() -> &'static [&'static str] {
    &["code", "title", "description"]
}
```

### `search_index_column`

```rust
fn search_index_column() -> Option<&'static str> { None }
```

Opt the list-page search box (and the global ⌘K palette and CSV-export filter) into Postgres full-text search instead of the default ILIKE OR-chain. Return the name of a `tsvector` column the project maintains; the framework switches to `<col> @@ websearch_to_tsquery('english', $N)`. Default `None` keeps the ILIKE path, so existing models are unchanged.

```rust
// 1. Add a generated tsvector column in a migration:
//    ALTER TABLE posts ADD COLUMN search_vector tsvector
//      GENERATED ALWAYS AS (to_tsvector('english',
//        coalesce(title,'') || ' ' || coalesce(body,''))) STORED;
//    CREATE INDEX posts_search_idx ON posts USING gin(search_vector);

// 2. Opt in:
fn search_index_column() -> Option<&'static str> { Some("search_vector") }

// 3. List the tsvector column in `Model::COLUMNS` (see below).
```

**Required: the column must be in `Model::COLUMNS`.** For injection-safety the
framework only uses the FTS column if it appears in `COLUMNS`; otherwise search
**silently falls back to a slow `ILIKE` scan** (and, since 0.25, logs a warning at
registration). The `tsvector` need not be a struct field — add it to `COLUMNS`
and `from_row` simply won't read it:

```rust
const COLUMNS: &'static [&'static str] =
    &["id", "title", "body", "created_at", "search_vector"];
```

### `ordering`

```rust
fn ordering() -> &'static [&'static str] { &["-id"] }
```

Default sort applied as `ORDER BY` in the list query. `-foo` for `foo DESC`, `foo` for `foo ASC`. Multi-element slices produce a multi-column sort.

```rust
fn ordering() -> &'static [&'static str] {
    &["-is_pinned", "-published_at"]   // pinned first, then newest
}
```

A user clicking a column header overrides the default via `?sort=col&dir=desc`. Column names in both the static slice and the URL are validated against `M::COLUMNS` — there's no SQL-injection vector even from a hand-crafted URL.

### `list_per_page`

```rust
fn list_per_page() -> usize { 50 }
```

Default rows-per-page on the list view. The user can override at runtime via `?per_page=N` (or via the per-page picker dropdown in the toolbar), but the param is allow-listed to `{25, 50, 100, 200}` so a malicious query can't OOM the worker. Override values outside the allow-list are silently ignored — the framework falls back to the model's default.

### `readonly_fields`

```rust
fn readonly_fields() -> &'static [&'static str] { &[] }
```

Columns the change form renders as **disabled**. Honoured on every form: on update the framework reloads the persisted value from the database rather than trusting the submitted body, so a read-only field can't be smuggled past via a hand-crafted POST. The macro's `editable: false` flag still owns the strict per-field gate (e.g. `id` and `created_at` stay non-editable regardless).

```rust
fn readonly_fields() -> &'static [&'static str] { &["created_at", "external_ref"] }
```

### `inlines`

```rust
fn inlines() -> &'static [Inline] { &[] }
```

Child rows shown on the parent's edit page (the "parent-with-children" pattern). Each `Inline` names a target model and the FK column linking back to this parent. Today the rows render with click-through navigation to the child; in-page row editing is on the roadmap.

```rust
use rustio_admin::admin::Inline;

fn inlines() -> &'static [Inline] {
    &[Inline {
        target_model:  "appointments",
        fk_field:      "patient_id",
        label:         Some("Appointments"),
        max_rows:      20,                 // then a "…and N more" link
        display_field: Some("status"),     // else name → title → … → #id
    }]
}
```

### `fieldsets`

```rust
fn fieldsets() -> &'static [Fieldset] { &[] }
```

Override the framework's name-heuristic grouping on the change form with explicit sections. Honoured by `render.rs` (`group_fields_by_fieldsets`); the heuristic only runs when a model declares no fieldsets. The struct:

```rust
pub struct Fieldset {
    pub title: &'static str,
    pub fields: &'static [&'static str],
}
```

### `validate`

```rust
fn validate(model: &Self) -> Result<(), Vec<FieldValidationError>> { Ok(()) }
```

Project-level validation run **before** the row hits the database, on both create and update. Return `FieldValidationError::field("col", "message")` to attach an error to a specific input, or `FieldValidationError::global("message")` for a form-wide error; both surface in the same UI as the framework's constraint-violation flash.

```rust
use rustio_admin::FieldValidationError;

fn validate(model: &Self) -> std::result::Result<(), Vec<FieldValidationError>> {
    let mut errs = Vec::new();
    if model.credit_hours == 0 {
        errs.push(FieldValidationError::field("credit_hours", "must be at least 1"));
    }
    if errs.is_empty() { Ok(()) } else { Err(errs) }
}
```

### `bulk_actions`

```rust
fn bulk_actions() -> &'static [BulkAction] { &[] }
```

Project-defined bulk actions surfaced as extra buttons in the list-view bulk bar (next to the framework's built-in Delete). Each button POSTs to `/admin/:model/bulk/:name`; the runtime dispatcher is `ModelAdmin::execute_bulk_action`.

```rust
use rustio_admin::BulkAction;

impl ModelAdmin for Post {
    fn bulk_actions() -> &'static [BulkAction] {
        &[
            BulkAction {
                name:        "publish",
                label:       "Publish selected",
                destructive: false,
                confirm:     false,
                permission:  None,                 // inherits the `change` gate
            },
            BulkAction {
                name:        "archive",
                label:       "Archive selected",
                destructive: true,
                confirm:     true,
                permission:  Some("archive"),      // also requires posts.archive_post
            },
        ]
    }
}
```

`BulkAction` is metadata only. To apply the work you override `execute_bulk_action` — a `ModelAdmin` method that returns a `BulkActionResult`:

```rust
use std::future::Future;
use std::pin::Pin;
use rustio_admin::{BulkAction, BulkActionContext, BulkActionResult, Db, ModelAdmin, Result};

impl ModelAdmin for Post {
    // ... bulk_actions() as above ...

    fn execute_bulk_action<'a>(
        action: &'a str,
        ids: &'a [i64],
        db: &'a Db,
        _ctx: &'a BulkActionContext<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<BulkActionResult>> + Send + 'a>> {
        Box::pin(async move {
            match action {
                "publish" => {
                    sqlx::query("UPDATE posts SET published = TRUE WHERE id = ANY($1)")
                        .bind(ids).execute(db.pool()).await?;
                    Ok(BulkActionResult::ok(ids.len()))
                }
                "archive" => {
                    sqlx::query("UPDATE posts SET archived_at = NOW() WHERE id = ANY($1)")
                        .bind(ids).execute(db.pool()).await?;
                    Ok(BulkActionResult::ok(ids.len()))
                }
                other => Err(rustio_admin::Error::BadRequest(
                    format!("unknown bulk action: {other}"))),
            }
        })
    }
}
```

The framework wraps each dispatch with **one audit row** (using the `BulkActionContext` actor + correlation id and the `BulkActionResult` outcome) — you don't audit the envelope yourself. Two failure channels: return `Err(...)` if the whole action failed (4xx/5xx page, attempt still audited), or `Ok(BulkActionResult::partial(ok, failed))` to report per-row failures (a partial-success audit row + per-id summary on the next request). The default impl returns a `BadRequest` naming the action, so a declared-but-undispatched action surfaces clearly rather than silently no-op'ing.

**Field semantics (`BulkAction`):**

- `name` — URL slug. Use snake_case identifiers. The framework reserves `delete` for its built-in cascade-aware delete (handled separately at `/bulk_delete`).
- `label` — Button text. Rendered as-is in the bulk bar and on the confirmation page header.
- `destructive` — `true` styles the button with the framework's danger-red variant; otherwise the default surface button.
- `confirm` — `true` renders a confirmation page listing every selected row before commit; `false` executes on the first POST. Use `confirm: true` for any action a user might regret.
- `permission` — `Some("foo")` additionally requires `<admin_name>.foo_<singular>` (or a role that bypasses group checks) on top of the route's `change` gate; `None` inherits — `change` is the only check. Use it to scope destructive actions to a narrower set of operators.

**Permission gate:** the bulk-action route is gated on the model's `change` permission (not `delete` — delete has its own `/bulk_delete` route); a `BulkAction.permission` adds a second, scoped check on top.

---

## Theming

`ModelAdmin` controls per-model behaviour. Cross-cutting site chrome lives on `Admin`:

```rust
use rustio_admin::admin::{Admin, AdminTheme, SiteBranding};

let admin = Admin::new()
    .site_branding(SiteBranding {
        site_title:       "Acme administration".into(),
        site_header:      "Acme".into(),
        index_title:      "Dashboard".into(),
        footer_copyright: "Acme Inc, 2026".into(),
        domain:           "acme.local".into(),
    })
    .accent_color("#2563EB")            // 90% of projects only need this
    .model::<Course>()
    .model::<Student>();
```

`AdminTheme` is an **override patch**, not a snapshot — every field is `Option<String>` and defaults to `None`, meaning *"don't override — let `admin.css` decide."* Out of the box the framework emits no inline `<style>` block at all; the stylesheet is the single source of truth for every design token. The framework is light-only — there is no dark-mode variant to worry about overriding.

If you need to override more than the accent, use the fluent builder:

```rust
use rustio_admin::admin::AdminTheme;

let theme = AdminTheme::new()
    .accent("#2563EB")
    .surface("#FAFAFA")
    .border("#E5E7EB");

let admin = Admin::new().theme(theme);
```

Available setters: `accent`, `bg`, `surface`, `text`, `text_muted`, `border`. Each accepts hex form (`"#A0341A"` or `"A0341A"`); the leading `#` is auto-normalised.

Fields you don't set inherit from `admin.css`.

The injected `<style>` block in `_theme.html` uses a single `html { ... }` selector so it wins cascade ties on source order without `!important`.

`Admin::accent()` returns `Option<&str>` — `None` means *"no override — admin.css owns it"*.

For a fuller re-skin than these six tokens, generate a complete, contrast-checked palette from a brand color with `rustio-admin theme generate --brand '#…'` and serve it at runtime with `RUSTIO_TOKENS_CSS=./tokens.css` — the file is appended after the baked CSS bundle. See [`design/DESIGN_THEME.md`](./design/DESIGN_THEME.md).

---

## Project user-profile extension

The built-in user profile page (`/admin/users/:id`) ships an empty `{% block project_user_fields %}` for projects to inject domain-specific sections:

```rust
use rustio_admin::admin::{UserProfileSection, UserProfileRow};

let admin = Admin::new()
    .user_profile_extension(|_db, user| Box::pin(async move {
        Ok(vec![UserProfileSection {
            label: "Account".into(),
            rows: vec![UserProfileRow {
                label: "Display name".into(),
                value: user.full_name.unwrap_or(user.email),
            }],
        }])
    }))
    .model::<Course>();
```

The closure runs on every Overview-tab render of the user view; sections append after the framework's show-grid.
