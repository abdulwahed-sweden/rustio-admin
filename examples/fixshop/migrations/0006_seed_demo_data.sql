-- A week in the life of the shop, so the admin opens onto something
-- readable instead of four empty lists.
--
-- Every date is relative to `CURRENT_DATE`, so the seed stays
-- meaningful no matter when you run it. Two jobs (FS-1001, FS-1003)
-- are past their promised date and still on the bench — those are the
-- ones the Jobs list pins to the top.
--
-- Staff accounts are NOT seeded here. Passwords are Argon2id-hashed
-- by the framework and there is no way to write one from SQL; create
-- Anna, Mike and Sara with `rustio-admin user create` (see README).

INSERT INTO customers (name, phone, created_at) VALUES
    ('Tom Baker',   '555 0114', NOW() - INTERVAL '21 days'),
    ('Ruth Palmer', '555 0127', NOW() - INTERVAL '18 days'),
    ('Dan Foster',  '555 0143', NOW() - INTERVAL '12 days'),
    ('Ellie Marsh', '555 0169', NOW() - INTERVAL '4 days'),
    ('Joe Nowak',   '555 0182', NOW() - INTERVAL '9 days');

INSERT INTO jobs (customer_id, ticket_no, item, problem, status, promised_date, created_at)
SELECT c.id, v.ticket_no, v.item, v.problem, v.status,
       CURRENT_DATE + v.promised_offset, NOW() - (v.age_days || ' days')::INTERVAL
  FROM (VALUES
        ('Tom Baker',   'FS-1001', 'iPhone 12',             'Screen cracked, touch still works', 'in_progress', -3,  9),
        ('Ruth Palmer', 'FS-1002', 'Dell XPS 13 laptop',    'Will not charge',                   'quoted',       2,  5),
        ('Dan Foster',  'FS-1003', 'Trek hybrid bike',      'Rear brake dragging',               'ready',       -1,  7),
        ('Ellie Marsh', 'FS-1004', 'Bosch washing machine', 'Drum will not spin',                'booked_in',    5,  1),
        ('Tom Baker',   'FS-1005', 'Kindle Paperwhite',     'Dead, will not turn on',            'diagnosed',    3,  2),
        ('Joe Nowak',   'FS-1006', 'Samsung 43" TV',        'Sound but no picture',              'declined',     1,  6),
        ('Ruth Palmer', 'FS-1007', 'Canon inkjet printer',  'Paper jam, rollers worn',           'collected',   -6, 14)
       ) AS v(customer, ticket_no, item, problem, status, promised_offset, age_days)
  JOIN customers c ON c.name = v.customer;

-- Quotes. The AFTER trigger from migration 0003 mirrors every
-- `approved` flag onto `jobs.quote_approved` as these land, so the
-- form-level rule in `Job::validate` is correct the moment the seed
-- finishes.
INSERT INTO quotes (job_id, amount, approved, created_at)
SELECT j.id, v.amount, v.approved, NOW() - (v.age_days || ' days')::INTERVAL
  FROM (VALUES
        ('FS-1001', 145.00, TRUE,   8),
        ('FS-1002', 210.00, FALSE,  3),
        ('FS-1003',  65.00, TRUE,   6),
        ('FS-1006', 320.00, FALSE,  5),
        ('FS-1007',  40.00, TRUE,  13)
       ) AS v(ticket_no, amount, approved, age_days)
  JOIN jobs j ON j.ticket_no = v.ticket_no;

-- The shop's own trail, back-dated to match. From here on every row
-- in this table is written by `src/workflow.rs` when someone presses
-- a button.
INSERT INTO job_events (job_id, from_status, to_status, actor, created_at)
SELECT j.id, v.from_status, v.to_status, v.actor, NOW() - (v.age_days || ' days')::INTERVAL
  FROM (VALUES
        ('FS-1001', 'booked_in',   'diagnosed',   'mike@fixshop.test', 9),
        ('FS-1001', 'diagnosed',   'quoted',      'mike@fixshop.test', 8),
        ('FS-1001', 'quoted',      'approved',    'anna@fixshop.test', 8),
        ('FS-1001', 'approved',    'in_progress', 'mike@fixshop.test', 7),
        ('FS-1002', 'booked_in',   'diagnosed',   'mike@fixshop.test', 4),
        ('FS-1002', 'diagnosed',   'quoted',      'mike@fixshop.test', 3),
        ('FS-1003', 'booked_in',   'diagnosed',   'mike@fixshop.test', 7),
        ('FS-1003', 'diagnosed',   'quoted',      'mike@fixshop.test', 6),
        ('FS-1003', 'quoted',      'approved',    'anna@fixshop.test', 6),
        ('FS-1003', 'approved',    'in_progress', 'mike@fixshop.test', 5),
        ('FS-1003', 'in_progress', 'ready',       'mike@fixshop.test', 2),
        ('FS-1005', 'booked_in',   'diagnosed',   'mike@fixshop.test', 1),
        ('FS-1006', 'booked_in',   'diagnosed',   'mike@fixshop.test', 6),
        ('FS-1006', 'diagnosed',   'quoted',      'mike@fixshop.test', 5),
        ('FS-1006', 'quoted',      'declined',    'anna@fixshop.test', 4),
        ('FS-1007', 'booked_in',   'diagnosed',   'mike@fixshop.test', 13),
        ('FS-1007', 'diagnosed',   'quoted',      'mike@fixshop.test', 13),
        ('FS-1007', 'quoted',      'approved',    'anna@fixshop.test', 12),
        ('FS-1007', 'approved',    'in_progress', 'mike@fixshop.test', 11),
        ('FS-1007', 'in_progress', 'ready',       'mike@fixshop.test',  8),
        ('FS-1007', 'ready',       'collected',   'anna@fixshop.test',  6)
       ) AS v(ticket_no, from_status, to_status, actor, age_days)
  JOIN jobs j ON j.ticket_no = v.ticket_no;

-- Same statement `refresh_overdue_flags` runs at boot and once a
-- minute after that (see `src/main.rs`). Running it here means
-- `rustio-admin migrate apply` alone leaves a correct database.
UPDATE jobs
   SET overdue = (promised_date < CURRENT_DATE
                  AND status NOT IN ('collected', 'declined'));
