# RustIO — Architectural Strategy

*A principal-architect brief for the consolidation phase, 0.9.0 → 1.0.0.*

---

## 1. Executive Summary

RustIO has passed the "does it work" phase. Through 0.8.2 the framework has
cleared every credibility threshold a serious admin platform needs: a
coherent design doctrine, a token-driven CSS architecture, audit-grade R1–R4
security, a working CLI, a self-hosted font stack, Arabic typography, dark
mode without flash, and a clean compile-time delivery model. **The framework
is good. It is not yet legible.**

Legibility is the work of the next two minor releases. 0.9.x and 0.10.x must
be devoted entirely to consolidating what already ships — public API surface,
contributor onboarding, AI-agent guidance, visual identity, and a credible
example portfolio — without adding new framework features. **1.0.0 is a
SemVer commitment, not a feature milestone.** It says: "the public API you
read today is the public API you read in five years."

Three concurrent workstreams drive this phase:

- **A. Structure & API surface.** Declare what's public, what's internal,
  what's archived. Migrate the doc tree into `docs/`. Cut the weak example.
  Lock the crate boundaries.
- **B. AI-native experience.** Make framework invariants and project-local
  context discoverable to coding agents at scaffold time. Generate,
  regenerate, validate — never decorate.
- **C. Visual identity.** Guarded theming via `rustio.theme.json`, a calm
  operational home page that is *not* a landing page, and two canonical
  examples (fleet-ops + library-circulation) that signal the framework's
  domain weight.

The strategy is **anti-rewrite**. Every change is an additive consolidation
of work already shipped. Nothing in this document proposes touching the
cascade, the token system, the audit machinery, the R4 recovery primitives,
or the macro surface.

---

## 2. Long-Term Vision

In five years, RustIO should be the answer to a specific question:

> *"I need to build an internal tool that several engineers will run for a
> decade. It must outlive every JavaScript framework I would have chosen. It
> must be obvious to onboard onto. It must not require a CDN. It must read
> the same in a six-pane operator screen at 2 AM as it does on a designer's
> Retina monitor at noon. It must not embarrass me when the security auditor
> opens it."*

Three constraints define the framework forever:

1. **One binary, no runtime toolchain.** Fonts, CSS, JS, migrations,
   scaffolds — all baked. There is never a `npm install` step. There is
   never a CDN dependency. There is never a build server outside `cargo`.
2. **Operational over marketing.** Every screen is built to be lived in, not
   shown off. Calm before flashy. Borders before shadows. Layering before
   glow.
3. **Cascade-stable.** A 2026 admin and a 2030 admin should look like
   cousins, not strangers. Token names, surface ladder, accent reservation
   rules, and component selectors are public API.

Five-year RustIO is *boring* in the best sense: predictable, maintainable,
and indistinguishable across versions for the human looking at it.

---

## 3. Design Philosophy

`DESIGN_DOCTRINE.md` is the canonical reference. This strategy inherits all
ten of its sections and extends them with three meta-principles that govern
future evolution:

- **Identity over flexibility.** When the framework offers a knob, that knob
  is a *setting*, not a *hint*. Three values, named. Never a slider. Never
  an arbitrary string. The cost of a new knob is paid forever.
- **Doctrine over decoration.** A new component arrives by extending a
  primitive (`.rio-dropdown` is the canonical example: filters, sort menus,
  future column togglers). A new component never arrives as a one-off
  bespoke surface.
- **Source order is API.** The CSS cascade order — tokens → themes → base →
  layout → components → pages → responsive → print — is part of the public
  contract. Changing it requires a major version bump.

The DESIGN_DOCTRINE establishes *what RustIO looks like*. The strategy doc
establishes *what RustIO does not become*.

---

## 4. AI-Native Philosophy

RustIO is one of the first frameworks where coding agents are first-class
consumers of the source tree alongside humans. The framework must be legible
to both, and the architecture must make doctrine violations *detectable*,
not merely *preventable*.

Three operating principles:

