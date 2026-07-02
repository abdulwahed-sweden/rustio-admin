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

## Why open-core here

The core's value is that it stays legible, auditable, and owned by the people
running it. Putting the core behind a paywall would break that. Open-core keeps
the engine free and inspectable, funds its maintenance through the commercial
edges, and gives companies a supported path when they need one — without ever
taking the open core away.

Support the open side through [sponsorship](./sponsorship.md).
