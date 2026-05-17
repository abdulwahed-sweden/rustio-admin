# PLAN_VISUAL_v2.md — Visual identity overhaul plan for v0.15.0

This document captures the design plan landed across the v0.15.0
release. It is the doctrine address for the visual changes that
ship under "Principles 9–11" added to `DESIGN_DOCTRINE.md`.

The plan is reproduced here verbatim so the rationale survives the
implementation. The actual values that ship are in the tokens
(`tokens/colors.css`, `tokens/shadows.css`, `tokens/typography.css`,
`themes/dark.css`, `themes/light.css`) and the component fragments
(`components/{cards,buttons,forms,tables}.css`,
`layout/{topbar,sidebar,footer}.css`).

## Why this exists

The visual identity calibrated in 0.1–0.13 was tuned for "operator
readability through a ten-hour shift." The goal still holds; the
execution drifted into **operator-bland**:

- Surface ladder too shallow (~3% steps) — eye can't separate
  canvas from card from row-hover.
- Chrome (topbar, sidebar) rendered on the same surface as cards;
  no load-bearing visual weight.
- Borders and shadows nearly invisible — cards look like flat
  rectangles drawn on the page.
- Typography uniform — no weight contrast between display, h1,
  table-row primary cell, and body. The page reads as one tone.
- Net: pages feel "blank" / "white-heavy" despite being technically
  correct. Not premium.

The overhaul targets *calm with authority* — closer to the
Stripe / Linear / Bloomberg professional-software aesthetic that
operators recognise as "expensive engineering" without crossing
into flashy territory the doctrine forbids.

## Three new principles (added to `DESIGN_DOCTRINE.md`)

> **Principle 9 — Deeper surface ladder.** Adjacent surfaces are
> ≥ 4% (≈ 10 RGB points) apart. The eye should never squint to
> tell canvas from card from table-header from row-hover.

> **Principle 10 — Chrome carries weight.** Topbar and sidebar sit
> on a distinct surface tier (`--rio-surface-chrome`), deeper than
> card surface and lighter than page canvas. Chrome is the
> load-bearing skeleton — visible without conscious attention.

> **Principle 11 — Typography hierarchy is a weight choice, not
> just a size.** Display sizes declare gravity through weight
> 700–800 *and* deliberate tracking. Body and table cells stay at
> 400. No middle ground at 600 unless it serves a specific UI
> affordance (active nav, table-row primary cell).

The pre-existing principles (operator-readability, single accent,
brand reserve, mobile-first, URL-as-truth) stay intact. The accent
(`#0F8C7E` teal-emerald) is unchanged. Dark-mode is unchanged in
direction; only the ladder is deepened.

## Five-commit landing plan

| Commit | Scope |
|---|---|
| 1 | Token redefinition — `tokens/{colors,shadows,typography}.css`, `themes/{dark,light}.css`. Doctrine amendment. |
| 2 | Component refinement — `components/{cards,buttons,forms}.css`. |
| 3 | Table redesign — `components/tables.css`. |
| 4 | Chrome — `layout/{topbar,sidebar,footer}.css`. |
| 5 | Validation — visual regression sweep, CHANGELOG, v0.15.0 release tag. |

Plus a follow-up: update `obddesk` to consume the new defaults and
drop its now-redundant `AdminTheme` override.

## Hard constraints preserved across the overhaul

- No new font family — Geist Variable already supports the weights
  we add. No new `@font-face` declarations, no CDN round-trip.
- No accent change — `#0F8C7E` stays a project invariant.
- No new JS — every change is CSS-only.
- No template HTML changes — the redesign rides on the existing
  class structure. Zero risk of breaking generated projects.
- No new build step — hand-written CSS, baked via `include_str!`.
- No marketing surfaces — no hero gradients, no shimmer, no
  illustration, no animation beyond the existing 50 ms hover ease.
  The `DESIGN_DOCTRINE.md` §6.7 "no marketing" rule is preserved.

## Backwards compatibility

`AdminTheme` keeps its six override slots (`accent`, `bg`,
`surface`, `text`, `text_muted`, `border`) — no change to the
public theme API. The new `--rio-surface-chrome` and
`--rio-surface-elevated` are internal tokens; projects that want
chrome customisation get it through a future `Admin::chrome(...)`
builder (out of v0.15 scope).

Existing projects with `AdminTheme` overrides keep working. Values
close to the new defaults (e.g. `obddesk`'s `#0F1115` / `#3B4148`
text overrides) become redundant — projects can drop them and the
visual result is essentially identical.

## See also

- `DESIGN_DOCTRINE.md` — the authoritative visual contract.
  Principles 9–11 added in v0.15.0.
- `DESIGN_SYSTEM.md` — token-ownership and theme-patch contract.
  Six override tokens unchanged.
- CHANGELOG.md — v0.15.0 release entry walks every visible change.