- **Invariants are files, not lore.** Anything an AI agent must respect
  lives in a versioned markdown file inside the project, generated at
  scaffold time, pinned to the framework version. Tribal knowledge is the
  failure mode.
- **Project context is regenerated, not handwritten.** Models, admin slugs,
  routes, permissions, migration state — the things that change every
  sprint — live in an auto-generated `AI_CONTEXT.md` that `rustio` rewrites
  on `rustio doctor sync`. Agents read it; humans never edit it.
- **Doctrine enforcement is a command, not a hope.** `rustio ai-check`
  validates the working tree against framework rules before commit. Agents
  and humans both run it. The framework refuses to lie about whether its
  rules are being followed.

This is **not** "AI hype." There is no chat interface, no embedded copilot,
no inference at runtime. RustIO simply admits that a meaningful percentage
of code touching its surface is now written with agent assistance, and
provides a disciplined, machine-legible foothold for that work.

---

## 5. Cleanup Roadmap

Five phases, each delivering a single coherent release. None of them adds a
framework feature.

| Phase | Release | Theme                                  | Time-budget |
|-------|---------|----------------------------------------|-------------|
| 0     | 0.8.x   | Documentation tree consolidation        | Now         |
| 1     | 0.9.0   | Public API audit + crate boundary lock  | 1 cycle     |
| 2     | 0.10.0  | Example portfolio replacement           | 1 cycle     |
| 3     | 0.11.0  | AI-native generation surface ships      | 1 cycle     |
| 4     | 1.0.0   | SemVer freeze on the declared API       | 1 cycle     |

**Phase 0 (now).** Move `DESIGN_DOCTRINE.md`, `VISIBILITY_AUDIT.md`, and
similar to `docs/`. Archive completed audits to `docs/archive/`. No code
touched. Goal: contributor sees one obvious entry point.

**Phase 1 (0.9.0).** Annotate every public item in `rustio-admin` with one
of: `// public`, `// internal`, `// deprecated`. The lint pass moves `pub`
items to `pub(crate)` where they're internal-by-design. Generates a
`docs/public-api.md` from the annotation pass. *No item is removed.* The
lint is purely descriptive — a freeze-prep step.

**Phase 2 (0.10.0).** Replace `examples/minimal` with two flagship examples
(fleet-ops and library-circulation). Old example is archived to
`examples/archive/minimal-0.8/`. CI builds and screenshot-tests both new
examples.

**Phase 3 (0.11.0).** Ship the AI-native scaffold surface: `CLAUDE.md`,
`AI_CONTEXT.md`, `rustio ai-check`, `rustio ai-prompt`. None of this is
required — `rustio startproject` ships them by default; existing projects
opt in with `rustio ai-init`.

**Phase 4 (1.0.0).** Public API committed. Cascade order committed. Token
names committed. Selectors committed. The five-year promise becomes a
contract.

---

## 6. Migration Phases

Each phase below states three things: *what changes*, *what stays
bit-identical*, and *the contributor escape hatch*.

### 0.9.0 — Public API audit

- **Changes:** `pub(crate)` substitutions on internal helpers. New
  `docs/public-api.md`. Re-exports tightened. Some types may be sealed with
  `pub(crate) use` to prevent accidental downstream construction.
- **Bit-identical:** every URL, every selector, every token, every
  behaviour. The audit is descriptive, not destructive.
- **Escape hatch:** any downstream project relying on an item that becomes
  `pub(crate)` files a tracking issue; we re-export it under a
  `rustio_admin::internal` namespace for one minor release and emit a
  deprecation warning.

### 0.10.0 — Example replacement

- **Changes:** `examples/fleet-ops` + `examples/library-circulation` ship.
  `examples/minimal` moves to `examples/archive/minimal-0.8/` with a README
  pointing at the new examples.
- **Bit-identical:** framework crates. Pure example surface change.
- **Escape hatch:** the archived example continues to build under CI for
  two minor releases.

### 0.11.0 — AI-native surface

