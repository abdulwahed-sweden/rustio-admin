# tokens.css Emission Contract

Status: Normative
Audience: any generator that emits a `tokens.css` for RustIO Admin —
the in-repo `rio-theme` engine **and** the external `rustio-design` tool.

A generated `tokens.css` is appended **after** the framework's baked CSS bundle
at runtime (`RUSTIO_TOKENS_CSS=<path>` → `/static/admin.css` = baked bundle +
override). This document is the contract the override must satisfy so it composes
correctly in **both** light and dark mode. The in-repo reference implementation is
`crates/rio-theme/src/emit.rs` with golden fixtures under
`crates/rio-theme/tests/golden/dark_*.css`.

---

> **Status — light-only runtime (Visual Contract v3.0 §12).** The framework is
> light-only: `tokens/colors.css` carries a single `:root` block, with no
> `@media (prefers-color-scheme: dark)` and no `[data-theme]` block. The §1
> hazard below therefore describes a historical state, and the §3 dark-block
> requirements are **retained unchanged** — they are what `rio-theme`'s
> `emit.rs` and its golden fixtures produce today, and changing them means
> changing the emitter. Reconciling this contract with the light-only runtime,
> together with the canonical-name alias pass, is the emitter pass of
> `docs/design/rustio-admin-composition-study/MIGRATION.md`. Until then: this
> document stays normative for *generators*, and `VISUAL-CONTRACT.md` is
> authoritative on whether the framework has a dark theme. Do not read §1–§5 as
> a licence to add dark blocks back to the framework's own stylesheets.

## 0. Token names: canonical core + `--rio-*` aliases

The framework's stylesheets are two layers (`VISUAL-CONTRACT.md` §1):
`tokens/canonical.css` declares the shared RustIO semantic core — `--page`,
`--surface`, the `--ink*` ramp, `--blue` / `--focus`, `--ctl` / `--th-h` /
`--td-h`, `--s1 … --s6` and the rest — and every `--rio-*` name is a
permanent alias onto one of them.

**A generated `tokens.css` keeps emitting `--rio-*` names.** They are the
public token contract that `AdminTheme`, this emitter and downstream project
stylesheets all target, and the override is appended *after* the bundle, so
re-pointing an alias re-points everything that consumes it. Emitting
canonical names instead would be both unnecessary and more fragile: a
project that set `--blue` but not `--rio-rust` would be overridden by the
alias layer's own declaration.

**Never emit `--rio-accent-focus`.** Focus is its own semantic role with a
verified contrast budget against every surface the ring can sit beside, and
a brand colour is not required to clear it. `AdminTheme` does not set it
either; `render::tests::a_project_theme_never_overrides_the_focus_token`
pins that. (RustIO's own `rustio.design.json` injection *does* set
`--focus` — this product diverges deliberately; see `VISUAL-CONTRACT.md`
§15.2.)

---

## 1. The hazard this contract prevents

Historically the framework shipped a dark theme as two blocks (`crates/rustio-admin/assets/static/admin/tokens/colors.css`):

```css
:root { /* light */ }
@media (prefers-color-scheme: dark) { :root { /* dark */ } }   /* auto */
[data-theme="dark"] { /* dark */ }                              /* explicit toggle */
```

A `:root`-only override (light values, no dark blocks) appended **after** that bundle:

- ties `[data-theme="dark"]` on specificity — both resolve to `(0,1,0)` — and **wins by source order** (it is later in the stream), so the explicit toggle leaks light; and
- ties the framework's `@media … :root` and, being later, wins the **auto** (OS-dark) case too.

Result: dark mode renders light surfaces under dark chrome. A `:root`-only override is therefore **forbidden** by this contract.

## 2. Required output structure (mirror the framework's guards)

Every generated `tokens.css` MUST emit three blocks, in this order:

```css
:root { /* light tokens */ }

:root[data-theme="dark"] { /* dark tokens */ }

@media (prefers-color-scheme: dark) {
  :root { /* dark tokens — identical to the block above */ }
}
```

- **`:root[data-theme="dark"]`** (specificity `(0,1,1)`) — strictly higher than the framework's `[data-theme="dark"]` `(0,1,0)`, so it tie-breaks the explicit toggle correctly regardless of source order.
- **`@media (prefers-color-scheme: dark) { :root { … } }`** — same `:root` guard the framework uses; being later in source order than this file's own `:root`, it wins the auto case.
- The two dark blocks MUST carry **identical** values so explicit and auto dark render the same.
- The dark blocks MUST set every surface/text/border/accent token the `:root` block sets — otherwise the unset ones leak from this file's own light `:root`.

