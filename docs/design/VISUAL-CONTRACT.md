# RustIO Admin Visual Contract

Version: 2.1
Status: Mandatory — the source of truth for all content-area token **values**
(colors, type scale, fonts, spacing, dark theme, RTL).

Changelog:
- **2.1** — §3 split into three section-pattern cases (form / list-table / single-group),
  derived from the §0 references after the §13 walkthrough surfaced that "legend-on-border
  everywhere" contradicted feature_flags.png and the simple-create cards; §13 checklist
  line updated to "section pattern matches its case"; encoded two label facts the
  references settle (lowercase status pills; "Current password").
- **2.0** — initial contract.

This contract is the single owner of the concrete visual numbers. Doctrine docs
(`DESIGN_DOCTRINE.md`, `DESIGN_SYSTEM.md`, …) own *principles and architecture*
and point here for values — one source of truth per fact. When this contract and
an older stylesheet/doc disagree, this contract wins. When it is silent, the
reference screenshot wins. When both are silent, ask.

Companion contracts: `TOKENS-EMIT-SPEC.md` owns the dark-aware `tokens.css`
emission contract for generators; `REMEDIATION_V2.md` records the phased rollout.

## 0. Reference screenshots

Source of truth lives at `docs/visual-reference/`:

```
admin_reset_password.png · feature_flags.png · form.png · group_edit.png
group_new.png · lock_user.png · password_change.png · user_new.png
```

If a page being changed has no screenshot, match the closest pattern and flag it.

## 1. Canonical tokens (measured, light theme)

```css
:root {
  /* Surfaces */
  --rio-bg: #fafcfb;            /* content-area background (barely-warm near-white) */
  --rio-surface: #ffffff;       /* cards, inputs, radio rows, table */
  --rio-surface-tint: #edf7f8;  /* inline code / kbd chip background */

  /* Text */
  --rio-text-strong: #1e293b;   /* titles, labels, table text */
  --rio-text: #475569;          /* lead paragraphs, descriptions */
  --rio-text-muted: #64748b;    /* section legends, hints, breadcrumb current */
  --rio-placeholder: #94a3b8;   /* input placeholder text */

  /* Borders */
  --rio-border-soft: #e2e8f0;   /* card borders, table dividers, action-bar hairline */
  --rio-border-input: #94a3b8;  /* input/select/textarea/radio-row borders */

  /* Accent — teal */
  --rio-accent: #119588;
  --rio-accent-hover: #0e7c72;
  --rio-accent-focus: #1f8987;                 /* focused-field border */
  --rio-accent-ring: rgba(17, 149, 136, 0.20); /* outer focus glow */

  /* Status */
  --rio-danger: #dc2626;
  --rio-danger-hover: #b91c1c;
  --rio-pill-on-bg: #f1f8f0;    --rio-pill-on-text: #3f9142;   /* "enabled" pill */
  --rio-pill-off-bg: #eef1f5;   --rio-pill-off-text: #475569;  /* "disabled" pill */

  /* Elevation */
  --rio-shadow-card: 0 1px 2px rgba(15, 23, 42, 0.04), 0 8px 24px rgba(15, 23, 42, 0.06);
  --rio-shadow-inset: inset 0 1px 1px rgba(15, 23, 42, 0.03);
}
```

Hard rules: the accent is **this teal** (`#119588`), not blue/emerald/a Tailwind
swatch. Titles are `#1e293b` — not black, not navy. The content background is
`#fafcfb` — a barely-warm near-white, not gray, not pure white.

The runtime token names differ (`--rio-rust*` / `--rio-text-hi` / `--rio-line*`);
`tokens/compat.css` aliases the contract names. The values above are canonical.

## 2. Typography

```css
--rio-font-sans: "Inter", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
--rio-font-mono: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
```

**Inter** is the single Latin face for body **and** titles (no serif). Per-script
Arabic fallbacks (Noto Naskh display, Tajawal body/mono) are appended.

```css
--rio-fs-xs: 0.875rem;       /* 14px — section legends, table headers, kbd */
--rio-fs-sm: 0.9375rem;      /* 15px — breadcrumbs, hints, pills */
--rio-fs-md: 1rem;           /* 16px — body, labels, inputs, buttons, table cells */
--rio-fs-lead: 1.0625rem;    /* 17px — page lead paragraph */
--rio-fs-display: 2.25rem;   /* 36px — page titles */
```

