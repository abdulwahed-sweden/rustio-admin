# What RustIO stops out of the box

RustIO carries the authority layer so a whole class of mistakes and attacks is
refused **by default** — before you write a line of security code. Everything
below is a real, shipping behaviour; each row names the module you can read to
verify it.

These are **defaults, not a guarantee.** They remove common footguns; they do
not make a system unbreakable, and they do not replace the domain rules you
write (see [What this does *not* do](#what-this-does-not-do)).

## The defaults

| If you — or an attacker — do this | RustIO's default response | Where it lives |
|---|---|---|
| Submit a form from another origin (**CSRF**) | Double-submit cookie: the `X-CSRF-Token` header (or `_csrf` field) must match the `rustio_csrf` cookie; a mismatch is refused with **403** | `middleware::csrf_protect` |
| Attempt a mutation **without permission** | Refusal-first: role + per-model permission is checked and an unauthorized request gets **403 before it reaches Postgres** | `auth::permissions` + route guards |
| Pass an **injected column name** in a filter or sort | Column names are validated against `Model::COLUMNS`; anything not on that allowlist is rejected — a query can only ever name real columns | `admin::ops::ConcreteOps` |
| **Flood** an endpoint (brute-force / abuse) | Per-IP **token-bucket** rate limit; once the bucket is empty the request is rejected | `middleware::rate_limit` |
| Frame the page (**clickjacking**) or rely on **MIME-sniffing** | `X-Frame-Options` and `X-Content-Type-Options: nosniff` are set on every response | `middleware::security_headers` |
| **Dump the database** | Passwords are **Argon2id**-hashed; session and reset tokens are stored only as a **SHA-256** hash — no plaintext credential is ever at rest | `auth::sessions`, `auth::users` |
| Probe login / recovery to **enumerate accounts** | A **uniform outward response** on every outcome; no audit row is written for an unknown email (so row counts can't enumerate); reset tokens are 256-bit with a 1-hour TTL | `DESIGN_RECOVERY` §4.5 |
| Keep using an **old session** after a password change / revoke | Every revocation goes through **one writer** (`invalidate_sessions` — Doctrine 22); `revoked_at` can be set from nowhere else | `auth::sessions` |
| Leak a **secret into a log or audit payload** | Audit `before` / `after` payloads route through redaction; `password_hash`, `mfa_secret`, and equivalents never appear | `admin::audit`, `DESIGN_AUDIT` |
| Change authority and **record nothing** | Audit-by-default: every authority mutation writes a typed row to `rustio_admin_actions` with a per-request correlation id — you wrote no logging code | `admin::audit` |

## What this does *not* do

Being honest about the edges is part of the point — a framework built to keep
systems answerable can't oversell its own guarantees:

- **It is not a WAF or an edge firewall.** It defends the application; network-,
  DDoS-, and TLS-termination concerns belong at your proxy or platform.
- **CSP and HSTS are not baked in.** RustIO ships the two headers above; add a
  Content-Security-Policy and HSTS at your reverse proxy (or your own
  middleware) if your threat model needs them.
- **It does not write your domain validation.** "This translator can't do
  Arabic" is *your* refusal, not the framework's — the framework refuses the
  *unauthorized actor*, you refuse the *wrong data*. This is the "two kinds of
  no" from the [request lifecycle](./architecture.md#request-lifecycle) and the
  [translation-agency Quick Start](./quickstart-translation-agency.md).
- **Rate limiting is in-memory, per process.** Good against casual abuse and
  brute-force; a multi-instance deployment behind a load balancer should still
  rate-limit at the edge for a global view.

Security is layered. RustIO's job is to make identity, permission, and the
record trustworthy by default — so the layers you add sit on solid ground.

## Read the contracts

Each of these behaviours is governed by a checked-in design contract, reviewed
alongside the code:

- [`DESIGN_SESSIONS.md`](./design/DESIGN_SESSIONS.md) — sessions, Doctrine 22, trust escalation.
- [`DESIGN_RECOVERY.md`](./design/DESIGN_RECOVERY.md) — uniform responses, throttling, token model.
- [`DESIGN_PERMISSIONS.md`](./design/DESIGN_PERMISSIONS.md) — the role ladder and per-model permissions.
- [`DESIGN_AUDIT.md`](./design/DESIGN_AUDIT.md) — the audit chain, redaction, and middleware ordering.
