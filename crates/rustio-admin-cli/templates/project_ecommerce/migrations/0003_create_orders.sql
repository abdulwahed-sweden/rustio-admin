CREATE TABLE orders (
    id           BIGSERIAL    PRIMARY KEY,
    customer_id  BIGINT       NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
    total        NUMERIC      NOT NULL DEFAULT 0,
    status       TEXT         NOT NULL DEFAULT 'pending'
                 CHECK (status IN ('pending', 'paid', 'shipped', 'delivered', 'cancelled')),
    placed_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX orders_customer_id_idx ON orders (customer_id);
CREATE INDEX orders_placed_at_idx   ON orders (placed_at DESC);

-- Example orders linked to the seeded customers by name, so the
-- foreign keys stay correct regardless of the generated ids.
INSERT INTO orders (customer_id, total, status, placed_at) VALUES
    ((SELECT id FROM customers WHERE full_name = 'Sarah Ahmed'), 128.90, 'paid',    NOW() - INTERVAL '2 days'),
    ((SELECT id FROM customers WHERE full_name = 'John Okoro'),   89.00, 'shipped', NOW() - INTERVAL '5 days'),
    ((SELECT id FROM customers WHERE full_name = 'Maria Lopez'),  14.50, 'pending', NOW() - INTERVAL '1 day');
