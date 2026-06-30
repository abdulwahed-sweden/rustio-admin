# DEFERRED

Work that is intentionally postponed, not forgotten. Each item says what it is,
why it's deferred, and what "done" would look like — so picking it up later is a
decision, not an archaeology dig.

## Adaptive View Layer (`crates/rustio-admin/src/view_layer/`)

The module is built and tested; these are the parts that exist in the spec/types
but have no runtime consumer yet, plus the gaps to close when the layer grows up.
None of these block trying the tool today (see `examples/shop/seeds/viewspec_customers.sql`).

### 1. A real adaptive `Table` mode (with column headers)
- **Now:** `ViewMode::Table` falls back to the legacy table; the adaptive path
  only renders `List` / `Cards` / `Compact`, which all share one template
  (`_list_adaptive.html` + `_row.html` + `_cell.html`) and differ only by the
  `av-list--<mode>` CSS class.
- **Why deferred:** the legacy table already works and owns search/sort/filter/
  pagination/bulk actions; re-implementing those in the view layer is a big lift
  for little gain right now.
- **Done looks like:** an adaptive table mode that renders `cell.label` as real
  column headers (today `label` is computed by the renderer but no template shows
  it), without losing the legacy table's behaviour.

### 2. Wire the unused `FieldViewSpec` flags into rendering
- **Now:** `sortable`, `filterable`, `default_filter`, `width`, and the
  spec-level `default_filters` are set by inference and editable in the designer,
  but the view layer **owns no search/sort/filter** (by explicit design in
  `mod.rs`), so nothing acts on them at render time.
- **Why deferred:** these belong to the list-controls/query subsystem, not the
  presentation layer; wiring them means deciding how the view layer and the
  existing list query cooperate.
- **Done looks like:** the adaptive list honours `sortable`/`filterable`/`width`
  (column widths in table mode; sort/filter controls driven by the spec) — or a
  decision that these stay owned by the legacy list path and are dropped from the
  spec.

### 3. `ComposeStyle::InlineIcon` should actually render an icon
- **Now:** the three compose styles (`Stacked`, `InlineIcon`, `BadgeInline`)
  only become a CSS class (`av-cell--<style>`); `InlineIcon` renders no actual
  icon element — it currently looks like the others unless CSS does something.
- **Why deferred:** needs an icon source/convention for composed cells (which
  field supplies the glyph, from where).
- **Done looks like:** an `InlineIcon` composition renders a leading icon + text,
  or the variant is removed if it isn't worth the wiring.

### 4. Decide `FieldKind::Json`'s fate
- **Now:** `FieldKind::Json` is defined and handled in `infer_view_spec`
  (→ `DetailOnly`), but `FieldMeta::from_admin_field` never produces it (there is
  no `FieldType::Json`), so the `Json` arm is unreachable from the real adapter.
- **Why deferred:** depends on whether the framework grows a JSON column type.
- **Done looks like:** either `AdminField`/`FieldType` gains a JSON variant that
  maps to `FieldKind::Json`, or the variant is removed as speculative.

### 5. Per-model list overrides don't opt into the adaptive branch
- **Now:** the framework's `list.html` has the adaptive branch
  (`{% if adaptive %}{% include "admin/_list_adaptive.html" %}{% else %}<legacy
  table>{% endif %}`), and `list_model` always loads any saved `ViewSpec` and
  passes `adaptive` to the template. But a project that ships a **per-model**
  override (`templates/admin/<model>/list.html`, via `RUSTIO_TEMPLATE_DIR`)
  shadows `list.html` for that model — and if the override renders a plain table
  with no `{% if adaptive %}` branch, the saved spec is read but never shown.
  In `examples/shop` this affects `customers`, `orders`, and `products` (their
  rustio-design templates are table-only); models without an override (e.g.
  `payment_methods`, used by the demo seed) render the adaptive view fine.
- **Why deferred:** opting those overrides in is a template change to the shop's
  generated rustio-design artifacts (and a docs note for downstream projects),
  not a framework change — out of scope for "make the layer tr-yable."
- **Done looks like:** either the shop's per-model `list.html` overrides include
  the same `{% if adaptive %}…_list_adaptive.html…{% else %}<their table>{% endif %}`
  wrapper, or the docs state plainly that a per-model list override opts the
  model **out** of the adaptive view layer unless it includes that branch.

## Studio

### 6. Live-LLM genesis ("AI suggests a schema") — out of the OSS repo ✅ RESOLVED
- **Resolved:** built as **`rustio-draft`** and now lives in its own repo:
  <https://github.com/abdulwahed-sweden/rustio-draft>. It turns a brief into a
  `schema.json` via Claude (F1), `--apply`-chains to `rustio-admin import`/`plan`
  (F2), refines an existing schema (F3), offers a local studio (F4), and pins its
  `FIELD_TYPES` to the builder's (F5). The OSS runtime + CLI stayed deterministic
  and AI-free throughout: `rustio-draft` *authors* a schema; `rustio-admin`
  *applies* it. Design: `docs/RUSTIO_DRAFT_SCOPE.md`.
- **Cross-repo note:** when the builder's `FIELD_TYPES`
  (`crates/rustio-admin-cli/src/builder/draft.rs`) gains a type, update the
  matching list in the rustio-draft repo (`src/schema.rs`) — the in-tree CI
  guard left with the code.

---

_Add new deferred items above this line. Keep each one: what / why deferred /
what done looks like._
