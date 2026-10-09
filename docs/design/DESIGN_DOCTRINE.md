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

> **Values live in the contract, not here.** As of the Visual Contract v3.0
> rollout, all concrete token **values** — colors, the type scale, fonts,
> the control and row scale — are owned by
> [`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md) (v3.0, Composition 3.1). This
> document keeps the *principles, architecture, and rationale*; where it used to
> restate hex/px/font values it now points there, so a rebrand changes one file.
> If a number below and the contract disagree, the contract wins.

---

## 1. Token philosophy

Five variable groups, one source of truth per group:

| Group       | File                          | Purpose                                       |
|-------------|-------------------------------|-----------------------------------------------|
| Colors      | `tokens/colors.css`           | Accent, surface ladder, the ink ramp, semantics |
| Spacing     | `tokens/spacing.css`          | 4 / 8 / 12 / 16 / 24 / 32 px scale             |
| Radius      | `tokens/radius.css`           | `sm` 6 · `control` 8 · card 14 (contract §1.7) |
| Shadows     | `tokens/shadows.css`          | sm (cards in a grid) · lg (overlays) — flat by default |
| Typography  | `tokens/typography.css`       | Fonts, sizes, line-heights, weights, tracking |

Three rules:

1. **One canonical value per token.** The framework is **light-only**: a single
   `:root` block in `tokens/colors.css`, `color-scheme: light`, and no
   `@media (prefers-color-scheme: dark)` block, no `[data-theme]` block, and no
   pre-paint theme script anywhere (see
   [`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md) §12). Canonical names carry the
   values and every `--rio-*` name is a permanent alias onto them, because
   `AdminTheme`, `rio-theme` and downstream projects target the `--rio-*`
   contract (contract §1).
2. **No hard-coded colours, spacing, or font sizes outside `tokens/`.**
   Every component resolves through `var(--rio-*)`. Projects override the
   framework by patching the token blocks from their own theme file; if a
   component bakes in `#ffffff`, that override silently fails.
3. **New token = CHANGELOG entry.** Tokens are public API; a new `--rio-*` token
   ships under a "Tokens" CHANGELOG note so branches can't drift the palette.

The brand accent is **RustIO Blue** — its canonical value lives in
[`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md) §1.3, not restated here. (The
`--rio-rust*` token names are kept deliberately as the established override
contract; the *name* is historical, the *value* is blue. The burnt-copper accent
of Visual Contract v2.x is retired.) It is reserved for affordances — primary
buttons, active state, links, dots, tints — and **never flood-filled across page
chrome**; surfaces stay neutral so the accent keeps its weight as a
call-to-action.

**Focus is a separate role, not a shade of the accent.** `--rio-accent-focus`
carries its own value with its own contrast budget and is never re-pointed at the
primary — including by `AdminTheme`, which overrides the accent and leaves focus
alone (contract §1.3, §15.2).

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
[`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md) §2 — body/labels/inputs/table cells at
**15px**, mono micro-labels (table heads, fieldset bands, `dl` labels, eyebrows)
at **13px**, and page titles at **24px**. There is **no size floor**: 13px is the
family's mono/micro size and 12px is the ladder's bottom step. What replaces it is
a *role and contrast* floor — operational text may not be pale, and 13px is
reserved for the mono/micro roles rather than for prose (contract §1.2, §2.1).
(The prior Geist / Spectral / Hanken faces were retired with the contract; the
v2.x 14px floor and 36px title were repealed in v3.0.)

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
*layering*, not drop shadow. The canonical values (the page ground, `--rio-surface`
white, the hover/head rungs, the code-chip tint) are owned by
[`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md) §1.1. Principles that hold regardless
of value:

- Never pure white as a page ground, never pure black as ink.
- Chrome (topbar / sidebar / footer) sits on its own surface so the operator
  skeleton reads without conscious attention. It is a *distinct* surface, not a
  dark one: the admin is light throughout (§5).
- Cards are **flat** — a white card on the page ground with a 1px border is
  already separated. A shadow is reserved for cards in a grid and for the auth
  card (contract §1.6).
- Tables carry **no zebra** — rows separate by soft dividers and hover, not striping.

Borders are two weights (contract §1.1): a **soft** card/divider border and a
**strong** border, plus a deeper dedicated field line (contract §1.5) that keeps
inputs clearly outlined rather than melting into the card.

### Shadow scale

Shadows are quiet by design, and flat is the default. `--rio-shadow-sm` is for
cards **in a grid**; `--rio-shadow-lg` is for transient overlays (dropdown
panels, popovers) and is a product token with no RustIO counterpart;
`--rio-shadow-xl` dresses the auth card alone. `--rio-shadow-md`,
`--rio-shadow-card` and `--rio-shadow-inset` are retired (contract §1.6).
Premium tooling
prefers borders + surface contrast over drop shadow; if you reach for
`box-shadow` for emphasis, reach for a darker border first.

---

## 4. Spacing scale

Six canonical steps, 4 → 32 px:

| Token | rem    | px |
|-------|--------|----|
| `s1`  | 0.25   | 4  |
| `s2`  | 0.5    | 8  |
| `s3`  | 0.75   | 12 |
| `s4`  | 1      | 16 |
| `s5`  | 1.5    | 24 |
| `s6`  | 2      | 32 |

Larger legacy steps (`--rio-space-40/48/64/80/96`) and the off-scale
`--rio-space-2/6/20` are audited out as components move onto the six
(contract §1.7).

Component rhythm uses `gap` on flex containers rather than per-element
margins. The form rule `gap: var(--rio-s5)` on `.rio-form` is canonical: it
spaces consecutive fields without double-margin collapse surprises and
without per-field margin accounting.

Two shell reservations live in `tokens/spacing.css` because some component
rules need to compute against them:

- `--rio-sidebar-w` — the contextual rail
- `--rio-topbar-h` — the utility row (aliases the shared `--masthead`)
- `--rio-modulebar-h` — the module row, a product token

Their values are owned by [`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md) §10, not
restated here.

