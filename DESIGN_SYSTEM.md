# RustIO Admin Design System

The framework owns the visual + authority vocabulary. Projects compose business
behaviour on top of that vocabulary. This document is the canonical reference
for both layers.

If a feature branch silently changes anything in this file or in the token
sections it points to, the PR template (`/.github/pull_request_template.md`)
requires the author to flag it. Token drift is the single biggest source of
visual regressions across feature branches.

---

## 1. Authority doctrine

Three principles guide every authorization decision the framework makes. They
predate any individual feature and survive every refactor.

### 1.1 UI hiding is reflection, not security

Every guard runs **server-side on POST**, regardless of what the form said.
Client-side affordances (filtered dropdowns, disabled checkboxes, hidden
buttons) exist to spare the operator a round-trip — never to gate access.

If you remove a UI element to "fix" an authorization problem, you have not
fixed it. The matching guard in `auth/guards.rs` is what fixes it.

### 1.2 Rank controls WHO. Permissions control WHAT.

Roles are a **rank ladder**: User (100) → Staff (300) → Supervisor (600) →
Administrator (900) → Developer (1000). The values are spaced for headroom and
should be compared **relatively**, never matched literally.

Rank decides **who may modify whom** — the cross-rank, role-ceiling, and
self-demote guards in `auth::guards` all answer that question.

Permissions are the granular `<table>.<action>_<singular>` codenames
(`orders.view_order`, `inventory.delete_inventory_item`). They decide **what a
user may do once they are inside the panel**.

A user with rank `Administrator` and zero direct permissions can do everything.
A user with rank `Staff` and the right group memberships can do exactly the
operations those groups grant — and nothing else.

