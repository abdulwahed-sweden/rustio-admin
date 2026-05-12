CREATE TABLE loans (
    id          BIGSERIAL   PRIMARY KEY,
    patron_id   BIGINT      NOT NULL REFERENCES patrons(id) ON DELETE RESTRICT,
    item_id     BIGINT      NOT NULL REFERENCES items(id)   ON DELETE RESTRICT,
    status      TEXT        NOT NULL DEFAULT 'active',
    borrowed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    due_at      TIMESTAMPTZ NOT NULL,
    returned_at TIMESTAMPTZ,
    CHECK (status IN ('active', 'returned', 'overdue'))
);

CREATE INDEX loans_patron_id_idx ON loans (patron_id);
CREATE INDEX loans_item_id_idx   ON loans (item_id);
CREATE INDEX loans_status_idx    ON loans (status);