## 3. Derivation policy (hybrid)

For the dark values, in priority order:

1. **Explicit** — if the design input declares dark values, emit them (lifted for AA, see §5).
2. **Auto-derive** — otherwise derive dark from the declared light tokens (§4).
3. **Light-only opt-out** — if the author sets a `light_only` flag, emit a dark block that **pins to the framework defaults** (§4 slate ladder + the framework's default accent). This is a *neutral* dark override — never light values, never empty. The brand simply does not appear in dark.

> An **empty** dark block is NOT acceptable: it would leave this file's light `:root` values winning in dark. "Light-only" means *framework-default dark*, not *no dark*.

## 4. The dark slate ladder (brand-agnostic)

Surfaces, text, and borders are brand-agnostic in dark — exactly as the light
`:root` keeps the chrome ladder brand-agnostic. Use these values for all three
policies (they mirror the framework's Phase-8 `[data-theme="dark"]`):

| Token | Value | | Token | Value |
|---|---|---|---|---|
| `--rio-bg` | `#0f172a` | | `--rio-text-strong` | `#f1f5f9` |
| `--rio-surface` | `#1e293b` | | `--rio-text` | `#cbd5e1` |
| `--rio-surface-2` | `#0b1120` | | `--rio-text-muted` | `#94a3b8` |
| `--rio-surface-3` | `#334155` | | `--rio-text-subtle` | `#64748b` |
| `--rio-surface-chrome` | `#0f172a` | | `--rio-border-soft` | `#334155` |
| `--rio-surface-elevated` | `#1e293b` | | `--rio-border` | `#334155` |
| `--rio-success` | `#6fb98a` | | `--rio-border-strong` | `#64748b` |
| `--rio-warning` | `#d6a65a` | | `--rio-danger` | `#ef4444` |

Semantic backgrounds are the semantic foreground mixed 84% toward `--rio-bg`
(`fg.mix(#0f172a, 0.84)`). `--rio-info-bg` is the dark accent mixed 86% toward `--rio-bg`.

## 5. The accent (the only policy-dependent part)

| Policy | Dark `--rio-accent` |
|---|---|
| **Auto** | the brand's large-fill accent (light `--rio-accent`), **lifted** until it clears AA (4.5:1) on `--rio-surface` (`#1e293b`). |
| **LightOnly** | the framework default: `--rio-accent: #2dd4bf`, `--rio-accent-hover: #5eead4`. |
| **Explicit** | the author's dark accent, lifted the same way. |

Lift algorithm (deterministic): step the accent toward white by 0.05 (sRGB mix)
until `contrast_ratio(accent, #1e293b) ≥ 4.5`, bounded at 40 iterations.
Derive `--rio-accent-hover` = `accent.lighten(0.12)`, `--rio-accent-soft` =
`accent.mix(#0f172a, 0.86)`, `--rio-accent-border` = `accent.mix(#1e293b, 0.35)`,
and `--rio-accent-rgb` = the space-separated 0–255 triple of the dark accent.
`--rio-brand-adaptive` = `var(--rio-brand-dark)` in both dark blocks.

## 6. The three golden cases

The reference goldens (`crates/rio-theme/tests/golden/`) use the Patina brand
`#0E6B5B` so the external generator can diff against known-good output:

- `dark_auto.css` — `Auto`: accent lifts to `#6a9b8f` (AA-clear on `#1e293b`).
- `dark_light_only.css` — `LightOnly`: dark accent pinned to `#2dd4bf`; the brand `#0e6b5b` appears only in the light `:root`.
- `dark_explicit.css` — `Explicit { accent: #14b8a6 }`: dark accent `#14b8a6` (already AA-clear, emitted as-is).

A generator conforms when, for the same input, its dark blocks match these
structurally (selectors, token set) and semantically (slate ladder + the §5 accent).

## 7. Action items for `rustio-design` (external repo)

1. Implement §2–§5 in the `rustio-design build` emitter.
2. Add a `light_only` key to `rustio.design.toml` (default `false`) wired to the §3 opt-out.
3. **Regenerate any committed `tokens.css`** once shipped. Until a generator emits dark blocks its override is light-only and **dark mode leaks light surfaces** — the framework logs a WARN at startup naming the file (`override_is_dark_leak_hazard` in `crates/rustio-admin/src/admin/routes.rs`). This is a generator gap, not a framework bug.
