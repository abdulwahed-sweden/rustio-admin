CREATE TABLE comments (
    id           BIGSERIAL    PRIMARY KEY,
    post_id      BIGINT       NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    author_name  TEXT         NOT NULL,
    body         TEXT         NOT NULL DEFAULT '',
    approved     BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX comments_post_id_idx     ON comments (post_id);
CREATE INDEX comments_created_at_idx  ON comments (created_at DESC);
