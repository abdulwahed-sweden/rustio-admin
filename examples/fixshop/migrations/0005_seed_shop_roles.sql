-- FixShop's two shop-floor groups, and what each is allowed to touch.
--
-- The *owner* is not a group. The owner signs in with the framework
-- role `administrator`, which bypasses group permission checks
-- entirely (`Role::bypasses_group_checks`) — that is the "Owner:
-- everything" line in the brief, and it needs no rows here.
--
-- Front desk and technician both sign in as `staff`, so every gate
-- below is a real lookup against these tables.
--
-- Permission names follow the framework's convention,
-- `<admin_name>.<action>_<singular>`. The four CRUD actions
-- (add/change/delete/view) are registered automatically by
-- `Admin::seed_permissions` at boot; inserting them here first is
-- idempotent (the name column is UNIQUE) and lets this migration
-- grant them in the same transaction.
--
-- The five extra verbs — diagnose / quote / approve / repair /
-- collect — are FixShop's own. They gate the status-ladder bulk
-- actions declared in `src/workflow.rs` via `BulkAction.permission`.

INSERT INTO rustio_groups (name, description) VALUES
    ('front_desk', 'Takes items in, records quote approval, hands repaired items back.'),
    ('technician', 'Diagnoses, prices, and repairs.')
ON CONFLICT (name) DO NOTHING;

INSERT INTO rustio_permissions (name, description) VALUES
    ('customers.add_customer',    'Book a new customer in.'),
    ('customers.change_customer', 'Correct a customer''s details.'),
    ('customers.view_customer',   'See customers.'),

    ('jobs.add_job',      'Book an item in.'),
    ('jobs.change_job',   'Edit a job.'),
    ('jobs.view_job',     'See jobs.'),
    ('jobs.diagnose_job', 'Move a job to diagnosed.'),
    ('jobs.quote_job',    'Mark a job as quoted to the customer.'),
    ('jobs.approve_job',  'Record the customer''s answer to a quote.'),
    ('jobs.repair_job',   'Start a repair and mark it ready.'),
    ('jobs.collect_job',  'Hand a repaired item back.'),

    ('quotes.add_quote',    'Write a quote.'),
    ('quotes.change_quote', 'Edit a quote after it was written.'),
    ('quotes.view_quote',   'See quotes.'),

    ('job_events.view_jobevent', 'Read the job history.')
ON CONFLICT (name) DO NOTHING;

-- Front desk: the counter. Creates customers and jobs, records what
-- the customer said about the quote, hands the item back.
-- Deliberately cannot write or edit a quote.
INSERT INTO rustio_group_permissions (group_id, permission_id)
SELECT g.id, p.id
  FROM rustio_groups g, rustio_permissions p
 WHERE g.name = 'front_desk'
   AND p.name IN ('customers.add_customer', 'customers.change_customer', 'customers.view_customer',
                  'jobs.add_job', 'jobs.change_job', 'jobs.view_job',
                  'jobs.approve_job', 'jobs.collect_job',
                  'quotes.view_quote',
                  'job_events.view_jobevent')
ON CONFLICT DO NOTHING;

-- Technician: the bench. Diagnoses, writes the quote, does the
-- repair. Cannot create customers, cannot record the customer's
-- approval, and cannot edit a quote once it is out.
INSERT INTO rustio_group_permissions (group_id, permission_id)
SELECT g.id, p.id
  FROM rustio_groups g, rustio_permissions p
 WHERE g.name = 'technician'
   AND p.name IN ('customers.view_customer',
                  'jobs.change_job', 'jobs.view_job',
                  'jobs.diagnose_job', 'jobs.quote_job', 'jobs.repair_job',
                  'quotes.add_quote', 'quotes.view_quote',
                  'job_events.view_jobevent')
ON CONFLICT DO NOTHING;
