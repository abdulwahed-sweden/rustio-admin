# Admin Chrome — Conventions for Operational Surfaces

The contract for the framework's *chrome* surfaces — the bars,
strips, and badges that wrap working content with operational
metadata. Governs the footer, the topbar, and any future
RustIO-owned shell element that carries runtime context rather
than primary content.

Companion to `DESIGN_SYSTEM.md` (which owns tokens and authority
doctrine). PR review for any new chrome surface is against this
document.

> **Status**
>
> Stabilised in 0.11.0 alongside the production footer and the
> multilingual typography stack. Topbar predates this document
> but is retroactively bound by sections 2 + 5; any topbar change
> after 0.11.0 is reviewed against the invariants below.


---


## 1. Purpose

### 1.1 What this document governs

Every framework-owned UI surface that is **not** primary content:

- The footer (`layout/footer.css` + `_base.html`).
- The topbar (`layout/topbar.css` + `_topbar.html`).
- The demo-session banner (`_base.html` `.rio-banner--demo`).
- Future chrome — runtime-status strips, multi-tenant org
  switchers, embedded shell previews in CLI output rendering,
  the bulk-action confirmation bar (Phase D pending).

Chrome is the framework speaking on behalf of itself: version,
environment, identity, navigation, render time. It is **not**
the project's content layer and **not** the consuming app's
branding canvas.

### 1.2 What this document does not cover

- Tokens themselves — `DESIGN_SYSTEM.md` §4 / §9.
- Per-page content layout — lives in the page's CSS fragment.
- Form, table, button, dropdown component shapes —
  `components/*.css`, individually doctrine-light.
- Project-level theme overrides — `Admin::theme(...)`; the
  override surface is documented in `DESIGN_SYSTEM.md` §4.2.
- Topbar specific behaviours (theme toggle, sidebar drawer
  handle) — those are interaction concerns, not chrome
  conventions.


---


## 2. Invariants

The rules every chrome surface must honour. PRs that violate
these are rejected on principle, not on taste.

### 2.1 Useful information, not marketing

Chrome carries metadata that an operator needs at-a-glance:
version, environment, identity, render time, navigation to
audit / sessions / docs. It does **not** carry:

- Slogans, taglines, or product copy.
- Oversized logos or brand wordmarks.
- "Powered by" attributions of any kind.
- Status messages dressed as banners (those belong to
  `.rio-flash`, not chrome).

### 2.2 Single concentrated point of emphasis

Each chrome surface has **at most one** saturated pixel range.
Everything else lives in the muted-text / surface / border
band. The footer's environment dot (6×6, calm green / amber /
neutral) is the canonical example: every other glyph in the
footer is `--rio-text-subtle` or `--rio-text-muted` on
`--rio-surface`; only the dot earns colour.

The motivation is operator focus. Two saturated elements in the
same bar compete; one wins.

### 2.3 Hairline separation, surface graft

Chrome sits on `--rio-surface` — the same surface the topbar
uses — with a single 1px `--rio-border` hairline as its only
edge against the working content. No filled backgrounds beyond
the surface token. No gradients. No shadows that lift the
chrome above the shell. The chrome belongs to the same plane as
the topbar.

### 2.4 Tabular numerals on operational data

Any chrome surface that displays numbers an operator reads —
version, timestamp, count, duration, request id — sets
`font-feature-settings: "tnum" 1` so columns and digits stay
aligned across requests. The footer's version + render
timestamp follow this rule.

### 2.5 Locale-respecting

Chrome inherits the document's language resolution. The
footer's user-email span renders in `--rio-font-arabic` when
the email is in Arabic and the closest `lang` ancestor is `ar`;
in Noto Sans JP when the closest ancestor is `ja`. Chrome
**does not** force a Latin-only family for "metadata".

What chrome **does not** auto-mirror: the layout direction.
Chrome columns stay left → centre → right even on RTL pages.
The reasoning is convention parity — Django, GitHub, and the
AWS console all keep operational chrome LTR on RTL surfaces
because chrome is read as a metadata strip, not a sentence.
Content inside each column shapes correctly per its lang
attribute.

