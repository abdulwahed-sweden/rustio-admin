# FixShop

A small repair shop, run entirely from `rustio-admin`.

Someone walks in with a broken thing — a phone, a laptop, a bike, a
washing machine. The shop takes it in, works out what is wrong, sends
the customer a price, and fixes it once the customer says yes. Then the
customer comes back and picks it up.

That is the whole domain. No payments, no parts inventory, no invoices.

```text
booked in ─▶ diagnosed ─▶ quoted ─┬─▶ approved ─▶ in progress ─▶ ready ─▶ collected
                                  └─▶ declined
```

**The one rule that matters:** a job cannot go to *in progress* until
its quote has been approved. Nobody starts work the customer has not
agreed to pay for.

Everything here is `rustio-admin` doing its job — the `RustioAdmin`
derive, the role ladder and group permissions, bulk actions, the
validation hook, and the audit log. There is no framework code in this
directory, only four models, a status ladder, and some SQL.

---

## Running it

You need Postgres and a stable Rust toolchain ≥ 1.94.

```sh
cd examples/fixshop
createdb fixshop_dev
cp .env.example .env          # edit DATABASE_URL if your Postgres differs
cargo run                     # applies migrations, seeds the shop, serves :8000
```

Then create the three people who work here. Passwords are Argon2id-hashed
by the framework and cannot be written from SQL, so the accounts are made
with the CLI rather than seeded:

The CLI's database verbs live behind its `db` feature, and it is a
member of the repository's workspace rather than this one — so run it
from the repository root:

```sh
cd ../..                      # back to the rustio-admin checkout
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/fixshop_dev
CLI="cargo run -q -p rustio-admin-cli --features db --"

$CLI user create --email anna@fixshop.test --role staff
$CLI user create --email mike@fixshop.test --role staff
$CLI user create --email sara@fixshop.test --role administrator

$CLI group add-user --group front_desk --email anna@fixshop.test
$CLI group add-user --group technician --email mike@fixshop.test
```

Each `user create` prompts for a password (minimum 10 characters); pass
`--password` to supply one inline.

Open <http://127.0.0.1:8000/admin> and sign in as any of them.

| | Signs in as | Group | Can |
|---|---|---|---|
| **Anna** — front desk | `staff` | `front_desk` | Create customers and jobs, record what the customer said about a quote, hand repaired items back |
| **Mike** — technician | `staff` | `technician` | Diagnose, write quotes, do the repair |
| **Sara** — owner | `administrator` | — | Everything, plus editing quotes and deleting |

Sara needs no group. `Role::Administrator` bypasses group permission
checks entirely, which is what "the owner can do everything" means in
framework terms. Anna and Mike are both plain `staff`, so every gate
below is a real permission lookup.

## Walking through it

Sign in as **Mike** and open Jobs. FS-1002 (Ruth Palmer's Dell laptop)
is sitting at *quoted* — he has already diagnosed it and written a 210
quote. Tick it and press **Customer approved**:

> 403 — Bulk action `record_approval` requires the `jobs.approve_job` permission.

That is not Mike's call to make. Sign in as **Anna** instead, tick
FS-1002, press **Customer approved** — the quote flips to approved and
the job moves on. Now Anna tries **Start repair**:

> 403 — Bulk action `start_repair` requires the `jobs.repair_job` permission.

Back to Mike, who presses **Start repair** and gets it.

Now the rule. FS-1004 (Ellie Marsh's washing machine) is still at
*booked in* with no quote at all. Select it together with a job that is
genuinely ready to move and press **Start repair**. The batch does not
fail — the job that can move moves, and the one that cannot is refused
by name:

> FS-1004 has no approved quote — the customer has to say yes first

The same rule guards the ordinary edit form. Try adding a new job with
its status set straight to *in progress* and the form comes back with:

> Status cannot be "in progress" until the customer has approved a quote.

## What is in the seed

Five customers and seven jobs spread across the ladder. Two of them —
FS-1001 and FS-1003 — are past their promised date and still in the
shop, so they sit at the top of the Jobs list with **Overdue: yes**.
FS-1007 is also past its date but was collected, so it is not overdue,
it is done.

Every date is relative to today, so the shop looks plausible whenever
you run it.

## How the pieces map to the framework

**Models** — `customer.rs`, `job.rs`, `quote.rs`, `job_event.rs`. Each is
a plain struct with `#[derive(RustioAdmin)]`, which generates both the
admin metadata and the ORM glue from the fields. `#[rustio(belongs_to)]`
turns `customer_id` into a link to the customer and the sidebar filter
into a customer picker; `#[rustio(choices)]` draws the status dropdown,
with a matching CHECK constraint in the migration as the backstop.

**The ladder** — `workflow.rs`. Each of the seven steps is a
`BulkAction` with its own `permission`, so the framework refuses the
wrong person before the shop's code runs. Steps apply per row and
collect refusals rather than failing the batch.

**The rule** — enforced twice, against one fact. `jobs.quote_approved`
is a mirror of "this job has an approved quote", kept equal to
`quotes.approved` by a trigger (migration 0003). `Job::validate` reads it
to guard the edit form; `workflow.rs` reads it to guard the ladder. The
mirror exists because `ModelAdmin::validate` is synchronous and cannot
query the database — so the fact has to be on the row.

**Overdue** — `jobs.overdue` is the other mirror, and the only one no
write can maintain: a job goes overdue because the clock moved, not
because anyone touched it. `main::refresh_overdue_flags` recomputes it at
boot and once a minute after that, in the same shape as the framework's
own housekeeping sweeper. In exchange the Jobs list gets a pill, a
one-click sidebar filter, and `ordering()` of `["-overdue",
"promised_date"]` — late jobs pinned to the top of page one.

**Two logs.** `rustio_admin_actions` is the framework's, written
automatically for every create, edit and delete plus one row per bulk
submission. `job_events` is the shop's, written by `workflow.rs`: what
moved, from where to where, who did it. It is registered
`read_only_model`, so not even Sara can rewrite history — a POST to it
comes back 403 *"Model `job_events` is frozen"*. The two are joined by
the correlation id the framework's middleware mints per request, so one
click on seven jobs is one chain.

## Things worth knowing

Three places where the framework behaves in a way the UI does not
telegraph:

- **The bulk bar shows every button to everyone.** `BulkAction.permission`
  is enforced on POST, not used to hide the button. Anna sees *Start
  repair* and gets a 403 if she presses it.
- **Per-row refusals land in the audit log, not on screen.** After a
  bulk action you are redirected back to the list; the reason each row
  was refused is in `rustio_admin_actions.metadata.failure_reasons`,
  visible at `/admin/history`.
- **`read_only_model` blocks writes, not pages.** The add and edit pages
  for `job_events` still render; the POST is what comes back 403.

One deliberate oddity in the SQL: the trigger function in migration 0003
is written with a single-quoted body rather than the usual `$$ … $$`.
The framework's migration runner splits files into statements on `;`,
and its dollar-quote handling mangles the opening tag, which makes the
rest of the file one unparseable statement. A quoted body takes the
string branch, which is correct.

## Tests

```sh
cargo test
```

Twelve unit tests, no database needed. They cover the rule from both
sides, that the status dropdown and the ladder offer the same set, and
that `in_progress` has exactly one way in and it checks for an approved
quote.

## Not a workspace member

This example declares its own empty `[workspace]`, so `cargo build
--workspace` at the repository root does not pick it up. `rustio-admin`
ships no bundled examples — `rustio-admin startproject --preset …` is
the supported way to get a working project — so this directory stays
out of the framework's build and lint gates. It depends on
`rustio-admin` by path, so it always compiles against this checkout.
