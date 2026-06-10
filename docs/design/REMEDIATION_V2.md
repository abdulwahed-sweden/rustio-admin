# Remediation Plan — RustIO Admin Visual Contract v2.0 Conformance

Status: Approved, in execution
Canonical source: the **RustIO Admin Visual Contract v2.0** (the contract wins over any
older stylesheet; when the contract is silent the reference screenshot wins; when both
are silent, ask — do not invent).

This document records the agreed Phase 0 decisions, the execution rules, and the
ordered, file-by-file remediation plan. It is the reviewable in-repo record of the work.

---

## Phase 0 decisions (locked)

1. **Inter hosting — Option (a).** Self-host `Inter-Variable.woff2`, following the existing
   baked-font convention in `base/fonts.css` (OFL, no FOUT, woff2 baked into the binary).
   Do NOT rely on the `system-ui` fallback; cross-platform rendering must be identical.
   Keep the Spectral/Hanken `@font-face` blocks and woff2 assets in place for now —
   removing them is a separate cleanup commit at the very end, after everything passes.

2. **Dot-grid — remove it from the content area.** The contract's `#fafcfb` background was
   measured from the reference screenshots and they are flat. Also set body
   `letter-spacing` to `0` (the negative tracking was a Hanken flourish).

3. **Dark theme — keep it.** Update `CLAUDE.md` to match the contract (dark theme exists,
   token-driven only, no per-component dark CSS). Phase 8 remains the LAST phase and must
   NOT merge until the maintainer has visually reviewed it. After approval, dark-mode
   reference screenshots will be captured into `docs/visual-reference/` for the future.

4. **Screenshots — added by the maintainer.** The nine reference PNGs will be placed in
   `docs/visual-reference/` using the exact filenames listed in contract §0, before
   Phase 9. Until then, work against the contract's measured values.

---

## Execution rules

- **Save this plan first** (done) so it is reviewable in-repo.
- **One commit per checkpoint.** Phases 1, 2, and 3 are individual checkpoints. Phases
  4–7 may be grouped into one checkpoint. Phase 8 is alone.
- At every checkpoint: **stop, summarize what changed (file-by-file), run the gates, and
  wait for explicit approval** before starting the next phase.
- **Gates at every checkpoint:** `cargo test --workspace` (the CSS-manifest lockstep test
  must pass), `cargo fmt --check`, `cargo clippy -D warnings`.
- **Do not add new CSS fragments** — fold changes into existing files so the `@import`
  list in `admin/admin.css` and the `ADMIN_CSS` concat block in `src/admin/routes.rs`
  stay untouched.
- **After Phase 2 specifically:** before requesting Phase 3, start the dev server, log in,
  and capture a full-page Playwright screenshot of the feature-flags page (light/LTR);
  present it for a manual go/no-go against `feature_flags.png`.
- **CHANGELOG.md:** add the design overhaul to `[Unreleased]` in the Phase 1 commit and
  extend it at each checkpoint; every new or changed `--rio-*` token must be disclosed.
- **Hard reminders (contract):** no UI text below 14px in the content area; all checkboxes
  teal via `accent-color` (including the permission grid); danger is solid `#dc2626`
  (replace the brightness-filter hack); no zebra tables; the top bar, sidebar, and footer
  are **frozen** — do not touch them.

---

## Phase 1 — Tokens (re-value in place; highest leverage, lowest risk)

Re-value the **light** blocks only (`:root` and `[data-theme="light"]`) in
`tokens/colors.css`; the dark blocks are deferred to Phase 8. Component plumbing already
points at these tokens, so most §1 conformance lands with zero component edits.

`tokens/colors.css` (light blocks):

