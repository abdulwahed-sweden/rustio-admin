# RustIO Admin — Design Doctrine

This document is the operator's manual for `rustio-admin`'s visual identity.
It captures the design decisions that have piled up across releases and
explains *why* they look the way they do — so contributors don't accidentally
redesign the framework one rule at a time.

The stylesheet source lives under `crates/rustio-admin/assets/static/admin/`,
organized as a Primer/Carbon-style multi-file architecture. The runtime
serves a single concatenated bundle at `/static/admin.css` — see `ADMIN_CSS`
in `src/admin/routes.rs` for the assembly order. The order matches
`admin/admin.css`'s `@import` manifest line-for-line; **both must be kept
in lock-step.**

---

## 1. Token philosophy

Five variable groups, one source of truth per group:

| Group       | File                          | Purpose                                       |
|-------------|-------------------------------|-----------------------------------------------|
| Colors      | `tokens/colors.css`           | Accent, surface ladder, slate text, semantics |
| Spacing     | `tokens/spacing.css`          | 4 / 8 / 12 / 16 / 24 / 32 / 48 px scale       |
| Radius      | `tokens/radius.css`           | `sm` (6), default (10), `lg` (14)             |
| Shadows     | `tokens/shadows.css`          | xs / default / lg — deliberately quiet        |
| Typography  | `tokens/typography.css`       | Fonts, sizes, line-heights, weights, tracking |

Three rules:

1. **One canonical value per token in `:root`.** The framework is
   light-only — there are no dark or alternate-mode overrides anywhere.
2. **No hard-coded colours, spacing, or font sizes outside `tokens/`.**
   Every component resolves through `var(--rio-*)`. Projects override the
   framework by patching `:root` from their own theme file; if a component
   bakes in `#ffffff`, that override silently fails.
3. **New token = CHANGELOG entry.** Tokens are public API. A new
   `--rio-accent-hover-2` (or whatever) ships under "Tokens — colors"
   so feature branches can't silently drift the palette.

The brand accent is **teal-emerald `#0F8C7E`**, lifted to **`#3FAA9D`**
inside the deep-slate chrome (topbar / sidebar / footer) where the base
hue muddles to ~3:1 contrast. It is reserved for affordances — primary
buttons, focus rings, active state, links, brand emblem. **It is never
flood-filled across page chrome.** Surfaces stay neutral so the accent
retains its weight as a call-to-action.

The crimson `#A0341A` used pre-0.3.0 was retired so projects can override
exactly one teal value to rebrand.

---

## 2. Typography system

Three families, one scale:

| Context     | Family                | When                                    |
|-------------|-----------------------|-----------------------------------------|
| Latin UI    | Geist Variable        | Default — body, headings, controls      |
| Code        | Geist Mono Variable   | `code`, `pre`, datetime cells, IDs      |
| Arabic UI   | Tajawal (400/500/700) | `lang="ar"` / `dir="rtl"` on UI surfaces|
| Arabic body | Noto Naskh Variable   | `lang="ar"` paragraphs, prose, help     |

All four are self-hosted from the binary (see `@font-face` in
`base/typography.css`). The font binaries ship inside the `rustio-admin`
crate; **there is no CDN round-trip, no FOUT, and no GDPR/tracking surface.**
Arabic faces are gated by `unicode-range`, so a Latin-only page pays zero
download cost for the Arabic shapers.

### Size scale (rem-based, 16 px html root)

| Token              | Pixels  | Used for                                  |
|--------------------|---------|-------------------------------------------|
| `--rio-fs-xs`      | 13      | kbd, footer, tiny meta, table headers     |
| `--rio-fs-sm`      | 14.4    | hint text, datetime cells, pagination     |
| `--rio-fs-md`      | 15      | sidebar, button label, nav link           |
| `--rio-fs-base`    | 16      | body, table cell                          |
| `--rio-fs-lg`      | 17      | main prose body                           |
| `--rio-fs-xl`      | 20      | section h3                                |
| `--rio-fs-h3`      | 22      | h3                                        |
| `--rio-fs-h2`      | 26      | h2                                        |
| `--rio-fs-h1`      | 30      | page title (calmer than 34 px)            |
| `--rio-fs-display` | 36      | login title                               |