### 2.6 Calm typography

Chrome typography is small, low-contrast, and unanimated:

- Size: `--rio-fs-xs` (13px). No exceptions.
- Default colour: `--rio-text-subtle`.
- Emphasis (brand, user email): `--rio-text-muted` with
  `--rio-fw-semibold` or `--rio-fw-medium`. Never `--rio-text`
  or `--rio-text-strong` — those belong to content headings.
- No animations beyond the 120ms colour/border-colour link
  hover. No spinners. No pulses.


---


## 3. The three-column rhythm

The canonical chrome layout. Used by the footer at 0.11.0; the
intended template for the bulk-action confirmation bar and any
future operational strip.

```text
┌────────────────────────────────────────────────────────────┐
│   identity                navigation              context   │
├────────────────────────────────────────────────────────────┤
│   brand · version · env   docs · audit · sessions   user · time │
└────────────────────────────────────────────────────────────┘
```

| Column | Carries | Examples |
|---|---|---|
| **Identity** (left) | What this surface *is*. | Framework name + version + environment badge. Bulk-action bar would carry "Bulk action — 17 selected". |
| **Navigation** (centre) | Where the operator can go from here. | Documentation, audit log, sessions. Bulk-action bar would carry per-action buttons. |
| **Context** (right) | Who and when. | Current user, render timestamp. Bulk-action bar would carry the same. |

The grid is `1fr auto 1fr` so the centre stays centre-anchored
even when the side columns are asymmetric. Below 720px the
columns stack vertically and the inline separators (`·`) remain
visible as punctuation.

### 3.1 Separator

The mid-dot `·` (U+00B7) is the only acceptable inline
separator in chrome. It is `aria-hidden="true"` so screen
readers skip it. Its colour is `--rio-text-subtle` at opacity
0.45, soft enough to read as rhythm rather than punctuation.

### 3.2 Information hierarchy

Within a column, the **most operationally significant** datum
sits leftmost. In the footer's identity column, "RustIO Admin"
(framework) precedes "v0.10.2" (version) precedes the
environment badge — because an operator reading left-to-right
gets framework first, then specifics. In the context column,
the operator's identity precedes the timestamp for the same
reason.


---


## 4. The environment badge

The canonical pattern for "this is a chip-shaped chrome
indicator with a single coloured dot of emphasis." Reusable
beyond the footer.

### 4.1 Anatomy

```html
<span class="rio-footer__env rio-footer__env--prod"
      title="Runtime environment (Production)">Production</span>
```

- Neutral chrome: 1px `--rio-border`, `--rio-surface-2`
  background, `--rio-radius-sm`, uppercase 0.06em-tracked label
  in `--rio-text-muted`.
- A 6×6 `::before` dot, `border-radius: 50%`, sits left of the
  label. The dot is the **only** coloured surface; everything
  else is neutral.

### 4.2 Kind discriminator

The CSS class suffix maps to a stable kind, computed from a
free-text label:

| Label (free text) | Kind | Dot colour |
|---|---|---|
| `Production`, `prod` | `prod` | `#16a34a` (calm green) |
| `Development`, `dev` | `dev` | `#d97706` (calm amber) |
| anything else (`Staging`, `Sandbox`, …) | `other` | `--rio-text-subtle` (neutral) |

Free-text labels collapse to neutral. The framework refuses to
guess a "staging dot colour" — operators set the label they
want; only the two operational extremes earn dots.

### 4.3 When to reuse

The dot-chip pattern is the framework's standard "metadata
indicator with subtle status emphasis." Apply it for:

- Future runtime indicators (e.g. read-replica vs primary DB).
- Tenant / org badges in multi-tenant chrome.
- Build-info chips (e.g. commit SHA short, build time).

Do not apply it for:

- Per-row status (use `.rio-pill` from `components/pills.css`).
- Form validation state (use `.rio-flash`).
- Authority / role badges in user lists (separate UI, separate
  rules).


---


## 5. Plumbing — server-side context

Chrome surfaces consume data from `BaseContext` (`src/admin/render.rs`),
which is built once per request inside `BaseContext::new` and
serialised into every template that extends `admin/_base.html`.

The chrome-relevant fields:

| Field | Type | Source |
|---|---|---|
| `framework_version` | `&'static str` | `env!("CARGO_PKG_VERSION")` — same source as `Cargo.toml`. |
| `environment_label` | `&'static str` | `RUSTIO_ENV` env var → `OnceLock`-cached. Falls back to "Development" for debug builds, "Production" for release. |
| `environment_kind` | `&'static str` | Derived from label; one of `prod` / `dev` / `other`. |
| `server_now` | `String` | `chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")` — formatted at render time. |
| `identity` | `Option<IdentityCtx>` | Session identity; chrome that surfaces the operator's email reads `identity.email`. |

The `RUSTIO_ENV` pattern (build-time default, env-var override,
process-cached lookup) is the recommended shape for any future
chrome-relevant fact:

1. Static default derived at compile time from `cfg!`.
2. Override path via a documented env var.
3. `std::sync::OnceLock` so the first request pays a syscall and
   no subsequent request does.

New chrome surfaces that need additional context should add
fields to `BaseContext`. The existing call sites pick them up
without signature changes because `BaseContext::new` is
parameterless beyond the per-request identity / CSRF.


---


## 6. Mobile collapse

The three-column grid stacks to single-column below 720px.
Separators remain inline; padding tightens from `--rio-s5` to
`--rio-s4` on the side axis to give the stacked rows breathing
room without bloating the chrome's vertical footprint on small
screens.

Stack order is preserved (identity → navigation → context),
because the operational rhythm reads the same way on phones —
operator sees framework first, then where to go, then who they
are.

Chrome surfaces **do not** disappear on small screens. They are
operational metadata; hiding them on mobile is hiding facts. If
something has to give, a column may collapse to fewer items
(the footer's navigation column drops audit + sessions for
unauthenticated users today via a `{% if identity %}` guard,
not via responsive CSS — the same pattern applies at the column
level if needed).


---


## 7. What chrome must never do

Hard refusals. Any of these turn up in a chrome PR, the PR gets
sent back.

- No JavaScript-driven content in chrome strips. Server-side
  rendering only. The theme-toggle button is the single
  exception, and lives in the topbar's own JS, not in chrome
  template logic.
- No tracking pixels, no analytics scripts, no third-party
  embeds. Chrome is the framework's plane; nothing in it
  reaches outward.
- No filled-colour bars (the brand-accent footer experiment in
  unreleased history was correctly retired). Saturated colour
  belongs to a single point of emphasis per surface.
- No content that requires `identity` outside an `{% if
  identity %}` guard. Chrome must render cleanly on the login
  page.
- No marketing language. "Built with rustio-admin" / "Powered
  by …" is style violation territory; the framework's value is
  evident from the surface itself.
- No CLS regressions. Variable-font weight axes are fine
  (Inter swaps weights without a metric shift); a new
  static-weight family in chrome would be flagged.


---


## 8. Open work

Anchored here so future chrome adoption stays cohesive.

- **Bulk-action confirmation bar** (Phase D, queued). Should
  follow §3's three-column rhythm: identity column carries
  "Bulk action — N selected"; navigation column carries the
  per-action buttons; context column carries the operator
  identity + a cancel link. Calmly styled; the action buttons
  are the single emphasis point.
- **Read-replica / DB-health chip** in the topbar, eventual.
  Slot below the framework brand using §4.3's reusable dot-chip.
- **Per-page render-timing breadcrumb**, optional. If added,
  goes in the footer's context column after `server_now` as
  `· 42 ms`.

Each of the above is a separate doctrine-light commit on top of
this contract.