**Forbidden: any content-area UI text below 14px.** No 11/12/13px anywhere.

### 2.1 Page header pattern

Breadcrumb (`--rio-fs-sm`; link segments `--rio-text-strong`/700; `·` separator
muted; current segment muted/400) → page title (36px/800/line-height 1.15,
`--rio-text-strong`) → lead (`--rio-fs-lead`/1.7/`--rio-text`, max-width ~70ch).
Rhythm: breadcrumb→title s3, title→lead s4, lead→first section s6.

## 3. Section patterns — three cases

The §0 references use **three** distinct section patterns. Pick by the page's
shape, not by habit. All three share the card shell: white, 14px radius,
`--rio-border-soft`, `--rio-shadow-card`, s6 (32px) padding.

### 3(a) Multi-section FORM pages → legend-on-border

For forms with two or more named sections (`CONTENT`, `IDENTITY`, `MODE`,
`REASON`, `DURATION`, `PERMISSIONS`). The section label sits **on the card's top
border**, fieldset-legend style. Use native `<fieldset>`/`<legend>`. Legend:
`--rio-fs-xs` (14px), 700, uppercase, `letter-spacing: 0.08em`,
`--rio-text-muted`. References: `form.png`, `user_new.png`, `lock_user.png`,
`admin_reset_password.png`, and `group_edit.png`'s `PERMISSIONS` card.

### 3(b) LIST / TABLE page sections → eyebrow heading

For content pages whose sections introduce a table or a sub-form (feature_flags'
`FLAGS` and `ADD`). The label is an **eyebrow + sub-heading stacked ABOVE the
card**, and the card itself is **unlabeled** (no legend cutting its border):

- Eyebrow: `--rio-fs-xs` (14px), 700, uppercase, `letter-spacing: 0.08em`,
  `--rio-text-muted`. Margin-block-end s1.
- Sub-heading: `--rio-fs-lead` (17px), 700, `--rio-text-strong`. Margin-block-end s4.
- Then the card (table or form), unlabeled.

Measured from `feature_flags.png`: `FLAGS` eyebrow over a `2 registered`
sub-heading over the flags table; `ADD` eyebrow over `Register a new flag` over
the add-flag card. Do **not** convert these to legend-on-border.

### 3(c) Single-field-group cards on simple create forms → legend-less

