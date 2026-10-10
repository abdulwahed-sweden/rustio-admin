# Page composition study

A design-only study of rustio-admin's **page composition and component
structure** on top of the accepted, frozen visual system. The theme (colours,
type ladder, token values, focus, radii, shadows, spacing scale, light-only
direction) is not touched. What is redesigned is how each page is put
together: toolbars, table scan path, filter/search/sort grouping, page-head
relationships, form grouping and action placement, related sections, the
permission matrix, sessions, API surface, history, dashboard, empty states,
narrow reflow.

**No runtime change.** No Rust, template, production CSS or JS, token,
AdminTheme, rio-theme, test or `main` file is modified. The branch holds
documents, boards and a mock stylesheet. No implementation PR, no merge.

## Baseline

| | Value |
|---|---|
| Branch studied | `design/phase-0-visual-contract @ b5fa45c` — the implemented visual system (Phases 0–7) plus `examples/fixshop` |
| Study branch | `design/page-composition-study`, created from that commit |
| Example project | `examples/fixshop`: Customers, **Jobs**, Quotes, Job events; the seven-step workflow ladder in the Jobs bulk bar; `front_desk` / `technician` groups |
| Priority page | **Jobs** (`/admin/jobs`) — redesigned first and deepest |
| Date | 2026-10-09 |

## How the boards were made

- **CURRENT** frames are the real templates
  (`crates/rustio-admin-assets/assets/templates/admin/*.html` at `b5fa45c`)
  rendered with fixshop contexts against the real 32-fragment CSS bundle in
  `ADMIN_CSS` order. They are the product as it renders, not a drawing.
- **REDESIGNED** frames use the same real shell (`_base.html`, `_topbar.html`,
  `_sidebar.html`, the footer) with redesigned page content, against the same
  untouched bundle plus `build/composition.css`.
- `build/composition.css` is the proposal as a stylesheet layer. It
  references only existing `--rio-*` tokens and introduces no new colour,
  size, height, radius, shadow or spacing value. It is a design artefact; the
  production home of each rule is in `HANDOFF.md`.
- Every page was rendered with Playwright/Chromium at 1440 and 390 and
  checked for document-level horizontal overflow (none). Neither repository
  was built or run.

## Reading order

1. `CRITIQUE.md` — per page: what is structurally wrong, inherited, noisy,
   wasteful, unclear; what can be simplified; what stays.
2. `boards/index.html` → `boards/Jobs-Data-Table.html` — the direction, on
   the priority page.
3. `SPEC.md` — each redesigned composition and its class
   (CSS ONLY · TEMPLATE + CSS · REQUIRES WIRING · UNCHANGED).
4. `HANDOFF.md` — template and CSS fragment for every change, the order to
   land it in, the tests that touch it.

## Boards

| Board | Shows |
|---|---|
| `index.html` | the frozen/moving split, the twelve structural problems, the twelve improvements, what stayed, the board list |
| `Jobs-Data-Table.html` | critique · CURRENT vs REDESIGNED · the selection state · the two strip rules |
| `View-Modes.html` | List, Cards, Compact with the head strip that earns its place |
| `Dense.html` | 61 jobs, two active filters, page 1 of 3; at 1100 |
| `Form.html` | Edit job: facts panel, related sections in an aside, one card with its own foot; 390 |
| `Dashboard.html` · `Users.html` · `Groups-Permissions.html` · `Sessions.html` · `API-Surface.html` · `History.html` | critique + CURRENT vs REDESIGNED |
| `Empty-States.html` | filtered and fresh, inside the surface |
| `Narrow.html` | Jobs (CURRENT vs REDESIGNED), form, dashboard, sessions, group editor at 390 |
| `Comparison.html` | every page CURRENT vs REDESIGNED, and the change table |

## What is preserved

The two-row header, module navigation, contextual rail, ⌘K, account menu,
notifications, filters (all kinds), sort, pagination, bulk actions, the three
save variants, inline related sections, permissions, sessions, API surface,
history, health, DB browser, feature flags, docs viewer, view designer,
AdminTheme, rio-theme. Every route, handler, hidden input, `_csrf`,
`data-rio-*` hook and permission gate. The arrangement changes; nothing is
removed.

## Completion pass (2026-10-10)

Composition 3.1 shipped on `main @ d495e80` (the list, form, dashboard, users
and groups pages). This pass audited **every** remaining admin page against
that shipped direction and redesigned the ones still on the old composition.
Same rules: theme frozen, runtime read-only, design artefacts only.

| | |
|---|---|
| Baseline | `origin/main @ d495e80` — CURRENT frames are the real templates at that commit |
| Reviewed | 58 templates (49 pages, 9 shell partials), each rendered at 1440 and 390 |
| Already Composition 3.1 | 7 templates |
| Special but complete | 15 templates (signed-out auth, confirmations, errors) |
| Redesigned | 27 templates + the search page (new); 17 renders on 9 boards, 11 spec-only |

Reading order:

1. `INVENTORY.md` — every page, its class (A / B / C), priority and reason;
   the cross-cutting shell findings.
2. `boards/completion/index.html` — the six mandatory pages, the search
   definition, the board list.
3. `SPEC-COMPLETION.md` — each redesigned composition: problems, new
   composition, reused patterns, added components, narrow behaviour,
   implementation notes.
4. `HANDOFF-COMPLETION.md` — exact templates and fragments, what to retire,
   the wiring list, the order to land it in, the tests that touch it.
5. `build/composition-completion.css` — the proposal as a stylesheet layer,
   tokens only.

Boards: `Audit-Log` · `API-Reference` · `Health` · `Docs` · `Sessions` ·
`Search` (mandatory) · `Utility-Pages` · `Developer-Pages` ·
`Account-Flows`.

## Polish pass (2026-10-10)

`POLISH.md` — a visual quality pass over the implemented branch
`feat/composition-completion`: 42 renders inspected at 1440 and 390, 22
defects with the exact correction, fragment and severity, and the order to
land them in. No design change.
