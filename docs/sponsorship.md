# Sponsorship

**Fund open Rust infrastructure for safer business systems.**

This document explains what RustIO is asking support *for*, how that support is
used, and the channels through which it can be given. The tier list lives in
[`SPONSORS.md`](../SPONSORS.md); the open-core/commercial split lives in
[`commercial-model.md`](./commercial-model.md).

RustIO is, and will remain, MIT-licensed. Sponsorship funds the open-source work
— it does not buy the core behind a paywall.

## What RustIO is

`rustio-admin` is a **Rust-first business-system engine** — a schema-driven
operational foundation for building serious business software on PostgreSQL. It
generates admin surfaces from plain Rust structs and treats the parts real
systems run on — authentication, sessions, recovery, role-based access, and a
complete audit trail — as one designed system rather than bolted-on parts.

It is deliberately narrow: Postgres-only, single-binary, no build step. It is
**not** a general-purpose web framework and does not try to be. It is the
foundation layer *beneath* operational software — a base for vertical business
applications across sectors (clinic and shop admins ship as examples today, with
logistics, dispatch, booking, and school systems on the roadmap).

## Why sponsor

RustIO was born from a real market problem. A fast-growing housing platform in
the Swedish market — serving well over a million customers — began to crack
under its own success, because the foundation it was built on was never designed
to be a heavy-duty operational engine. That failure is not unique: across the
industry, teams rebuild the same operational foundation and hit the same wall.

Most operational software is either hand-rolled from scratch or rented as an
expensive SaaS stack. RustIO is a third option: an open, auditable engine you run
yourself, cheaper to operate and easier to keep control of as it grows.
Sponsoring it is backing that infrastructure — not a personal donation — and it
reduces the enormous, repeated waste of companies rebuilding foundations that
were never built to last.

It is worth supporting if you:

- build internal tools, admin panels, or workflow/dispatch software in Rust;
- want safe-by-construction, PostgreSQL-first business systems;
- are a company or team scaling past a first-generation stack, or tired of SaaS lock-in;
- care about performance, auditability, and EU/open-source digital sovereignty.

## What sponsorship funds

Sponsorship funds the **open-source** work, in roughly the order it appears on
the public [`ROADMAP.md`](../ROADMAP.md). A funding-oriented view of that roadmap:

**Now — stabilize the core**
- Harden the `rustio-admin` core and its authority/session/audit surfaces.
- Improve documentation and onboarding.
- Improve schema-driven admin generation.
- Add realistic examples.

**Next — reference verticals**
- Additional example verticals beyond `clinic` and `shop`.
- Better first-run and onboarding experience.

**Later — commercial edges (open-core)**
- The commercial layer (hosted option, vertical packs, private support,
  integrations) is developed under the separate `rustio-pro` line, so the core
  stays open. See [`commercial-model.md`](./commercial-model.md).

This is a funding view, not a second roadmap — the authoritative plan is
[`ROADMAP.md`](../ROADMAP.md).

## Funding channels

**Active**

- **GitHub Sponsors** — the primary channel, monthly and one-time.
  Tiers: [`SPONSORS.md`](../SPONSORS.md). Configured via
  `.github/FUNDING.yml`.

**Optional / planned** (not active yet — no live links are published until they
are)

- Open Collective — for transparent, itemized budgets if the project grows a
  shared treasury.
- Polar.sh — for issue/feature funding tied to the repo.
- Liberapay — recurring, fee-light alternative.
- **Feature bounties / issue sponsorship** — funding a specific issue or roadmap
  item directly.
- **Swedish/EU innovation support** — programs such as Vinnova or Almi become
  relevant *later*, only if RustIO becomes company-backed. Noted as a direction,
  not a current source.

## Company and roadmap sponsorship

Companies that depend on a specific capability can fund it directly — a roadmap
feature, a reference vertical, or a documentation sprint. Reach out first so the
scope is clear and the work lands in the open core where it belongs.

---

## Outreach templates

Ready-to-use text for contacting developers, companies, and technical supporters.

### Short outreach message

> Hi,
>
> I'm building RustIO, a Rust-first admin and business systems engine focused on
> safe, fast, PostgreSQL-backed operational software.
>
> The goal is to make it easier to build serious internal tools, admin panels,
> dispatch systems, booking workflows, and audit-friendly business applications
> in Rust — without depending on heavy SaaS stacks.
>
> I'm currently looking for early sponsors, technical supporters, or companies
> interested in funding specific parts of the roadmap.
>
> Sponsorship can support public open-source work, documentation, examples, and
> real vertical templates such as logistics, booking, clinics, schools, and
> interpretation dispatch.
>
> Would you be open to taking a quick look?

### Short LinkedIn post

> I'm building RustIO: a Rust-first admin and business systems engine.
>
> The goal is simple: build serious operational software in Rust — admin panels,
> workflows, dispatch engines, booking systems, audit trails — without the usual
> SaaS bloat.
>
> Safe by construction. PostgreSQL-first. Zero unsafe. Built for real business
> systems, not toy demos.
>
> I'm now looking for early sponsors, technical backers, and companies interested
> in supporting the roadmap.
>
> If you believe Rust belongs in business software, not only low-level systems,
> I'd love to connect.
