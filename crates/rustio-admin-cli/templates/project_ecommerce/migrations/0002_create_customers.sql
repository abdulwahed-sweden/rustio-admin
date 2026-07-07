CREATE TABLE customers (
    id          BIGSERIAL    PRIMARY KEY,
    full_name   TEXT         NOT NULL,
    email       TEXT         NOT NULL DEFAULT '',
    phone       TEXT         NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX customers_full_name_idx ON customers (full_name);

-- Example customers, referenced by the seeded orders below by name so
-- the foreign keys stay correct regardless of the generated ids.
INSERT INTO customers (full_name, email, phone) VALUES
    ('Sarah Ahmed', 'sarah.ahmed@example.com', '+1 555 0102'),
    ('John Okoro',  'john.okoro@example.com',  '+1 555 0148'),
    ('Maria Lopez', 'maria.lopez@example.com', '+1 555 0190');
