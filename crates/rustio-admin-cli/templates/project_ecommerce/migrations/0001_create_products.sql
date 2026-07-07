CREATE TABLE products (
    id          BIGSERIAL    PRIMARY KEY,
    name        TEXT         NOT NULL,
    sku         TEXT         NOT NULL DEFAULT '',
    price       NUMERIC      NOT NULL DEFAULT 0,
    in_stock    BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX products_name_idx       ON products (name);
CREATE INDEX products_created_at_idx ON products (created_at DESC);

-- Example products so a freshly-scaffolded admin opens onto real rows
-- instead of an empty list. Delete them whenever you like.
INSERT INTO products (name, sku, price, in_stock) VALUES
    ('Aurora Desk Lamp',   'LMP-001', 39.90,  TRUE),
    ('Nomad Backpack 24L', 'BAG-014', 89.00,  TRUE),
    ('Terra Ceramic Mug',  'MUG-007', 14.50,  TRUE),
    ('Halcyon Headphones', 'AUD-233', 129.00, FALSE);
