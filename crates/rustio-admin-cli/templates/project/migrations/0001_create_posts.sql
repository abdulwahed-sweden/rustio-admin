CREATE TABLE posts (
    id          BIGSERIAL    PRIMARY KEY,
    title       TEXT         NOT NULL,
    body        TEXT         NOT NULL DEFAULT '',
    published   BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX posts_created_at_idx ON posts (created_at DESC);

-- Demo rows so a freshly-scaffolded admin isn't a blank slate.
-- The dashboard's "Recent activity" sparkline and the Posts list
-- both need something to render against; an empty database makes
-- the layout feel broken (one tall bar on the chart, "0 rows" on
-- every tile). Spread across the last week so the sparkline shows
-- multiple bars rather than collapsing onto today.
INSERT INTO posts (title, body, published, created_at) VALUES
    ('Welcome to your admin', 'This is a demo post — delete it whenever you like.', TRUE,  NOW() - INTERVAL '6 days'),
    ('Drafting the launch announcement', 'Notes for the team.', FALSE, NOW() - INTERVAL '4 days'),
    ('Field trial results — week 1', 'Numbers look good.', TRUE,  NOW() - INTERVAL '3 days'),
    ('Customer feedback round-up', 'Five interview summaries.', TRUE,  NOW() - INTERVAL '1 days'),
    ('Hello world',           'Your first published post.',    TRUE,  NOW());
