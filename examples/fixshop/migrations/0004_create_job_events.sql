-- The shop's own trail: one row per status change, who did it, when.
--
-- This sits *alongside* the framework's `rustio_admin_actions` audit
-- log rather than replacing it. The framework records who pressed
-- which button; this table records what it meant to the shop, in the
-- shop's vocabulary, on a page the front desk can read.
--
-- Registered read-only in `src/main.rs` — a log nobody can edit.

CREATE TABLE job_events (
    id          BIGSERIAL   PRIMARY KEY,
    job_id      BIGINT      NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    from_status TEXT        NOT NULL,
    to_status   TEXT        NOT NULL,
    actor       TEXT        NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX job_events_job_id_idx     ON job_events (job_id, created_at DESC);
CREATE INDEX job_events_created_at_idx ON job_events (created_at DESC);
