CREATE TABLE items (
    id         BIGSERIAL   PRIMARY KEY,
    branch_id  BIGINT      NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    title      TEXT        NOT NULL,
    kind       TEXT        NOT NULL,
    isbn       TEXT,
    status     TEXT        NOT NULL DEFAULT 'available',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (kind   IN ('book', 'audiobook', 'dvd')),
    CHECK (status IN ('available', 'on_loan', 'lost', 'withdrawn'))
);

CREATE INDEX items_branch_id_idx ON items (branch_id);
CREATE INDEX items_status_idx    ON items (status);
