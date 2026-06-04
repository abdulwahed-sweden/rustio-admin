# RustIO UI — Design System Reference

A reusable admin UI layer for all RustIO projects. The visual values are
extracted **verbatim** from the purchased Smarty 5 template (`core.css`) and
re-exposed under a clean `rio-*` namespace. No Bootstrap/Smarty class names
appear in the output — only `rio-*` classes plus Bootstrap Icons (`bi bi-*`,
MIT-licensed, loaded via CDN).

> **For an AI agent / Claude Code reading this:** treat `rio-tokens.css` as the
> single source of truth for values, and `rio-admin.css` as the component layer.
> When asked to build a new page or component, reuse existing `rio-*` classes;
> only add a new class when no existing one fits, and follow the conventions in
> section 6. Never hardcode colors, spacing, radius, or shadows — always
> reference a `--rio-*` token. Never reintroduce Bootstrap/Smarty class names.

---

## 1. Files

| File | Role |
|------|------|
| `rio-tokens.css` | Tokens only. Colors, type, spacing, radius, shadows, layout, topbar. Single source of truth. |
| `rio-admin.css` | Layout + all components. Reads tokens, never hardcodes values. |
| `rio-admin.html` | Reference dashboard. Links the two CSS files. |
| `rio-admin-standalone.html` | Same page, CSS inlined — for instant preview. |
| `build-standalone.mjs` | Generator: inlines the two CSS files into `rio-admin-standalone.html`. Run `node build-standalone.mjs`. |

Load order in any page: fonts -> Bootstrap Icons -> `rio-tokens.css` -> `rio-admin.css`.

```html
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;700&display=swap" rel="stylesheet">
<link href="https://cdn.jsdelivr.net/npm/bootstrap-icons@1.11.3/font/bootstrap-icons.min.css" rel="stylesheet">
<link rel="stylesheet" href="rio-tokens.css">
<link rel="stylesheet" href="rio-admin.css">
```

---

## 2. Design decisions (the "why", so they aren't undone)

- **Primary = Deep Navy `#1e3a5f`.** Chosen over the template's purple `#574fec`
  for a more formal, corporate tone. Everything brand-colored reads
  `var(--rio-primary)`, so re-theming = changing one token.
- **Topbar is dark navy** (`--rio-topbar-bg`, matches primary) with light text;
  the sidebar is **light** (white). This intentional contrast frames the content
  (GitHub-style). Topbar has its own `--rio-topbar-*` token group.
- **Stat-card icons are neutral gray**, not colored gradients — keeps the page
  calm and professional. Status badges stay colored because color = meaning.
- **Typography is dense and dark.** Small text was enlarged and darkened for
  legibility; large headings were left alone. Body weight leans `medium`.
- **Generous, deliberate spacing.** Card inner padding `1.5rem`, roomy rows,
  content max-width `1760px` (wide but capped).
- **Sidebar hierarchy:** group labels have a top divider + a small primary dot;
  the active item has a left accent bar. This separates "section heading" from
  "clickable item" at a glance.

> **Pure white note:** `#fff` is used only as the *foreground on solid brand/
> status fills* (button text, avatar/badge text). It is a non-themeable contrast
> colour, not a brand value, so it stays fixed when you re-theme. It is the only
> literal colour in `rio-admin.css`; everything else is a token.

---

## 3. Token reference (`rio-tokens.css`)

All values are Smarty-exact unless noted. RGB triplets exist for `rgba()` use.

### Brand / status
`--rio-primary` `#1e3a5f` · `--rio-primary-rgb` `30, 58, 95` ·
`--rio-primary-emphasis` `#15293f` (hover) · `--rio-primary-subtle` `#e3e9f0` (tint bg) ·
`--rio-primary-hover-bg` `#eef2f7` (nav hover).
`--rio-success` `#6dbb30` · `--rio-info` `#73e7f7` · `--rio-warning` `#fad776` ·
`--rio-danger` `#f64e60` · plus `-subtle` / `-emphasis` variants for soft badges.

### Surfaces / text / border
`--rio-surface` `#fff` · `--rio-surface-2` `#f1f4f8` · `--rio-surface-3` `#f9fbfd` ·
`--rio-text` `#1c0950` · `--rio-text-muted` `rgba(28,9,80,.85)` ·
`--rio-text-faint` `rgba(28,9,80,.68)` ·
`--rio-border` `#dde4ea` · `--rio-border-soft` `#eef2f5`.

