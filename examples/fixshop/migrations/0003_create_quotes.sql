-- The price the shop sent the customer, and whether the customer said
-- yes. `quotes.approved` is the single source of truth for approval.

CREATE TABLE quotes (
    id         BIGSERIAL      PRIMARY KEY,
    job_id     BIGINT         NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    amount     NUMERIC(10, 2) NOT NULL,
    approved   BOOLEAN        NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ    NOT NULL DEFAULT NOW()
);

CREATE INDEX quotes_job_id_idx ON quotes (job_id);

-- `jobs.quote_approved` is a mirror of "this job has an approved
-- quote", kept honest by this trigger. It fires on every write to
-- `quotes`, so the owner editing a quote by hand in the admin and the
-- front desk clicking "Customer approved" both land in the same place.
--
-- This is why `Job::validate` — which is synchronous and cannot query
-- the database — can still enforce the shop's one hard rule on the
-- ordinary edit form.
-- The function body is written as a single-quoted string rather than
-- the more usual `$$ … $$`. The framework's migration runner splits a
-- file into statements on `;`, and its dollar-quote handling mangles
-- the opening tag; a quoted body takes the string branch, which is
-- correct. The body contains no apostrophes, so nothing needs
-- doubling up.
CREATE FUNCTION fixshop_sync_quote_approved() RETURNS TRIGGER AS '
DECLARE
    target BIGINT := COALESCE(NEW.job_id, OLD.job_id);
BEGIN
    UPDATE jobs
       SET quote_approved = EXISTS (
               SELECT 1 FROM quotes WHERE job_id = target AND approved
           )
     WHERE id = target;
    RETURN NULL;
END;
' LANGUAGE plpgsql;

CREATE TRIGGER quotes_sync_job_approval
    AFTER INSERT OR UPDATE OR DELETE ON quotes
    FOR EACH ROW EXECUTE FUNCTION fixshop_sync_quote_approved();
