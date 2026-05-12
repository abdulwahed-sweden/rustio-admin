CREATE TABLE patrons (
    id          BIGSERIAL   PRIMARY KEY,
    card_number TEXT        NOT NULL UNIQUE,
    full_name   TEXT        NOT NULL,
    email       TEXT        NOT NULL UNIQUE,
    is_active   BOOLEAN     NOT NULL DEFAULT TRUE,
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