### Gray ramp
`--rio-gray-100 ... 900` (`#f9fbfd` -> `#1b2a4e`), Smarty-exact.

### Typography
`--rio-font` (Inter stack) · `--rio-font-mono` · `--rio-fs-body` `1rem` ·
`--rio-lh` `1.5` · `--rio-fw-light/normal/medium/bold` `300/400/500/700` ·
`--rio-h4` `calc(1.275rem + 0.3vw)` (page title size).

### Radius / shadow
`--rio-radius-sm` `.2rem` · `--rio-radius` `.35rem` · `--rio-radius-lg` `.45rem` ·
`--rio-radius-xl` `.6rem` · `--rio-radius-pill` `50rem`.
`--rio-shadow-xs` (card) · `--rio-shadow-sm` (hover) · `--rio-shadow` · `--rio-shadow-lg`.

### Buttons / cards / layout
`--rio-btn-py` `.46rem` · `--rio-btn-px` `1rem` · `--rio-btn-fs` `1rem`.
`--rio-card-px` `1.5rem` · `--rio-card-py` `1.35rem`.
`--rio-sidebar-w` `280px` · `--rio-header-h` `68px` · `--rio-nav-px/py`.

### Topbar (dark navy frame)
`--rio-topbar-bg` `#1e3a5f` · `--rio-topbar-border` `#15293f` ·
`--rio-topbar-fg` `#fff` · `--rio-topbar-fg-muted` `rgba(255,255,255,.65)` ·
`--rio-topbar-field` `rgba(255,255,255,.10)` (inputs/buttons) ·
`--rio-topbar-field-fg` · `--rio-topbar-line` (borders) · `--rio-topbar-hover`.

### Dark sidebar (available, not active by default)
`--rio-sidebar-dark-bg` `#1b2a4e` · `--rio-sidebar-dark-fg` · `--rio-sidebar-dark-hover`.

---

## 4. Component classes (`rio-admin.css`)

### Shell
- `.rio-layout` — flex shell: sidebar + content.
- `.rio-sidebar` — fixed-width (`--rio-sidebar-w`) white sidebar, sticky, scrolls.
- `.rio-content` — flex column for header + main.
- `.rio-main` — page body, padded, max-width `1760px`, centered.

### Sidebar
- `.rio-brand` / `.rio-brand-mark` / `.rio-brand-name` — logo row, matches header height.
- `.rio-nav` — nav container.
- `.rio-nav-label` — group heading (CORE/DATA...). Has top divider + primary dot;
  the first one drops the divider automatically.
- `.rio-nav-item` — nav link (`<a>`). Add `.is-active` for the current page
  (adds left accent bar + bold). Hover tints with `--rio-primary-hover-bg`.
- `.rio-nav-item .bi` — leading icon. `.rio-nav-badge` — trailing count pill.

### Topbar
- `.rio-header` — dark navy bar, sticky, height `--rio-header-h`.
- `.rio-search` (wraps `<i.bi>` + `<input>` + `.rio-search-btn`) — translucent
  search field; `.rio-search-btn` is the inline submit button.
- `.rio-header-actions` — right-aligned cluster.
- `.rio-topbar-link` — text+icon link for the bar (e.g. View Site).
  Add `.rio-logout` for the red-tinted sign-out hover.
- `.rio-topbar-sep` — thin vertical divider.
- `.rio-icon-btn` — square icon button (e.g. notifications); `.rio-dot` = red badge.
- `.rio-user` — borderless account menu trigger: `.rio-user-badge` (initials
  circle) + `.rio-user-meta` (`.rio-user-name` + `.rio-user-role`) +
  `.rio-user-caret` (dropdown chevron). Intended to open a menu later.

### Page header
- `.rio-page-head` — title row (title left, primary action right).
- `.rio-page-title` (size `--rio-h4`) · `.rio-page-sub` (muted line under it).

### Stat cards
- `.rio-stats` — 4-col responsive grid.
- `.rio-stat` -> `.rio-stat-top` (`.rio-stat-label` + `.rio-stat-glyph` neutral icon),
  `.rio-stat-value`, `.rio-stat-delta` with `.up` / `.down`.

### Card
- `.rio-card` -> `.rio-card-header` (`.rio-card-title`) + `.rio-card-body`
  (add `.flush` for zero padding, e.g. when it holds a table).
