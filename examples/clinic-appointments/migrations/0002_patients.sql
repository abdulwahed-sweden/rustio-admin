CREATE TABLE patients (
    id            BIGSERIAL   PRIMARY KEY,
    chart_number  TEXT        NOT NULL UNIQUE,
    full_name     TEXT        NOT NULL,
    email         TEXT        NOT NULL UNIQUE,
    is_active     BOOLEAN     NOT NULL DEFAULT TRUE,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
