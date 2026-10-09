# Critique — v3, against RustIO `main @ 00b933c`

Baselines:

| Repository | Commit | Note |
|---|---|---|
| rustio-admin | `d4b61fa` (`main`) | unchanged since v1 |
| **RustIO `main`** | **`00b933c`** | **authoritative.** PR #6 squash-merged. `admin.css` byte-identical to the PR #6 snapshot v2 used |
| RustIO PR #5 | `884bf3b` | unmerged, not authoritative |

v2 was written while PR #5 and PR #6 were both open and judged between them
per component. That judgement is withdrawn: `main` decides. This critique
keeps §A–G from v2 where they still hold, rewrites the points that were
grounded in PR #5, and adds §H, the reclassification of every PR #5-only
idea.

---

## A · What from v1 and v2 remains correct

1. **The gap is composition, scale and density — not colour.** `main`'s
   `:root` carries the same values rustio-admin's `tokens/colors.css` already
   has (`#F1F1EE`, `#1F5797`, `#174578`, `#DFEAFB`, `#B7CDEA`, `#2F7BD6`,
   `#D5DCE5`, `#B9C5D3`, `#F4F6F9`). `main` changed no token value.
2. **The primary stays `#1F5797`.** It is `main`'s default `--blue` and
   rustio-admin's `--rio-rust`.
3. **Workspace geometry.** `main`: `.main { width: min(100% - 2 * var(--gutter),
   var(--content)); margin: 0 auto }`, `--gutter` 32 / 24 / 16. rustio-admin's
   `.rio-ws-inner` is the same geometry already. KEEP.
4. **Token strategy: canonical names + `--rio-*` aliases.** Unchanged.
5. **Quiet row actions, unframed badges, flat tiles and empty states, one
   form action area, List as one surface, Compact as a denser surface, find
   row + data-surface head.** All on `main` (`.button-quiet`, `.row-actions`,
   `.badge` without border, `.stat` flat, `.find`, `.data-head`,
   `.record-list-*`, `.record-compact-item`). ALIGN.
6. **Identity columns hug.** `main`'s `column_hugs` → `.cell-fit`. ALIGN.
7. **The contract conflict** (`VISUAL-CONTRACT.md` v2.1: 44px controls, no
   text under 14px) is still the one real blocker and still a document.

## B · What is outdated — because v2 followed PR #5

1. **Page head.** v2 proposed a band with a rule beneath, title left and
   actions right, on PR #5's measured reversal. `main`'s `.page-head` is the
   grid `"crumb crumb" / "title action" / "lead lead"`: the primary action
   one gap after the title, `justify-self: start`, the lead beneath, no rule.
   **v3 follows `main`.** The band is DROPPED. For rustio-admin this is v1's
   proposal again, with the lead inside the first wrapper as the real
   templates have it.
