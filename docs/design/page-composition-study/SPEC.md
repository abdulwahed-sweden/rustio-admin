# Spec — the redesigned compositions

Each composition and its class:

| Class | Meaning |
|---|---|
| **CSS ONLY** | a rule in `composition.css` on markup that already exists |
| **TEMPLATE + CSS** | a template rearranges or conditions markup the context already supplies; no Rust change |
| **REQUIRES WIRING** | needs a context field or a saved ViewSpec that exists in the runtime but is not on the page today |
| **UNCHANGED** | kept exactly as shipped |

The theme is frozen: every rule uses existing `--rio-*` tokens only.

## The list page (Jobs)

**Head.** Title · actions beside it (as shipped). The generic lead is
dropped on list pages; the count lives on the find row. TEMPLATE + CSS.

**Find row.** `search · filter triggers · (Reset) · count`. Up to **four**
filters inline; "More filters" only beyond four. Reset appears only for a
search with no active filter; when filters are active, the pill line's
*Clear all* is the one reset. TEMPLATE + CSS.

**Data surface — two strip rules.**

| Strip | Exists when | Holds |
|---|---|---|
| Selection strip | ≥ 1 row selected | count · Clear · non-destructive bulk actions · divider · destructive bulk actions · Delete selected |
| Head strip | a saved ViewSpec offers view modes, **or** the layout has no column heads (List / Cards / Compact) | view-mode switch · passive sort statement · Sort dropdown (adaptive layouts only) |
| Table, no selection | — | none: the table starts at the top of the surface |

The selection strip takes the head slot; the two never stack. Grouping uses
`btn.destructive`, which the context already carries. TEMPLATE + CSS.

**Foot.** `Showing 1–25 of 61 · 25 per page ▾ · pagination`. Rows-per-page
moves here in every mode. TEMPLATE + CSS.

**Cells.**
- Identity column hugs, mono, semibold (the ticket). CSS ONLY on `.rio-cell-fit`.
- Choice fields: the value humanised (`in_progress` → *in progress*) and
  worn as a neutral badge. TEMPLATE + CSS (`replace("_", " ")` on
  `kind == "select"` cells).
- Booleans: `yes` as a semibold mark, false as a faint `—`. A generic list
  cannot know whether *true* is good, so it stops claiming it is. TEMPLATE +
  CSS. Colour by meaning (an overdue job's status in the warning face) is
  the view layer's `badge` semantic through a saved ViewSpec — REQUIRES
  WIRING on the generic list; shown on the View modes board where it exists.
- Dates tabular, actions quiet: UNCHANGED.

**Checkbox column** 40px. CSS ONLY.

**Empty states.** Inside the surface; the find row stays; filtered says
"No jobs match … Clear all"; fresh says "No jobs yet" with a secondary Add.
TEMPLATE + CSS.

**Narrow (≤ 760).** Title and both actions on one line as small controls;
search full width; filters in one horizontally scrolling row; count on its
own line; foot wraps. CSS ONLY.

## View modes

Head strip present (the layouts have no column heads): view switch, the
sort statement, the Sort dropdown pushed right. Body: `adaptive-views.css`
as shipped. Foot as above. TEMPLATE + CSS; the layouts UNCHANGED.

## Form (Edit job)

**Measure.** Standard (1120) when the page has an aside — read-only fields
or inlines; otherwise the centred form measure as today. TEMPLATE
(`page_measure` block conditional on `has_readonly or inlines`).

**Layout.** `.rio-form-layout`: card (728) left, aside right; single column
≤ 1040. CSS ONLY.

**One card, one foot.** Sections as bands inside the card (mono band
heading when a section has a title; none when it does not); the action bar
is the card's own foot on `--rio-sunken`. The three save variants and
History · Delete · Cancel stay in their order. TEMPLATE + CSS.

**Read-only facts.** Fields the context marks `disabled` are rendered as a
definition list (status as a badge) in the aside, not as greyed inputs.
The framework re-injects stored values on save, so no input is needed.
TEMPLATE + CSS.

**Related sections.** Each inline is a surface in the aside: head
(title · count · Add), compact rows with quiet text actions (replacing the
icon buttons), *View all* in the foot when truncated. TEMPLATE + CSS.

**Inputs sized by content.** `date`, `datetime-local`, `number` cap at
240px inside their cell. CSS ONLY.

## Dashboard

Facts line under the title: models · rows · recent events (link) · version ·
environment — the same numbers as the four tiles. The models as the one
surface with a title band (`Models` · app label; the label becomes a column
only with more than one app). Browse / Add quiet on 48px rows. Docs as a
quiet control; Audit log secondary. TEMPLATE + CSS.

## Users

The special-cased grid table becomes the generic operational table:
identity hugs with the id as a mono suffix, role chip, status pill, Created
with the zone, quiet actions, 48px rows; the lead is dropped. TEMPLATE + CSS
(the `.rio-dtable--users` grid rules retire).

## Groups / Permissions

Standard measure; one card; bands *Group* and *Permissions*; the matrix flat
inside the card (head on `--rio-raised`, 40px rows); the extras `<details>`
open by default and rendered as a titled checklist grid; action bar as the
foot. TEMPLATE + CSS.

## Sessions

One list surface, one row per session: device (identity) with *you, now*
on the current one; ip · signed in (with zone) · last active · expires as a
meta line; trust badge; Revoke quiet-danger or *This session*. The bulk
actions beneath, unchanged. TEMPLATE + CSS.

## API surface

One flat section per model on the standard measure: head (name · base
path · field count), body in two columns (endpoints | fields). No glyph.
The three head actions stay; the playground is the primary. TEMPLATE + CSS.

## History

No avatars; the By link is the email; when and ip as metadata; date
dividers sticky under the head; the diff rows as shipped. CSS ONLY (one
span removed in the template).

## What this study does not do

- No new colour, size, height, radius, shadow or spacing value.
- No new data, filter, action, route, permission or navigation.
- No change to the shell.
- No change to auth pages, health, DB browser, flags, docs, view designer,
  branding, MFA, notifications — out of scope here; they would take the
  same list/form grammar.