- **Changes:** new scaffold files (`CLAUDE.md`, `AI_CONTEXT.md`,
  `RUSTIO_DOCTRINE.md`). New CLI: `rustio ai-init`, `rustio ai-check`,
  `rustio ai-prompt`, `rustio doctor sync`. New `rustio.theme.json` schema.
- **Bit-identical:** the framework binary's runtime behaviour. Everything
  here is opt-in scaffolding.
- **Escape hatch:** `rustio startproject --no-ai` skips the AI-scaffold
  entirely.

### 1.0.0 — SemVer commitment

- **Changes:** the version number, the SemVer policy, the public-API
  guarantees. No code change beyond a doc + CHANGELOG flip.
- **Bit-identical:** 0.11.0 source tree.
- **Escape hatch:** breaking changes after 1.0.0 require a new major. They
  are not casual.

---

## 7. Priority Ordering

If only one phase ships, ship Phase 1. The public API audit is what unlocks
everything downstream — confident contributor onboarding, AI guidance
precision, example stability. Without it, every later phase compounds drift.

If two phases ship, add Phase 3. The AI-native surface is the
highest-leverage differentiator the framework has. It is also the easiest
to get wrong if rushed past Phase 1.

Examples (Phase 2) is third. Visible to outsiders, low risk to the
framework, but its leverage is *perception* — not *capability*. Worth doing
well; not worth doing first.

Phase 4 (1.0.0) is calendar-driven: it ships when 0.11.0 has been in use
for at least one quarter without an emergency patch.

**Do not invert this order.** Tagging 1.0.0 before the public API is
annotated is the single largest failure mode for a framework at this stage.

---

## 8. Risk Analysis

| Risk                                                              | Likelihood | Impact | Mitigation                                                             |
|-------------------------------------------------------------------|-----------:|-------:|------------------------------------------------------------------------|
| Public API audit accidentally privatises a downstream-used type   | Medium     | High   | One-minor-release re-export window under `internal::` namespace.       |
| Examples become outdated and rot                                  | High       | Medium | CI builds + screenshots both examples on every PR.                     |
| AI-scaffold files diverge from framework reality after upgrades   | High       | High   | `AI_CONTEXT.md` is regenerated, never hand-edited. `rustio doctor` warns if stale. |
| Theme config becomes a Tailwind clone over time                   | Medium     | Existential | Three-value enums only. No free-form strings. Schema-versioned.    |
| `CLAUDE.md` gets perceived as a marketing gimmick                 | Medium     | Medium | Frame as "AI-aware framework engineering" in docs, never as a feature. |
| Cascade order accidentally changes during a future refactor       | Low        | High   | Lock the `concat!` order in routes.rs and the `@import` manifest in admin.css with a "lockstep" comment + a CI grep test. |
| Phase 1 reveals public-API churn that delays 1.0.0 by a year      | Medium     | Low    | Acceptable. 1.0.0 is a quality commitment; date is not the constraint. |
| First-impression home page drifts toward marketing over time      | Medium     | High   | Document it in DESIGN_DOCTRINE.md as a non-marketing surface. Code-review rule. |

---

## 9. Naming Proposals

The framework's existing naming is strong (`--rio-*`, `.rio-*`,
`Admin::theme()`, `#[rustio(...)]`). Future additions should extend it, not
diverge from it.

**Workspace crates** (current + future)

- `rustio-admin` — the framework (stays)
- `rustio-admin-macros` — proc-macros (stays)
- `rustio-admin-cli` — CLI tooling (stays)
- `rustio-admin-icons` *(future, only if needed)* — separate icon set crate
  so the binary stays slim when icons are unused
- `rustio-admin-fixtures` *(future, only if examples need shared seed
  data)* — shared seed/fixture machinery for examples

**Project-local config files**

- `rustio.toml` — framework-level project config (replaces ad-hoc env
  splatter for non-secret settings)
- `rustio.theme.json` — guarded design overrides (Part 3)
- `rustio.lock.json` — architectural snapshot (Section 11)

**AI-guidance files**

- `CLAUDE.md` — Claude-specific guidance, project-local. Convention name;
  tool-aware.