| Token | From → To | Drives |
|---|---|---|
| `--rio-bg` | `#F3F6FD` → `#fafcfb` | page background |
| `--rio-text-hi` | `#111722` → `#1e293b` | titles, labels, table text |
| `--rio-text` | `#3B4453` → `#475569` | body / lead |
| `--rio-text-mute` | `#677083` → `#64748b` | legends, hints |
| `--rio-text-faint` | `#99A1B2` → `#94a3b8` | placeholders |
| `--rio-line` | `#DBE0EB` → `#e2e8f0` | card/divider = `--rio-border-soft` |
| `--rio-line-strong` | `#C2CAD9` → `#94a3b8` | input/radio border = `--rio-border-input` |
| `--rio-rust` / `--rio-rust-solid` | `#0F8C7B` → `#119588` | accent (text + fill) |
| `--rio-rust-hover` / `-solid-hover` | `#0C7567` → `#0e7c72` | accent hover |
| `--rio-rust-active` / `-solid-active` | `#095E53` → `#0b655c` (derived) | accent active |
| `--rio-rust-tint` | `…15,140,123,.08` → `…17,149,136,.08` | pill/badge tint |
| `--rio-rust-tint-2` | `…15,140,123,.16` → `…17,149,136,.16` | tint-2 |
| `--rio-rust-ring` | `…15,140,123,.38` → `…17,149,136,.20` | focus ring = `--rio-accent-ring` |
| `--rio-danger` | `#AE382C` → `#dc2626` | danger |
| `--rio-danger-tint` | `…174,56,44,.10` → `…220,38,38,.10` | danger wash |
| `--rio-syntax-key` | `#0F8C7B` → `#119588` | accent-led syntax |

New §1 tokens added to the light blocks (dark equivalents deferred to Phase 8):

- `--rio-surface-tint: #edf7f8` — inline code / kbd chip
- `--rio-accent-focus: #1f8987` — measured focused-field border line
- `--rio-danger-hover: #b91c1c`
- `--rio-pill-on-bg: #f1f8f0; --rio-pill-on-text: #3f9142;`
- `--rio-pill-off-bg: #eef1f5; --rio-pill-off-text: #475569;`

`tokens/compat.css` — add contract-name aliases (theme-aware, reference engine tokens):

- `--rio-accent-hover: var(--rio-rust-hover)`
- `--rio-accent-ring: var(--rio-rust-ring)`
- `--rio-border-input: var(--rio-line-strong)`
- `--rio-shadow-card: 0 1px 2px rgba(15,23,42,.04), 0 8px 24px rgba(15,23,42,.06)`
- update `--rio-shadow-inset` → `inset 0 1px 1px rgba(15,23,42,.03)` (contract value)

`base/base.css`:

- Remove the dot-grid `background-image` + `background-size` from `body` and `.rio-canvas`
  (keep `background-color: var(--rio-bg)`).
- `body { letter-spacing: 0 }` (was `--rio-tracking-tight`).

*Risk: low. After this phase ~70% of §1 visual conformance is in place with no component
edits. Dark theme intentionally still shows the old palette until Phase 8.*

## Phase 2 — Typography & font pipeline

- `tokens/typography.css`: `--rio-font-body` and `--rio-font-display` → the contract Inter
  stack (retires Spectral from titles/stats — the contract has no serif); `--rio-font-mono`
  → contract SFMono stack. Kill all sub-14px: raise the smallest content token to `0.875rem`
  (14px); `--rio-fs-display` 44px → **36px**; keep `--rio-fs-md` 16px.
