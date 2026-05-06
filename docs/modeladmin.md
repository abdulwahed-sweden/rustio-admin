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
| `list_filter`      | `&[]` (none) | Sidebar filter chips |
| `search_fields`    | `&[]` (search box decorative) | `?q=term` ILIKE search |
| `ordering`         | `&["-id"]` (newest first) | List page `ORDER BY` |
| `list_per_page`    | `50` | Default page size for `?per_page=` |
| `readonly_fields`  | `&[]` | Form field disabling (UI hint) |
| `fieldsets`        | `&[]` (heuristic grouping) | Form section ordering |

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

Default rows-per-page on the list view. The user can override at runtime via `?per_page=N`, but the param is allow-listed to `{10, 25, 50, 100}` so a malicious query can't OOM the worker.

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
    .theme(AdminTheme {
        accent:     "#2563EB".into(),
        bg:         "#F4F6FB".into(),
        surface:    "#FFFFFF".into(),
        text:       "#111827".into(),
        text_muted: "#4B5563".into(),
        border:     "#D1D5DB".into(),
    })
    .model::<Course>()
    .model::<Student>();
```

The six theme tokens are injected into every page as `--rio-*` CSS custom properties; the hand-written `admin.css` derives every hover, focus ring, stripe, chip, and badge from them via `rgba(var(--rio-accent-rgb) / N)` style. A re-skin via `Admin::accent_color("#FF8800")` immediately re-tints the whole UI without touching CSS.

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
