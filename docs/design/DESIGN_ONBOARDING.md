# Onboarding Architecture

Onboarding is the developer's first hour with RustIO. It is the
moment the framework either earns trust or loses it.

This document is the contract for how that hour feels: which
steps are guided, which errors are humanised, which commands
appear first, what the scaffold ships, and where the line falls
between the beginner surface and the advanced platform underneath.

Pull request review for any onboarding-touching change runs
against this document, not only the diff.

> **Governing sentence**
> The CLI must stop being a silent command executor and become a
> calm, senior developer sitting next to the beginner — guiding
> the next step, explaining every error, and proving the project
> is alive at every moment.

> **Central rule**
> Guided explicitness, never hidden explicitness.

---

## 1. Purpose

### 1.1 What this governs

- The shape of the first 15 minutes from `cargo install` to a
  reachable `/admin/login`.
- The split between Phase 1 (beginner surface) and Phase 2
  (power-user reveal), and the rule that the split is
  presentation only.
- The identity of the default scaffold and the line between
  *demo data* and *structural defaults*.
- The default permission groups and their exact-name contract
  with `rustio user create --role`.
- The error-message shape every onboarding-facing failure must
  take.
- Progress, motion, and terminal-feedback rules.
- The command surface a beginner sees first, and the rules for
  surfacing advanced commands without dropping them.
- The scaffold's homepage at `/` — what it must communicate and
  what it must not replace.

### 1.2 What this does not cover

- Session lifecycle and revocation → `DESIGN_SESSIONS.md`.
- Recovery flows → `DESIGN_RECOVERY.md`, `DESIGN_R2_ORGANISATIONAL.md`.
- MFA and emergency operator paths → `DESIGN_R3_MFA.md`, `DESIGN_R4_EMERGENCY.md`.
- Builder declarative compilation → `DESIGN_BUILDER.md`.
- Visual token system → `DESIGN_SYSTEM.md`, `DESIGN_DOCTRINE.md`.

### 1.3 Closing principle

Onboarding is now a first-class product surface, not a residue
of scattered CLI messages. It is owned by this document, it has
acceptance criteria, and it is reviewed.

---

## 2. Core diagnosis — feature saturation

Between 0.5.0 and 0.19.0, RustIO acquired serious platform
capability:

- `startproject`, `startapp`, `new`
- the Builder (declarative compiler, append-only history,
  deterministic plan/commit)
- typed audit events, correlation IDs, per-request audit chain
- session lifecycle, trust escalation, centralised invalidation
- MFA, recovery, admin-driven password reset
- notifications, feature flags, health surface
- runtime overrides
- the `rio-theme` engine and the v0.19 visual language
- contracts and schemas as future-facing power surfaces
- the admin runtime and the manual runtime side-by-side

This is strength. The cost is that the first five minutes became
crowded. A new developer typing `rustio --help` for the first
time meets twenty verbs at once, with no signal about which
three to use today.

**RustIO is not weaker. It became stronger faster than its
onboarding did.** The fix is not to remove capability. The fix
is to give the beginner a clear path through it.

---

## 3. The Two-Phase Doctrine

### 3.1 Phase 1 — the first fifteen minutes

A beginner, in their first session, should reach all of the
following without seeing the full advanced surface:

- project directory created
- database name chosen once
- `DATABASE_URL` written to `.env`, with `.gitignore` excluding it
- platform-specific PostgreSQL guidance printed
- migrations applied successfully
- first admin user created
- server running on a known port
- homepage alive at `/`
- admin surface reachable at `/admin`
- at least one simple CRUD path visible end-to-end

Phase 1 is the *visible* surface. Everything else still exists
and runs — it is simply not promoted on day one.

### 3.2 Phase 2 — power-user reveal

Phase 2 surfaces become progressively discoverable from Phase 1
without being thrown in the beginner's face:

- contracts and schemas
- the Builder declarative workflow
- audit deep-dives and correlation-chain inspection
- runtime overrides
- theme generation beyond defaults
- advanced admin docs
- AI / codegen / generative workflows when they ship

### 3.3 Presentation, not amputation

This is the most important rule in this document.

The Phase 1 / Phase 2 split is a **presentation split**, not a
**capability split**. Advanced features keep existing, keep
running, and keep being reachable from second one. Phase 1
hides complexity from view. It must never remove capability.

A change that satisfies the Phase 1 experience by deleting a
Phase 2 feature, by silencing an audit event, by skipping a
session invariant, or by short-circuiting any security path is
out of contract.

---

## 4. Guided explicitness, not hidden magic

RustIO's value proposition is engineering honesty. That value
proposition is preserved by these rules:

- **Do not hide PostgreSQL.** Beginners learn that this is a
  Postgres-first framework on minute one, and the CLI helps them
  install and configure it.
- **Do not hide backend reality.** Migrations, sessions, audit,
  and recovery all exist by name. Beginners do not have to
  understand them on day one, but they are never hidden.
- **Do not silently rewrite a working command's behaviour.** Add
  aliases and wizards around existing commands; do not change
  what a known command does today.
