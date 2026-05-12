# library-circulation

Flagship example for [`rustio-admin`](https://github.com/abdulwahed-sweden/rustio-admin).
Boots an admin panel for a small public-library circulation system:
branches, patrons, items, and loans.

## Domain

Four tables, three foreign keys.

| Table     | Rows seeded | Purpose                                          |
|-----------|------------:|--------------------------------------------------|
| branches  | 5           | Library branches.                                |
| patrons   | 20          | Library members.                                 |
| items     | 80          | Books, audiobooks, DVDs across the 5 branches.   |
| loans     | 30          | Borrowing records (returned / active / overdue). |

Foreign keys (all `ON DELETE RESTRICT`):

- `items.branch_id` → `branches.id`
- `loans.patron_id` → `patrons.id`
- `loans.item_id` → `items.id`

## Running locally

Requires Postgres and a `rustio-admin-cli` install.

```sh
# 1. Create the database.
createdb library_circulation_demo

# 2. Set DATABASE_URL (or rely on the localhost default).
export DATABASE_URL=postgres://localhost/library_circulation_demo

# 3. Boot the admin. Migrations + seed apply on first run.
cargo run -p library-circulation

# 4. In a second shell, create a superuser to sign in with.
cargo install rustio-admin-cli      # if not already installed
rustio user create --email admin@example.test --role developer

# 5. Open http://127.0.0.1:3000/admin/ and sign in.
```

## What this example demonstrates

- Relational modelling across four tables with three foreign keys.
- `Admin::new().model::<T>()` wiring for every model.
- The R0-canonical middleware chain (logger → correlation_id →
  security_headers → csrf_protect).
- Time-bound records with `Option<DateTime<Utc>>`
  (`Loan.returned_at`).
- State machines via plain `String` + SQL CHECK:
  - `items.status` ∈ `available | on_loan | lost | withdrawn`
  - `loans.status` ∈ `active | returned | overdue`
  - `items.kind`   ∈ `book | audiobook | dvd`
- 135 rows of deterministic seeded data populate the admin to a
  clickable state on first boot.

## What this example does not yet demonstrate

Two framework capabilities are not exercised here because they
need follow-up framework work:

- **Custom bulk actions.** Need a public project-side dispatch
  hook; the existing `AdminOps` trait is `pub(crate)`.
- **Seeded permission groups.** Need an idempotent group-seeding
  helper; `permissions::create_group` is not idempotent today.

Both deferrals are also recorded in `migrations/0005_seed.sql`'s
footer comment.

## File layout

```text
examples/library-circulation/
├── Cargo.toml
├── README.md                ← this file
├── .env.example
├── migrations/
│   ├── 0001_branches.sql
│   ├── 0002_patrons.sql
│   ├── 0003_items.sql
│   ├── 0004_loans.sql
│   └── 0005_seed.sql        ← deterministic 135-row demo dataset
└── src/
    ├── main.rs              ← 10-step linear boot path
    └── models/
        ├── mod.rs
        ├── branch.rs
        ├── patron.rs
        ├── item.rs
        └── loan.rs
```

## Doctrine

The framework's design contracts:

- [`docs/DESIGN_DOCTRINE.md`](../../docs/DESIGN_DOCTRINE.md) — visual identity and token philosophy.
- [`docs/design/`](../../docs/design/) — long-form design specs (audit, sessions, recovery, MFA, R2/R3/R4).
- [`docs/public-api.md`](../../docs/public-api.md) — declared public API surface.
