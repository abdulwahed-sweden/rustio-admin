# `ModelAdmin` reference

`ModelAdmin` is the customisation surface for every model registered via `Admin::new().model::<M>()`. Every method on the trait has a default body, so the minimum you need is:

```rust
impl ModelAdmin for Post {}            // accept every default
```

Override only what you care about; the rest inherit framework defaults.

## Why no blanket impl?

An earlier prototype shipped `impl<T: AdminModel> ModelAdmin for T {}` so every derived model would auto-pick-up defaults. That collides with Rust's coherence rules — without `feature(specialization)` (nightly-only), a blanket impl forbids per-type impls, which would block the project overrides this trait exists for. The opt-in `impl ModelAdmin for X {}` is the standard stable-Rust pattern (serde's `Serialize`, axum's `Handler`, std's various marker traits).

## The hooks

| Method | Default | Used by |
|---|---|---|
| `list_display`     | `&[]` (every column on `M::FIELDS`) | List page columns |
| `list_filter`      | `&[]` (none) | Filters dropdown chips |
| `search_fields`    | `&[]` (search box decorative) | `?q=term` ILIKE search |
| `ordering`         | `&["-id"]` (newest first) | List page `ORDER BY`; default for the Sort dropdown |
| `list_per_page`    | `50` | Default page size; user override via `?per_page=` |
| `readonly_fields`  | `&[]` | Form field disabling (UI hint) |
| `fieldsets`        | `&[]` (heuristic grouping) | Form section ordering |
| `bulk_actions`     | `&[]` (Delete only) | Extra buttons in the list-view bulk bar |

Every method returns `&'static [&'static str]` (or `&'static [Fieldset]`) so the values are captured into `AdminEntry` once at registration time and read straight from the entry on every request. No per-request virtual dispatch beyond the existing `dyn AdminOps`.

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

### `readonly_fields` (planned)

```rust
fn readonly_fields() -> &'static [&'static str] { &[] }
```

Columns the change form should render as disabled. **Currently captured but not yet honoured by `form_ctx`** — wires up before v0.1.0. The macro's `editable: false` flag still owns the strict per-field gate (e.g. `id` and `created_at` stay non-editable regardless).

### `fieldsets` (planned)

```rust
fn fieldsets() -> &'static [Fieldset] { &[] }
```

Override the framework's name-heuristic grouping on the change form (Default / System / Advanced) with explicit sections. The struct:

```rust
pub struct Fieldset {
    pub title: &'static str,
    pub fields: &'static [&'static str],
}
```

**Currently captured but not yet honoured by `form_ctx`** — wires up before v0.1.0.

### `bulk_actions`

```rust
fn bulk_actions() -> &'static [BulkAction] { &[] }
```

Project-defined bulk actions surfaced as extra buttons in the list-view bulk bar (next to the framework's built-in Delete). Each button POSTs to `/admin/:model/bulk/:name`; the runtime dispatcher is `AdminOps::execute_bulk_action`.

```rust
use rustio_admin::admin::BulkAction;

impl ModelAdmin for Post {
    fn bulk_actions() -> &'static [BulkAction] {
        &[
            BulkAction {
                name:        "publish",
                label:       "Publish selected",
                destructive: false,
                confirm:     false,
            },
            BulkAction {
                name:        "archive",
                label:       "Archive selected",
                destructive: true,
                confirm:     true,
            },
        ]
    }
}
```

`BulkAction` is metadata only. To actually apply the work you override `AdminOps::execute_bulk_action` on the model:

```rust
impl AdminOps for ConcreteOps<Post> {
    // ... existing methods ...

    fn execute_bulk_action<'a>(
        &'a self,
        db: &'a Db,
        name: &'a str,
        ids: &'a [i64],
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            match name {
                "publish" => {
                    sqlx::query("UPDATE posts SET published = TRUE WHERE id = ANY($1)")
                        .bind(ids)
                        .execute(db.pool())
                        .await?;
                    Ok(())
                }
                "archive" => {
                    sqlx::query("UPDATE posts SET archived_at = NOW() WHERE id = ANY($1)")
                        .bind(ids)
                        .execute(db.pool())
                        .await?;
                    Ok(())
                }
                other => Err(Error::BadRequest(format!("unknown bulk action: {other}"))),
            }
        })
    }
}
```

The default trait impl returns a `BadRequest` with the action name embedded, so an action you registered but didn't dispatch surfaces as a clear 400 page rather than a silent no-op.

**Field semantics:**

- `name` — URL slug. Use snake_case identifiers. The framework reserves `delete` for its built-in cascade-aware delete (handled separately at `/bulk_delete`). Registering an action named `delete` is a runtime error.
- `label` — Button text. Rendered as-is in the bulk bar and on the confirmation page header.
- `destructive` — `true` styles the button with the framework's danger-red variant; otherwise it uses the default surface button.
- `confirm` — `true` renders a confirmation page listing every selected row before commit; `false` executes on the first POST. Use `confirm: true` for any action a user might regret.

**Permission gate:** the bulk-action route is gated on the model's `change` permission (not `delete` — delete has its own `/bulk_delete` route). Override `execute_bulk_action` to apply finer-grained checks (e.g. block destructive actions for non-admins).

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