- `base/fonts.css`: add `@font-face` for self-hosted Inter (Phase 0 #1).
- `base/base.css`: titles use Inter at weight 800, line-height 1.15.
- Fix literal sub-14px in content pages: `pages/list.css` (11px), `.rio-dropdown-badge`
  (11px), `permissions.css` (12–13px) → 14px floor. (Frozen sidebar/topbar 11px stay.)
- **Gate + Playwright feature-flags screenshot for manual go/no-go before Phase 3.**

## Phase 3 — Components (restyle existing classes to §-specs)

- `components/forms.css`: `.rio-input` focus ring `3px` → **4px** + inset; mono placeholder
  utility for identifier fields.
- `components/buttons.css`: `.rio-btn` base → 44px / weight 700 / radius 8px; `.rio-btn--danger`
  → solid `#dc2626`, hover `--rio-danger-hover` (drop the `filter: brightness` hack); add
  `.rio-action-link` + `--danger` / `--muted` (red text-link Delete, muted History).
- `components/data.css`: `.rio-table th` → 14px / weight 800; cells → `--rio-text-strong`,
  padding `--rio-s4`; wrapper radius 12px; remove/neuter `.rio-table--zebra` (no zebra).
- `components/code.css` + tokens: inline code/kbd chip per §10.3 (`--rio-surface-tint` bg,
  mono, radius 6px, `0.125rem 0.5rem`, `0.875em`); the block `.rio-code` stays.

## Phase 4 — Sections: legend-on-border (§3) — structural

- `pages/form.css`: `.rio-fieldset` → border `--rio-line`, radius **14px**, `--rio-shadow-card`,
  padding `--rio-s6`. Restyle `.rio-fieldset-legend` to overlap the top border (legend-on-border),
  uppercase 14px weight 700 `--rio-text-mute`, non-mono.
- Templates: prefer native `<fieldset><legend>`; migrate section wrappers across ~7 templates
  (`form`, `lock_user`, `admin_reset_password`, `user_new`, `group_edit`, `feature_flags`,
  `password_change`).

## Phase 5 — Action bar (§8.2) — structural

- `pages/form.css`: `.rio-form-actions` → add `border-top` hairline, `margin-top --rio-s6`,
  `padding-top --rio-s5`, `gap --rio-s4`; replace the flex spacer with
  `.rio-action-bar-end { margin-inline-start: auto }` (one wrapping row, no orphaned Cancel).
- Templates (3 files): reorder primary → secondary → (auto) → muted/danger text-links; wire
  Delete to `.rio-action-link--danger`.

## Phase 6 — Pills (§9) — consolidation

- Define canonical `.rio-pill` + `--on` (`#f1f8f0`/`#3f9142`) + `--off` (`#eef1f5`/`#475569`)
  in one place (`components/data.css`); reconcile the two competing `.rio-pill` definitions
  (`pages/tools.css`, `.rio-pill-stock` in `console.css`); repoint `feature_flags.html`
  `--success`/`--neutral`.

## Phase 7 — Page header + shells (§2.1, §10.1)

- Masthead `h1` 42px → **36px / 1.15 / weight 800**; add breadcrumb (`--rio-fs-sm`, link segs
  weight 700, `·` separator) and lead (`--rio-fs-lead` 17px, max-width ~70ch).
- Add `.rio-form-shell { max-inline-size: 880px }` (was 1100px) and `.rio-content-shell
  { max-inline-size: 1040px }`.
- Two-col grid: gap `--rio-s5` (24px), breakpoint 768px.

## Phase 8 — Dark theme re-derivation + RTL (§12) — LAST, alone, review-gated

- `tokens/colors.css` dark blocks: re-derive from the new slate/teal §1 light tokens (invert
  surfaces, lift accent for AA). Add dark values for all Phase 1 new tokens. No per-component
  dark CSS.
- Update `CLAUDE.md` (dark theme exists, token-driven only).
- Confirm RTL `letter-spacing: 0` neutralization for `.rio-fieldset > legend` and `.rio-table th`.
- **Must not merge until the maintainer visually reviews dark mode.**

## Phase 9 — Lockstep, CI, verification

- CSS lockstep: no new fragments (folded into existing files) so the `@import` list and the
  `ADMIN_CSS` concat in `src/admin/routes.rs` stay in lock-step (test at `routes.rs` guards it).
- Tier-2 symbol guard: untouched.
- `CHANGELOG.md` `[Unreleased]`: design overhaul + every changed `--rio-*` token.
- `cargo test --workspace`, `cargo fmt --check`, `cargo clippy -D warnings`.
- §13 walkthrough against `docs/visual-reference/` once the maintainer adds the screenshots.

---

## Sequencing & sizing

| Phase | Risk | Size | Gate |
|---|---|---|---|
| 1 Tokens | low | S | — |
| 2 Type/fonts | med | S–M | Phase 0 #1; Playwright screenshot before Phase 3 |
| 3 Components | low | M | — |
| 4–7 Structural | med | L | grouped checkpoint |
| 8 Dark/RTL | high | M | alone; maintainer visual review |
| 9 Lockstep/verify | low | S | screenshots added by maintainer |
