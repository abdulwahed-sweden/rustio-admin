# Visual + Authority Vocabulary

The contract for the framework's visual + authority system.
Governs design tokens, the canonical accent palette, the
typography philosophy, and the authority doctrine that every
RBAC decision rests on.

Companion to `DESIGN_SESSIONS.md`, `DESIGN_AUDIT.md`, and
`DESIGN_RECOVERY.md`. PR review is against this document.

> **Status**
>
> Token surface stable from 0.3.0. The teal-emerald accent
> palette is permanent. Token drift across feature branches
> is the single biggest source of visual regressions; the
> PR template's *Token disclosure* section enforces it.

---


## 1. Purpose

### 1.1 What this document governs

The framework owns the visual and authority vocabulary that
every project inherits. This document is the contract for
both layers — the design tokens (color, typography, spacing,
surfaces) and the authority doctrine that drives every
permission decision.

### 1.2 What this document does not cover

- Recovery flows — `DESIGN_RECOVERY.md`.
- Session lifecycle — `DESIGN_SESSIONS.md`.
- Audit row shape — `DESIGN_AUDIT.md`.
- Per-feature UI design — lives in feature commits and any
  feature-specific design docs.

### 1.3 Closing principle

The framework is no longer "a CRUD admin". It is becoming an
**authority-oriented admin platform** — a stack that takes
seriously who controls whom, who can change what, who can
recover whose access, and how the audit trail of those
decisions survives staff turnover.

Every design decision is measured against that mission. A
change is worth shipping if it makes authority clearer, the
boundaries safer, or the operator's intent more visible. A
change that adds visual noise, conflates roles with
permissions, or buries the rank model behind a clever
abstraction is moving in the wrong direction — even if the
diff looks elegant.

---


## 2. Invariants

The contract has two layers. The first is the authority
doctrine — three axioms that predate any individual feature
and survive every refactor. The second is the set of
prohibitions on the visual system — the rules whose violation
produces the visual regressions this framework is designed
to prevent.

### 2.1 Authority axioms

> **UI hiding is reflection, not security.**
> Every guard runs server-side on POST. Client-side
> affordances spare the operator a round-trip; they never
> gate access. See §7.1 for the full doctrine.

> **Rank controls WHO. Permissions control WHAT.**
> Roles are a rank ladder. Permissions are granular
> `<table>.<action>_<singular>` codenames. The two axes are
> orthogonal. See §7.2.

> **Groups are permission bundles, not authority roots.**
> A group is a named operational set of permissions. Group
> membership says nothing about authority — that comes from
> rank plus the protected-roles invariant. See §7.3.

### 2.2 What must never happen

> **No project redefines `--rio-*` tokens.**
> Project CSS reads framework tokens via `var(--rio-*)`;
> overrides go through `Admin::accent_color()` or its
> siblings, never through a `:root` redefinition. See §8.2.

> **No flood-fill accent.**
> The accent is a call-to-action signal, never a panel or
> chrome colour. See §12.

> **No Arabic text on a Latin face.**
> Both Latin and Arabic stacks are declared so resolution is
> deterministic. See §10.1.

> **No literal pixel values in component CSS.**
> Components resolve through `--rio-fs-*`, `--rio-s*`, and
> the surface ladder. Project-scoped class wrappers may
> declare project-local tokens (`--bsk-*`, `--app-*`). See
> §10.2.

> **No two open branches changing overlapping tokens
> silently.**
> Cross-coordinate before either merges, not after. See
> §11.1.

> **No reintroduction of red-family accents.**
> The terracotta accent retired in 0.3.0; it survives only
> as a comment reference in `admin.css`. See §9.

---


## 3. Risk model

A design system has no adversaries. It has failure modes —
ways the visual + authority contract slips when teams move
fast, branches diverge, or new features land without the
right review. This section catalogues the failure modes the
framework defends against and the mitigations that hold them
in check.

### 3.1 Failure modes