These are **two orthogonal axes**. Conflating them ("Admin = manager group =
some roles") produces the chaotic permission models this framework is
designed to prevent.

### 1.3 Groups are permission bundles, not authority roots

A group is a *named operational set of permissions*. Users belong to zero or
more groups and inherit the union of their permissions. Group membership says
nothing about authority: an Administrator without any group still bypasses
group checks; a Staff user can hold dozens of groups and still cannot edit
another Staff user.

Authority comes from rank + the protected-roles invariant. Groups come from
operational need.

---

## 2. Token ownership

`crates/rustio-admin/assets/static/admin.css` is the single source of truth
for the framework's design tokens. Three canonical sections live there, each
marked with a banner comment so a diff in those blocks is visually loud:

```
=== Tokens — colors ===
=== Tokens — typography ===
=== Tokens — spacing ===
```

### 2.1 What belongs in framework tokens

- `--rio-accent` and friends — the brand colour
- `--rio-bg`, `--rio-surface*` — the surface ladder
- `--rio-text*` — the slate text scale
- `--rio-border*` — outline weights
- `--rio-success / --rio-warning / --rio-danger` — semantic colours
- `--rio-fs-*` — typography scale (12 / 14 / 15 / 16 / 17 / 19 / 24 / 30 px)
- `--rio-font-sans / --rio-font-arabic / --rio-font-mono` — font stacks
- `--rio-s1`–`--rio-s7` — spacing scale (4 / 8 / 12 / 16 / 24 / 32 / 48 px)
- `--rio-radius / --rio-shadow*` — radii + shadows

Touching any of these is a design-system change. Read the PR template's
"Token disclosure" rules before opening the PR.

### 2.2 What does NOT belong in projects

A project must **not** redefine framework tokens in its own stylesheet:

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

The `accent_color()` builder writes one `<style>` block in `_theme.html` after
`admin.css` so it wins the cascade without `!important` and without
duplicating the variable definition. There is exactly one source of truth per
token at any time.

### 2.3 What projects MAY do

- Add new component classes (`.my-feature-tile`) that **read** framework
  tokens via `var(--rio-*)`.
- Define **new** project-local tokens prefixed `--bsk-*` / `--app-*` /
  `--<project>-*` (see the dashboard.css `--bsk-c-*` operational-palette
  tokens for an example).
- Override `Admin::accent_color()` if the project's brand requires a
  non-default accent.

Projects must **not** redefine `--rio-*` variables in CSS.

---

## 3. Canonical accent palette

The framework default since 0.3.0 is teal-emerald.

| Mode  | `--rio-accent` | `--rio-accent-hover` | `--rio-accent-rgb` |
|-------|----------------|----------------------|---------------------|
| Light | `#0F8C7E`      | `#0A6E62`            | `15 140 126`        |
| Dark  | `#3FAA9D`      | `#5FBFB3`            | `63 170 157`        |

The previous terracotta accent (`#A0341A` / `#C84934`) was retired in 0.3.0.
It survives only as a comment reference in `admin.css`. Do not reintroduce
red-family accents unless the framework identity changes deliberately.

The teal palette is permanent. It was chosen because it is calmer and more
operational for long admin sessions, has a stronger contrast story in dark
mode, and reads cleaner with the Geist + Tajawal + Noto Naskh typography
stack. Branding overrides via `Admin::accent_color()` are still supported per
project, but the framework default does not re-shift.

---

## 4. Typography philosophy

Three families; one consistent ladder.

| Role            | Family               | Notes                                       |
|-----------------|----------------------|---------------------------------------------|
| Latin UI        | **Geist** (variable) | Variable axis 100–900; UI surface default   |
| Arabic UI       | **Tajawal**          | Static 400/500/700; compact admin surfaces  |
| Arabic body     | **Noto Naskh Arabic** (variable) | Paragraph long-read fallback        |
| Code / mono     | **Geist Mono**       | Tabular figures; all `<code>` and `<pre>`   |

All four families live under `assets/static/fonts/` and are served as
`woff2-variations` (Geist / Geist Mono / Noto Naskh) or static `woff2`
(Tajawal). The fallback chain is documented inline in `admin.css` at the
`--rio-font-*` token block.

### 4.1 Arabic typography rules

- **Compact admin surfaces** (buttons, navbars, sidebar labels, table cells,
  badges, form labels) → Tajawal first, Noto Naskh as graceful fallback.
- **Paragraph body** (long-read help text, descriptions) → Noto Naskh Arabic
  first.
- **Numbers and code** stay in Geist Mono regardless of the surrounding script
  so timestamps, counts, and identifiers align in tabular columns.
- **Never** allow Arabic text to land on a Latin face. The browser picks the
  first family in the fallback chain; both Latin and Arabic stacks are
  declared so resolution is deterministic.

### 4.2 Size ladder

The framework ladder is `--rio-fs-xs` (12px) through `--rio-fs-3xl` (30px).
Components must resolve through these tokens, not pick literal `font-size:
14px;` values. A project that needs a tighter or looser scale (POS terminal
on a reflective display, density-tuned dashboard) overrides via its own
project-scoped class wrapper, never by redefining the framework size tokens.

---

## 5. Branch + merge expectations

### 5.1 The framework has one canonical branch — `main`

Every feature branch eventually merges into `main`. The framework refuses to
accumulate parallel visual systems across branches; this is exactly the bug
that produced the visual regression in the 0.3.0 cycle.

Concretely:

- A feature branch may diverge from `main` for the duration of its review.
- Once merged, the merge commit becomes the new floor for every subsequent
  feature branch.
- A feature branch must not be cut from another feature branch unless the
  parent is on a clear path to landing in the same release window.
- If two open branches change overlapping CSS or tokens, they must
  cross-coordinate before either merges — not after.

### 5.2 PR review responsibilities

Any PR that touches one of the following triggers the **Token disclosure**
section of the PR template:

- `crates/rustio-admin/assets/static/admin.css`
- Any file containing `--rio-*` token definitions
- Font family declarations / `@font-face` blocks
- Any `:root { ... }` block

The PR author must list:

1. Which tokens changed
2. The visual migration impact for downstream projects
3. The visual regression risk (which surfaces would look different)

Reviewers must walk the visual checklist below before approving:

- [ ] Login page
- [ ] Dashboard
- [ ] Tables (list view + bulk select)
- [ ] Forms (create + edit + validation states)
- [ ] Dark mode
- [ ] Arabic rendering on a representative page
- [ ] Mobile width (≤ 480px)
- [ ] Permission matrix (`/admin/groups/<id>/edit`)

If the PR cannot pass that checklist, it does not merge.

---

## 6. Versioning of the design system

Token-level changes are visible to every consumer of the crate. They follow
SemVer:

- **Patch** (0.3.0 → 0.3.1): no token changes; component CSS only.
- **Minor** (0.3.x → 0.4.0): new tokens added, no semantics changed.
- **Major-impact change** (warrants a CHANGELOG-front banner): existing
  tokens get a new value (e.g. accent recolouring) or get retired.

Projects pin to a major.minor in `Cargo.toml`. The framework will not
silently change the visible accent or text colour inside a patch release.

---

## 7. Where to look when the UI looks wrong

| Symptom                                  | First place to check                                              |
|------------------------------------------|-------------------------------------------------------------------|
| Accent looks "old" / red                 | Browser cache. Hard-refresh. If still wrong: project's CSS may have a redundant `:root { --rio-accent: ... }` redefinition (see §2.2). |
| Permission matrix is a flat checkbox list | The framework version is older than 0.3.0, OR `group_edit.html` is being shadowed by a project template. |
| Arabic text rendering as Latin glyphs    | Page is missing the `lang="ar"` / `dir="rtl"` attribute, or a project has redefined `--rio-font-arabic`. |
| FK columns rendering as numeric IDs      | Model is missing `#[rustio(belongs_to = "X", display = "Y")]`, or list-cell hydration is being skipped because the relation registry is empty. |
| Buttons look full-bleed accent everywhere | Project violated §2 — never flood-fill chrome with the accent. The accent is a call-to-action signal, not a panel colour. |

---

## 8. Closing principle

The framework is no longer "a CRUD admin". It is becoming an
**authority-oriented admin platform** — a stack that takes seriously who
controls whom, who can change what, who can recover whose access, and how the
audit trail of those decisions survives staff turnover.

Every design decision should be measured against that mission. A change is
worth shipping if it makes authority clearer, the boundaries safer, or the
operator's intent more visible. A change that adds visual noise, conflates
roles with permissions, or buries the rank model behind a clever abstraction
is moving in the wrong direction — even if the diff looks elegant.