- `AI_CONTEXT.md` — auto-generated project facts, project-local
- `RUSTIO_DOCTRINE.md` — framework invariants, project-local (copy of the
  framework's canonical version, pinned to the project's framework version)
- `CONTRIBUTOR_AI_GUIDE.md` — *framework-level*, not project-local. Lives in
  the framework repo.

**Naming principles**

- ALL_CAPS for human-meant top-level documents (DESIGN_DOCTRINE.md,
  CHANGELOG.md, CONTRIBUTOR_AI_GUIDE.md)
- kebab-case.md for sub-pages inside `docs/`
- snake_case.json / snake_case.toml for machine-read config
- `rustio.<scope>.<ext>` for project-local config

---

## 10. File / Folder Proposals

Target repository layout end of Phase 1:

```text
rustio-admin/
├── crates/
│   ├── rustio-admin/                ← framework
│   ├── rustio-admin-macros/         ← proc-macros
│   └── rustio-admin-cli/            ← CLI
├── examples/
│   ├── fleet-ops/                   ← (lands in Phase 2)
│   ├── library-circulation/         ← (lands in Phase 2)
│   └── archive/
│       └── minimal-0.8/             ← (moved in Phase 2)
├── docs/
│   ├── DESIGN_DOCTRINE.md           ← moved from root in Phase 0
│   ├── ARCHITECTURE.md              ← new in Phase 0
│   ├── public-api.md                ← generated in Phase 1
│   ├── contributing.md              ← new in Phase 0
│   ├── ai/
│   │   ├── CONTRIBUTOR_AI_GUIDE.md  ← for framework contributors
│   │   ├── RUSTIO_DOCTRINE.md       ← canonical, copied into projects
│   │   └── prompts/                 ← prompt templates used by `rustio ai-prompt`
│   ├── tutorials/
│   │   └── getting-started.md       ← absorbs the role of `examples/minimal`
│   └── archive/
│       └── VISIBILITY_AUDIT.md      ← moved in Phase 0
├── CHANGELOG.md
├── README.md
├── LICENSE
└── Cargo.toml
```

Project-local layout (scaffolded by `rustio startproject foo` end of Phase 3):

```text
foo/
├── src/
├── migrations/
├── rustio.toml
├── rustio.theme.json
├── CLAUDE.md
├── AI_CONTEXT.md             ← auto-generated; do not edit
├── RUSTIO_DOCTRINE.md        ← pinned copy
├── README.md
└── Cargo.toml
```

---

## 11. Guardrail Systems

Three guardrail systems, each operating at a different layer.

**Layer 1 — Theme guardrails (`rustio.theme.json`).**

Allowed overrides:

- 5 token groups (colors, spacing, radius, shadows, typography) via named
  profile + per-token overrides
- Brand name, brand emblem URL, footer text, login title
- Density profile: `comfortable | compact | spacious` (three values, no
  slider)
- Locale defaults, RTL default

Forbidden in `rustio.theme.json`:

- Selector names
- Component markup
- Cascade order
- Surface ladder semantics
- The 16 px html root
- The accent-reservation rule (chrome stays neutral)

Implementation: JSON Schema validation in `rustio doctor`. Schema version
pinned to framework minor. Unknown keys are a hard error, not a warning.

**Layer 2 — AI doctrine enforcement (`rustio ai-check`).**

Runs as a pre-commit hook (opt-in) and a CI step. Validates:

- Every `pub` item in `crates/rustio-admin` is annotated `// public` or
  `// internal`
- No CSS rule outside `tokens/` hard-codes a colour, font-size, or spacing
  value
- No template introduces a selector outside the `rio-*` namespace
- `AI_CONTEXT.md` is fresh (modtime within N minutes of the most recent
  migration / route / admin change)
- The `concat!` block in `routes.rs` matches the `@import` manifest in
  `admin.css` line-for-line

Failures are diagnostic, not prescriptive. The tool says *"rule X violated
at line Y"*, never *"here's the fix."* Fixes are the contributor's
responsibility — AI or human.