---

## 5. Light only

The framework ships **one theme** (contract §12). `tokens/colors.css` declares a
single `:root` with `color-scheme: light`. There is no
`@media (prefers-color-scheme: dark)` block, no `[data-theme="dark"]` block, no
pre-paint theme script, and no per-component theme CSS.

- **One calm, light surface everywhere.** Hierarchy comes from the surface ladder
  and borders (§3), not from a second palette.
- **One set of WCAG pairings to audit.** Because there is one block, contrast is
  checked once per token pair rather than per theme per component.
- **Projects rebrand via the token block.** A generated `tokens.css` override is
  appended after the baked bundle and needs only `:root` to compose correctly.
- **History.** Visual Contract v2.1 §12 declared a dark theme *mandatory* and
  described a slate dark ladder. The shipped stylesheets have been light-only
  since the blue-accent pass, so v2.1 described a theme that did not exist;
  contract v3.0 corrected the record. `TOKENS-EMIT-SPEC.md` still carries the
  dark-aware emission rules and is reconciled in the emitter pass — until then
  the contract wins on whether a dark theme exists.

---

## 6. Operational UI principles

`rustio-admin` is operator software. It exists to keep an admin productive
through a ten-hour shift — not to convert a free-trial user. Concretely:

1. **Calm over flashy.** Buttons swap surface colour on hover instead of
   dimming with opacity. Tables separate rows with soft dividers and a quiet
   hover — **no zebra striping**, no accent-tinted overlay. Cards layer with
   borders, not glow.
2. **Reserve the accent for affordances.** Anything the user can act on
   may wear the blue accent; anything that's just content stays neutral.
   If everything is accented, nothing is. Focus is not an accent — it is its
   own role (§1).
3. **Mobile-first, and the measure is capped.** Narrow widths collapse the rail;
   wider ones pin it. Every page is capped by its `page_measure` so a 4 K monitor
   doesn't stretch a table row across the user's whole field of view, and the
   workspace gutter steps down with the viewport. The measures, the gutter steps
   and the breakpoints are owned by
   [`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md) §10 and §2.2.
4. **One canonical accent across every admin page.** Projects override
   the accent to rebrand, through `AdminTheme` — which does not touch focus
   (§1, contract §15.2).
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
8. **Operator readability first.** The default body size sits at **15 px** and
   table cells at 15 px (contract §2.1). Density comes from the row and control
   scale — 48 px rows, 40 px heads, 38 px controls — plus `gap` and surface
   contrast, not from shrinking prose. The 13 px mono size is for micro-labels
   that name a thing, never for text the operator has to read in quantity, and
   no operational text may be pale (contract §1.2).
9. **Legible surface ladder.** Adjacent surfaces sit far enough apart that the
   eye never squints to tell canvas from card from table-header from row-hover.
   The rungs that exist are the page ground, the card surface, the
   hover/sunken rung, the head/raised rung, the rail and the overlay — named and
   valued in [`VISUAL-CONTRACT.md`](VISUAL-CONTRACT.md) §1.1. (An earlier
   six-rung scheme with `--rio-surface-2/3/chrome/elevated` was planned in
   `PLAN_VISUAL_v2.md` and never shipped; those tokens do not exist.)
10. **Chrome carries weight.** Topbar, rail, and footer render on their own
    surfaces (`--rio-rail-bg` and the ladder's upper rungs) so the operator
    skeleton is visible without conscious attention. Chrome is *distinct*, not
    *dark*: the admin is light throughout (§5), and the earlier "dark-frame"
    direction — with its chrome-scope cascade flipping text, surface, border and
    accent tokens to light-on-dark — is retired along with the dark theme.
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
│   ├── colors.css       ← the single light :root token block
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