- `.rio-grid-2` — main content / side column split (`1.8fr / 1fr`).

### Toolbar / buttons / forms
- `.rio-toolbar` — action row above a table; `.rio-spacer` pushes items right.
- `.rio-btn` + one of `.rio-btn-primary` / `-secondary` / `-danger` / `-ghost`;
  `.rio-btn-sm` for compact.
- `.rio-field` (label + control) wrapping `.rio-input` or `.rio-select`.

### Table
- `.rio-table` — full-width; uppercase muted `thead`, hover-tinted rows.
- `.rio-cell-user` (`.rio-cell-avatar` initials + name/sub),
  `.rio-cell-name`, `.rio-cell-sub`, `.rio-cell-ref` (monospace reference).

### Badge / pagination / feed
- `.rio-badge` + `.rio-badge-success` / `-warning` / `-danger` (soft, dot prefix).
- `.rio-pagination` (`.rio-page-info`, `.rio-page-btn` with `.is-active`).
- `.rio-feed-item` (`.rio-feed-glyph` + `.rio-feed-text` + `.rio-feed-time`);
  glyph gets `.f-success` for green.

### State / modifier conventions
- State: `.is-active` (current nav item / page button).
- Direction: `.up` / `.down` (stat delta). Layout: `.flush` (no card padding),
  `.rio-spacer` (flex push).

---

## 5. Re-theming a single project

Override tokens in a small file loaded **after** `rio-tokens.css`. Components
don't change. Example — give one project a teal identity:

```css
/* my-project-theme.css */
:root {
  --rio-primary: #0f766e;
  --rio-primary-rgb: 15, 118, 110;
  --rio-primary-emphasis: #115e59;
  --rio-primary-subtle: #cce7e3;
  --rio-primary-hover-bg: #effaf8;
  --rio-topbar-bg: #0f766e;       /* keep topbar in sync */
  --rio-topbar-border: #115e59;
}
```

To switch to the dark sidebar variant, add a small rule using the
`--rio-sidebar-dark-*` tokens that already exist.

---

## 6. How to add a NEW class (keep it consistent)

1. **Name it `rio-<thing>` (kebab-case).** Sub-parts: `rio-<thing>-<part>`
   (e.g. `rio-modal`, `rio-modal-header`). Variants: `rio-<thing>-<variant>`
   (e.g. `rio-btn-primary`). State: `.is-<state>`.
2. **Values come from tokens only.** Color -> `var(--rio-*)`; spacing -> the
   `rem` rhythm already in use; radius/shadow -> the radius/shadow tokens.
   If a needed value doesn't exist as a token and will be reused, add a token
   first, then reference it.
3. **Match the existing density:** card padding `1.5rem`, control padding
   ~`0.6rem 0.85rem`, transitions `0.18s`, font sizes from the existing scale.
4. **Topbar elements** use the `--rio-topbar-*` group (light-on-dark), not the
   normal text/border tokens.
5. **Icons** are Bootstrap Icons: `<i class="bi bi-..."></i>`. Keep them sized
   ~`1rem`-`1.1rem` and colored via the parent's `color`/token.
6. **Don't** introduce Bootstrap/Smarty classes, inline hex, or `localStorage`.

### Mini example — a new "tabs" component
```css
.rio-tabs { display: flex; gap: 0.25rem; border-bottom: 1px solid var(--rio-border); }
.rio-tab {
  padding: 0.6rem 0.9rem; font-size: 0.95rem; font-weight: var(--rio-fw-medium);
  color: var(--rio-text-muted); border-bottom: 2px solid transparent;
  cursor: pointer; transition: color 0.18s, border-color 0.18s;
}
.rio-tab:hover { color: var(--rio-primary); }
.rio-tab.is-active { color: var(--rio-primary); border-bottom-color: var(--rio-primary); }
```
```html
<div class="rio-tabs">
  <div class="rio-tab is-active">Overview</div>
  <div class="rio-tab">Settings</div>
</div>
```

---

## 7. Provenance & licensing note

Tokens were measured from the purchased Smarty 5 admin template (`core.css`):
primary, text, surfaces, gray ramp, radius, shadows, font, 265px sidebar metric,
etc. This kit reuses **design values** and ships its own `rio-*` class layer and
Bootstrap Icons (MIT) — it does not redistribute Smarty's CSS. For commercial /
SaaS use with paying users, confirm the Smarty license tier covers it.