**Layer 3 — Architectural lock (`rustio.lock.json`).**

A periodically-regenerated JSON snapshot of the project's architectural
shape (not its dependencies — that's `Cargo.lock`'s job). It records:

- The list of admin models, with slugs and field shapes
- The list of mounted routes
- The middleware order
- The set of permissions
- The set of audit-event kinds emitted

The lock file is checked in. AI agents read it as ground truth. `rustio
doctor` warns when the working tree drifts from the lock without an
explicit `rustio lock sync`. The lock prevents silent architectural drift —
exactly the failure mode where an agent renames a model and ten downstream
references rot.

---

## 12. Example Strategy

Examples are **product-positioning assets**, not tutorials. Tutorials live
in `docs/tutorials/`. Examples exist to answer one question for an
evaluating engineer: *"is this framework serious about my domain?"*

**Remove**: `examples/minimal`. Hello-world examples actively damage
framework perception at this stage. A weak example is worse than no example.

**Ship two examples, no more.** Three is more than a maintainer can keep
polished; two is the sweet spot for credibility-per-maintenance-hour.

1. **fleet-ops** — Vehicle, driver, and assignment management for a
   hypothetical transport operator. Models: `Vehicle`, `Driver`,
   `Assignment`, `MaintenanceWindow`, `IncidentReport`. Demonstrates: FK
   relationships across five entities, search joining vehicle + driver +
   branch, permissions sliced by branch/region, bulk archive of
   decommissioned vehicles, audit trail on every status change. Domain is
   universally legible without compliance overhead.

2. **library-circulation** — A real-world public library system. Models:
   `Patron`, `Item`, `Hold`, `Loan`, `Branch`, `StaffMember`. Demonstrates:
   state machines (item: available → on-hold → checked-out → in-transit),
   time-bound records (loans, holds with expiry), multi-tenant slicing (per
   branch), notification audit logs. Domain projects operational complexity
   without selling fear (compliance, surveillance, etc.).

**What examples must demonstrate:**

- Three or more linked models with non-trivial FK relationships
- A custom action beyond CRUD (a state transition, a bulk operation, an
  export)
- A non-default permission scheme (per-branch, per-role)
- A populated database seeded by a fixture script
- A screenshot that an evaluator can recognise as "operations software"

**What examples must NOT be:**

- Todo apps, blog apps, chat apps, e-commerce stores
- Tutorials with step-by-step prose interleaved in source
- Demos of every framework feature at once
- "Living style guides" disguised as applications

---

## 13. Home-Page Philosophy

The default `/admin/` home is the framework's first words to the operator.
It must read like:

> *"Welcome back. You ran three migrations yesterday. Two users haven't
> enrolled MFA. The audit log shows no failed logins in the last 24 hours."*

Not like:

> *"🎉 Welcome to RustIO! Take a tour. Build something amazing today."*

**Information hierarchy** (top to bottom):

1. **Identity strip.** "*Logged in as `abdulwahed`, last active 12 minutes
   ago, MFA on, sessions: 2*". Tabular nums, no avatar, no big card.