The floor is **13 px** for true micro-text only. Everyday surfaces land
between 14 and 16 px so a ten-hour operator session never strains.

### Line height is tuned per script

| Token              | Value | Use                                  |
|--------------------|-------|--------------------------------------|
| `--rio-lh-tight`   | 1.25  | h1 / h2 / h3, big display            |
| `--rio-lh-ui`      | 1.5   | dense UI — buttons, tags, table rows |
| `--rio-lh-body`    | 1.65  | English paragraph body               |
| `--rio-lh-arabic`  | 1.95  | Arabic paragraph body                |

Arabic gets 1.95 because Naskh letterforms hang above and below the
baseline, and 1.5 doesn't give them room.

### Tracking (Latin only)

Geist is drawn for slight negative tracking on display sizes:
`--rio-tracking-display: -0.022em`, `heading: -0.012em`, `body: -0.003em`.
Arabic resets to 0 — applied automatically to anything tagged
`:lang(ar)` / `[dir="rtl"]` in `base/typography.css`.

---

## 3. Surface hierarchy

Surfaces lift in **2–4 % steps** from page canvas up to popovers. Depth comes
from *layering*, not from drop shadow. Five rungs (light mode):

| Token             | Hex       | Lives on                                          |
|-------------------|-----------|---------------------------------------------------|
| `--rio-bg`        | `#EEF1F6` | Page canvas                                       |
| `--rio-surface`   | `#FFFFFF` | Cards, topbar, sidebar, table body, inputs        |
| `--rio-surface-2` | `#F7F9FC` | Table head, zebra stripe, panel base              |
| `--rio-surface-3` | `#EFF2F7` | Hover, pressed state, secondary chip              |
| Accent wash       | 6–10 % α  | Selected row, active filter, sidebar `.is-active` |

Never pure white, never pure black. The deepest surface in the
framework is the chrome floor `#1F2A37` — desaturated blue-slate, not
`#000` or `#111` — used for the topbar, sidebar, and footer.

Borders work in three weights:

| Token                  | Use                                              |
|------------------------|--------------------------------------------------|
| `--rio-border-soft`    | In-card row dividers — almost invisible          |
| `--rio-border`         | Card outlines, default divider                   |
| `--rio-border-strong`  | Hover state, focused control                     |

### Shadow scale

Shadows are quiet by design. Use `--rio-shadow-xs` for buttons,
`--rio-shadow` for cards, and `--rio-shadow-lg` *only* for transient
overlays (dropdown panels, modal-equivalent surfaces). Premium tooling
prefers borders + surface contrast over drop shadow; if you reach for
`box-shadow` for emphasis, reach for a darker border first.

---

## 4. Spacing scale

Seven steps, 4 → 48 px:

| Token | rem    | px |
|-------|--------|----|
| `s1`  | 0.25   | 4  |
| `s2`  | 0.5    | 8  |
| `s3`  | 0.75   | 12 |
| `s4`  | 1      | 16 |
| `s5`  | 1.5    | 24 |
| `s6`  | 2      | 32 |
| `s7`  | 3      | 48 |

Component rhythm uses `gap` on flex containers rather than per-element
margins. The form rule `gap: var(--rio-s5)` on `.rio-form` is canonical: it
spaces consecutive fields without double-margin collapse surprises and
without per-field margin accounting.

Two shell reservations live in `tokens/spacing.css` because some component
rules need to compute against them:

- `--rio-sidebar-w`: 240 px (260 px ≥ 1280 px)
- `--rio-topbar-h`: 64 px

