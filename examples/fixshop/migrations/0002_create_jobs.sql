-- One job = one broken item on the bench.
--
-- `status` is the shop's ladder. The CHECK constraint pins the same
-- set that `#[rustio(choices = ...)]` renders in the form — the form
-- offers the valid values, Postgres refuses everything else.
--
-- Two boolean columns exist so the *list page* and the synchronous
-- `ModelAdmin::validate` hook can both see facts that would otherwise
-- need a second query:
--
--   `quote_approved` mirrors "this job has an approved quote". It is
--   maintained by a trigger on `quotes` (migration 0003), so it can
--   never drift from the quotes table.
--
--   `overdue` mirrors "promised date has passed and the item is still
--   here". Time moves on its own, so no write to this table can keep
--   it current — `src/main.rs` refreshes it on boot and once a minute
--   after that.

CREATE TABLE jobs (
    id             BIGSERIAL   PRIMARY KEY,
    customer_id    BIGINT      NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
    ticket_no      TEXT        NOT NULL UNIQUE,
    item           TEXT        NOT NULL,
    problem        TEXT        NOT NULL DEFAULT '',
    status         TEXT        NOT NULL DEFAULT 'booked_in'
                   CHECK (status IN ('booked_in', 'diagnosed', 'quoted', 'approved',
                                     'in_progress', 'ready', 'collected', 'declined')),
    promised_date  DATE        NOT NULL,
    quote_approved BOOLEAN     NOT NULL DEFAULT FALSE,
    overdue        BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX jobs_customer_id_idx   ON jobs (customer_id);
CREATE INDEX jobs_status_idx        ON jobs (status);
CREATE INDEX jobs_promised_date_idx ON jobs (promised_date);