2. **Operational pulse.** Three to six counters that matter for *this*
   deployment, populated by the project's `Admin::dashboard()` builder.
   Counters are not KPIs. They are facts. ("847 active users. 12 pending
   password resets. 3 R4 emergency recoveries this quarter.")
3. **Quick actions.** Four to six verbs the operator most commonly
   performs, chosen by the project (`Add user`, `Review history`, `Cut
   release`, `Run integrity check`). Plain `.rio-button`s, not gradient
   cards.
4. **Recent activity.** The last 10 audit events, formatted with the
   timeline component. Calm; high-signal; no decoration.

**First-run UX** (same shell, different content):

- A setup checklist replaces the operational pulse until every step is
  checked.
- "✓ Database connected.  ✓ First superuser created.  ○ MFA enrollment
  pending.  ○ R4 emergency-access account configured.  ○
  `rustio.theme.json` reviewed."
- Once all items are checked, the checklist disappears permanently. There
  is no "Welcome tour" to dismiss.

**Forbidden on the home page, forever:**

- Marketing copy
- Illustrations, mascots, decorative graphics
- "Get started" CTAs
- Product tours
- Empty-state hero illustrations
- Gradient KPI cards
- Carousels of any kind

**Typography rules:**

- One h1 (the workspace name + greeting)
- All numbers in tabular nums
- All times in monospace ISO-8601-adjacent format
- Body line-height 1.6, never tighter

The home page is the framework's identity statement. **A new contributor
who sees this page should immediately know what RustIO is *not*.**

---

## 14. Theme Customization Philosophy

The framework is generous with values, **stingy with shapes.**

**What `rustio.theme.json` controls (Phase 3 schema, v1):**

```json
{
  "schema_version": 1,
  "brand": {
    "name": "Acme Internal Tools",
    "emblem_url": "/static/brand.svg",
    "footer": "© Acme 2026"
  },
  "tokens": {
    "profile": "default" | "mono" | "enterprise-blue",
    "overrides": {
      "--rio-accent": "#0F8C7E",
      "--rio-accent-hover": "#0A6E62"
    }
  },
  "density": "comfortable" | "compact" | "spacious",
  "locale": {
    "default": "en",
    "rtl_default": false
  }
}
```

**Three token profiles, named not numbered:**

- `default` — RustIO teal-emerald
- `mono` — neutral greys only; for projects that want zero brand color
- `enterprise-blue` — corporate IT default; corporate-blue accent on the
  same surface ladder

**Three density profiles, named not numbered:**

- `comfortable` — current values (the operator default)
- `compact` — denser rows + tighter padding for data-heavy admin work
- `spacious` — wider rhythms for accessibility / projection use

**Never customizable:**

- The cascade order
- Selector names
- Component markup
- Surface ladder semantics (page < surface < surface-2 < surface-3)
- The 16 px html root and the rem-derived size scale relationships
- The accent reservation rule (no flood-fill chrome)
- The "no-#000, no-#111" dark-mode floor
- The mobile-first breakpoint set (768 / 1280)

**Why the constraints matter:** the framework's identity survives across
thousands of deployments only if customisation can't reshape it. Three
named density values is a setting. A `--density` numeric slider is a hint
that we're not sure what density should be — and that uncertainty leaks
into every screen.

A `rustio.theme.json` should feel like configuring a thermostat. Not like
writing Tailwind.

---

## 15. Future Expansion Opportunities

Optional crates and modules that *may* land after 1.0.0. Each is opt-in,
none required.

- **`rustio-admin-icons`.** A separately-versioned icon set. Today the
  framework uses inline SVG; an icon crate lets projects switch sets
  without forking the framework.
- **`rustio-admin-fixtures`.** Shared seed/fixture machinery for examples
  and tests. Currently each example would re-invent its own; this
  consolidates.
- **`rustio-admin-rtl`.** If projects emerge that ship in pure Arabic /
  Hebrew / Farsi with no Latin context, a thin crate that flips chrome
  direction defaults.
- **`rustio-admin-exports`.** CSV / XLSX / PDF export hooks on list views.
  Many admin domains need this; the framework should not.
- **`rustio` workspace shortcut.** A meta-crate that re-exports the
  canonical set; `rustio = "1.0"` instead of three sibling deps.
- **`rustio doctor sync`.** Continuous regeneration of `AI_CONTEXT.md` from
  the working tree.
- **`rustio ai-prompt <task>`.** Outputs a tuned prompt for common tasks
  (new model, new admin action, custom permission), populated with current
  project context.

Every item on this list is *deferrable forever.* Nothing on it gates 1.0.0.

---

## 16. Non-Goals

Non-goals are louder than goals. The framework explicitly will **not** do
any of the following, regardless of pressure:

- **No marketing site.** The framework's home page on the web is
  `https://github.com/abdulwahed-sweden/rustio-admin`. Not a Next.js
  portfolio site.
- **No design tokens marketplace.** Token customisation is bounded by the
  schema. There is no community theme upload, no marketplace, no theme
  picker UI.
- **No GraphQL / REST surface for admin operations.** RustIO is
  server-rendered. If a project needs an API, the project builds an API.
  The framework's admin UI is not exposed as JSON.
- **No CSS-in-JS, no Tailwind interop, no SCSS adapter, no PostCSS
  pipeline.** Pure CSS, hand-written, baked.
- **No NPM presence of any kind.** No `@rustio/icons` on npm, no
  `@rustio/themes` on npm, no plugin system that requires a JS toolchain.
- **No first-party plugins.** The framework's surface is the framework's
  surface. Project-local extension happens through Rust traits and the
  macro system, not through a plugin registry.
- **No "Welcome to RustIO" landing.** Ever.
- **No mascot, no logo redesign cycle, no "rebrand."** The teal accent is
  permanent.
- **No utility classes beyond what `base/utilities.css` already ships.**
  `.rio-num`, `.rio-prose`, `.rio-text-*`, `.rio-tight`, `.rio-icon`. That
  set is closed. We do not add `.rio-mt-4` or `.rio-flex-col`.
- **No AI features that run at runtime in the admin UI.** No copilot panel,
  no embedded chat, no inference endpoint. AI is a *contributor* tool, not
  a *user* tool.

---

## 17. Critical Mistakes to Avoid

The risks below would damage the framework permanently. Each has a
near-miss precedent in adjacent ecosystems.

1. **Shipping 1.0.0 before the public API is annotated.** This is the
   single largest failure mode at this stage. A 1.0.0 with ambiguous public
   API surface is a 1.0.0 in name only, and we'd carry the debt forever.
2. **Letting `rustio.theme.json` grow free-form string keys.** The moment a
   theme can set `"buttonRadius": "12.5px"` instead of `"radius": "lg"`,
   identity is gone. Hold the line on named enums.
3. **Adding a fourth density profile.** The third value (`spacious`) is
   itself a stretch. If a real accessibility need emerges, *adjust the
   existing profiles*; don't add new ones.
4. **Allowing the home page to acquire one decorative element.** The first
   illustration, the first gradient, the first hero card — once present,
   they multiply. The home-page rule is enforceable only as a hard "zero
   of these things, ever."
5. **Coupling AI scaffold files to a single vendor.** `CLAUDE.md` is
   allowed because it's a convention name, not a Claude-specific schema.
   The actual rules in it must apply to any agent. The day RustIO ships a
   `CLAUDE.md` schema that other agents can't honour is the day the
   AI-native story dies.
6. **Treating `AI_CONTEXT.md` as a human-editable file.** The moment
   someone hand-edits it and forgets, every downstream agent acts on lies.
   The file must be regenerated, must contain a "do not edit" header, and
   `rustio doctor` must warn when its hash mismatches the regeneration
   target.
7. **Mistaking examples for tutorials.** A tutorial-shaped example is half
   the size it needs to be to look serious. Tutorials live in
   `docs/tutorials/`. Examples are full apps.
8. **Adding a "Themes" page to the admin UI.** Theme is configured in
   `rustio.theme.json`, in source control, code-reviewed. Never
   click-to-pick. The day a project's accent colour gets bikeshedded in a
   UI is the day operators stop trusting their own framework.
9. **Refactoring the cascade order "just to clean it up."** Cascade order
   is API. A cleanup PR that swaps `responsive.css` to before
   `components/tables.css` would silently corrupt every consuming project's
   sidebar layout below 768 px. Lock it with a CI grep test.
10. **Skipping Phase 1.** Phases 2 and 3 are flashier; Phase 1 is the
    foundation. Order is not negotiable.
11. **Allowing AI hype framing to leak into the framework's voice.** RustIO
    does not say "AI-powered." It says "AI-aware." The distinction is the
    whole story.
12. **Adding a CDN dependency anywhere.** Fonts, icons, themes, AI
    helpers — all baked. The day RustIO depends on a CDN is the day it
    stops being suitable for the air-gapped operator deployments that are
    its most important constituency.

---

*End of strategy.*
