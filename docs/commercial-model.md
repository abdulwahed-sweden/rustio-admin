# Commercial model

RustIO follows an **open-core** model. The core is open and MIT-licensed and
stays that way. A separate commercial layer funds sustained development without
compromising the core's guarantees.

This document describes the *direction*. Most of the commercial side is planned,
not shipped — it is written here so the intent is clear, not to imply it exists
today.

## The split

### Open source — `rustio-admin` (MIT)

The engine and everything needed to build real systems with it:

- the core admin engine (authority, sessions, recovery, RBAC, audit);
- the schema/model format and `#[derive(RustioAdmin)]` admin generation;
- the CRUD/admin surface and the CLI;
- documentation, guides, and onboarding;
- example and reference verticals (`examples/clinic`, `examples/shop`).

The open core is complete enough to build and run serious admin systems on its
own. It is never gated behind the commercial layer.

### Commercial — the `rustio-pro` line (planned)

Advanced and operational capabilities live in a **separate `rustio-pro-*` family
of crates, never inside `rustio-admin` itself**. This is an existing
architectural rule in the project (see [`VISION.md`](./VISION.md) and
[`ROADMAP.md`](../ROADMAP.md)), not a new invention: features that would widen or
complicate the doctrine-governed core are kept out of it by design.

Planned commercial offerings, developed under that line:

- a hosted / managed version;
- vertical templates and industry packs (see [`verticals.md`](./verticals.md));
- private support and company integrations;
- premium templates;
- paid setup and onboarding.

> Status: the `rustio-pro` line is a reserved direction. It is documented here
> and in the roadmap; it is not a shipped product today.

## Vertical packs

Reference verticals show what the engine is for. Some ship as open examples;
industry-specific packs are the planned commercial side. Honest status:

**Existing (open examples)**
- Clinic admin — [`examples/clinic`](../examples/clinic/)
- Shop / e-commerce admin — [`examples/shop`](../examples/shop/)

**Planned (commercial packs)**
- Waste logistics
- Interpretation dispatch
- School admin
- Booking systems
- Other service-operations packs

Full list and status: [`verticals.md`](./verticals.md).

## Companion tool — `rustio-draft` (open-core split)

`rustio-draft` is the setup-time companion: a natural-language brief →
`schema.json` for `rustio-admin` (it may call Claude to draft; RustIO Admin
applies the result deterministically via `import` → `plan` → `commit`). It lives
in its **own public, MIT-licensed repository**
([abdulwahed-sweden/rustio-draft](https://github.com/abdulwahed-sweden/rustio-draft),
`v0.1.0`) and is **not** published to crates.io.

The same open-core discipline applies: the free core stays genuinely useful on
its own — a crippled free tier would kill the adoption a paid tier depends on.
Paid value is **scale, teams, convenience, and curated content**, never
"unlock the basics."

**Free core (public, MIT — `rustio-draft`)**

- `new` (single brief → `schema.json`), bring-your-own `ANTHROPIC_API_KEY`.
- The full safety pipeline: `generate → validate → diff → protect → human review`.
- `doctor`, basic `refine`, and a basic local Studio (`serve`).

**Planned commercial (`rustio-pro-draft`, private / hosted)**

- **Studio Pro** — multi-model visual designer, saved projects, themes, richer editing.
- **Managed / no-key mode** — a hosted endpoint so users don't need their own LLM key (metered).
- **Vertical template packs** — curated industry briefs (clinic, booking, logistics, school…), aligned with the vertical packs above.
- **Team / enterprise** — draft audit log, policy (allowed field types, naming conventions), private template libraries, SSO, CI/batch drafting.
- Priority support / SLAs.

**Mechanism.** Keep `rustio-draft` as the free core. Put every paid feature in a
**separate private repo** — `rustio-pro-draft`, under the reserved `rustio-pro-*`
family (never inside the open core) — and/or a **hosted Studio / managed API**
that sells access and convenience rather than code. A hosted service is the
easier thing to protect, since MIT code can always be forked.

> **Honest caveat.** MIT is a one-way door for shipped versions: code already
> released under MIT (including `rustio-draft` `v0.1.0`) stays free and forkable.
> The clean model is therefore *free core stays MIT/public; every paid feature
> lives only in the private pro repo or the hosted service* — and the free/paid
> line is decided up front, not retro-restricted later.
>
> Status: a sketch / reserved direction. Nothing paid ships today.

## Why open-core here

The core's value is that it stays legible, auditable, and owned by the people
running it. Putting the core behind a paywall would break that. Open-core keeps
the engine free and inspectable, funds its maintenance through the commercial
edges, and gives companies a supported path when they need one — without ever
taking the open core away.

Support the open side through [sponsorship](./sponsorship.md).
