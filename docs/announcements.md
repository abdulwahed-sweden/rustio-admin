# Announcements

Ready-to-use announcement copy for RustIO. Every version is deliberately
honest about maturity (early, open, **pre-1.0**) and makes no scale or
performance claims. Keep them that way when you edit.

Links use absolute URLs so they work when pasted off-repo.

---

## GitHub Sponsors — welcome message

Paste into the [Sponsors dashboard](https://github.com/sponsors/abdulwahed-sweden/dashboard)
→ **Welcome message** (markdown supported).

> Thank you — genuinely.
>
> RustIO is early, open, and pre-1.0, and backing it now is the kind of support that actually moves it forward. You're not buying a product with a marketing budget behind it — you're funding focused, unglamorous work: authentication, permissions, recovery, and audit built into the core, done carefully rather than quickly.
>
> A few places to start:
>
> - **Roadmap** — what's next, and why: https://github.com/abdulwahed-sweden/rustio-admin/blob/main/ROADMAP.md
> - **Quick Start** — build a working admin in a few minutes: https://github.com/abdulwahed-sweden/rustio-admin/blob/main/docs/quickstart-translation-agency.md
> - **Manifesto** — what RustIO stands for, and refuses to be: https://github.com/abdulwahed-sweden/rustio-admin/blob/main/MANIFESTO.md
>
> If you have a use case, a rough edge, or a question, open an issue — sponsor input carries real weight in what I build next.
>
> *Built in Rust. Governed by engineers. Guarded by design.*
>
> — Abdulwahed

---

## X / Twitter — launch thread

**1/**
RustIO is now on GitHub Sponsors. 🦀

Django Admin for Rust — with authentication, permissions, recovery, and audit built into the core, not bolted on.

github.com/sponsors/abdulwahed-sweden

**2/**
The stance: an admin framework that refuses the unauthorized actor by default, and writes a typed audit trail for every authority change. You govern who can do what; the framework enforces it.

Postgres-first. No build step. No magic.

**3/**
It's early — pre-1.0, built in the open over the last few months. Sponsoring now is backing focused open Rust infrastructure at the stage where support actually changes the trajectory.

Built in Rust. Governed by engineers. Guarded by design.

---

## LinkedIn

> I've opened GitHub Sponsors for RustIO.
>
> RustIO is an administrative framework for Rust — think Django Admin, but with authentication, permissions, account recovery, and audit designed as one system in the core rather than assembled from separate parts. It's Postgres-first, refusal-first (unauthorized actors are denied by default), and audit-by-default (every authority change writes a typed, correlated audit record).
>
> I'm honest about where it is: early, open source, and pre-1.0, built over the last few months. There's no marketing budget behind it — I'm building it carefully in the open. Sponsorship is early backing for open Rust infrastructure, at the stage where support actually changes the trajectory.
>
> If secure-by-default admin tooling for Rust is something you'd use — or want to see exist — you can back it here: https://github.com/sponsors/abdulwahed-sweden. Feedback and issues are just as welcome; they shape what I build next.
>
> Built in Rust. Governed by engineers. Guarded by design.

---

## r/rust — technical post

**Title:** RustIO: a Postgres-first admin framework where auth, permissions, recovery, and audit are in the core (early, pre-1.0)

**Body:**

I've been building **RustIO** — an administrative framework for Rust. The short pitch is "Django Admin for Rust," but the part I actually care about is that authentication, sessions, permissions, account recovery, and audit are designed as *one system in the core*, not assembled from separate crates you wire together yourself.

You derive a model and get both the admin pages and the ORM contract from the same struct:

```rust
#[derive(RustioAdmin)]
#[rustio(table = "tasks")]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub deadline: NaiveDate,
    #[rustio(choices = ["available", "in_progress", "review", "completed"])]
    pub status: String,
}

impl ModelAdmin for Task {
    fn list_display() -> &'static [&'static str] { &["title", "status", "deadline"] }
    fn search_fields() -> &'static [&'static str] { &["title"] }
}

// in main:
let admin = Admin::new().model::<Task>();
```

The `derive` emits the list/form rendering **and** the `impl Model` (table, columns, row decoder, insert binder) — so there's nothing to keep in sync by hand.

### Opinionated design decisions (the interesting part)

- **One runtime, no schema-driven magic.** There's a single concrete ops implementation, `ConcreteOps<M>`. No trait-object "second runtime," no reflection layer. Macros emit obvious code — `cargo expand` shows the whole picture.
- **Refusal-first.** RBAC checks deny *before* the query runs; list-page SQL validates every column name against `Model::COLUMNS` (allowlist, not string-building). Unauthorized actors are denied by default.
- **Audit-by-default.** Every authority mutation writes a typed audit event with a correlation ID (a UUID v7 threaded through the middleware chain), so framework and project audit rows join on one column.
- **No build step.** Hand-written CSS/JS, templates baked in via `include_str!`, single binary. No Tailwind/PostCSS/bundler.
- **No plaintext at rest.** Argon2id for passwords, SHA-256 for session/reset tokens. Session invalidation has a single writer by contract.
- **Postgres only, on purpose.** It's a `sqlx` shim over Postgres semantics, not a database abstraction.

### Non-goals (so you know if it's not for you)

Multi-database support, a general-purpose ORM, schema-driven metadata at runtime, and a frontend build step are all explicitly out of scope. If a feature wants those, it belongs in a future layer, not this one.

### Where it honestly is

Early. It's about four months of work, currently `v0.31.0`, **pre-1.0**, and the API will still change. The admin, the auth system, migrations, and the CLI scaffolding all work today; it is not battle-tested in production and I wouldn't claim otherwise.

I'd genuinely value criticism — on the design decisions above, the security model, or anything that looks wrong.

- Repo: https://github.com/abdulwahed-sweden/rustio-admin
- Quick Start: https://github.com/abdulwahed-sweden/rustio-admin/blob/main/docs/quickstart-translation-agency.md

(It's open source; I recently opened GitHub Sponsors, but the thing I actually want from this post is feedback and issues.)

---

## Mastodon (≤500 chars)

> RustIO is now on GitHub Sponsors 🦀
>
> It's Django Admin for Rust — but authentication, permissions, account recovery, and audit are built into the core, not bolted on. Postgres-first, refusal-first (unauthorized denied by default), audit-by-default.
>
> Early and open, pre-1.0. Sponsoring now is backing focused open Rust infrastructure at the stage it matters.
>
> Built in Rust. Governed by engineers. Guarded by design.
>
> github.com/sponsors/abdulwahed-sweden
>
> #rustlang #opensource

---

## Bluesky (≤300 chars)

**Sponsors-led (for the launch):**

> RustIO is now on GitHub Sponsors 🦀
>
> Django Admin for Rust — with auth, permissions, recovery & audit built into the core, not bolted on. Postgres-first, refusal-first, audit-by-default.
>
> Early & open, pre-1.0.
>
> github.com/sponsors/abdulwahed-sweden

**Framework-led (evergreen):**

> RustIO: an admin framework for Rust where authentication, permissions, recovery & audit live in the core — not bolted on. Postgres-first, refusal-first, audit-by-default. No build step, no magic.
>
> Early & open, pre-1.0. Feedback welcome:
>
> github.com/abdulwahed-sweden/rustio-admin

---

## One-liner (anywhere)

> RustIO — Django Admin for Rust, with auth, permissions, recovery & audit in the core — is now on GitHub Sponsors. Early, open, pre-1.0. github.com/sponsors/abdulwahed-sweden
