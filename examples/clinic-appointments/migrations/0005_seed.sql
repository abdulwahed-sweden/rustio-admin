-- 0005_seed.sql — deterministic demo dataset.
--
-- 5 clinics + 20 patients + 30 practitioners + 30 appointments.
-- Public-domain literary names; @example.test emails; timestamps
-- are relative to the boot day via NOW() - INTERVAL arithmetic, so
-- a freshly-seeded DB always has "recent" data.
--
-- Intended for local development and demo only. Tracked by
-- rustio_migrations and runs exactly once per database.


-- ============================================================
-- Clinics (5)
-- ============================================================

INSERT INTO clinics (name, address, is_open, created_at) VALUES
    ('Downtown Family Health', '120 Main Street',    TRUE, NOW() - INTERVAL '1800 days'),
    ('Northgate Medical',      '47 Birch Avenue',    TRUE, NOW() - INTERVAL '1440 days'),
    ('Eastside Pediatrics',    '8 Pine Crescent',    TRUE, NOW() - INTERVAL '1080 days'),
    ('West Heart & Vascular',  '212 Oak Boulevard',  TRUE, NOW() - INTERVAL '720 days'),
    ('Riverside Urgent Care',  '301 River Road',     TRUE, NOW() - INTERVAL '360 days')
ON CONFLICT (name) DO NOTHING;


-- ============================================================
-- Patients (20)
-- ============================================================
-- Public-domain literary characters. Two patients (CLN000019,
-- CLN000020) flagged is_active = FALSE for variety; the rest are
-- active and able to book appointments.

INSERT INTO patients (chart_number, full_name, email, is_active, registered_at) VALUES
    ('CLN000001', 'Elizabeth Bennet',  'elizabeth.bennet@example.test',  TRUE, NOW() - INTERVAL '1200 days'),
    ('CLN000002', 'Fitzwilliam Darcy', 'fitzwilliam.darcy@example.test', TRUE, NOW() - INTERVAL '1180 days'),
    ('CLN000003', 'Jane Eyre',         'jane.eyre@example.test',         TRUE, NOW() - INTERVAL '1100 days'),
    ('CLN000004', 'Edward Rochester',  'edward.rochester@example.test',  TRUE, NOW() - INTERVAL '1060 days'),
    ('CLN000005', 'Catherine Earnshaw','catherine.earnshaw@example.test',TRUE, NOW() - INTERVAL '1000 days'),
    ('CLN000006', 'Heathcliff Linton', 'heathcliff.linton@example.test', TRUE, NOW() - INTERVAL '980 days'),
    ('CLN000007', 'Dorothea Brooke',   'dorothea.brooke@example.test',   TRUE, NOW() - INTERVAL '920 days'),
    ('CLN000008', 'Tertius Lydgate',   'tertius.lydgate@example.test',   TRUE, NOW() - INTERVAL '880 days'),
    ('CLN000009', 'Anna Karenina',     'anna.karenina@example.test',     TRUE, NOW() - INTERVAL '820 days'),
    ('CLN000010', 'Konstantin Levin',  'konstantin.levin@example.test',  TRUE, NOW() - INTERVAL '780 days'),
    ('CLN000011', 'Hester Prynne',     'hester.prynne@example.test',     TRUE, NOW() - INTERVAL '720 days'),
    ('CLN000012', 'Arthur Dimmesdale', 'arthur.dimmesdale@example.test', TRUE, NOW() - INTERVAL '660 days'),
    ('CLN000013', 'Pip Pirrip',        'pip.pirrip@example.test',        TRUE, NOW() - INTERVAL '600 days'),
    ('CLN000014', 'Estella Havisham',  'estella.havisham@example.test',  TRUE, NOW() - INTERVAL '540 days'),
    ('CLN000015', 'David Copperfield', 'david.copperfield@example.test', TRUE, NOW() - INTERVAL '480 days'),
    ('CLN000016', 'Agnes Wickfield',   'agnes.wickfield@example.test',   TRUE, NOW() - INTERVAL '420 days'),
    ('CLN000017', 'Marianne Dashwood', 'marianne.dashwood@example.test', TRUE, NOW() - INTERVAL '360 days'),
    ('CLN000018', 'Elinor Dashwood',   'elinor.dashwood@example.test',   TRUE, NOW() - INTERVAL '300 days'),
    ('CLN000019', 'Bertha Mason',      'bertha.mason@example.test',      FALSE, NOW() - INTERVAL '240 days'),
    ('CLN000020', 'Miss Havisham',     'miss.havisham@example.test',     FALSE, NOW() - INTERVAL '180 days')
ON CONFLICT (chart_number) DO NOTHING;


-- ============================================================
-- Practitioners (30)
-- ============================================================
-- Spread across the 5 clinics — 5 to 7 per clinic. Mix of
-- specialties matching the CHECK constraint in 0003_practitioners.sql.
-- Three practitioners (Dr. Faust, Dr. Tulp, Dr. Bovary) are
-- non-active for status variety.

