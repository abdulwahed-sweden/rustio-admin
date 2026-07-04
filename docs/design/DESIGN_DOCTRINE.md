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

> **Values live in the contract, not here.** As of the Visual Contract v2.0
> rollout, all concrete token **values** — colors, the type scale, fonts,
> light/dark — are owned by [`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md). This
> document keeps the *principles, architecture, and rationale*; where it used to
> restate hex/px/font values it now points there, so a rebrand changes one file.
> If a number below and the contract disagree, the contract wins.

---

## 1. Token philosophy

Five variable groups, one source of truth per group:

| Group       | File                          | Purpose                                       |
|-------------|-------------------------------|-----------------------------------------------|
| Colors      | `tokens/colors.css`           | Accent, surface ladder, slate text, semantics |
| Spacing     | `tokens/spacing.css`          | 4 / 8 / 12 / 16 / 24 / 32 / 48 px scale       |
| Radius      | `tokens/radius.css`           | `sm` 6 · `control` 8 · `md` 9 · `lg` 12 · `xl` 16 |
| Shadows     | `tokens/shadows.css`          | sm / md / lg / xl + card / inset — quiet      |
| Typography  | `tokens/typography.css`       | Fonts, sizes, line-heights, weights, tracking |

Three rules:

1. **One canonical value per token, per theme block.** The framework ships
   **light and dark**, token-driven only — light in `:root`, dark re-derived in
   the `@media (prefers-color-scheme: dark)` and `[data-theme="dark"]` blocks of
   `tokens/colors.css`. No per-component dark CSS (see
   [`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md) §12).
2. **No hard-coded colours, spacing, or font sizes outside `tokens/`.**
   Every component resolves through `var(--rio-*)`. Projects override the
   framework by patching the token blocks from their own theme file; if a
   component bakes in `#ffffff`, that override silently fails.
3. **New token = CHANGELOG entry.** Tokens are public API; a new `--rio-*` token
   ships under a "Tokens" CHANGELOG note so branches can't drift the palette.

The brand accent is **rust** — its canonical value (`--rio-accent`) lives in
[`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md) §1, not restated here. It is reserved
for affordances — primary buttons, focus rings, active state, links — and **never
flood-filled across page chrome**; surfaces stay neutral so the accent keeps its
weight as a call-to-action.

---

## 2. Typography system

| Context     | Family                | When                                    |
|-------------|-----------------------|-----------------------------------------|
| Latin UI    | **Inter** Variable    | Default — body, headings, controls       |
| Code / mono | SFMono system stack   | `code`, `pre`, datetime cells, IDs (self-hosted JetBrains Mono stays baked for brand overrides) |
| Arabic UI   | Tajawal (400/500/700) | `lang="ar"` / `dir="rtl"` on UI surfaces|
| Arabic body | Noto Naskh Variable   | `lang="ar"` paragraphs, prose, help     |

Latin faces are self-hosted from the binary (`@font-face` in `base/fonts.css`);
**no CDN round-trip, no FOUT, no GDPR/tracking surface.** Arabic faces are gated
by `unicode-range`, so a Latin-only page pays zero download for them. The exact
font stacks and the type scale are owned by
[`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md) §2 — body/labels/inputs/tables at 16px,
legends/headers/kbd at 14px, page titles 36px, and a hard **14px floor: no
content-area UI text below 14px.** (The prior Geist / Spectral / Hanken faces and
the 13px floor were retired with the contract.)

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

Inter reads refined with a hair of negative tracking at display and UI sizes;
the exact `--rio-tracking-*` values live in `tokens/typography.css`. Arabic
resets to 0 — applied automatically to anything tagged `:lang(ar)` / `[dir="rtl"]`
in `base/base.css`, and to `.rio-fieldset > legend` / `.rio-table th` per the
contract §12 (connected script breaks under tracking).

---

## 3. Surface hierarchy