When a create/edit form has exactly **one** field group and no second section
(`group_new`, `group_edit`'s name/description card, `password_change`), the card
carries **no legend and no eyebrow** — fields sit directly inside the card. A
lone section needs no name. References: `group_new.png`, `password_change.png`,
`group_edit.png` (top card).

## 4. Labels, required markers, hints

Bold label (`--rio-fs-md`/700/`--rio-text-strong`); required asterisk in
`--rio-danger`; inline hint a normal-weight parenthetical on the same line in
`--rio-text-muted`. Gap label→control: s2. Never let labels touch inputs.

Settled label fact: the password-change form's first field is **"Current
password"** (per `password_change.png`), not "Old password".

## 5. Inputs, textareas, selects

44px min height, `--rio-border-input` border, 8px radius, `--rio-shadow-inset`,
`--rio-fs-md`. Focus: `--rio-accent-focus` border + `--rio-shadow-inset, 0 0 0 4px
--rio-accent-ring`. Mono placeholders for code identifiers (slug, flag key). 36/38px
control heights are forbidden.

## 6. Radio rows

Full-width bordered rows, stacked, tappable end-to-end: min-height 52px, s4
padding, `--rio-border-input` border, 10px radius, 20px teal `accent-color`
control aligned to the first text line, s3 gap between rows. Primary phrase 700;
trailing description 400 in `--rio-text`.

## 7. Checkboxes

18px, **teal `accent-color`** everywhere — including the permission grid (the
reference grid showing native blue is a known defect, normalize to teal).

## 8. Buttons and the action bar

All buttons 44px / weight 700 / 8px radius. Primary: `--rio-accent` fill, white
text, hover `--rio-accent-hover`. Secondary: white, `--rio-text-strong`,
`--rio-border-input` border. Danger primary: solid `--rio-danger`, hover
`--rio-danger-hover`. Destructive **secondary** is a red text link with a trash
icon (`.rio-action-link--danger`), never a solid red button beside Save. History
is a muted text link (`.rio-action-link--muted`).

### 8.2 Action bar

One wrapping row with a hairline top border (`--rio-border-soft`), s6 margin-top,
s5 padding-top, s4 gap. Order: primary → secondary save variants → (auto gap via
`margin-inline-start:auto`) → muted/destructive text actions + Cancel. No orphaned
Cancel on its own line.

## 9. Tables and status pills

`th` 14px / weight 800 / uppercase / `--rio-text-muted`; cells `--rio-fs-md` /
`--rio-text-strong`; s4 padding; soft row dividers; **no zebra**. Row actions are
700-weight text links. Pills: `inline-block`, `0.25rem 0.75rem`, pill radius,
`--rio-fs-sm`/700; `--on` and `--off` use the §1 pill tokens. Status-pill **text
is lowercase** (`enabled` / `disabled`, per `feature_flags.png`) — set it where
the pill is emitted, not by capitalizing the source string.

## 10. Layout primitives

- `.rio-form-shell { max-inline-size: 880px; margin-inline: auto; }`
- `.rio-content-shell { max-inline-size: 1040px; margin-inline: auto; }`
- Two-column field grid: `1fr 1fr`, gap s5 (24px), stacks at 768px.
- Inline code/kbd chip: `--rio-surface-tint` bg, mono, 6px radius,
  `0.125rem 0.5rem`, `0.875em`.
- Spacing scale: s1=4, s2=8, s3=12, s4=16, s5=24, s6=32, s7=48 (px).

## 11. Known reference defects (normalize, do not copy)

1. `group_edit.png` — permission-grid checkboxes render native blue. Target: teal.
2. `form.png` — Cancel wraps to an orphaned second line. Target: one wrapping row.
3. `admin_reset_password.png` — a radio control is misaligned against the bold line.
   Target: control centered on the first text line.

### 11.1 Documented deferral — user_new "Active" checkbox

`user_new.png` pairs **Role | Active** (2-col) in the IDENTITY section. The user
model carries `is_active`, but `create_user` always inserts `is_active = TRUE` —
a functional Active toggle on the **create** form would need new create-path
behaviour (and a product decision on whether the admin should mint inactive
accounts). It is **deferred, not built**: the create form shows Role full-width
and omits Active. This is an adjudicated deviation, not an oversight. (An *edit*
form, where `is_active` is real existing state, may surface it later.)

## 12. Dark theme and RTL

Dark theme is **mandatory and token-driven only** — re-derived from the §1 light
tokens in the dark blocks of `tokens/colors.css` (`@media (prefers-color-scheme:
dark)` for auto + `[data-theme="dark"]` for an explicit toggle; auto before
explicit so an explicit theme wins by source order). No per-component dark CSS.
Body text must clear 4.5:1 on its dark surface; the accent lifts for AA. Generated
overrides must follow `TOKENS-EMIT-SPEC.md`.

RTL: all new CSS uses logical properties (`margin-inline-start`, `text-align:
start`). `letter-spacing` is neutralized to 0 for Arabic/RTL on
`.rio-fieldset > legend` and `.rio-table th` (connected script breaks under tracking).

## 13. Acceptance checklist

Verify each touched page light/LTR against its screenshot, then light/RTL,
dark/LTR, dark/RTL: header rhythm (§2.1); **section pattern matches its §3 case**
(legend-on-border for multi-section forms, eyebrow-heading for list/table
sections, legend-less for single-group create cards); 14px-min cards with the §1
surfaces; 44px inputs with the teal 4px focus ring; full-width teal radio rows;
all checkboxes teal; the §8 button/action-bar taxonomy; §9 tables (16px cells,
14px/800 headers, no zebra, lowercase §9 pills); inline code chips; **no UI text
below 14px**; nothing cramped; top bar/sidebar/footer untouched.

## 14. Implementation approach

Update **tokens**, not scattered selectors. Map existing class names onto this
contract before inventing new ones. Hand-written CSS only — no build step, no
Tailwind, no JS framework, no second runtime. Postgres-only, baked stylesheets.