2. **Compact.** v2 carried PR #5's `--th-h-compact: 32px` / `--td-h-compact:
   38px` and kept Compact a table. `main`'s Compact is `.record-compact-item`:
   no column head, `min-height: var(--th-h)` (40px), title · badge · actions.
   **v3 follows `main`.** The tokens are DROPPED; rustio-admin's
   `av-list--compact` takes the 40px line.
3. **Form measure.** v2 credited "728 centred" to PR #5's `--form-measure`.
   `main` has no such token: the form card is `minmax(0, calc(680px + 2 *
   var(--s5)))` (728) in a `.form-layout` grid beside a System aside, inside
   the 1120 `--content` measure — left-aligned, not centred. rustio-admin has
   no readonly aside and already centres its form on `rio-page--form`. **v3
   keeps rustio-admin's centred measure, widened to 728 so the card matches
   `main`'s.** INTENTIONAL DIVERGENCE, reason stated.
4. **Timestamps.** v2 credited the family format to PR #5. On `main`, record
   cells render the stored value; `main`'s own history, actions and error
   pages use `%Y-%m-%d %H:%M UTC`. rustio-admin disagrees with itself across
   four pages. The rule is a rustio-admin consistency fix: IMPROVE
   RUSTIO-ADMIN, with the format rustio-admin's user detail and footer
   already use (which is also `main`'s audit format).
5. **Cells by kind (nowrap, tabular digits).** Not on `main`. rustio-admin
   emits `rio-td--{{ f.kind }}` already; the rule is its own. IMPROVE.
6. **Sticky actions column.** Not on `main`. IMPROVE; unconditional, inert
   while the table fits.
7. **Card identity floor (12rem, `break-word`).** `main`'s
   `.record-card-identity` is a plain wrapping flex row. IMPROVE, for
   rustio-admin's longer record names and API card titles.
8. **"One blue, focus never follows the accent."** v2 cited PR #5's removal
   of the sign-out page's accent injection. On `main` both `session.rs` and
   `shell.rs` still inject `--blue`, `--blue-dark` and `--focus` from the
   project's `rustio.design.json`. So on `main`: the *defaults* are one blue
   with a distinct focus; a project may override both. rustio-admin's
   `AdminTheme` overrides the primary and never touches `--rio-accent-focus`.
   That stays as it is: INTENTIONAL DIVERGENCE (existing behaviour, kept).
9. **Filter control width, disclosure chevron.** PR #5-only, with no
   rustio-admin counterpart (dropdown triggers; native `<summary>` marker).
   DROPPED.

## C · rustio-admin components that are already strong and should remain

Unchanged from v2: the three-measure `page_measure` system; the two-row
header; the ⌘K palette; the filter dropdown panels and active-filter pills;
bulk selection; the users grid; the form action bar order; inline related
sections; the permission matrix; the session list; API cards; history diffs;
the docs viewer; the full-width footer; `AdminTheme` as a patch layer that
never touches focus.

## D · Components that are visually weak and worth improving

Unchanged from v2: the boxed page head; hover-revealed icon row actions;
three badge systems; the 16/44 scale; the hero search and `--lg` triggers;
monogram tiles; five stacked session cards; padded empty states; four
timestamp formats; the AdminTheme override that flattens hover and active.

## E · What should NOT be changed

Unchanged: every product capability, route, handler, the `page_measure`
mechanism, the footer, `AdminTheme` and `rio-theme` behaviour, Postgres-only
and no-build-step rules.

## F · Token conclusions that remain valid

All of `TOKEN-MAPPING.md`. `main` adds `--gutter` (ALIGN). `main` has no
`--form-measure` and no compact tokens; v2's rows for them are withdrawn.
`main` still has no overlay / popover / large-shadow token; rustio-admin
keeps its own.

## G · What should change in the earlier studies

| v2 said | v3 says | Why |
|---|---|---|
| Head is a band with a rule | Head is `main`'s grid: action beside the title, lead beneath | PR #5 only |
| Compact = 32 / 38 tokens, a table | Compact = 40px lines, no head | PR #5 only; `main` differs |
| Form 728 centred "per PR #5" | Form 728 centred as rustio-admin's own mechanism; divergence from `main`'s left-aligned card + aside | the number is `main`'s card width; the centring is rustio-admin's |
| One timestamp format "per PR #5" | One timestamp format as a rustio-admin fix | not on `main` as a cell rule |
| Cells by kind, sticky actions, card floor "per PR #5" | rustio-admin improvements | not on `main` |
| Identity hug "per PR #6" | Identity hug per `main` | merged |
| "Focus never follows accent, RustIO settled one blue" | `main` keeps per-project `rustio.design.json` injection of primary and focus; rustio-admin keeps `AdminTheme` which never touches focus | `main` differs; divergence kept |
| RustIO is forked; two boards depend on the outcome | No fork; `main` decides | PR #6 merged |

## H · Reclassification of every PR #5-only idea

| PR #5 idea (commit) | v2 carried it as | v3 class | Reason |
|---|---|---|---|
| Page head band with rule; action at the far edge (`87995b3`) | ALIGN | **DROP** | `main`'s `.page-head` puts the action beside the title |
| Form on its own measure, `--form-measure` 728, centred (`ee950bb`) | ALIGN | **INTENTIONAL DIVERGENCE** | rustio-admin already centres its form; it has no System aside; the card width 728 matches `main`'s card. `main` left-aligns the card in the 1120 column |
| Compact tokens `--th-h-compact` 32 / `--td-h-compact` 38; Compact as a table (`57a48d8`) | ALIGN | **DROP** | `main`'s Compact is 40px lines with no head |
| Timestamp cells `YYYY-MM-DD HH:MM UTC` (`3d82af9`) | ALIGN + IMPROVE | **IMPROVE RUSTIO-ADMIN** | not a cell rule on `main`; rustio-admin disagrees with itself; the format is `main`'s own audit format and rustio-admin's user-detail format |
| `data-role` nowrap on timestamp / badge cells; tabular digits (`62165e3`) | ALIGN + IMPROVE | **IMPROVE RUSTIO-ADMIN** | not on `main`; rustio-admin already emits cell kinds |
| Sticky actions column 761–1100 (`62165e3`) | IMPROVE (widened) | **IMPROVE RUSTIO-ADMIN** | not on `main`; unconditional in v3 |
| Card identity floor 12rem, no `margin-left: auto` on the state group (`884bf3b`) | ALIGN | **IMPROVE RUSTIO-ADMIN** | `main`'s card identity is a plain flex row; the floor protects rustio-admin's longer names |
| Filter `select` max-width 220px (`2b32c8f`) | "no change" | **DROP** | rustio-admin's filters are dropdown triggers |
| Disclosure chevron on `<summary>` (`2b32c8f`) | "keep native marker" | **DROP** | nothing to do; native marker already present |
| Removal of sign-out accent injection; one blue (`e307f31`) | evidence for "one blue" | **DROP** (as evidence) · **INTENTIONAL DIVERGENCE** (as behaviour) | `main` keeps the injection in `session.rs` and `shell.rs`; the shared *default* blue stands; rustio-admin's `AdminTheme` never retargets focus and keeps that |
| `--content--form` shell width class (`ee950bb`) | implied | **DROP** | `main` uses `.form-layout` inside `--content`; rustio-admin uses `page_measure` |
| Layered `admin-composition.css` appended by `build.rs` | not adopted | **DROP** | `main` is one `admin.css`; irrelevant to rustio-admin's 30-fragment bundle anyway |

Nothing else in v2 came from PR #5 alone.
