# Documentation

Every human-readable document for `rustio-admin` that does not live
in the workspace `README.md`, `CLAUDE.md`, `CHANGELOG.md`, or
`ROADMAP.md` at the repository root.

Three tiers — **guides** for hands-on use, **reference** for the
public surface, and **design** for the doctrine that pull requests
are reviewed against. Everything else lives in **archive**.

For the why behind the project, start with
[`VISION.md`](VISION.md) — what RustIO is for and what it
deliberately refuses to become.

## Guides

User-facing entry points. Read these first.

- [`getting-started.md`](getting-started.md) — from an empty
  directory to a running admin with your own model: install, create a
  project, add a model, sign in.
- [`tutorial-rustio-draft.md`](tutorial-rustio-draft.md) — beginner
  walkthrough: describe an app in one sentence, let **rustio-draft** draft a
  `schema.json`, then `import → plan → commit` it into a running admin.
- [`architecture.md`](architecture.md) — module map and how the
  library, macros, and CLI crates compose.
- [`modeladmin.md`](modeladmin.md) — authoring guide for
  `ModelAdmin` (list pages, search, filters, ordering, bulk actions).
- [`cli.md`](cli.md) — the `rustio-admin` command-line surface:
  scaffolding, migrations, users/groups/permissions, audit, theme.
- [`memory.md`](memory.md) — the `rustio-admin memory` project-memory
  feature that drives `CLOUD.md` and the AI-assistant policy.

## Reference

- [`public-api.md`](public-api.md) — enumerated `pub` surface across
  the workspace. Descriptive, not normative. Annotation does not
  itself guarantee SemVer stability before 1.0.

## Design

Doctrine for security-sensitive subsystems and the visual identity.
Pull requests touching these areas are reviewed against the doctrine,
not just the diff.

### Visual identity

- [`design/DESIGN_DOCTRINE.md`](design/DESIGN_DOCTRINE.md) — token
  philosophy, surface ladder, typography conventions, and the eleven
  numbered principles that govern every CSS change.
- [`design/DESIGN_SYSTEM.md`](design/DESIGN_SYSTEM.md) — token
  ownership, accent palette, authority vocabulary.
- [`design/DESIGN_CHROME.md`](design/DESIGN_CHROME.md) — operational
  chrome conventions (topbar, sidebar, footer, environment badge).
- [`design/DESIGN_THEME.md`](design/DESIGN_THEME.md) — the build-time
  `rio-theme` engine: brand colors → a WCAG-safe `tokens.css`, and the
  `RUSTIO_TOKENS_CSS` runtime override.
- [`design/VISUAL-CONTRACT.md`](design/VISUAL-CONTRACT.md) — the
  canonical Visual Contract (v2.1): the rules a rendered admin must
  satisfy, with reference screenshots as tie-breakers.
- [`design/TOKENS-EMIT-SPEC.md`](design/TOKENS-EMIT-SPEC.md) — the
  emission contract any generator must honour when producing a
  `tokens.css` for RustIO Admin.
- [`design/REMEDIATION_V2.md`](design/REMEDIATION_V2.md) — the
  conformance plan that brought the stylesheet in line with Visual
  Contract v2.0.

### Authority and security

- [`design/DESIGN_SESSIONS.md`](design/DESIGN_SESSIONS.md) — session
  lifecycle and the single-writer invalidation contract (Doctrine 22).
- [`design/DESIGN_AUDIT.md`](design/DESIGN_AUDIT.md) — audit-row
  contract, correlation-ID threading, required middleware ordering.
- [`design/DESIGN_RECOVERY.md`](design/DESIGN_RECOVERY.md) —
  self-service password recovery (R1).
- [`design/DESIGN_R2_ORGANISATIONAL.md`](design/DESIGN_R2_ORGANISATIONAL.md)
  — admin-driven recovery, account lockout, re-auth wall, forced
  rotation (R2).
- [`design/DESIGN_R3_MFA.md`](design/DESIGN_R3_MFA.md) — TOTP
  multi-factor authentication with single-use backup codes (R3).
- [`design/DESIGN_R4_EMERGENCY.md`](design/DESIGN_R4_EMERGENCY.md)
  — CLI emergency-recovery primitives (R4).
- [`design/DESIGN_AI_ASSISTANT.md`](design/DESIGN_AI_ASSISTANT.md) —
  permissions / approval / audit layer over external AI coding
  assistants: the `.rustio/ai.toml` policy, the `rustio ai` proposal
  lifecycle, and opt-in `rustio_admin_actions` mirroring (0.23.0).

### Framework conventions

- [`design/DESIGN_EMAIL.md`](design/DESIGN_EMAIL.md) — every email
  the framework dispatches on a project's behalf (recovery today,
  MFA enrolment and security alerts tomorrow).
- [`design/DESIGN_BUILDER.md`](design/DESIGN_BUILDER.md) — the
  deterministic project compiler (`rustio-admin builder new`, `rustio-admin add`,
  `rustio-admin commit`), its thirteen numbered invariants, and the five
  CI-enforced grep proofs. (`rustio-admin new` is now the friendly scaffold
  alias; see `design/DESIGN_ONBOARDING.md` §10.)

## Archive

Historical artefacts kept for institutional memory. Not load-bearing;
not linked from `CLAUDE.md`, the PR template, or source code.

- [`archive/RUSTIO_STRATEGY.md`](archive/RUSTIO_STRATEGY.md) —
  consolidation roadmap drafted pre-1.0; partially obsoleted by the
  v0.13–v0.15 work.
- [`archive/ROADMAP_BUILDER.md`](archive/ROADMAP_BUILDER.md) —
  long-horizon Builder + Advisory AI vision. Explicitly *not yet*
  doctrine; sibling to the workspace `ROADMAP.md`.
- [`archive/PLAN_VISUAL_v2.md`](archive/PLAN_VISUAL_v2.md) — plan
  for the v0.15.0 visual overhaul. The doctrine landed as Principles
  9–11 in `design/DESIGN_DOCTRINE.md`.
- [`archive/REVIEW_BUILDER_DOCTRINE.md`](archive/REVIEW_BUILDER_DOCTRINE.md)
  — one-time critical review of `DESIGN_BUILDER.md` (38 findings).
  Resolved findings tracked in the doctrine; outstanding ones
  annotated in source comments.
- [`archive/VALIDATION_BUILDER_MVP.md`](archive/VALIDATION_BUILDER_MVP.md)
  — manual verification record for the Builder MVP (v0.14.0).
- [`archive/VISIBILITY_AUDIT.md`](archive/VISIBILITY_AUDIT.md) —
  0.8.0 framework-surface audit that drove the 0.8.1 visibility
  recovery.
- [`archive/STRATEGIC_RESET_PLAN.md`](archive/STRATEGIC_RESET_PLAN.md)
  — the 2026-05-06 strategic reset record that shipped as v0.1.0.
- [`archive/APIS_AND_DOCS_PLAN.md`](archive/APIS_AND_DOCS_PLAN.md)
  — pre-0.2 APIs / docs page design exercise; never shipped in this
  form.
- [`archive/REDESIGN_AUDIT.md`](archive/REDESIGN_AUDIT.md) —
  inspection-only audit of the admin presentation surface; the
  Phase 1 presentation-only polish it proposed has since shipped.
