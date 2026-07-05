# Why refusal-first?

If you have used a typical web framework, you are trained to expect a system
that **opens everything by default** and trusts you to lock it down later. You
add authentication when you need it, wire up permissions when a feature demands
them, and bolt on an audit log after the first incident.

RustIO does the opposite. It is **default-deny**: an action that isn't
explicitly authorized is refused, and every authority change is recorded whether
or not you asked for it. The first time RustIO says *no* to something you
expected to work, it can feel like the framework is fighting you.

It isn't. This page is why that inversion exists — and why it makes you faster,
not slower.

## Default-allow is where breaches live

The failures that sink real companies are rarely exotic. They are the
**forgotten restriction**: the endpoint someone never got around to gating, the
role check that was copy-pasted and quietly wrong, the admin action that shipped
without an owner. In a default-allow system, every one of those omissions is
*open* until someone notices. The gap **is** the default.

Refusal-first removes the whole category. If you didn't grant it, it's closed. A
forgotten permission **fails safe** — the request is refused — instead of
failing open. You find the missing grant the first time you exercise the
feature, in development, not in a post-incident review.

## What it concretely means

- **Unauthorized requests are rejected before the database.** The authority gate
  checks the session, resolves the identity and role, and evaluates the
  per-model permission — an unauthorized request gets a `403` before your handler
  or Postgres is ever touched (see the [request
  lifecycle](./architecture.md#request-lifecycle)).
- **Permissions are explicit and per-model.** Every model you register seeds
  `view_` / `add_` / `change_` / `delete_` permissions at boot. Nothing is
  granted by accident.
- **The record is not optional.** Every authority change writes a typed row to
  `rustio_admin_actions` — audit-by-default. You can't forget to log, because you
  never wrote the logging.

## Refusal-first is leverage, not a cage

Strict defaults are the reason you can move fast. The dangerous parts — auth,
sessions, recovery, permission checks, the audit trail — are already assembled
and already safe, so you are not reconstructing them (correctly, under pressure)
for every project. The guardrails never sleep; you build on top of them.

And the "no" is easy to turn into "yes" — **explicitly**:

- **Roles** — a five-tier ladder (User · Staff · Supervisor · Administrator ·
  Developer); `rustio-admin user create --role …` sets one.
- **Grants** — `rustio-admin perm grant-user …` / `perm grant-group …` open a
  specific capability to a specific user or group, and the grant is itself
  recorded.

If RustIO refuses something you expected to work, the fix is almost always a
**missing explicit grant** — not a workaround.

## Where your freedom actually is

Refusal-first governs **authority** — *who may act*. It says nothing about how
you model your domain, what workflows you build, or what rules you enforce on
your own data. Those are entirely yours. This is the **"two kinds of no"**:

> The framework refuses the **unauthorized actor**. You refuse the **wrong
> data** — "this translator can't do Arabic" is your validation, not RustIO's.

So the strictness is narrow and deliberate: it hardens the parts that *must* be
trustworthy — identity, permission, the record — and stays out of the way of the
parts that are your business.

## The honest edge

Refusal-first is default-deny on *authority*. It is not a web-application
firewall, it does not write your domain validation, and it is not a guarantee of
security on its own — see [What RustIO stops out of the box](./security.md),
including the "what this does *not* do" section. It removes a class of footguns
so the security you add sits on solid ground.

## If you remember nothing else

- Default-allow leaves the forgotten restriction **open**; default-deny **fails
  safe**.
- If it's not granted, it's closed — and granting is **explicit** (`perm
  grant-user` / roles) and recorded.
- The strictness is about **authority only**; your domain is yours.
- You move faster because the dangerous defaults are already safe.

The deeper *why* — keeping a system answerable across its whole life — lives in
[`VISION.md`](./VISION.md).
