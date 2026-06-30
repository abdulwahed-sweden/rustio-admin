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

### 6. Live-LLM genesis ("AI suggests a schema") — out of the OSS repo
- **Now:** the deterministic half of genesis ships — `rustio-admin import
  <schema.json>` (Phase 4) loads a schema into the Builder. The *authoring* of
  that JSON by an AI is done by an **external** assistant (governed by
  `.rustio/ai.toml`); RustIO itself runs no AI and the CLI has no HTTP/LLM deps.
- **Why deferred:** a built-in live-LLM call would break the explicit *RustIO
  runs no AI* stance and add a network client + LLM SDK + API-key handling to a
  deliberately network-free, dependency-disciplined codebase.
- **Done looks like:** a *separate* `rustio-forge` / future `rustio-pro` tool
  (not the OSS runtime or CLI) that turns a natural-language brief into a
  `schema.json` and hands it to `rustio-admin import` — keeping the OSS layer
  deterministic and AI-free.

---

_Add new deferred items above this line. Keep each one: what / why deferred /
what done looks like._