-- Downtown Family Health (clinic_id = 1) — family medicine focus
INSERT INTO practitioners (clinic_id, full_name, specialty, license_no, status, created_at) VALUES
    (1, 'Dr. Atticus Finch',   'family_medicine', 'LIC-10001', 'active',   NOW() - INTERVAL '1700 days'),
    (1, 'Dr. Sherwin Nuland',  'family_medicine', 'LIC-10002', 'active',   NOW() - INTERVAL '1600 days'),
    (1, 'Dr. Oliver Sacks',    'family_medicine', 'LIC-10003', 'active',   NOW() - INTERVAL '1500 days'),
    (1, 'Dr. Henry Marsh',     'family_medicine', 'LIC-10004', 'active',   NOW() - INTERVAL '1400 days'),
    (1, 'Dr. Mary Seacole',    'family_medicine', 'LIC-10005', 'active',   NOW() - INTERVAL '1300 days'),
    (1, 'Dr. Heinrich Faust',  'family_medicine', NULL,        'retired',  NOW() - INTERVAL '1200 days'),

-- Northgate Medical (clinic_id = 2) — mixed primary + cardiology
    (2, 'Dr. William Osler',   'family_medicine', 'LIC-20001', 'active',   NOW() - INTERVAL '1100 days'),
    (2, 'Dr. Helen Taussig',   'cardiology',      'LIC-20002', 'active',   NOW() - INTERVAL '1050 days'),
    (2, 'Dr. Maude Abbott',    'cardiology',      'LIC-20003', 'active',   NOW() - INTERVAL '1000 days'),
    (2, 'Dr. Charles Mayo',    'family_medicine', 'LIC-20004', 'active',   NOW() - INTERVAL '950 days'),
    (2, 'Dr. Virginia Apgar',  'pediatrics',      'LIC-20005', 'active',   NOW() - INTERVAL '900 days'),
    (2, 'Dr. Nicolaes Tulp',   'family_medicine', 'LIC-20006', 'on_leave', NOW() - INTERVAL '850 days'),

-- Eastside Pediatrics (clinic_id = 3) — all peds
    (3, 'Dr. Benjamin Spock',  'pediatrics',      'LIC-30001', 'active',   NOW() - INTERVAL '800 days'),
    (3, 'Dr. T. Berry Brazelton','pediatrics',    'LIC-30002', 'active',   NOW() - INTERVAL '760 days'),
    (3, 'Dr. Mary Putnam',     'pediatrics',      'LIC-30003', 'active',   NOW() - INTERVAL '720 days'),
    (3, 'Dr. Helen Mahoney',   'pediatrics',      'LIC-30004', 'active',   NOW() - INTERVAL '680 days'),
    (3, 'Dr. Sara Josephine',  'pediatrics',      'LIC-30005', 'active',   NOW() - INTERVAL '640 days'),

-- West Heart & Vascular (clinic_id = 4) — cardiology
    (4, 'Dr. Christiaan Barnard', 'cardiology',   'LIC-40001', 'active',   NOW() - INTERVAL '600 days'),
    (4, 'Dr. Denton Cooley',      'cardiology',   'LIC-40002', 'active',   NOW() - INTERVAL '560 days'),
    (4, 'Dr. Michael DeBakey',    'cardiology',   'LIC-40003', 'active',   NOW() - INTERVAL '520 days'),
    (4, 'Dr. Helen Brooke',       'cardiology',   'LIC-40004', 'active',   NOW() - INTERVAL '480 days'),
    (4, 'Dr. Norman Shumway',     'cardiology',   'LIC-40005', 'active',   NOW() - INTERVAL '440 days'),
    (4, 'Dr. Emma Bovary',        'cardiology',   NULL,        'pending',  NOW() - INTERVAL '400 days'),

-- Riverside Urgent Care (clinic_id = 5) — broad mix
    (5, 'Dr. Joseph Lister',    'orthopedics',    'LIC-50001', 'active',   NOW() - INTERVAL '360 days'),
    (5, 'Dr. Marie Curie',      'dermatology',    'LIC-50002', 'active',   NOW() - INTERVAL '320 days'),
    (5, 'Dr. Carl Jung',        'psychiatry',     'LIC-50003', 'active',   NOW() - INTERVAL '280 days'),
    (5, 'Dr. Anna Freud',       'psychiatry',     'LIC-50004', 'active',   NOW() - INTERVAL '240 days'),
    (5, 'Dr. Karl Landsteiner', 'family_medicine','LIC-50005', 'active',   NOW() - INTERVAL '200 days'),
    (5, 'Dr. Florence Sabin',   'family_medicine','LIC-50006', 'active',   NOW() - INTERVAL '160 days'),
    (5, 'Dr. Rebecca Lancefield','dermatology',   'LIC-50007', 'active',   NOW() - INTERVAL '120 days');