- **Do not invent state the user did not request.** If the CLI
  writes a file, persists progress, or seeds rows, it says so on
  screen.

The rule that summarises all of the above:

> **Guided explicitness, never hidden explicitness.**

---

## 5. The first 15 minutes — target flow

The flow below is the contract Phase 1 ships against. Every
command must work on macOS and Ubuntu without external
documentation. Each step is one line on screen plus, where
useful, a short next-step hint.

```text
cargo install rustio-admin-cli
rustio --version
rustio --help
rustio new project school
cd school
# automatic .env generation
# automatic .gitignore generation
# platform-specific PostgreSQL guidance
# database name chosen once
rustio migrate apply
rustio user create --email admin@school.local --role administrator
rustio startapp student
# model registration guidance
rustio migrate apply
cargo run
```

The final URLs the beginner is told to open:

- `http://127.0.0.1:8000/` — homepage; project is alive.
- `http://127.0.0.1:8000/admin` — admin surface.

Anything that pulls a beginner off this path during their first
session is, by default, a Phase 2 surface.

---

## 6. Default scaffold identity

### 6.1 The current problem

The scaffold today ships `post.rs` / `posts` — a blog-shaped
demo. A developer who installed RustIO for a school management
system, a clinic, or an inventory tool is immediately editing
someone else's blog example. The first impression is "I am in
the wrong tool."

### 6.2 The doctrine

- Generic blog content is not a valid default identity.
- Starter content must be either **project-type-aware** or
  **absent**. There is no third option.
- Demo data is permitted only when its purpose is to make the
  admin feel alive on first load.
- Demo data must be clearly disposable — clearly named, clearly
  documented as a starter, and easy to remove.
- A school project should feel like a school project on first
  open. A clinic project should feel like a clinic project.
- The set of project types is curated and small (for example:
  `school`, `clinic`, `inventory`, `blog`, `custom`). The
  `custom` choice yields no starter model, which is the correct
  default for users who know what they want.

---

## 7. Demo data vs structural defaults

This distinction is load-bearing. Conflating the two has caused
real security mistakes in other frameworks.

### 7.1 Demo data

| Property             | Value                                          |
|----------------------|------------------------------------------------|
| Lifecycle            | Disposable                                     |
| Purpose              | Educational, "the admin is alive"              |
| Project-typed        | Yes — matches the project type chosen          |
| Safe to delete       | Yes                                            |
| Example              | Five `student` rows in a `school` project      |
| Where it lives       | A migration the user can drop without harm    |

### 7.2 Structural defaults

| Property             | Value                                          |
|----------------------|------------------------------------------------|
| Lifecycle            | Foundational — built upon, not deleted         |
| Purpose              | Security and authorisation baseline            |
| Project-typed        | No — identical for every project               |
| Safe to delete       | No — application security relies on them      |
| Example              | The three default permission groups            |
| Where it lives       | A migration the framework owns                 |

### 7.3 The default permission groups

The framework seeds exactly three groups on a fresh database:

| Group           | Permissions                                                                                   |
|-----------------|-----------------------------------------------------------------------------------------------|
| `administrator` | Full system access.                                                                           |
| `editor`        | Create / read / update content models. No user, group, settings, or framework-admin actions. |
| `viewer`        | Read-only access to content models.                                                           |

### 7.4 The exact-name contract

The three group names MUST exactly match the `--role` values
accepted by `rustio user create`. If either side renames, the
other renames in the same commit. CI enforces the match.

This means `administrator` is `administrator` — not `admin`,
not `superuser`, not `root`. `editor` is `editor`, not
`writer`. `viewer` is `viewer`, not `reader` or `read-only`.

### 7.5 Rules

- Default groups are seeded only on a fresh database. If any
  user-defined group exists, structural seeding is skipped — the
  framework never silently mutates an existing project's
  authorisation surface.
- The framework never auto-generates broad, optimistic, or
  domain-specific permission groups (no `staff`, no `manager`,
  no `support`). Those are application decisions and belong in
  the application.
- Removing a structural default is an explicit operator action,
  never a default behaviour.

---

## 8. Error-message doctrine

Every onboarding-facing CLI error MUST follow this four-part
shape:

```text
Problem:  <what failed, in plain English>
Why:      <the most likely cause, one sentence>
Fix:      <the exact command or file edit needed>
Retry:    <the exact command to run again>
```

Errors that already follow a different shape inside the
framework's runtime (typed `Error` enum, audit events) are
unaffected — this doctrine governs the CLI's onboarding-facing
output, not internal error types.

### 8.1 Worked examples

**`DATABASE_URL` is missing**

```text
Problem:  DATABASE_URL is not set.
Why:      The CLI looks for a .env file in the current directory.
Fix:      Create .env with: DATABASE_URL=postgres://localhost/<your-db>
Retry:    rustio migrate apply
```

**PostgreSQL service is not running**

```text
Problem:  Cannot connect to PostgreSQL at <host>:<port>.
Why:      The PostgreSQL server is not running, or it is listening on a different address.
Fix:      Start PostgreSQL — on macOS: `brew services start postgresql`.
Retry:    rustio migrate apply
```

