# translation-agency — a RustIO example

A small, runnable **interpreter / translation dispatch** admin: translators,
translation tasks with a status state, and an **automatic audit trail** — the
worked domain behind the
[translation-agency Quick Start](../../docs/quickstart-translation-agency.md).

It path-deps the in-repo framework (always builds against HEAD), exactly like
[`examples/shop`](../shop). A real downstream project would instead pin the
published crate: `rustio-admin = "0.31"`.

## What's inside

| Model | Fields |
|-------|--------|
| `Translator` | name, email, languages, active |
| `Task` | title, source/target lang, word count, deadline, `translator_id` (FK), `status` (`available → in_progress → review → completed`, or `reassigned`) |

Both migrations are **seeded with example rows**, so the admin is non-empty on
first sign-in.

## Run it

```sh
cd examples/translation-agency
cp .env.example .env                 # defaults are fine for local Postgres
createdb translation_agency_dev
cargo run                            # applies migrations, then serves
```

In another shell, create your login (the CLI is the `rustio-admin` binary):

```sh
rustio-admin user create --email coordinator@agency.local --role administrator
```

Then open **<http://127.0.0.1:8000/admin>**, sign in, and you have a working
dispatch admin — list, create, edit, search, delete, plus the audit trail at
**<http://127.0.0.1:8000/admin/history>**.

## The line this example draws

RustIO carries the **authority layer** — login, per-model permissions
(`view_task` / `change_task` / …), and the audit trail. **You** write the
**business logic**: the matching rule (which translator is *right* for a job)
and the legal status transitions live in your own code — see `src/task.rs`'s
`validate()` for where a domain rule goes, and
[`docs/security.md`](../../docs/security.md) for the "two kinds of no".
