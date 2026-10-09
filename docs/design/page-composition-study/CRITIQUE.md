# Critique — the current pages, as they render

Written against the real templates at `b5fa45c` rendered with
`examples/fixshop` data. Each page: what is structurally wrong, what feels
inherited, what makes noise, what wastes space, what hierarchy is unclear,
what can be simplified without losing function, what should stay exactly
as it is.

## Jobs (the list page) — priority

**Structurally wrong**
- The page's real work — moving a job through the ladder (Diagnosed → Quote
  sent → Customer approved / declined → Start repair → Ready for collection →
  Collected) — exists only in the bulk bar, which is hidden until a checkbox
  is ticked. The visible controls are Sort and rows-per-page.
- When a selection exists, the bulk bar appears **above** the sort strip:
  two strips stack on the surface.
- The data-surface head duplicates the table: column heads already sort;
  the Sort and Direction dropdowns are a second way to do the same thing.
  Rows-per-page sits up here, away from the pagination it belongs with.
- Three filters are rendered as two plus "More filters" holding the third —
  the `loop.index <= 2` rule in `list.html` was written for many filters and
  applied to three.

**Inherited / dated**
- The head strip is the old command bar cut in two.
- `status` shows the wire value `in_progress`; booleans are generic on/off
  pills.
- The lead "All jobs in FixShop." is a template default that says nothing.

**Visual noise**
- Green "yes" pills in two columns (one of them *overdue*), red Delete on
  every row, the grey strip, a breadcrumb that repeats the rail.

**Wasted space**
- 56px of strip for two rarely used controls; a 48px checkbox column with
  16px padding on both sides; a lead line.

**Unclear hierarchy**
- Nothing tells the operator that FS-1001 and FS-1003 are the two overdue
  jobs except two green pills that read as good news.

**Simplify without losing function**
- Drop the strip in Table mode; move rows-per-page to the foot; show all
  three filters; one Clear; humanise choice values; booleans as marks; the
  selection strip in the head slot, grouped by what the data already says
  (`destructive`).

**Stays as is**
- The page head grid, the search field, the dropdown panels and active-filter
  pills, column heads and sort affordance, 48px rows, quiet text Edit /
  Delete, the sticky actions column, pagination.

## Form (Edit job)

**Structurally wrong**
- Read-only fields (`status`, `quote_approved`, `overdue` — the shop's three
  most important facts) render as disabled, greyed inputs at the bottom of
  the card.
- The action bar sits between the fields and the related sections; the page
  ends with Quotes and History, not with Save.
- Inline rows use icon-only Edit / Delete (`rio-iconbtn`) — the pattern the
  list pages already replaced with text.

**Inherited / dated**
- One 32px-padded card with a two-column grid applied to every field: a date
  input stretched to half the card; a textarea the width of a checkbox box.
- "Editing fields below — saves are atomic and audited." as a lead.

**Wasted space**
- The 728 measure leaves most of a 1440 screen empty while the related
  sections double the page height.

**Simplify**
- Facts panel for read-only fields; related sections as surfaces in an
  aside; the action bar as the card's foot; inputs sized by what they hold.

**Stays as is**
- The three save variants, History · Delete · Cancel, every widget,
  validation, hidden inputs, the POST shape.

## Dashboard (Site administration)

- **Wrong**: Docs is the page's primary (blue) action; four stat tiles for
  two numbers an operator uses (rows, recent events) and two they do not
  (model count, version).
- **Inherited**: every row repeats the app label beneath the model name;
  "3 fields" in mono as if it were data.
- **Simplify**: one facts line under the title; the models as the one
  surface with a title band; Browse / Add as quiet actions on 48px rows.
- **Stays**: Audit log (administrators), Docs, per-model Browse and Add, the
  counts.

## Users

- **Wrong**: the users grid is a `display: grid` table whose head rule is
  drawn per cell and breaks into segments; 72px rows for one line of data.
- **Inherited**: the email with `#2` on a second line; a lead promising
  last sign-in and MFA, which the table does not show; Created without the
  zone.
- **Simplify**: an ordinary table — identity hugs, id as a mono suffix on
  the same line, 48px rows, the family timestamp.
- **Stays**: Add user, the role chip, Active / Inactive, Edit / Delete.

## Groups / Permissions (Edit group)

- **Wrong**: the five workflow permissions (diagnose, quote, approve,
  repair, collect) — the reason the `front_desk` group exists — are folded
  under "Other permissions (5)"; the page sits on the 1600 measure for a
  four-column matrix.
- **Inherited**: legend-on-border fieldsets; a bordered, rounded matrix
  inside a bordered, rounded card; mono "All" buttons.
- **Simplify**: one card on the standard measure, two bands; the matrix
  flat; the extras open as a checklist grid; the action bar as the foot.
- **Stays**: every checkbox name, the row-toggle script hook, Save changes,
  Delete group, Back to groups.

## Sessions

- **Wrong**: three facts per session cost a card each with a four-column
  definition list and a footer; the current session is a 220px blue block.
- **Simplify**: one list, one row per session — device as the identity, ip
  and the three times as a meta line, trust badge, action at the edge.
- **Stays**: Revoke per session, Sign out of every other device, Sign out
  everywhere as the solid danger.

## API surface

- **Inherited**: a glyph tile per card; the same five endpoint rows per
  model with their own borders; two cards per row with a ragged bottom.
- **Simplify**: one flat section per model — name and base path in the
  head, endpoints left, fields right.
- **Stays**: the three head actions, every path, the field table.

## History

- **Noise**: an avatar circle on every row; relative time in 15px beside
  mono date dividers; the IP in 15px mono.
- **Simplify**: no avatars; metadata as metadata; date dividers sticky
  under the head.
- **Stays**: per-field diffs, the By filter, the pills.

## Narrow (390)

- The list page spends ~400px on chrome before the first row: two
  full-width head buttons, three wrapped lines of filters, a two-line sort
  strip.
- **Simplify**: head actions as small controls on the title line; search
  full width; filters in one scrolling row; no strip; the foot wraps.

## Cross-cutting

- **Leads that say nothing** on every list page.
- **Three ways to reset**: Reset on the find row, Clear all on the pill row,
  Clear all inside More filters.
- **Two badge semantics in one column**: `on`/`off` pills carry a success
  colour regardless of what *true* means.
- **The shell is right.** Rail, two-row header, ⌘K, account menu, footer:
  nothing to change.
