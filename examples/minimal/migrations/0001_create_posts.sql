CREATE TABLE posts (
    id          BIGSERIAL    PRIMARY KEY,
    title       TEXT         NOT NULL,
    body        TEXT         NOT NULL DEFAULT '',
    published   BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX posts_created_at_idx ON posts (created_at DESC);
