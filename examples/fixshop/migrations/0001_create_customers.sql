-- A customer is whoever walked in with the broken thing. Name and
-- phone is all the shop keeps.

CREATE TABLE customers (
    id         BIGSERIAL   PRIMARY KEY,
    name       TEXT        NOT NULL,
    phone      TEXT        NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX customers_name_idx ON customers (name);
