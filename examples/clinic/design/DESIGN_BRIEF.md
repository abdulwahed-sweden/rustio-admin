---
artifact: DESIGN_BRIEF
layer: why
status: active
source: reverse-engineered from examples/clinic (read-only analysis — no redesign)
updated: 2026-06-07
---

# Design Brief — WHY (Clinic)

> Extracted from the actual codebase for Claude Design. This is *what the system
> is*, not a proposal. Nothing here redesigns the clinic; it reduces it to a
> design-oriented understanding so a design can be reasoned about without reading
> Rust.

## Business Context

- **Domain:** the back-office admin for an **outpatient clinic** — managing the
  people it treats, the visits it books, the measurements it records, and the
  money it bills. It is an internal operational tool, not a patient-facing portal.
- **What the software is:** a `rustio-admin` admin panel over a Postgres database,
  organised as three business capabilities — **patients**, **scheduling**,
  **billing** — plus shared foundation and a server crate.
- **Primary operators (who logs in):** clinic staff — front-desk/reception
  (register patients, book visits), clinical staff (record vitals, work the day's
  appointments), billing staff (raise and chase invoices), and an administrator
  (accounts, permissions, oversight). The clinic does **not** define its own roles;
  it maps these people onto the framework's RBAC ladder (see User Roles).
- **Hard constraints (inherited, non-negotiable):** Postgres-only; security-first
  (5-tier RBAC, per-model permissions, audit-by-default with correlation IDs,
  Argon2id/CSRF/rate-limit); **no front-end build step** (hand-written CSS, theming
  via `--rio-*` tokens); **light-only** (no dark mode); append-only migrations.
- **Brand voice (as built):** calm, clinical, trustworthy — an emerald-green
  accent (`#059669`) on a near-white slate canvas. Restrained, not playful.

## Design Intent (as evidenced by the build)

- **It should feel:** fast to look something up and quietly authoritative —
  the screen an operator lives in across a shift without friction.
- **The success moment it optimises for:** *find a patient and act* — the patient
  list is the one model with full-text search; everything else hangs off a patient.
- **It must never feel:** like data is at risk. This is PHI/PII and money; the UI
  must make destructive and financial actions legible and deliberate.
- **Non-goals (deliberately absent today — do not assume them):** no reporting/
  analytics, no patient portal, no external search engine, no dark mode, no
  CSS-framework coupling. A capability appears only when the business has it.

## User Roles

The clinic relies entirely on the framework's **5-tier role ladder** (highest to
lowest authority): **Developer → Administrator → Supervisor → Staff → User**.
Access is refined by **groups + per-model permissions** (the server calls
`seed_permissions`, so each model has view/add/change/delete perms). Every
authority mutation is written to an **audit trail** (`/admin/history`) with a
correlation ID. There are no clinic-specific role names in the code — design for
"a signed-in staff member with a subset of model permissions," plus an
administrator who manages users/groups.

## Visual & Accessibility Floor

- **Palette (now owned by `rustio.design.toml` `[colors]`, served via
  `RUSTIO_TOKENS_CSS`; was `static/tokens.css`):** accent `#059669` (emerald-600),
  accent-hover `#047857` (emerald-700), bg `#f8fafc` (slate-50), surface `#ffffff`,
  text `#0f172a` (slate-900), muted `#475569` (slate-600), border `#e2e8f0`.
- **Accessibility:** WCAG AA is the floor; the framework's rio-theme is the
  contrast authority. Note: emerald-600 on white is borderline for *text* — the
  build already reserves the darker emerald-700 for stronger contrast. Light-only,
  ≥16px readable type doctrine, keyboard-navigable, semantic status colours.
