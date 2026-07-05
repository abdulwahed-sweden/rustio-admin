# Quick Start: a translation-agency admin

Build a real **interpreter / translation dispatch** admin — translators,
translation tasks, and an **automatic audit trail of who did what** — in about
ten minutes. It is the kind of internal system a language agency actually runs
on: work comes in, the right translator takes it, circumstances change, the job
is reassigned, and every step has to be accountable for invoicing.

This guide is deliberately honest about the line between the two halves of such
a system:

> **RustIO carries the authority layer — login, roles, per-model permissions,
> and the audit trail. You write the business logic — who is the *right*
> translator, and which status may follow which.** RustIO makes that logic safe
> to write; it does not write it for you.

If you have not run a RustIO project before, skim
[`getting-started.md`](./getting-started.md) first — this guide uses the same
commands on a richer domain.

---

## What you'll build

- A **`Translator`** model — name, contact, the languages they cover, whether
  they're active.
- A **`Task`** model — a translation job with a source/target language, a word
  count, a deadline, an assigned translator, and a **status** drawn from a fixed
  set (`available → in_progress → review → completed`, or `reassigned`).
- An **audit trail** you did not write — every change to a task is recorded with
  who and when — plus login, roles, and recovery, all for free.

---

## Before you start

- **Rust 1.94+** (`rustup show`) and a reachable **PostgreSQL** (Postgres-only,
  by design). To run one in Docker:

  ```sh
  docker run --name rio-pg -e POSTGRES_PASSWORD=dev -p 5432:5432 -d postgres:16
  ```

No Node, bundler, or frontend toolchain.

---

## 1. Scaffold the project and sign in

```sh
cargo install rustio-admin-cli          # installs the `rustio-admin` binary
rustio-admin new translation-agency     # at "Project type", choose 1) custom
cd translation-agency
createdb translation_agency_dev         # the database name the wizard showed you
rustio-admin migrate apply              # a fresh custom project has no model tables yet — expected
rustio-admin user create --email coordinator@agency.local --role administrator
cargo run                               # first build takes a few minutes
```

Open **<http://127.0.0.1:8000/admin>** and sign in as the coordinator. You have
a working, secured admin with **zero models yet** — login, the 5-tier role
ladder, recovery, and an audit trail are already there.

---

## 2. Add the `Translator` model

One command scaffolds the model, its table, an admin page, and its permissions:

```sh
rustio-admin startapp translator \
  --field name:str \
  --field email:email \
  --field languages:str \
  --field active:bool
```

`startapp` never edits `main.rs` for you — it prints three lines to paste under
their `// rustio:` markers, so you stay the author:

```rust
mod translator;                          // under `// rustio: modules`
use translator::Translator;              // under `// rustio: imports`

let admin = Admin::new()
    .model::<Translator>();              // under `// rustio: models`
```

Then:

```sh
rustio-admin migrate apply              # creates the `translators` table
```

> `languages` is free text here (e.g. `"en, ar, sv"`) to keep the Quick Start
> short. In a real system you'd model it as its own table and a join — RustIO's
> `fk:` handles that when you're ready.

---

## 3. Add the `Task` model — status set + assigned translator

Two field types do the interesting work here: **`choice`** (a fixed status set,
enforced by a Postgres `CHECK` and rendered as a dropdown) and **`fk`** (a
foreign key that renders as a link to the translator):

```sh
rustio-admin startapp task \
  --field title:str \
  --field source_lang:str \
  --field target_lang:str \
  --field word_count:int \
  --field deadline:date \
  --field status:choice:available,in_progress,review,completed,reassigned \
  --field translator_id:fk:Translator
```

Paste its three `// rustio:` lines into `src/main.rs` as before
(`mod task; use task::Task; .model::<Task>()`), then:

```sh
rustio-admin migrate apply              # creates the `tasks` table
cargo run
```

Your pages are live:

- **<http://127.0.0.1:8000/admin/translators>**
- **<http://127.0.0.1:8000/admin/tasks>**

List, create, edit, search, delete — and the `translator_id` column links
straight to the assigned person, not a raw id.

> A ready-to-run version of everything below lives at
> [`examples/translation-agency/`](../examples/translation-agency/) — compiled,
> tested, and seeded with example rows.

---

## 4. The scenario — and what is framework vs. your code

Walk the everyday agency workflow, and notice which half is doing the work.

**1 · The coordinator creates a task** — an English → Arabic medical file,
1,200 words, due Friday, `status: available`.
→ *Framework.* Models, the admin form, the `CHECK` that stops a typo like
`status: dnoe`.

**2 · It must go to a translator who covers Arabic — not a French-only one.**
→ *This is your code, and RustIO is honest about it.* The `fk` links a task to a
translator; it does **not** know that this translator can't do Arabic. That
**matching rule — intersecting language, availability, and rating — is business
logic you write.** What RustIO *does* enforce here is a different, equally
important refusal: **who is even allowed to assign a task.** A user without the
`change_task` permission is rejected before the request reaches the database.

> **Two kinds of "no":** RustIO refuses the *unauthorised actor* (authority);
> you refuse the *wrong translator* (domain). Keep them separate and both stay
> simple.

**3 · The translator accepts, then goes quiet as the deadline nears.**
→ *Shared.* The `choice` field guarantees `status` is always one of your five
values (framework). Enforcing that `available` may become `in_progress` but not
jump straight to `completed` — the legal transitions — is a small rule you add
in your own code (yours). RustIO gives you the constrained set to build on.

**4 · The coordinator reassigns the job and RustIO records it — automatically.**
→ *Framework, and this is the payoff.* You wrote no logging code, yet every
change to authority and every admin action lands in `rustio_admin_actions` with
a per-request correlation id: **who reassigned, when.** For an agency that bills
by the job, that trail is the difference between a clean invoice and an argument.
Browse it at **<http://127.0.0.1:8000/admin/history>**.

---

## What the framework carried — for free

- **Login, the 5-tier role ladder, and recovery** — Argon2id passwords,
  hashed-at-rest session tokens, self-service and admin-driven reset/lock/revoke.
- **Per-model permissions** — `view_task` / `add_task` / `change_task` /
  `delete_task` (and the same for `translator`), seeded at boot, editable per
  group. This is where "who may assign" is enforced.
- **The audit trail** — every authority change recorded, no code from you.
- **Foreign keys that read like data** and a **status dropdown backed by a
  `CHECK`** — from two words in the `startapp` command.

## What you write — your business

- The **matching rule** (language + availability + rating).
- The **legal status transitions** (a small state-machine guard).
- **Pricing and invoicing** off the audit trail.

That division is the whole point: RustIO refuses to guess how your agency works,
and instead makes the parts that *must* be trustworthy — identity, permission,
and the record — trustworthy by default.

---

## Where to go next

- [`ModelAdmin` reference](./modeladmin.md) — shape the list, search, filters,
  and ordering on your `Task` and `Translator` pages.
- [`architecture.md`](./architecture.md) — how a request travels through the
  authority layers to the database.
- [`memory.md`](./memory.md) — record *why* you chose this status set or that
  matching rule, so the reasoning outlives the conversation.
