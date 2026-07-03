# Verticals

RustIO is a **foundation for vertical business applications** — real operational
systems built on the `rustio-admin` engine, not generic CRUD demos.

## The foundation problem is universal

Most operational software across critical sectors is built on stacks that were
never designed to become heavy-duty engines. As data grows, workflows expand,
and operational pressure rises, the same failure repeats: slower performance,
climbing infrastructure cost, fragile changes, and constant firefighting. The
problem is rarely the team — it is a weak foundation, rebuilt from scratch again
and again.

RustIO exists to be the reliable operational foundation these sectors can build
on and evolve for years. The domains that hit this problem hardest include:

- **Healthcare systems** — patients, records, scheduling, audit-heavy workflows.
- **Education platforms** — students, staff, enrolment, records.
- **Housing** — tenants, queues, applications, operations at scale.
- **Logistics** — routes, pickups, dispatch, tracking.
- **Interpretation dispatch** — assignment, scheduling, and dispatch workflows.
- **Clinic management** — appointments, staff, patient administration.
- **Field operations / service** — jobs, technicians, scheduling, status.

The common thread: authority, audit, and correctness matter, the system must
stay fast as it grows, and a rebuild is expensive. That is exactly what RustIO is
built to underpin.

## What ships today vs. what's planned

Below is the honest status. Open examples live in the repository; industry-
specific packs are the planned commercial side (see
[`commercial-model.md`](./commercial-model.md)).

### Existing — open examples

| Vertical | Status | Location |
|----------|--------|----------|
| Clinic admin | ✅ Example ships in the repo | [`examples/clinic`](../examples/clinic/) |
| Shop / e-commerce admin | ✅ Example ships in the repo (also a standalone project) | [`examples/shop`](../examples/shop/) · [`abdulwahed-sweden/shop`](https://github.com/abdulwahed-sweden/shop) |

### Planned — vertical packs

On the roadmap and/or reserved for the commercial `rustio-pro` line. These are
**not implemented yet** and are listed here as direction only.

| Vertical | Status | Notes |
|----------|--------|-------|
| Waste logistics | 🟡 Planned | Routes, pickups, dispatch, audit trail. |
| Interpretation dispatch | 🟡 Planned | Assignment, scheduling, and dispatch workflows. |
| School admin | 🟡 Planned | Students, staff, enrolment, records. |
| Booking systems | 🟡 Planned | Resources, availability, reservations. |
| Service operations | 🟡 Planned | Jobs, technicians, scheduling, status. |
| Healthcare / clinic | 🟡 Planned pack | Beyond the `clinic` example — a fuller sector pack. |

## Sponsoring a vertical

A vertical can be funded directly — as a one-time **Vertical Pack Sponsor** (see
[`SPONSORS.md`](../SPONSORS.md)) or as company-backed roadmap work. Sponsored
reference verticals land in the open core; industry-specific packs are part of
the commercial layer. Reach out first so the scope is clear.