**Database does not exist**

```text
Problem:  Database "<name>" does not exist on <host>.
Why:      PostgreSQL is running, but the database has not been created yet.
Fix:      createdb <name>
Retry:    rustio migrate apply
```

**Migration SQL failed**

```text
Problem:  Migration <file> failed at line <n>.
Why:      <captured SQL error, prefixed verbatim>
Fix:      Edit <file> and re-run, or write a new migration that corrects the schema.
Retry:    rustio migrate apply
```

**Role / group name mismatch**

```text
Problem:  Role "<role>" is not one of the framework's known roles.
Why:      Known roles are: administrator, editor, viewer.
Fix:      Re-run with --role administrator (or editor / viewer).
Retry:    rustio user create --email <email> --role administrator
```

The pattern is consistent: name the failure, name the cause,
name the exact next keystroke, name the exact retry.

---

## 9. Motion and feedback doctrine

A long operation must never look frozen. A user who cannot tell
whether the CLI is working or hung will assume hung.

### 9.1 Required behaviours

- Database connection checks show a spinner.
- Migration application shows per-file progress.
- Multi-step wizards print a `✓ <step name>` after each step.
- Successful commands end with a single, calm confirmation line.

### 9.2 Constraints

- Respect `NO_COLOR` — drop ANSI under it.
- Respect non-TTY — degrade to plain text when stdout is not a
  terminal (CI logs, piping).
- Respect `--quiet` and `--no-progress` flags where present.
- Use a single, sober Unicode `✓`. No emoji. No animated banners.
- Never alter the bytes of machine-readable output (JSON,
  structured logs).
- Progress is feedback, not theatre.

---

## 10. Command-surface doctrine

### 10.1 Beginner-facing first

- `rustio new project <name>` is the beginner-facing command.
- `rustio startproject <name>` remains for compatibility and
  scripts; it is not removed.
- The Builder's `new` verb lives under a Builder-namespaced path
  so the universal `new` is free for the beginner.

### 10.2 `--help` shape

- `rustio --help` shows the Phase 1 surface first, under a
  clearly labelled "Start here" block.
- Advanced verbs (Builder, audit deep-dives, contract tools)
  appear below a separator, or behind `rustio --help advanced`.
- No command is removed from `--help`. Surfacing is controlled
  by ordering and grouping, not by amputation.

### 10.3 Compatibility

- A command that works today continues to work after any
  onboarding change.
- A new wizard wraps an existing command; it does not silently
  replace it.
- Deprecations carry at least one minor-release grace window and
  print a one-time notice.

---

## 11. Homepage doctrine

The scaffold ships a homepage at `/`. Its purpose is emotional
and instructional, not functional.

### 11.1 What it must communicate

- The project is alive — the server booted.
- Which files the scaffold generated.
- Database connection status.
- Migration status.
- Number of registered models.
- Next-step links: `/admin`, `/admin/docs`, `/admin/health`.

### 11.2 What it must not do

- It must not replace `/admin`. The admin surface remains where
  every admin user expects to find it.
- It must not contain real application content. Projects
  override the route to ship their own homepage; the scaffold's
  is a starter.
- It must not become a marketing page. Calm, factual, useful.

### 11.3 Override rule

If a project registers its own `/` route handler, the project's
handler wins. The scaffold's homepage is a starter, not a fixed
fixture.

---

## 12. Non-negotiable rules

These rules are violated only by explicit doctrine amendment.

1. **Do not hide PostgreSQL.** Beginners learn it; the CLI helps
   them with it.
2. **Do not add SQLite** as part of this onboarding work — not
   as a fallback, not as a "for tests," not as a convenience.
3. **Do not dumb RustIO down.** Power-user features stay
   present, reachable, and documented.
4. **Do not remove advanced features.** Phase 1 hides; it never
   amputates.
5. **Do not break existing commands.** Aliases and wizards wrap
   today's behaviour; they do not silently change it.
6. **Do not turn guidance into magic.** Every automatic action
   is announced on screen.
7. **Do not create noisy or childish CLI output.** No emoji-rich
   banners, no animated celebration art, no infantilising tone.
8. **Do not make security defaults broad.** The three default
   groups are conservative and minimal.
9. **Do not seed structural permission defaults as demo data.**
   They are foundational; they live in a framework-owned
   migration and survive scaffold cleanup.
10. **Do not show raw backend errors to beginners without
    explanation.** Wrap them in the four-part error shape; keep
    the raw text inside the `Why:` block where it remains
    diagnostic but no longer terrifying.

---

## 13. Acceptance criteria for this document

This document is complete when:

- It can be cited by future onboarding PRs as the constitution.
- It explains what RustIO should automate and what must remain
  explicit, in concrete terms.
- It separates Phase 1 and Phase 2 clearly enough that a
  contributor can place a new feature on one side or the other
  without ambiguity.
- It separates demo data from structural security defaults
  clearly enough that no contributor will conflate the two.
- It gives reviewers a paragraph to point at when a PR drifts.

When in doubt, the governing sentence in this document's header
is the tiebreaker.
