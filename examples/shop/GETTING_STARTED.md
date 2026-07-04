# Getting started with this shop

A 5-minute orientation for a developer opening this project for the first
time. The generated `README.md` covers install and the CLI in depth — this
file is the **shop-specific cheat sheet**: what's already here, how to log in,
where things live, and how to extend it without guessing.

---

## 1. Run it

```sh
createdb shop_dev            # once; skip if it already exists
rustio-admin migrate apply   # applies migrations in ./migrations
cargo run                    # serves on http://127.0.0.1:8000
```

Two pages on boot:

| URL | What it is |
|-----|------------|
| <http://127.0.0.1:8000/> | Public homepage with live store stats (`templates/home.html`) |
| <http://127.0.0.1:8000/admin> | The admin panel — sign in here |

**Create a login** (the CLI prompts for the password twice):

```sh
rustio-admin user create --email you@shop.local --role administrator
```

---

## 2. The admin URLs (so you don't guess like I did)

Routes are **plural model name + verb** — not Django's `/change/`:

| Action | URL |
|--------|-----|
| List | `/admin/products` |
| Add new | `/admin/products/new` |
| Edit row | `/admin/products/<id>/edit` |
| Delete row | `/admin/products/<id>/delete` |

Same shape for every model: `orders`, `customers`, `categories`, …

---

## 3. The data model — and how it's connected

Nine models, already seeded with demo data (30 products, 40 orders, 20
customers). The foreign keys are wired into the admin as **inlines**, so
related rows show up right on the parent's edit page:

```text
Customer ──< Order ──< OrderItem >── Product ──< ProductImage
   │           │
   └─< Address └─< Payment
```

- Open a **Customer** → see their **Orders** and **Addresses** inline.
- Open an **Order** → see its **Line items** and **Payments** inline.
- Open a **Product** → see its **Images** inline.

That wiring lives in each model's `ModelAdmin::inlines()` (e.g.
`src/order.rs`). Copy that block to connect any new relationship.

---

## 4. Three recipes you'll reach for

### Add a field to an existing model
A model is plain Rust in `src/<model>.rs`. To add `sku: String` to `Product`:

1. **New migration** (never edit an applied one):
   `migrations/0010_add_product_sku.sql`
   ```sql
   ALTER TABLE products ADD COLUMN sku TEXT NOT NULL DEFAULT '';
   ```
2. **Edit `src/product.rs`** in the four matching spots: the struct field,
   `COLUMNS`, `from_row` (`sku: row.get_string("sku")?`), and
   `insert_values` / `INSERT_COLUMNS`.
3. `rustio-admin migrate apply && cargo run`.

### Add a whole new model
```sh
rustio-admin startapp review --field product:fk:Product --field rating:int --field body:text
```
This writes `src/review.rs` + a migration, and prints the two lines to paste
into `src/main.rs` (a `mod review;` and a `.model::<Review>()`). The `// rustio:`
markers in `main.rs` show exactly where.

### Show related rows on an edit page (inline)
In the **parent** model's `impl ModelAdmin`, add:
```rust
fn inlines() -> &'static [Inline] {
    &[Inline {
        target_model: "Review",   // the child struct name
        fk_field: "product_id",   // the FK column on the child
        label: Some("Reviews"),
        max_rows: 50,
        display_field: Some("rating"),
    }]
}
```

---

## 5. Where everything lives

```text
src/main.rs        app wiring: models registered, routes, middleware
src/<model>.rs     one file per model (struct + Model + ModelAdmin)
migrations/*.sql   schema, applied in numeric order (append-only)
seeds/*.sql        demo data: psql "$DATABASE_URL" -f seeds/shop_demo.sql
templates/         project overrides (home.html, admin/ theme)
.env              DB_URL, RUSTIO_SECRET_KEY, template dir
```

That's the whole surface. Everything else is the framework.
