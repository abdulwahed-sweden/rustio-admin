# Token mapping

Strategy: **shared semantic core + product-specific tokens.** Canonical names
(RustIO's) carry the values; every `--rio-*` name becomes an alias and is
kept permanently because `AdminTheme` overrides, `rio-theme` emission and
downstream projects target it. `tokens/compat.css` already has this two-layer
shape; the proposal extends it rather than inventing a third layer.

```
canonical (value)        --page: #F1F1EE;
   ↓ alias
product name             --rio-bg: var(--page);
   ↓ consumed by
components, AdminTheme, rio-theme, downstream projects   (unchanged)
```

Semantics were checked, not names. "exact" means the value is already
identical at `d4b61fa`.

## Surfaces and lines

| rustio-admin | → canonical | Note |
|---|---|---|
| `--rio-bg` #F1F1EE | `--page` | exact |
| `--rio-surface` #FFFFFF | `--surface` | exact |
| `--rio-sunken` #F7F9FC | `--surface-soft` | exact; row hover in 3.1 (rustio-admin hovers on `--rio-raised`) |
| `--rio-raised` #F1F4F8 | `--surface-head` | exact; table heads, data-surface head |
| `--rio-overlay`, `--rio-surface-tint` | — | popovers, palette, code chips: product tokens, keep |
| `--rio-line` #D5DCE5 / `--rio-line-strong` #B9C5D3 | `--border` / `--border-strong` | exact |
| `--rio-rail-bg` #F4F6F9 / `--rio-rail-line` | `--rail-bg` / `--rail-line` | exact |

## Ink

| rustio-admin | → canonical | Note |
|---|---|---|
| `--rio-text-hi` #000000 | `--ink` #171B22 | **value change**: 3.1 never uses pure black |
| `--rio-text` #0B0C0E | `--ink-body` #202733 | value change |
| `--rio-text-mute` #30343A | `--ink-2` #3F4A59 | value change; 9:1 on white |
| `--rio-text-faint` #4B535C | `--ink-hint` (hints, placeholders) and `--ink-3` (tertiary metadata only) | one token becomes two uses; 3.1 forbids pale operational text |
| `--rio-text-onrust` #FFFFFF | literal white | 3.1 has no on-colour token; keep the name as a product token |

## Accent

| rustio-admin | → canonical | Note |
|---|---|---|
| `--rio-rust` #1F5797 / `-hover` #174578 | `--blue` / `--blue-dark` | exact |
| `--rio-rust-active` #123A66 | — | no 3.1 equivalent; product token, keep |
| `--rio-rust-tint` #DFEAFB / `-tint-2` #B7CDEA | `--blue-soft` / `--blue-line` | exact |
| `--rio-rust-solid*`, `--rio-on-solid` | `--blue`, white | two accent roles fold into one; names kept as aliases |
| `--rio-accent-focus` #2F7BD6 | `--focus` | exact; **must stay distinct from `--blue`** (PR #5 restates why); `_theme.html` already leaves it alone |
| `--rio-rust-ring` (4px rgba glow) | — | retired for the solid 3px offset ring |

## Status

| rustio-admin | → canonical | Note |
|---|---|---|
| `--rio-success` #16704A + alpha tint | `--green` #19724B / `--green-soft` | near-exact; opaque soft |
| `--rio-warn`, `--rio-gold` #8A5A0B | `--amber` #935B0A / `--amber-soft` | near-exact; warn and gold were already one value |
| `--rio-danger` #B42318 / hover | `--red` #A23F3A / `--red-soft` | value change; calmer; no hover shade |
| `--rio-pill-on/off-*` | green / grey pairs | exact in meaning |

## Geometry

| rustio-admin | → canonical | Note |
|---|---|---|
| `--rio-space-4 … 32` | `--s1 … --s6` | 4 / 8 / 12 / 16 / 24 / 32 map 1:1 |
| `--rio-space-2, 6, 20, 40, 48, 64, 80, 96` | — | off the 3.1 scale; audit uses (20 and 48 are the common ones) |
| `--rio-shell-pad-x` 32 / 16 | `--gutter` 32 / 24 / 16 | **new in v2** (RustIO PR #6); the 24 step at ≤ 760 is added |
| `--rio-page-wide` 1480 / `-standard` 1120 / `-narrow` 920 | `--content-wide` 1600 / `--content` 1120 / product | one number changes; narrow stays product-specific |
| `--rio-page-form` 672 | `--form-measure` 728 | **new in v2** (RustIO PR #5) |
| `--rio-radius-sm` 6 / `-control` 8 | `--radius-sm` / `--radius-btn` | exact |
| `--rio-radius-md` 9 / `-lg` 12 / `-xl` 16 / `-pill` 999 | `--radius` 14 | collapse; pill radius kept only if the env pill keeps it |
| `--rio-shadow-sm` | `--shadow-sm` | equivalent |
| `--rio-shadow-md/card/inset/xl` | — | cards go flat; retire |
| `--rio-shadow-lg` | — | product token for dropdowns and the palette; keep |
| control heights (hard-coded 44 / 46) | `--ctl` 38 / `--ctl-sm` 31 / `--ctl-utility` 32 | tokens replace literals; ~12 rules |
| row heights (56–64, hard-coded) | `--th-h` 40 / `--td-h` 48 | tokens replace literals |
| compact rows | `--th-h-compact` 32 / `--td-h-compact` 38 | **new in v2** (RustIO PR #5) |
| `--rio-topbar-h` 52 / `--rio-modulebar-h` 48 | — | product tokens; values 56 / 40 |

## Typography

| rustio-admin | → canonical | Note |
|---|---|---|
| `--rio-text-12 … 64` | the 3.1 scale 12 / 13 / 14 / 15 / 16 / 18 / 24 / 33 | **the D item**: names say px, values do not (`--rio-text-12` renders 15px). Re-pointing the values re-scales every component from one file; the hard-coded heights do not |
| base 16 | 15 | — |
| titles 36 / 800 | 24 / 800 | — |
| mono labels 14 / 700 uppercase | 13 / 700 uppercase, `letter-spacing .08em` | the contract's 14px floor is the conflict |

## Product-specific, no equivalent — keep

`--rio-syntax-*`, `--rio-hit-*`, `--rio-overlay`, `--rio-shadow-lg`,
`--rio-topbar-h`, `--rio-modulebar-h`, `--rio-on-solid`, `--rio-rust-active`,
`--rio-page-narrow`, motion tokens.

## Emission

`rio-theme` emits legacy names and dark blocks that the runtime already
warns about. The emitter and `docs/design/TOKENS-EMIT-SPEC.md` need the same
alias pass (Phase 7 of `MIGRATION.md`) so an emitted override and the baked
bundle agree on names. Not touched by this study.
