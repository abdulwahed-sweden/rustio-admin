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

## Running locally with real email (Gmail SMTP)

By default the example uses the framework's `LogMailer` — password-
recovery emails print to stdout instead of being delivered. To send
a real email to your own Gmail inbox, configure SMTP via the
environment.

### Generate a Gmail App Password (one-time, ~5 minutes)

App Passwords are 16-character credentials Google issues to non-
browser clients like SMTP libraries. They bypass interactive sign-in
flow and 2FA.

1. Sign in to <https://myaccount.google.com>.
2. **Security → How you sign in → 2-Step Verification.** Enable it
   if it isn't already. App Passwords require 2FA on the account.
3. Open <https://myaccount.google.com/apppasswords>.
4. Type a label (e.g. `rustio-admin local`) and click **Create**.
5. Google shows a 16-character value formatted as four 4-letter
   groups separated by spaces (e.g. `abcd efgh ijkl mnop`).
   **Copy the value and remove the spaces** — SMTP_PASSWORD is the
   16-char concatenation, no whitespace.
6. Click **Done**. The App Password is shown only once. Store it
   immediately in your `.env`.

### Configure the SMTP env vars

Copy the template and fill the Gmail block:

```sh
cp .env.example .env
$EDITOR .env
```

`.env` is gitignored — your password never enters source control.
The Gmail block in `.env.example` already has the right defaults;
you just paste your 16-char App Password into `SMTP_PASSWORD=` and
set `MAIL_FROM` / `SMTP_USER` to your own address.

`MAIL_FROM` accepts either a bare address (`you@gmail.com`) or the
`Name <addr>` form. The display name is what your recipients see in
their inbox sender row.

### Validate the SMTP setup before booting the app

The CLI ships a one-shot doctor that runs the same boot-time
handshake the app does — TLS + AUTH against the configured server:

```sh
rustio doctor email
```

That's a read-only round-trip (no email is sent). If you want a
full end-to-end test that lands a message in an inbox, add the
`--to` flag:

```sh
rustio doctor email --to your.address@gmail.com
```

Either way you'll see ✓ / ⚠ / ✗ per check + a structured
diagnostic on failure. Run this before `cargo run` and you'll
never be surprised by a silent SMTP misconfiguration.

### Boot with the env loaded

The example's `main.rs` calls `dotenvy::dotenv()` at startup so
`.env` is loaded automatically — no `source` ritual required:

```sh
cargo run -p library-circulation
```

On startup, the mailer log line should read:

```
env: loaded /Users/you/.../examples/library-circulation/.env
mailer: validating SMTP to smtp.gmail.com:465 (TLS=implicit)… OK
mailer: SMTP authenticated; recovery emails will be delivered as Library Circulation <your.address@gmail.com>
```

If you instead see `mailer: SMTP_HOST unset; using LogMailer`,
`.env` either doesn't exist or doesn't contain `SMTP_HOST=`. The
boot-time handshake refuses to start the app silently against a
misconfigured SMTP — you'll always see exactly why.

### Trigger a real reset email

1. Sign in once as your superuser, then sign out (or open an
   incognito window).
2. Click **Forgot password** on `/admin/login`, or visit
   `/admin/forgot-password` directly.
3. Enter the email address that's registered on the account you
   want to recover.
4. The page returns a generic "if that email has an account, we
   sent a sign-in link" response — same for known and unknown
   addresses (anti-enumeration).
5. Open your Gmail inbox. The reset email arrives within a few
   seconds. Click **Set a new password** in the email body — that
   opens `http://127.0.0.1:3000/admin/reset-password/<token>` in
   your browser.
6. Choose a new password. The framework enforces its password
   policy (length, etc.) before persisting; failures show inline.
7. On success, all existing sessions for that account are revoked
   per Doctrine 22; sign in again with the new password.

### Production-style alternative: Resend

Same env-var contract, different credentials. Sign up at
<https://resend.com>, generate an API key, then set:

```
MAIL_FROM=RustIO Admin <noreply@yourdomain.com>
SMTP_HOST=smtp.resend.com
SMTP_PORT=465
SMTP_USER=resend
SMTP_PASSWORD=re_xxxxxxxxxxxxxxxxxxxx   # the API key
SMTP_TLS=implicit
```

Resend requires a verified sending domain in production — for first
test runs you can use the `onboarding@resend.dev` sandbox sender.

## What this example demonstrates

- Relational modelling across four tables with three foreign keys.
- `#[rustio(belongs_to = "Target", display = "field")]` on each
  FK column. The admin renders proper `<select>` dropdowns on
  create / edit and turns the FK cell in list views into a
  navigation link to the related row's edit page.
- `Admin::new().model::<T>()` wiring for every model.
- The R0-canonical middleware chain (logger → correlation_id →
  security_headers → csrf_protect).
- Time-bound records with `Option<DateTime<Utc>>`
  (`Loan.returned_at`).
- State machines via plain `String` + SQL CHECK:
  - `items.status` ∈ `available | on_loan | lost | withdrawn`
  - `loans.status` ∈ `active | returned | overdue`
  - `items.kind`   ∈ `book | audiobook | dvd`
- **Project-defined bulk actions** via the public
  `ModelAdmin::execute_bulk_action` hook. `Loan` declares
  `mark_overdue` and `mark_returned`; each runs a
  SELECT-then-UPDATE round-trip, partitions the selection into
  eligible vs. ineligible rows, and returns a `BulkActionResult`
  with a per-id failure list + operator-facing summary line.
  Partial-failure paths exercised (e.g. selecting an already-
  returned loan for `mark_overdue` skips the row with a clear
  reason). The framework emits one audit row per submission.
- 135 rows of deterministic seeded data populate the admin to a
  clickable state on first boot.

## Out of this example's scope

Permission-group seeding. As of 0.10.2
`permissions::create_group` is idempotent and safe to call
repeatedly from Rust seed code at boot, but SQL migrations run
*before* `admin.seed_permissions()` — so the SQL seed file can't
bind groups to permission rows that don't exist yet at migration
time. Group seeding belongs in Rust alongside the boot path; left
out here to keep `main.rs` linear and teaching-focused.

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
