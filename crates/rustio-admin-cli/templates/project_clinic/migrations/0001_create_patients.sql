CREATE TABLE patients (
    id             BIGSERIAL    PRIMARY KEY,
    full_name      TEXT         NOT NULL,
    date_of_birth  DATE         NOT NULL,
    phone          TEXT         NOT NULL DEFAULT '',
    email          TEXT         NOT NULL DEFAULT '',
    created_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX patients_full_name_idx  ON patients (full_name);
CREATE INDEX patients_created_at_idx ON patients (created_at DESC);

-- Example patients so a freshly-scaffolded admin opens onto real
-- rows instead of an empty list. Delete them whenever you like.
INSERT INTO patients (full_name, date_of_birth, phone, email) VALUES
    ('Sarah Ahmed', DATE '1989-03-14', '+1 555 0102', 'sarah.ahmed@example.com'),
    ('John Okoro',  DATE '1975-11-02', '+1 555 0148', 'john.okoro@example.com'),
    ('Maria Lopez', DATE '1996-07-21', '+1 555 0190', 'maria.lopez@example.com');
