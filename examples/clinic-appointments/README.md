# clinic-appointments

Flagship example for [`rustio-admin`](https://github.com/abdulwahed-sweden/rustio-admin).
Boots an admin panel for a small multi-site clinic system:
clinics, patients, practitioners, and appointments.

## Domain

Four tables, three foreign keys.

| Table          | Rows seeded | Purpose                                              |
|----------------|------------:|------------------------------------------------------|
| clinics        | 5           | Clinic locations.                                    |
| patients       | 20          | Patient roster (chart number + contact email).       |
| practitioners  | 30          | Clinicians attached to a clinic, with specialty.     |
| appointments   | 30          | Bookings tying a patient to a practitioner + time.   |

Foreign keys (all `ON DELETE RESTRICT`):

- `practitioners.clinic_id` → `clinics.id`
- `appointments.patient_id` → `patients.id`
- `appointments.practitioner_id` → `practitioners.id`

Status enums (Postgres `CHECK` constraints):

- `practitioners.specialty` ∈ `family_medicine | pediatrics | cardiology | dermatology | orthopedics | psychiatry`
- `practitioners.status` ∈ `active | on_leave | retired | pending`
- `appointments.status` ∈ `scheduled | completed | cancelled | no_show`

Bulk actions on `appointments`:

- **Mark no-show** — `scheduled` → `no_show`. Useful for the morning sweep on appointments whose `scheduled_at` is already in the past with no `checked_in_at`.
- **Mark completed** — `scheduled` → `completed`, sets `ended_at = NOW()`.

Both bulk actions follow the SELECT-then-UPDATE pattern: read each row's current status, partition into eligible-vs-ineligible-with-reason, run one batched UPDATE on the eligible ids, and return a `BulkActionResult` carrying per-id failure reasons + an operator-facing summary.

## Running locally

Requires Postgres and a `rustio-admin-cli` install.

```sh
# 1. Create the database.
createdb clinic_appointments_demo

# 2. Set DATABASE_URL (or rely on the localhost default).
export DATABASE_URL=postgres://localhost/clinic_appointments_demo

# 3. Boot the admin. Migrations + seed apply on first run.
cargo run -p clinic-appointments

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

```sh
# 1. Enable 2-Step Verification on the Google account.
#    https://myaccount.google.com/security
#
# 2. Generate a 16-character App Password.
#    https://myaccount.google.com/apppasswords
#
# 3. Copy `.env.example` to `.env` and fill in the credentials.
#    The example reads it automatically via `dotenvy`.
cp .env.example .env
$EDITOR .env

# 4. Validate the SMTP config before booting the app.
rustio doctor email --to your-email@gmail.com

# 5. Boot. The startup banner reports SMTP authentication status.
cargo run -p clinic-appointments
```

The `LettreSmtpMailer` in `src/mailer.rs` is the canonical reference
for "how to wire a real mailer." The framework keeps `lettre` out of
its dependency tree on principle — projects own their email
transport choice.

## What this example demonstrates

- **Manual `Model` impls** alongside the `RustioAdmin` derive — both styles ship side-by-side so consumers can pick.
- **FK metadata** via `#[rustio(belongs_to = "Clinic", display = "name")]` — drives the list-page FK column rendering and the form-side relation picker.
- **Status enums via `CHECK` constraints** in the migration. The framework's CRUD path translates Postgres constraint violations into inline form errors.
- **Bulk actions** with partial-success semantics. `ModelAdmin::bulk_actions()` declares them; `execute_bulk_action` runs the SELECT-then-UPDATE logic and returns a `BulkActionResult`.
- **Real SMTP transport** wired via `Admin::mailer(...)` in `main.rs` and the standalone `LettreSmtpMailer` in `src/mailer.rs`.
- **Project-identity branding** — `Admin::app_name`, `app_tagline`, `support_email`, `public_url`. The framework name `RustIO` is intentionally absent from user-facing surfaces.

## Deterministic seed

The 5 / 20 / 30 / 30 dataset uses public-domain literary names
(`Elizabeth Bennet`, `Atticus Finch`, …) and `@example.test`
emails. Timestamps are relative to boot day via `NOW() - INTERVAL`
arithmetic, so a fresh seed always shows "recent" data.

A handful of rows are deliberately off-axis for variety:
- Two patients with `is_active = FALSE`.
- Three practitioners with `status` of `retired` / `on_leave` /
  `pending` — exercises the status-pill rendering on list pages.
- Three appointments with `scheduled_at` in the past but still
  `scheduled` — perfect bulk-`mark_no_show` candidates.