| Failure | What goes wrong | Where it surfaces |
|---------|-----------------|---------------------|
| **Token drift across feature branches** | Two open branches edit overlapping `--rio-*` tokens; the second to merge overwrites or conflicts with the first | `admin.css` token blocks; visible as colour or spacing inconsistencies in the merged main |
| **Project-side `:root` redefinition** | A project's own CSS silently forks a framework token (`:root { --rio-accent: … }`) | Project stylesheet; visible when the framework default fails to roll forward |
| **Accessibility regression** | A token's value shifts, breaking a contrast pair under WCAG | Light palette; visible only on audit |
| **Theme divergence between branches** | Feature branches accumulate parallel visual systems instead of converging on `main` | Cross-branch diffs; visible as "this branch looks different" surprise at merge time |
| **Authority misconception** | UI hiding gets used as a security mechanism (a hidden button is "safe") | Server-side guards; visible only when an attacker submits a forged POST |
| **Font resolution failure** | Arabic text lands on a Latin face because the fallback chain was edited | Any Arabic-rendering surface |
| **Accent flooding** | The accent colour gets used as a panel or chrome surface, breaking its call-to-action signal | Any page using the accent for non-CTA elements |

### 3.2 Mitigations summary

- **Token disclosure** in the PR template forces authors to
  list which tokens changed, the migration impact, and the
  regression risk before any token-touching PR can merge.
- **Single canonical branch** — `main` is the only place
  visual systems converge; feature branches diverge during
  review and merge promptly. See §11.1.
- **8-item visual regression checklist** walked by reviewers
  on every token-touching PR. See appendix B.
- **SemVer policy on token changes** — patch releases never
  shift token values; minor adds tokens; major-impact changes
  carry a CHANGELOG-front banner. See appendix C.
- **Server-side authority guards** — every guard runs on POST
  regardless of what the form said. UI affordances are a
  courtesy, not a gate. See §7.1.
- **Deterministic font resolution** — both Latin and Arabic
  stacks declared in priority order; the `:lang(ar)` /
  `[dir="rtl"]` rules pick the right family without browser
  guesswork. See §10.1.
- **Accent override goes through `Admin::accent_color()`** —
  the typed builder writes one `<style>` block in
  `_theme.html`, never a `:root` redefinition. See §8.2.

---


## 4. Token system overview

The token system is layered. Each layer has explicit
ownership; the chain runs from framework defaults through one
optional instance override to project component classes that
read but never redefine.

### 4.1 The three layers

```
Framework tokens (admin.css)        ← single source of truth
    │                                  --rio-accent, --rio-fs-*, --rio-s*, …
    │
    │  optional override at boot via Admin::accent_color()
    ▼
Instance theme block                ← one-shot value override
    │                                  emitted as a single <style> in _theme.html
    │
    │  read via var(--rio-*)
    ▼
Project component classes           ← read framework tokens; define
    │                                  their own --bsk-*, --app-*, --<project>-*
    │
    ▼
Rendered UI                         ← cascade resolves through the chain
```

### 4.2 Override paths

A project that needs a different accent colour:

```rust
// ✅ Typed override at boot. Emits a single _theme.html style block.
let admin = Admin::new()
    .accent_color("#0F8C7E");
```

A project that needs new tokens for its own components:

```css
/* ✅ Project-local tokens, prefixed. No --rio-* redefinition. */
:root {
  --bsk-c-cash:   #008D6D;
  --bsk-c-card:   #3D7AC8;
  --bsk-c-swish:  #D26F58;
}
```

A project consuming framework tokens in a new component:

```css
/* ✅ Read framework tokens via var(); never redefine them. */
.my-card {
  background: var(--rio-surface);
  border: 1px solid var(--rio-border);
  padding: var(--rio-s4);
  font-size: var(--rio-fs-base);
}
```

There is exactly one source of truth per token at any time.
The framework owns the canonical value; the typed builder
patches it once at boot if needed; project CSS reads through
the chain and never short-circuits it.

---


## 5. Guarantees

The framework makes these promises to project authors,
operators, and reviewers. Each is enforced by the patterns in
§7–§12; the callouts below are the contract.

### 5.1 Single source of truth

> **One source of truth per token at any time.**
> `admin.css` owns the canonical values; `Admin::accent_color()`
> patches one block at boot; project CSS reads via
> `var(--rio-*)` and never redefines.

### 5.2 Deterministic font resolution

> **Latin and Arabic stacks declared in priority order.**
> Browser resolution is deterministic. Arabic text never
> falls back to a Latin face; Latin text never picks an
> Arabic glyph for a numeric column.

### 5.3 Accessibility floor

> **Every text/surface pair clears WCAG AAA in both themes.**
> Light and dark palettes are validated on every token
> change. The visual checklist (appendix B) walks the
> high-traffic surfaces before a token-touching PR merges.

