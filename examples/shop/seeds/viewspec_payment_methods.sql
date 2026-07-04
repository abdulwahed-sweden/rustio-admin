-- ============================================================================
--  Adaptive View Layer — demo ViewSpec seed (shop example)
--
--  Saves ONE ViewSpec, for the `payment_methods` model, so its list page
--  renders the adaptive view layer (a card grid) out of the box — without
--  opening the View designer and clicking Save.
--
--  Why payment_methods (and not customers/orders/products): the shop ships its
--  own per-model list templates for those three (templates/admin/<model>/
--  list.html, loaded via RUSTIO_TEMPLATE_DIR), and those overrides render a
--  plain table with no adaptive branch — so a saved spec can't show there. The
--  payment_methods model has no such override, so it uses the framework's
--  adaptive-aware list.html and the spec renders. (See DEFERRED.md.)
--
--  How it works: rustio-admin stores one ViewSpec per model in the
--  `rustio_admin_view_specs` table (created lazily by the framework on first
--  read/write). On a fresh database that table may not exist yet, so this seed
--  creates it with the framework's exact schema first, then inserts the spec.
--  `spec_json` is a verbatim serialized `ViewSpec` (generated from the real Rust
--  types, so the framework's loader accepts it).
--
--  Roles: display_name -> primary (card title); provider + country -> secondary;
--  currency -> neutral badge; is_active -> green "Yes/No" badge (the adaptive
--  path humanises booleans); id / code / category -> detail-only (hidden from
--  the card). default_mode "cards" shows the grid immediately; the switcher
--  still offers list / compact / table. Only payment_methods is affected.
--
--  Safe to run repeatedly: ON CONFLICT DO NOTHING never overwrites a spec you
--  later author in the designer.
--
--  Apply:   psql "$DATABASE_URL" -f seeds/viewspec_payment_methods.sql
--  Undo:    psql "$DATABASE_URL" -c "DELETE FROM rustio_admin_view_specs WHERE model = 'payment_methods';"
-- ============================================================================

-- Mirror the framework's lazy-created schema (admin/view_specs.rs). IF NOT
-- EXISTS makes this a no-op when the framework already created the table.
CREATE TABLE IF NOT EXISTS rustio_admin_view_specs (
    model      TEXT        PRIMARY KEY,
    spec_json  TEXT        NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO rustio_admin_view_specs (model, spec_json)
VALUES (
    'payment_methods',
    '{"model":"payment_methods","default_mode":"cards","allowed_modes":["cards","list","compact","table"],"fields":[{"field_name":"id","role":"detail_only","priority":0,"sortable":false,"filterable":false,"default_filter":false},{"field_name":"code","role":"detail_only","priority":5,"sortable":false,"filterable":false,"default_filter":false},{"field_name":"display_name","role":"primary","priority":10,"sortable":true,"filterable":false,"default_filter":false},{"field_name":"category","role":"detail_only","priority":15,"sortable":false,"filterable":false,"default_filter":false},{"field_name":"provider","role":"secondary","priority":20,"sortable":false,"filterable":false,"default_filter":false},{"field_name":"country","role":"secondary","priority":30,"sortable":false,"filterable":false,"default_filter":false},{"field_name":"currency","role":"badge","priority":40,"sortable":false,"filterable":false,"default_filter":false},{"field_name":"is_active","role":"badge","priority":50,"sortable":false,"filterable":false,"default_filter":false,"semantic_class":"success"}],"compositions":[],"default_filters":[],"version":1}'
)
ON CONFLICT (model) DO NOTHING;
