# The RustIO Admin Manifesto

**Built in Rust. Governed by engineers. Guarded by design.**

RustIO Admin is not a template system, a low-code shortcut, or an AI-driven
black box.

It is an **early but architecturally serious foundation** for building
administrative systems in Rust — where authority, permissions, recovery, and
audit are treated as **core infrastructure, not features bolted on later**.

## You own the business logic. The framework carries the authority layer.

RustIO Admin does not decide how your company works. It does not force your
domain into pre-built templates, and it does not hide your system behind magic.
You define the models, the workflows, the rules, and the boundaries. **The code
stays yours.**

What the framework carries — so you don't reassemble it from separate crates —
is the heavy, security-sensitive layer underneath: identity and sessions,
role-based permissions, password recovery, and a complete audit trail, designed
as **one system** and governed by checked-in contract documents.

## AI is governed, not embedded.

RustIO Admin ships **no model and no planner**. When you choose to use an
external AI assistant, it is deliberately restricted: it may propose a schema,
suggest structure, or accelerate a first draft — but it **does not decide, does
not grant permissions, and does not execute business logic**. Every proposal is
a draft a human accepts, edits, or rejects.

The engineer remains the final authority — and the assistant is given *fewer*
powers than a developer, never more. It is a guest in a governed house; it
never becomes the house.

## Refusal-first. Audit-by-default.

The system is designed to protect itself:

- **Refusal-first** — an action that isn't explicitly authorized is rejected,
  not quietly allowed.
- **Audit-by-default** — every change to authority, identity, access, or
  recovery leaves a typed, correlated trace: who acted, when, and on what basis.
- **Append-only history** — the record grows; it is superseded, never rewritten.
- **Code is authoritative** — the recorded *why* sits beside the code, never
  above it. On any conflict, the code wins.

These are not obstacles. They are the guardrails that let the framework carry
the operational burden while you build.

## What it is for

This is the kind of foundation real operations need — interpreter dispatch,
medical and legal workflows, logistics and waste services, staff operations,
invoicing, and other regulated business processes — where roles, availability,
compliance, and an accurate history all matter at once.

RustIO gives you the **structure** to build these systems with discipline. It
does **not** promise to write your business for you. It gives you a Rust-based
authority core, a strict design doctrine, and a controlled foundation — so you
can build serious systems without surrendering control.

## What we will not claim

RustIO is young — pre-1.0, on the alpha track, roughly four months of focused
work. We claim only what the pipeline actually enforces. We would rather
understate a young foundation than sell a guarantee we don't keep: **a project
built to keep systems answerable cannot begin by overstating itself.**

---

**Built in Rust. Governed by engineers. Guarded by design.**

> The internal north star behind this stance lives in
> [`docs/VISION.md`](./docs/VISION.md); the security-sensitive behaviour it
> describes is governed by the contracts under [`docs/design/`](./docs/design/).