### 5.4 SemVer for tokens

> **Patch: no token changes. Minor: additive tokens.
> Major-impact: existing token values change.**
> Major-impact token changes carry a CHANGELOG-front banner.
> Projects that pin to a major.minor in `Cargo.toml` will
> not see a silent accent or text-colour shift inside a
> patch release.

### 5.5 Single canonical branch

> **The framework has one canonical visual system, on `main`.**
> Feature branches diverge during review, never accumulate.
> Two open branches changing overlapping tokens
> cross-coordinate before either merges.

---


## 6. Implementation notes

The remainder of this document is the engineering reference
for the visual + authority system. Sections cover the full
authority doctrine (§7), token ownership rules (§8), the
canonical accent palette (§9), the typography philosophy
(§10), branch and merge expectations (§11), and a
troubleshooting table for common visual regressions (§12).

Appendices A–C carry locked decisions, the PR review
checklist, and the versioning policy.

---


## 7. Authority doctrine

Three principles guide every authorization decision the
framework makes. They predate any individual feature and
survive every refactor. The §2.1 callouts distil each one;
the prose below is the load-bearing definition.

### 7.1 UI hiding is reflection, not security

Every guard runs **server-side on POST**, regardless of what
the form said. Client-side affordances (filtered dropdowns,
disabled checkboxes, hidden buttons) exist to spare the
operator a round-trip — never to gate access.

If you remove a UI element to "fix" an authorization problem,
you have not fixed it. The matching guard in
`auth/guards.rs` is what fixes it.

### 7.2 Rank controls WHO. Permissions control WHAT.

Roles are a **rank ladder**: User (100) → Staff (300) →
Supervisor (600) → Administrator (900) → Developer (1000).
The values are spaced for headroom and should be compared
**relatively**, never matched literally.

Rank decides **who may modify whom** — the cross-rank,
role-ceiling, and self-demote guards in `auth::guards` all
answer that question.

Permissions are the granular `<table>.<action>_<singular>`
codenames (`orders.view_order`,
`inventory.delete_inventory_item`). They decide **what a
user may do once they are inside the panel**.

A user with rank `Administrator` and zero direct permissions
can do everything. A user with rank `Staff` and the right
group memberships can do exactly the operations those groups
grant — and nothing else.

These are **two orthogonal axes**. Conflating them
("Admin = manager group = some roles") produces the chaotic
permission models this framework is designed to prevent.

### 7.3 Groups are permission bundles, not authority roots

A group is a *named operational set of permissions*. Users
belong to zero or more groups and inherit the union of their
permissions. Group membership says nothing about authority:
an Administrator without any group still bypasses group
checks; a Staff user can hold dozens of groups and still
cannot edit another Staff user.

Authority comes from rank + the protected-roles invariant.
Groups come from operational need.

---


## 8. Token ownership

`crates/rustio-admin/assets/static/admin.css` is the single
source of truth for the framework's design tokens. Three
canonical sections live there, each marked with a banner
comment so a diff in those blocks is visually loud:

```
=== Tokens — colors ===
=== Tokens — typography ===
=== Tokens — spacing ===
```

### 8.1 What belongs in framework tokens

- `--rio-accent` and friends — the brand colour
- `--rio-bg`, `--rio-surface*` — the surface ladder
- `--rio-text*` — the slate text scale
- `--rio-border*` — outline weights
- `--rio-success / --rio-warning / --rio-danger` — semantic colours
- `--rio-fs-*` — typography scale (12 / 14 / 15 / 16 / 17 / 19 / 24 / 30 px)
- `--rio-font-sans / --rio-font-arabic / --rio-font-mono` — font stacks
- `--rio-s1`–`--rio-s7` — spacing scale (4 / 8 / 12 / 16 / 24 / 32 / 48 px)
- `--rio-radius / --rio-shadow*` — radii + shadows

Touching any of these is a design-system change. Read the PR
template's "Token disclosure" rules before opening the PR.

### 8.2 What does NOT belong in projects

A project must **not** redefine framework tokens in its own
stylesheet:

```css
/* ❌ Wrong — silently forks the framework's accent */
:root {
  --rio-accent: #0F8C7E;
}
```

Two right answers:

```rust
// ✅ Typed override at boot. Emits a single _theme.html style block.
let admin = Admin::new()
    .accent_color("#0F8C7E");
```

