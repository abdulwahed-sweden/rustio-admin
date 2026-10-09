# Token mapping (v3)

Strategy: **shared semantic core + product-specific tokens.** Canonical names
(RustIO `main @ 00b933c`'s `:root`) carry the values; every `--rio-*` name
is an alias, kept permanently because `AdminTheme`, `rio-theme` and
downstream projects target it. `tokens/compat.css` already has this shape.

```
canonical (value)        --page: #F1F1EE;
   ↓ alias
product name             --rio-bg: var(--page);
   ↓ consumed by
components, AdminTheme, rio-theme, downstream projects   (unchanged)
```

`main` changed no token *value* relative to 0.13; it added `--gutter`.
It has **no** `--form-measure` and **no** compact-row tokens — v2's rows for
those came from PR #5 and are withdrawn.

## Surfaces and lines

| rustio-admin | → canonical | Note |
|---|---|---|
| `--rio-bg` #F1F1EE | `--page` | exact |
| `--rio-surface` | `--surface` | exact |
| `--rio-sunken` #F7F9FC | `--surface-soft` | exact; row hover in `main` |
| `--rio-raised` #F1F4F8 | `--surface-head` | exact |
| `--rio-overlay`, `--rio-surface-tint` | — | product tokens, keep |
| `--rio-line` / `--rio-line-strong` | `--border` / `--border-strong` | exact |
| `--rio-rail-bg` / `--rio-rail-line` | `--rail-bg` / `--rail-line` | exact |

## Ink

| rustio-admin | → canonical | Note |
|---|---|---|
| `--rio-text-hi` #000000 | `--ink` #171B22 | value change: no pure black |
| `--rio-text` #0B0C0E | `--ink-body` #202733 | value change |
| `--rio-text-mute` #30343A | `--ink-2` #3F4A59 | value change |
| `--rio-text-faint` #4B535C | `--ink-hint` / `--ink-3` | one token, two uses |
| `--rio-text-onrust` | literal white | keep the name |

## Accent

| rustio-admin | → canonical | Note |
|---|---|---|
| `--rio-rust` / `-hover` | `--blue` / `--blue-dark` | exact |
| `--rio-rust-active` | — | product token |
| `--rio-rust-tint` / `-tint-2` | `--blue-soft` / `--blue-line` | exact |
| `--rio-rust-solid*`, `--rio-on-solid` | `--blue`, white | aliases kept |
| `--rio-accent-focus` #2F7BD6 | `--focus` | exact as a default. `main`'s `rustio.design.json` injection may set `--focus` per project; `AdminTheme` never sets `--rio-accent-focus` — INTENTIONAL DIVERGENCE, kept |
| `--rio-rust-ring` (glow) | — | retired for the solid 3px offset ring |

## Status

| rustio-admin | → canonical | Note |
|---|---|---|
| `--rio-success` + alpha tint | `--green` / `--green-soft` | near-exact; opaque soft |
| `--rio-warn`, `--rio-gold` | `--amber` / `--amber-soft` | near-exact |
| `--rio-danger` #B42318 | `--red` #A23F3A / `--red-soft` | value change |
| `--rio-pill-on/off-*` | green / grey pairs | exact in meaning |

## Geometry

| rustio-admin | → canonical | Note |
|---|---|---|
| `--rio-space-4 … 32` | `--s1 … --s6` | 1:1 |
| `--rio-space-2, 6, 20, 40, 48, 64, 80, 96` | — | off-scale; audit |
| `--rio-shell-pad-x` 32 / 16 | `--gutter` 32 / 24 / 16 | ALIGN (`main`); the 24 step is added |
| `--rio-page-wide` 1480 / `-standard` 1120 / `-narrow` 920 | `--content-wide` 1600 / `--content` 1120 / product | one number changes |
| `--rio-page-form` 672 | — (no `main` token) | **728**, rustio-admin's own; matches `main`'s card `680 + 2 × --s5`. INTENTIONAL DIVERGENCE in placement (centred vs `main`'s left-aligned `.form-layout`) |
| `--rio-radius-sm` / `-control` | `--radius-sm` 6 / `--radius-btn` 8 | exact |
| `--rio-radius-md/lg/xl/pill` | `--radius` 14 | collapse |
| `--rio-shadow-sm` | `--shadow-sm` | equivalent |
| `--rio-shadow-md/card/inset/xl` | — | retire |
| `--rio-shadow-lg` | — | product token, keep |
| control heights (hard-coded 44 / 46) | `--ctl` 38 / `--ctl-sm` 31 / `--ctl-utility` 32 | tokens replace literals |
| row heights (56–64) | `--th-h` 40 / `--td-h` 48 | tokens replace literals |
| compact rows | `--th-h` reused (`main` `.record-compact-item`) | no new token; v2's `--th-h-compact` / `--td-h-compact` DROPPED |
| `--rio-topbar-h` / `--rio-modulebar-h` | — | product tokens; values 56 / 40 |

## Typography

| rustio-admin | → canonical | Note |
|---|---|---|
| `--rio-text-12 … 64` | 12 / 13 / 14 / 15 / 16 / 18 / 24 / 33 | the D item: names say px, values do not |
| base 16 | 15 | |
| titles 36 / 800 | 24 / 800 | |
| mono labels 14 / 700 | 13 / 700, `.08em` | the contract's 14px floor is the conflict |

## Product-specific, no equivalent — keep

`--rio-syntax-*`, `--rio-hit-*`, `--rio-overlay`, `--rio-shadow-lg`,
`--rio-topbar-h`, `--rio-modulebar-h`, `--rio-on-solid`, `--rio-rust-active`,
`--rio-page-narrow`, `--rio-page-form`, motion tokens.

## Emission

`rio-theme` and `docs/design/TOKENS-EMIT-SPEC.md` need the alias pass
(Phase 7). Not touched by this study.
