# DESIGN_DS_MIGRATION — adopting the RustIO Design System

Status: **Phase 0 complete** (foundation). This doc tracks the phased
migration of the admin from its pre-DS visual identity to the RustIO
Design System (cobalt accent, warm-stone neutrals, Spectral / Hanken
Grotesk / JetBrains Mono type, light + dark themes).

The migration is staged so the admin never ships broken between phases.
Each phase is a separately reviewable PR.

## Decisions (locked)

- **Re-skin, not re-architecture.** `rio-theme`, the single runtime,
  server-rendered minijinja, hand-written CSS baked via `include_str!`,
  and the no-build-step rule all stay. Only shape, colour, and type
  change.
- **Two themes.** Light is the default; a dim-slate dark variant is
  added (this supersedes the former light-only doctrine — see
  `DESIGN_DOCTRINE.md` §5).
- **Full token rewrite.** Call sites move from the pre-DS vocabulary
  (`--rio-accent`, `--rio-s4`, `--rio-fs-sm`, `--rio-text-strong`, …) to
  the DS vocabulary (`--rio-rust`, `--rio-space-16`, `--rio-text-14`,
  `--rio-text-hi`, …). This happens in Phase 1, not all at once.
- **Scope:** framework admin **and** `examples/shop` + `examples/clinic`.

## Phase 0 — foundation (this PR)

- Self-host the three DS Latin faces (Spectral, Hanken Grotesk, JetBrains
  Mono) as OFL woff2; bake + serve them; add `@font-face`.
- Replace the token files with the DS palette/scale, **light + dark**
  (`tokens/colors.css`, `tokens/shadows.css`), plus `tokens/motion.css`
  (new). The `admin.css` `@import` list and the `ADMIN_CSS` `concat!` in
  `routes.rs` stay in lock-step.
- **Compatibility alias bridge:** each token file defines the pre-DS
  names as `var()` aliases onto the new tokens, so the not-yet-rewritten
  CSS + templates keep resolving — and flip with the theme for free,
  because they reference tokens redefined in the dark blocks.
- Swap the Latin body/mono font stacks to the DS faces; tracking moves to
  DS values (the face changed). Sizes/line-heights keep their current
  values this phase so nothing reflows.

Net effect of Phase 0: the admin renders in the new palette + type, with
a working dark theme, and **zero structural breakage** — every old token
name still resolves through the bridge.

## Phase 1 — rewrite call sites

Rewrite the 36 admin CSS files + 52 templates to the DS vocabulary;
apply the Spectral serif to display headings surface-by-surface; adopt
the 14px product base; **delete the alias bridge**.

## Phase 2 — dark-mode UI

Topbar theme toggle + persistence (`[data-theme]`), and a WCAG contrast
pass in both themes.

## Phase 3 — examples

Re-skin `examples/shop` and `examples/clinic` to the DS.

## Phase 4 — docs & polish

Finalise `DESIGN_DOCTRINE.md` / `DESIGN_SYSTEM.md`, the PR token
disclosure, the visual-regression checklist, and CHANGELOG.