```css
/* ✅ Read framework tokens; never redefine them. */
.my-card {
  background: var(--rio-surface);
  border: 1px solid var(--rio-border);
}
```

The `accent_color()` builder writes one `<style>` block in
`_theme.html` after `admin.css` so it wins the cascade
without `!important` and without duplicating the variable
definition. There is exactly one source of truth per token
at any time.

### 8.3 What projects MAY do

- Add new component classes (`.my-feature-tile`) that
  **read** framework tokens via `var(--rio-*)`.
- Define **new** project-local tokens prefixed `--bsk-*` /
  `--app-*` / `--<project>-*` (see the dashboard.css
  `--bsk-c-*` operational-palette tokens for an example).
- Override `Admin::accent_color()` if the project's brand
  requires a non-default accent.

Projects must **not** redefine `--rio-*` variables in CSS.

---


## 9. Canonical accent palette

The framework default since 0.3.0 is teal-emerald.

| Scope                                  | `--rio-accent` | `--rio-accent-hover` | `--rio-accent-rgb` |
|----------------------------------------|----------------|----------------------|---------------------|
| Page canvas (default)                  | `#0F8C7E`      | `#0A6E62`            | `15 140 126`        |
| Chrome scope (topbar/sidebar/footer)   | `#3FAA9D`      | `#5FBFB3`            | `63 170 157`        |

The chrome-scope lift exists because the base teal muddles to ~3:1
contrast against the `#1F2A37` slate; the lifted variant restores
~5.3:1 while staying in the same hue family. The lift is applied via
the cascade inside `layout/shell.css`, not via a separate theme.

The previous terracotta accent (`#A0341A` / `#C84934`) was
retired in 0.3.0. It survives only as a comment reference in
`admin.css`. Do not reintroduce red-family accents unless
the framework identity changes deliberately.

The teal palette is permanent. It was chosen because it is
calmer and more operational for long admin sessions, holds
contrast cleanly across the surface ladder, and reads cleaner
with the Geist + Tajawal + Noto Naskh typography stack. Branding
overrides via `Admin::accent_color()` are still supported
per project, but the framework default does not re-shift.

---


## 10. Typography philosophy

Three families; one consistent ladder. Both Latin and Arabic
stacks are declared so the browser's font resolution is
deterministic on every page.

| Role            | Family               | Notes                                       |
|-----------------|----------------------|---------------------------------------------|
| Latin UI        | **Geist** (variable) | Variable axis 100–900; UI surface default   |
| Arabic UI       | **Tajawal**          | Static 400/500/700; compact admin surfaces  |
| Arabic body     | **Noto Naskh Arabic** (variable) | Paragraph long-read fallback        |
| Code / mono     | **Geist Mono**       | Tabular figures; all `<code>` and `<pre>`   |

All four families live under `assets/static/fonts/` and are
served as `woff2-variations` (Geist / Geist Mono / Noto
Naskh) or static `woff2` (Tajawal). The fallback chain is
documented inline in `admin.css` at the `--rio-font-*` token
block.

### 10.1 Arabic typography rules

- **Compact admin surfaces** (buttons, navbars, sidebar
  labels, table cells, badges, form labels) → Tajawal first,
  Noto Naskh as graceful fallback.
- **Paragraph body** (long-read help text, descriptions) →
  Noto Naskh Arabic first.
- **Numbers and code** stay in Geist Mono regardless of the
  surrounding script so timestamps, counts, and identifiers
  align in tabular columns.
- **Never** allow Arabic text to land on a Latin face. The
  browser picks the first family in the fallback chain;
  both Latin and Arabic stacks are declared so resolution
  is deterministic.

### 10.2 Size ladder

The framework ladder is `--rio-fs-xs` (12px) through
`--rio-fs-3xl` (30px). Components must resolve through these
tokens, not pick literal `font-size: 14px;` values. A
project that needs a tighter or looser scale (POS terminal
on a reflective display, density-tuned dashboard) overrides
via its own project-scoped class wrapper, never by
redefining the framework size tokens.

---


## 11. Branch + merge expectations

Visual changes converge on a single canonical branch. The
framework refuses to accumulate parallel visual systems
across feature branches; this is exactly the bug that
produced the visual regression in the 0.3.0 cycle.

### 11.1 The framework has one canonical branch — `main`