-- ============================================================
-- Appointments (30)
-- ============================================================
-- Mix of statuses: ~17 scheduled (upcoming + recent), 8 completed,
-- 3 cancelled, 2 no_show. Dates spread from 30 days ago to 30 days
-- ahead so the list page always shows a useful timeline regardless
-- of when the seed runs. Bulk-action tests benefit from having
-- several 'scheduled' rows whose `scheduled_at` is already in the
-- past — `mark_no_show` is the obvious next move on those.

INSERT INTO appointments (patient_id, practitioner_id, status, scheduled_at, checked_in_at, ended_at) VALUES
    -- Past, completed — already wrapped up
    (1,  1,  'completed', NOW() - INTERVAL '28 days',  NOW() - INTERVAL '28 days' + INTERVAL '5 minutes',  NOW() - INTERVAL '28 days' + INTERVAL '35 minutes'),
    (3,  2,  'completed', NOW() - INTERVAL '25 days',  NOW() - INTERVAL '25 days' + INTERVAL '10 minutes', NOW() - INTERVAL '25 days' + INTERVAL '50 minutes'),
    (5,  7,  'completed', NOW() - INTERVAL '22 days',  NOW() - INTERVAL '22 days',                         NOW() - INTERVAL '22 days' + INTERVAL '45 minutes'),
    (7,  13, 'completed', NOW() - INTERVAL '20 days',  NOW() - INTERVAL '20 days' + INTERVAL '8 minutes',  NOW() - INTERVAL '20 days' + INTERVAL '40 minutes'),
    (9,  19, 'completed', NOW() - INTERVAL '18 days',  NOW() - INTERVAL '18 days' + INTERVAL '4 minutes',  NOW() - INTERVAL '18 days' + INTERVAL '55 minutes'),
    (11, 24, 'completed', NOW() - INTERVAL '15 days',  NOW() - INTERVAL '15 days' + INTERVAL '12 minutes', NOW() - INTERVAL '15 days' + INTERVAL '45 minutes'),
    (13, 3,  'completed', NOW() - INTERVAL '12 days',  NOW() - INTERVAL '12 days',                         NOW() - INTERVAL '12 days' + INTERVAL '30 minutes'),
    (15, 14, 'completed', NOW() - INTERVAL '10 days',  NOW() - INTERVAL '10 days' + INTERVAL '6 minutes',  NOW() - INTERVAL '10 days' + INTERVAL '35 minutes'),

    -- Past, never wrapped up — perfect bulk `mark_no_show` candidates
    (2,  8,  'scheduled', NOW() - INTERVAL '7 days',  NULL, NULL),
    (4,  20, 'scheduled', NOW() - INTERVAL '5 days',  NULL, NULL),
    (6,  15, 'scheduled', NOW() - INTERVAL '3 days',  NULL, NULL),

    -- Cancelled — operator cancelled ahead of time
    (8,  9,  'cancelled', NOW() - INTERVAL '4 days',   NULL, NULL),
    (10, 25, 'cancelled', NOW() - INTERVAL '2 days',   NULL, NULL),
    (12, 1,  'cancelled', NOW() + INTERVAL '6 days',   NULL, NULL),

    -- No-show — patient didn't arrive
    (14, 21, 'no_show',   NOW() - INTERVAL '9 days',   NULL, NULL),
    (16, 4,  'no_show',   NOW() - INTERVAL '6 days',   NULL, NULL),

    -- Future, scheduled — the main upcoming-appointments view
    (1,  10, 'scheduled', NOW() + INTERVAL '1 day',    NULL, NULL),
    (3,  16, 'scheduled', NOW() + INTERVAL '2 days',   NULL, NULL),
    (5,  22, 'scheduled', NOW() + INTERVAL '3 days',   NULL, NULL),
    (7,  27, 'scheduled', NOW() + INTERVAL '4 days',   NULL, NULL),
    (9,  2,  'scheduled', NOW() + INTERVAL '5 days',   NULL, NULL),
    (11, 11, 'scheduled', NOW() + INTERVAL '7 days',   NULL, NULL),
    (13, 17, 'scheduled', NOW() + INTERVAL '9 days',   NULL, NULL),
    (15, 23, 'scheduled', NOW() + INTERVAL '11 days',  NULL, NULL),
    (17, 28, 'scheduled', NOW() + INTERVAL '12 days',  NULL, NULL),
    (18, 5,  'scheduled', NOW() + INTERVAL '14 days',  NULL, NULL),
    (2,  18, 'scheduled', NOW() + INTERVAL '17 days',  NULL, NULL),
    (4,  26, 'scheduled', NOW() + INTERVAL '20 days',  NULL, NULL),
    (6,  29, 'scheduled', NOW() + INTERVAL '24 days',  NULL, NULL),
    (8,  30, 'scheduled', NOW() + INTERVAL '28 days',  NULL, NULL);