---

## 5. Light-only stance

The framework ships a **single, light palette**. No dark variant, no
`prefers-color-scheme` media block, no theme toggle, no
`data-rio-theme` attribute. The single `:root` block in
`tokens/colors.css` is the only colour definition the framework owns.

Why light-only:

- **One palette to audit.** WCAG contrast pairs are checked once.
  A dark variant doubled the surface area where token drift could
  introduce a regression invisible to half the userbase.
- **Operator software, not consumer.** Admin sessions are short and
  bright-office by default; the cost of maintaining a parallel scale
  was higher than the value it returned.
- **Projects that need dark can override `:root`.** The token surface
  is the same; a project stylesheet patching `--rio-bg`,
  `--rio-surface*`, `--rio-text*`, `--rio-border*` can re-skin to a
  dark palette without forking the framework.

The accent lifts from `#0F8C7E` to `#3FAA9D` **inside the deep-slate
chrome surfaces only** (topbar / sidebar / footer — see
`layout/shell.css`). On the light page canvas the base accent has
sufficient contrast; inside chrome it muddles, hence the lifted
variant. The lift is scoped via CSS custom-property cascade through
`.rio-topbar, .rio-sidebar, .rio-footer`, not a separate theme.

---

## 6. Operational UI principles

`rustio-admin` is operator software. It exists to keep an admin productive
through a ten-hour shift — not to convert a free-trial user. Concretely:

1. **Calm over flashy.** Buttons swap surface colour on hover instead of
   dimming with opacity. Tables zebra-stripe with neutral surface-2, not
   an accent-tinted overlay. Cards layer with borders, not glow.
2. **Reserve the accent for affordances.** Anything the user can act on
   may wear teal; anything that's just content stays neutral. If
   everything is teal, nothing is.
3. **Mobile-first, three breakpoints.** `< 768 px` collapses the sidebar
   off-canvas behind a hamburger; `≥ 768 px` pins it; `≥ 1280 px` widens
   it to 260 px and caps `.rio-main` at 1280 px so a 4 K monitor doesn't
   stretch a table row across the user's whole field of view.
4. **One canonical accent across every admin page.** Projects override
   exactly one teal value to rebrand.
5. **Reuse before invention.** The `.rio-dropdown` machinery is generic
   on purpose: filters, sort menus, per-page pickers, and future column
   togglers all live on top of it. Don't bake a one-off floating panel
   for the next feature; extend the primitive.
6. **URL is the source of truth.** Filters, sort, page, search all live
   in the query string. Chips and dropdown items are anchors, not form
   controls — clicking commits without an Apply step. JS only adds
   `is-open` / `is-active` decoration; remove the JS and the framework
   still works (mostly).
7. **No marketing surfaces.** Sessions, MFA enrolment, recovery codes —
   all read like a settings page, not a SaaS auth dashboard. No hero,
   no gradient, no "secure your account" illustration.
8. **Operator readability first.** The default body font size sits at
   16 px, the floor at 13 px, table cells at 16 px, line-height at 1.6.
   Density is achieved with `gap` and surface contrast, not by shrinking
   text.
9. **Deeper surface ladder.** Adjacent surfaces sit ≥ 4 % apart so the
   eye never squints to tell canvas from card from table-header from
   row-hover. Added v0.15.0; details in
   [`PLAN_VISUAL_v2.md`](../archive/PLAN_VISUAL_v2.md). The surface scale
   carries six rungs (`--rio-bg` → `--rio-surface` → `--rio-surface-2` →
   `--rio-surface-3` → `--rio-surface-chrome` → `--rio-surface-elevated`).