Every feature branch eventually merges into `main`.
Concretely:

- A feature branch may diverge from `main` for the duration
  of its review.
- Once merged, the merge commit becomes the new floor for
  every subsequent feature branch.
- A feature branch must not be cut from another feature
  branch unless the parent is on a clear path to landing in
  the same release window.
- If two open branches change overlapping CSS or tokens,
  they must cross-coordinate before either merges — not
  after.

### 11.2 PR review responsibilities

Any PR that touches one of the following triggers the
**Token disclosure** section of the PR template:

- `crates/rustio-admin/assets/static/admin.css`
- Any file containing `--rio-*` token definitions
- Font family declarations / `@font-face` blocks
- Any `:root { ... }` block

The PR author must list:

1. Which tokens changed
2. The visual migration impact for downstream projects
3. The visual regression risk (which surfaces would look
   different)

The visual checklist appendix B walks the surfaces a
reviewer must visit before approving. If the PR cannot pass
that checklist, it does not merge.

---


## 12. Troubleshooting

When the UI looks wrong, work the table below before reaching
for token edits. Most regressions resolve at the project-CSS
or template-shadowing layer, not in `admin.css`.

| Symptom                                  | First place to check                                              |
|------------------------------------------|-------------------------------------------------------------------|
| Accent looks "old" / red                 | Browser cache. Hard-refresh. If still wrong: project's CSS may have a redundant `:root { --rio-accent: ... }` redefinition (see §8.2). |
| Permission matrix is a flat checkbox list | The framework version is older than 0.3.0, OR `group_edit.html` is being shadowed by a project template. |
| Arabic text rendering as Latin glyphs    | Page is missing the `lang="ar"` / `dir="rtl"` attribute, or a project has redefined `--rio-font-arabic`. |
| FK columns rendering as numeric IDs      | Model is missing `#[rustio(belongs_to = "X", display = "Y")]`, or list-cell hydration is being skipped because the relation registry is empty. |
| Buttons look full-bleed accent everywhere | Project violated §8 — never flood-fill chrome with the accent. The accent is a call-to-action signal, not a panel colour. |

---


## Appendices


### A. Locked decisions

Carry from the design doctrine. Do not re-litigate.

| Decision | Value | Override path |
|----------|-------|---------------|
| Accent palette | **Teal-emerald** (`#0F8C7E` light / `#3FAA9D` dark). Permanent since 0.3.0 | `Admin::accent_color("#…")` per project |
| Retired accent | **Terracotta** (`#A0341A` / `#C84934`). Do not reintroduce | None — framework identity decision |
| Typography stack | **Geist** (Latin UI), **Tajawal** (Arabic UI), **Noto Naskh Arabic** (Arabic body), **Geist Mono** (code) | Project-scoped class wrappers may layer; cannot redefine `--rio-font-*` |
| Size ladder | `--rio-fs-xs` (12px) → `--rio-fs-3xl` (30px) | Project-scoped class wrappers; no token redefinition |
| Spacing scale | `--rio-s1` (4px) → `--rio-s7` (48px) | Same |
| Project token prefix | `--bsk-*` / `--app-*` / `--<project>-*` | None — convention |
| Token authority | `crates/rustio-admin/assets/static/admin.css` | None — single source of truth |
| Branch policy | One canonical branch (`main`). Feature branches converge | None — operational |


### B. PR review checklist

Reviewers must walk the visual checklist below before
approving any token-touching PR:

- [ ] Login page
- [ ] Dashboard
- [ ] Tables (list view + bulk select)
- [ ] Forms (create + edit + validation states)
- [ ] Dark mode
- [ ] Arabic rendering on a representative page
- [ ] Mobile width (≤ 480px)
- [ ] Permission matrix (`/admin/groups/<id>/edit`)

If the PR cannot pass that checklist, it does not merge.


### C. Versioning

Token-level changes are visible to every consumer of the
crate. They follow SemVer:

- **Patch** (0.3.0 → 0.3.1): no token changes; component
  CSS only.
- **Minor** (0.3.x → 0.4.0): new tokens added, no semantics
  changed.
- **Major-impact change** (warrants a CHANGELOG-front
  banner): existing tokens get a new value (e.g. accent
  recolouring) or get retired.

Projects pin to a major.minor in `Cargo.toml`. The framework
will not silently change the visible accent or text colour
inside a patch release.