Surfaces lift in small steps from page canvas to popovers — depth comes from
*layering*, not drop shadow. The canonical light values (`--rio-bg` `#fafcfb`,
`--rio-surface` white, the code-chip tint, and the dark-theme inversions) are
owned by [`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md) §1 (and §12 for dark).
Principles that hold regardless of value:

- Never pure white, never pure black.
- Chrome (topbar / sidebar / footer) sits on a distinct deep-slate surface so the
  operator skeleton reads without conscious attention. The chrome stays dark in
  both light and dark themes.
- Tables carry **no zebra** — rows separate by soft dividers and hover, not striping.

Borders are two weights (contract §1): a **soft** card/divider border
(`--rio-border-soft`) and a **strong** input/control border (`--rio-border-input`)
that keeps fields clearly outlined rather than melting into the card.

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

## 5. Light and dark

The framework ships **both themes, token-driven only** (contract §12). Light is
the default `:root`; dark is re-derived in the `@media (prefers-color-scheme:
dark)` (auto) and `[data-theme="dark"]` (explicit toggle) blocks of
`tokens/colors.css` — the auto block is placed before the explicit one so an
explicit theme wins by source order. There is **no per-component dark CSS**:
components reference tokens, and only the token blocks carry per-theme values.

- **Slate dark, not charcoal.** Dark inverts the surface ladder to a slate family
  and lifts the accent so it clears AA on dark surfaces; body text clears 4.5:1
  on its background. Exact dark values live in
  [`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md) §12 + `tokens/colors.css`.
- **One re-derivation per theme block to audit.** Because dark is token-only, the
  WCAG pairings are checked per block, not per component — no parallel component CSS.
- **Projects rebrand via the token blocks.** A generated `tokens.css` override
  must itself be dark-aware ([`TOKENS-EMIT-SPEC.md`](TOKENS-EMIT-SPEC.md)); a
  light-only override leaks light into dark mode, and the runtime logs a WARN
  naming the file at startup.

---

## 6. Operational UI principles

`rustio-admin` is operator software. It exists to keep an admin productive
through a ten-hour shift — not to convert a free-trial user. Concretely:

1. **Calm over flashy.** Buttons swap surface colour on hover instead of
   dimming with opacity. Tables separate rows with soft dividers and a quiet
   hover — **no zebra striping**, no accent-tinted overlay. Cards layer with
   borders, not glow.
2. **Reserve the accent for affordances.** Anything the user can act on
   may wear the rust accent; anything that's just content stays neutral.
   If everything is accented, nothing is.
3. **Mobile-first, three breakpoints.** `< 768 px` collapses the sidebar
   off-canvas behind a hamburger; `≥ 768 px` pins it; `≥ 1280 px` widens
   it to 260 px and caps `.rio-main` at 1280 px so a 4 K monitor doesn't
   stretch a table row across the user's whole field of view.
4. **One canonical accent across every admin page.** Projects override
   exactly one rust value to rebrand.
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
   16 px, the floor at **14 px** (contract §2 — no content-area text below it),
   table cells at 16 px, line-height at 1.6. Density is achieved with `gap` and
   surface contrast, not by shrinking text.
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

```text
crates/rustio-admin/assets/static/admin/
├── admin.css            ← contributor-facing @import manifest
├── tokens/              ← single source of truth for the visual scale
│   ├── colors.css       ← light :root + dark @media / [data-theme] blocks
│   ├── compat.css       ← contract-name aliases onto the engine tokens
│   ├── spacing.css · radius.css · shadows.css · motion.css · typography.css
├── base/
│   ├── base.css         ← element defaults, body, headings, RTL neutralisation
│   ├── fonts.css        ← @font-face (Inter, JetBrains Mono, Arabic faces)
│   └── typography-i18n.css ← lang-gated CJK / Thai / Devanagari faces
├── layout/
│   └── console.css      ← the console shell: rail, masthead, board, ledger
├── components/          ← reusable UI primitives
│   ├── buttons.css · forms.css · data.css (tables/pills/cards) · code.css
│   ├── feedback.css · navigation.css
├── pages/               ← screen-specific overrides
│   ├── form.css · list.css · detail.css · account.css · auth.css
│   ├── dashboard.css · permissions.css · states.css · tools.css
└── print/print.css
```

**Import order matters:** tokens before base before layout before components
before pages before print. The exact order is owned by the `@import` manifest in
`admin/admin.css` and the `ADMIN_CSS` concat in `routes.rs` — kept in lock-step
(a test enforces it). If a change depends on cascade order, document it inline.

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