10. **Chrome carries weight.** Topbar, sidebar, and footer render on
    `--rio-surface-chrome` — a surface that is visually distinct from
    both card and canvas so the operator skeleton is visible without
    conscious attention. Direction is a project-aesthetic choice:
    chrome between card and canvas (subtle frame, v0.15.0 default) or
    chrome darker than canvas (dark-frame, v0.15.1 default — preferred
    for premium operator-software feel). When chrome goes dark in
    light mode, a chrome-scope cascade in `layout/shell.css` flips
    `--rio-text-*`, `--rio-surface-2/3`, `--rio-border-*`, and
    `--rio-accent` to light-on-dark variants for every descendant —
    no per-component edits needed. Added v0.15.0; reframed v0.15.1.
11. **Typography hierarchy is a weight choice, not just a size.**
    Display sizes (h1, h2, login title) declare gravity through weight
    700–800 *and* tracking that reads as deliberate. Body and table
    cells stay at 400 for ten-hour-shift legibility. The middle ground
    at 600 is reserved for specific UI affordances (active nav, button
    label, table-row primary cell). Added v0.15.0.

---

## 7. Source layout

```
crates/rustio-admin/assets/static/admin/
├── admin.css            ← contributor-facing @import manifest
├── tokens/              ← single source of truth for the visual scale
│   ├── colors.css
│   ├── spacing.css
│   ├── radius.css
│   ├── shadows.css
│   └── typography.css
├── base/                ← reset + body + headings + utilities
│   ├── reset.css
│   ├── base.css
│   ├── typography.css   ← @font-face lives here
│   └── utilities.css
├── layout/              ← page-level shell
│   ├── shell.css
│   ├── topbar.css
│   ├── sidebar.css
│   ├── footer.css
│   └── responsive.css   ← imported AFTER all components
├── components/          ← reusable UI primitives
│   ├── cards.css
│   ├── buttons.css
│   ├── forms.css
│   ├── tables.css
│   ├── filters.css
│   ├── dropdowns.css
│   ├── pagination.css
│   ├── pills.css
│   ├── flashes.css
│   ├── timeline.css
│   └── tabs.css
├── pages/               ← screen-specific overrides
│   ├── auth.css
│   ├── dashboard.css
│   ├── permissions.css
│   ├── sessions.css
│   └── errors.css
└── print/
    └── print.css
```

**Import order matters:** tokens before base before layout before
components before pages before responsive before print.
`responsive.css` is the only file in `layout/` that loads late on purpose
— its `display: none` sidebar rule is meant to override the desktop layout
in `sidebar.css`. If your change depends on cascade order, document it
inline so the next contributor doesn't innocently break it.

---

## 8. How the bundle is delivered

The framework binary concatenates every fragment at compile time and
serves one bundle at `/static/admin.css`. The mechanism is a single
`concat!(include_str!(…), …)` block in `src/admin/routes.rs` — see
`ADMIN_CSS`. There is no build step, no bundler, no PostCSS, no SCSS.
Pure CSS, baked into the rustio-admin binary at compile time.

This keeps the framework's deploy story intact: **one binary, no CDN
round-trip, no FOUT, no third-party fetches.**

---

## 9. Adding a new fragment

1. Drop a new `.css` file in the appropriate subdirectory.
2. Add a section header at the top of the file (see existing files for the
   `============` block template). Header should explain what the file
   contains and any notable cascade dependencies.
3. Add an `@import url("…")` line to `admin/admin.css` at the right
   cascade position.
4. Add the matching `include_str!(…)` line to `ADMIN_CSS` in
   `src/admin/routes.rs` **at the same position**. The two lists must stay
   in lock-step or the served bundle will silently drift from the
   manifest.
5. If the fragment introduces a new token, add a CHANGELOG entry under
   the appropriate "Tokens — *" heading.

---

## 10. What this document is not

- **Not a style guide for designers.** RustIO ships one identity; designs
  that want a different one fork the framework or override `:root` from
  their own theme file.
- **Not a usage manual for end-users** — that's the project README.
- **Not a complete enumeration of every CSS rule.** The files are the
  source of truth; this document explains the *why* behind their shape.
