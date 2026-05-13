-- 0005_seed.sql — deterministic demo dataset.
--
-- 5 branches + 20 patrons + 80 items + 30 loans. Public-domain
-- literary names + classic book titles; @example.test emails.
-- Timestamps are relative to the boot day via NOW() - INTERVAL
-- arithmetic, so a freshly-seeded DB always has "recent" data.
--
-- Intended for local development and demo only. Tracked by
-- rustio_migrations and runs exactly once per database.


-- ============================================================
-- Branches (5)
-- ============================================================

INSERT INTO branches (name, address, is_open, created_at) VALUES
    ('Central',    '120 Main Street',   TRUE, NOW() - INTERVAL '1800 days'),
    ('North',      '47 Birch Avenue',   TRUE, NOW() - INTERVAL '1440 days'),
    ('East',       '8 Pine Crescent',   TRUE, NOW() - INTERVAL '1080 days'),
    ('West',       '212 Oak Boulevard', TRUE, NOW() - INTERVAL '720 days'),
    ('Bookmobile', 'Mobile Unit',       TRUE, NOW() - INTERVAL '360 days')
ON CONFLICT (name) DO NOTHING;


-- ============================================================
-- Patrons (20)
-- ============================================================
-- Public-domain literary characters. Two patrons (LIB000019,
-- LIB000020) flagged is_active = FALSE for variety; the rest are
-- active and able to borrow.

INSERT INTO patrons (card_number, full_name, email, is_active, joined_at) VALUES
    ('LIB000001', 'Elizabeth Bennet',     'elizabeth.bennet@example.test',     TRUE,  NOW() - INTERVAL '1095 days'),
    ('LIB000002', 'Fitzwilliam Darcy',    'fitzwilliam.darcy@example.test',    TRUE,  NOW() - INTERVAL '1080 days'),
    ('LIB000003', 'Elinor Dashwood',      'elinor.dashwood@example.test',      TRUE,  NOW() - INTERVAL '1020 days'),
    ('LIB000004', 'Marianne Dashwood',    'marianne.dashwood@example.test',    TRUE,  NOW() - INTERVAL '960 days'),
    ('LIB000005', 'Jane Eyre',            'jane.eyre@example.test',            TRUE,  NOW() - INTERVAL '900 days'),
    ('LIB000006', 'Edward Rochester',     'edward.rochester@example.test',     TRUE,  NOW() - INTERVAL '840 days'),
    ('LIB000007', 'Jean Valjean',         'jean.valjean@example.test',         TRUE,  NOW() - INTERVAL '780 days'),
    ('LIB000008', 'Cosette Fauchelevent', 'cosette.fauchelevent@example.test', TRUE,  NOW() - INTERVAL '720 days'),
    ('LIB000009', 'Anna Karenina',        'anna.karenina@example.test',        TRUE,  NOW() - INTERVAL '660 days'),
    ('LIB000010', 'Konstantin Levin',     'konstantin.levin@example.test',     TRUE,  NOW() - INTERVAL '600 days'),
    ('LIB000011', 'Sherlock Holmes',      'sherlock.holmes@example.test',      TRUE,  NOW() - INTERVAL '540 days'),
    ('LIB000012', 'John Watson',          'john.watson@example.test',          TRUE,  NOW() - INTERVAL '510 days'),
    ('LIB000013', 'Alice Liddell',        'alice.liddell@example.test',        TRUE,  NOW() - INTERVAL '450 days'),
    ('LIB000014', 'Edmond Dantes',        'edmond.dantes@example.test',        TRUE,  NOW() - INTERVAL '390 days'),
    ('LIB000015', 'Hester Prynne',        'hester.prynne@example.test',        TRUE,  NOW() - INTERVAL '330 days'),
    ('LIB000016', 'Sydney Carton',        'sydney.carton@example.test',        TRUE,  NOW() - INTERVAL '270 days'),
    ('LIB000017', 'Pip Pirrip',           'pip.pirrip@example.test',           TRUE,  NOW() - INTERVAL '210 days'),
    ('LIB000018', 'Heathcliff Earnshaw',  'heathcliff.earnshaw@example.test',  TRUE,  NOW() - INTERVAL '150 days'),
    ('LIB000019', 'Estella Havisham',     'estella.havisham@example.test',     FALSE, NOW() - INTERVAL '120 days'),
    ('LIB000020', 'Lord Henry Wotton',    'lord.henry.wotton@example.test',    FALSE, NOW() - INTERVAL '90 days')
ON CONFLICT (card_number) DO NOTHING;


-- ============================================================
-- Items at Central (16)
-- ============================================================
-- 3 on_loan (Emma, Northanger Abbey, Mansfield Park), 13 available.

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Pride and Prejudice', 'book', '978-0-141-43951-8', 'available', NOW() - INTERVAL '1600 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Sense and Sensibility', 'book', '978-0-141-43966-2', 'available', NOW() - INTERVAL '1580 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Emma', 'book', '978-0-141-43958-7', 'on_loan', NOW() - INTERVAL '1560 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Mansfield Park', 'book', '978-0-141-43980-8', 'on_loan', NOW() - INTERVAL '1540 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Persuasion', 'book', '978-0-141-43968-6', 'available', NOW() - INTERVAL '1520 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Northanger Abbey', 'book', '978-0-141-43979-2', 'on_loan', NOW() - INTERVAL '1500 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Jane Eyre', 'book', '978-0-141-44114-6', 'available', NOW() - INTERVAL '1480 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Wuthering Heights', 'book', '978-0-141-43955-6', 'available', NOW() - INTERVAL '1460 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Tenant of Wildfell Hall', 'book', '978-0-141-43947-1', 'available', NOW() - INTERVAL '1440 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Bleak House', 'book', '978-0-141-43972-3', 'available', NOW() - INTERVAL '1420 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'David Copperfield', 'book', '978-0-141-43974-7', 'available', NOW() - INTERVAL '1400 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Oliver Twist', 'book', '978-0-141-43974-8', 'available', NOW() - INTERVAL '1380 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Pride and Prejudice (audio)', 'audiobook', NULL, 'available', NOW() - INTERVAL '1360 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Jane Eyre (audio)', 'audiobook', NULL, 'available', NOW() - INTERVAL '1340 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Wuthering Heights (audio)', 'audiobook', NULL, 'available', NOW() - INTERVAL '1320 days'
FROM branches WHERE name = 'Central';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Persuasion (BBC, 1995)', 'dvd', NULL, 'available', NOW() - INTERVAL '1300 days'
FROM branches WHERE name = 'Central';


-- ============================================================
-- Items at North (16)
-- ============================================================
-- 3 on_loan (The Three Musketeers, The Brothers Karamazov, The
-- Sign of the Four), 13 available.

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Les Miserables', 'book', '978-0-451-41943-4', 'available', NOW() - INTERVAL '1280 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Three Musketeers', 'book', '978-0-19-953833-0', 'on_loan', NOW() - INTERVAL '1260 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Count of Monte Cristo', 'book', '978-0-14-044926-4', 'available', NOW() - INTERVAL '1240 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Twenty Years After', 'book', '978-0-19-953896-5', 'available', NOW() - INTERVAL '1220 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Anna Karenina', 'book', '978-0-14-303500-7', 'available', NOW() - INTERVAL '1200 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'War and Peace', 'book', '978-0-14-303999-9', 'available', NOW() - INTERVAL '1180 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Crime and Punishment', 'book', '978-0-14-310463-7', 'available', NOW() - INTERVAL '1160 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Brothers Karamazov', 'book', '978-0-14-044924-0', 'on_loan', NOW() - INTERVAL '1140 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Idiot', 'book', '978-0-14-044792-5', 'available', NOW() - INTERVAL '1120 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Notes from Underground', 'book', '978-0-679-73452-9', 'available', NOW() - INTERVAL '1100 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'A Study in Scarlet', 'book', '978-0-14-043908-1', 'available', NOW() - INTERVAL '1080 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Sign of the Four', 'book', '978-0-14-043907-4', 'on_loan', NOW() - INTERVAL '1060 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Les Miserables (audio)', 'audiobook', NULL, 'available', NOW() - INTERVAL '1040 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'War and Peace (audio)', 'audiobook', NULL, 'available', NOW() - INTERVAL '1020 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Crime and Punishment (audio)', 'audiobook', NULL, 'available', NOW() - INTERVAL '1000 days'
FROM branches WHERE name = 'North';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Sherlock Holmes (Granada, complete)', 'dvd', NULL, 'available', NOW() - INTERVAL '980 days'
FROM branches WHERE name = 'North';


-- ============================================================
-- Items at East (16)
-- ============================================================
-- 3 on_loan (Alice's Adventures in Wonderland, Treasure Island,
-- Moby-Dick), 1 lost (Frankenstein), 12 available.

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Alice''s Adventures in Wonderland', 'book', '978-0-19-955829-1', 'on_loan', NOW() - INTERVAL '960 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Through the Looking-Glass', 'book', '978-0-19-955830-7', 'available', NOW() - INTERVAL '940 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Wonderful Wizard of Oz', 'book', '978-0-19-953991-7', 'available', NOW() - INTERVAL '920 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Treasure Island', 'book', '978-0-14-043768-1', 'on_loan', NOW() - INTERVAL '900 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Kidnapped', 'book', '978-0-14-043794-0', 'available', NOW() - INTERVAL '880 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Master of Ballantrae', 'book', '978-0-14-043924-1', 'available', NOW() - INTERVAL '860 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Moby-Dick', 'book', '978-0-14-243724-7', 'on_loan', NOW() - INTERVAL '840 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Scarlet Letter', 'book', '978-0-14-243726-1', 'available', NOW() - INTERVAL '820 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The House of the Seven Gables', 'book', '978-0-14-039005-4', 'available', NOW() - INTERVAL '800 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Frankenstein', 'book', '978-0-14-143947-1', 'lost', NOW() - INTERVAL '780 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Dracula', 'book', '978-0-14-143984-6', 'available', NOW() - INTERVAL '760 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Strange Case of Dr Jekyll and Mr Hyde', 'book', '978-0-14-143973-0', 'available', NOW() - INTERVAL '740 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Treasure Island (audio)', 'audiobook', NULL, 'available', NOW() - INTERVAL '720 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Dracula (audio)', 'audiobook', NULL, 'available', NOW() - INTERVAL '700 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Frankenstein (audio)', 'audiobook', NULL, 'available', NOW() - INTERVAL '680 days'
FROM branches WHERE name = 'East';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Treasure Island (Disney, 1950)', 'dvd', NULL, 'available', NOW() - INTERVAL '660 days'
FROM branches WHERE name = 'East';


-- ============================================================
-- Items at West (16)
-- ============================================================
-- 3 on_loan (A Tale of Two Cities, Great Expectations, The
-- Picture of Dorian Gray), 1 lost (Kim), 1 withdrawn (The Black
-- Arrow), 11 available.

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'A Tale of Two Cities', 'book', '978-0-14-143960-0', 'on_loan', NOW() - INTERVAL '640 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Great Expectations', 'book', '978-0-14-143956-3', 'on_loan', NOW() - INTERVAL '620 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Hard Times', 'book', '978-0-14-143967-9', 'available', NOW() - INTERVAL '600 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Nicholas Nickleby', 'book', '978-0-14-043512-0', 'available', NOW() - INTERVAL '580 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Picture of Dorian Gray', 'book', '978-0-14-143957-0', 'on_loan', NOW() - INTERVAL '560 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Importance of Being Earnest', 'book', '978-0-14-043606-6', 'available', NOW() - INTERVAL '540 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Hound of the Baskervilles', 'book', '978-0-14-043786-5', 'available', NOW() - INTERVAL '520 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Adventures of Sherlock Holmes', 'book', '978-0-14-043909-8', 'available', NOW() - INTERVAL '500 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Kim', 'book', '978-0-14-018352-7', 'lost', NOW() - INTERVAL '480 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Jungle Book', 'book', '978-0-14-118338-8', 'available', NOW() - INTERVAL '460 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Captains Courageous', 'book', '978-0-14-118364-7', 'available', NOW() - INTERVAL '440 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Black Arrow', 'book', '978-0-14-043756-8', 'withdrawn', NOW() - INTERVAL '420 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Great Expectations (audio)', 'audiobook', NULL, 'available', NOW() - INTERVAL '400 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'A Tale of Two Cities (audio)', 'audiobook', NULL, 'available', NOW() - INTERVAL '380 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Picture of Dorian Gray (audio)', 'audiobook', NULL, 'available', NOW() - INTERVAL '360 days'
FROM branches WHERE name = 'West';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Great Expectations (BBC, 1999)', 'dvd', NULL, 'available', NOW() - INTERVAL '340 days'
FROM branches WHERE name = 'West';


-- ============================================================
-- Items at Bookmobile (16)
-- ============================================================
-- 3 on_loan (The Memoirs of Sherlock Holmes, Anna Karenina
-- audio, Walden), 1 lost (Self-Reliance), 1 withdrawn (The
-- Vicomte of Bragelonne), 11 available.

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Memoirs of Sherlock Holmes', 'book', '978-0-14-043913-5', 'on_loan', NOW() - INTERVAL '320 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Walden', 'book', '978-0-14-243930-2', 'on_loan', NOW() - INTERVAL '300 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Self-Reliance', 'book', '978-0-14-310531-3', 'lost', NOW() - INTERVAL '280 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Vicomte of Bragelonne', 'book', '978-0-19-955576-4', 'withdrawn', NOW() - INTERVAL '260 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Man in the Iron Mask', 'book', '978-0-19-953890-3', 'available', NOW() - INTERVAL '240 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Adventures of Huckleberry Finn', 'book', '978-0-14-243717-9', 'available', NOW() - INTERVAL '220 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Adventures of Tom Sawyer', 'book', '978-0-14-243719-3', 'available', NOW() - INTERVAL '200 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'A Connecticut Yankee in King Arthur''s Court', 'book', '978-0-14-243778-0', 'available', NOW() - INTERVAL '180 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Little Women', 'book', '978-0-14-039069-6', 'available', NOW() - INTERVAL '160 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Anne of Green Gables', 'book', '978-0-14-036741-4', 'available', NOW() - INTERVAL '140 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Secret Garden', 'book', '978-0-14-130910-7', 'available', NOW() - INTERVAL '120 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Wind in the Willows', 'book', '978-0-14-036751-3', 'available', NOW() - INTERVAL '100 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Call of the Wild', 'book', '978-0-14-018651-1', 'available', NOW() - INTERVAL '80 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Anna Karenina (audio)', 'audiobook', NULL, 'on_loan', NOW() - INTERVAL '60 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'The Adventures of Tom Sawyer (audio)', 'audiobook', NULL, 'available', NOW() - INTERVAL '40 days'
FROM branches WHERE name = 'Bookmobile';

INSERT INTO items (branch_id, title, kind, isbn, status, created_at)
SELECT id, 'Anne of Green Gables (CBC, 1985)', 'dvd', NULL, 'available', NOW() - INTERVAL '20 days'
FROM branches WHERE name = 'Bookmobile';


-- ============================================================
-- Loans — returned (15)
-- ============================================================
-- borrowed in the past, returned ~3 weeks later. The items used
-- below all have status = 'available' (the return cleared the
-- on_loan flag).

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000001'),
    (SELECT id FROM items WHERE title = 'Wuthering Heights'),
    'returned',
    NOW() - INTERVAL '180 days',
    NOW() - INTERVAL '159 days',
    NOW() - INTERVAL '161 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000002'),
    (SELECT id FROM items WHERE title = 'Pride and Prejudice'),
    'returned',
    NOW() - INTERVAL '240 days',
    NOW() - INTERVAL '219 days',
    NOW() - INTERVAL '220 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000003'),
    (SELECT id FROM items WHERE title = 'Sense and Sensibility'),
    'returned',
    NOW() - INTERVAL '90 days',
    NOW() - INTERVAL '69 days',
    NOW() - INTERVAL '70 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000004'),
    (SELECT id FROM items WHERE title = 'Persuasion'),
    'returned',
    NOW() - INTERVAL '120 days',
    NOW() - INTERVAL '99 days',
    NOW() - INTERVAL '100 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000005'),
    (SELECT id FROM items WHERE title = 'Jane Eyre'),
    'returned',
    NOW() - INTERVAL '60 days',
    NOW() - INTERVAL '39 days',
    NOW() - INTERVAL '40 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000006'),
    (SELECT id FROM items WHERE title = 'Dracula'),
    'returned',
    NOW() - INTERVAL '150 days',
    NOW() - INTERVAL '129 days',
    NOW() - INTERVAL '128 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000007'),
    (SELECT id FROM items WHERE title = 'Les Miserables'),
    'returned',
    NOW() - INTERVAL '180 days',
    NOW() - INTERVAL '159 days',
    NOW() - INTERVAL '160 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000008'),
    (SELECT id FROM items WHERE title = 'Little Women'),
    'returned',
    NOW() - INTERVAL '60 days',
    NOW() - INTERVAL '39 days',
    NOW() - INTERVAL '35 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000009'),
    (SELECT id FROM items WHERE title = 'War and Peace'),
    'returned',
    NOW() - INTERVAL '120 days',
    NOW() - INTERVAL '99 days',
    NOW() - INTERVAL '98 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000010'),
    (SELECT id FROM items WHERE title = 'Crime and Punishment'),
    'returned',
    NOW() - INTERVAL '90 days',
    NOW() - INTERVAL '69 days',
    NOW() - INTERVAL '70 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000011'),
    (SELECT id FROM items WHERE title = 'A Study in Scarlet'),
    'returned',
    NOW() - INTERVAL '210 days',
    NOW() - INTERVAL '189 days',
    NOW() - INTERVAL '190 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000012'),
    (SELECT id FROM items WHERE title = 'The Hound of the Baskervilles'),
    'returned',
    NOW() - INTERVAL '150 days',
    NOW() - INTERVAL '129 days',
    NOW() - INTERVAL '128 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000013'),
    (SELECT id FROM items WHERE title = 'Through the Looking-Glass'),
    'returned',
    NOW() - INTERVAL '60 days',
    NOW() - INTERVAL '39 days',
    NOW() - INTERVAL '39 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000014'),
    (SELECT id FROM items WHERE title = 'The Count of Monte Cristo'),
    'returned',
    NOW() - INTERVAL '180 days',
    NOW() - INTERVAL '159 days',
    NOW() - INTERVAL '158 days'
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000015'),
    (SELECT id FROM items WHERE title = 'The Scarlet Letter'),
    'returned',
    NOW() - INTERVAL '90 days',
    NOW() - INTERVAL '69 days',
    NOW() - INTERVAL '65 days'
);


-- ============================================================
-- Loans — active (12)
-- ============================================================
-- borrowed_at in the past N days; due_at = borrowed_at + 21 days
-- (still in the future); returned_at NULL. The items used below
-- all have status = 'on_loan'.

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000016'),
    (SELECT id FROM items WHERE title = 'A Tale of Two Cities'),
    'active',
    NOW() - INTERVAL '10 days',
    NOW() + INTERVAL '11 days',
    NULL
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000017'),
    (SELECT id FROM items WHERE title = 'Great Expectations'),
    'active',
    NOW() - INTERVAL '15 days',
    NOW() + INTERVAL '6 days',
    NULL
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000018'),
    (SELECT id FROM items WHERE title = 'The Picture of Dorian Gray'),
    'active',
    NOW() - INTERVAL '7 days',
    NOW() + INTERVAL '14 days',
    NULL
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000001'),
    (SELECT id FROM items WHERE title = 'Emma'),
    'active',
    NOW() - INTERVAL '5 days',
    NOW() + INTERVAL '16 days',
    NULL
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000002'),
    (SELECT id FROM items WHERE title = 'Northanger Abbey'),
    'active',
    NOW() - INTERVAL '3 days',
    NOW() + INTERVAL '18 days',
    NULL
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000003'),
    (SELECT id FROM items WHERE title = 'Mansfield Park'),
    'active',
    NOW() - INTERVAL '12 days',
    NOW() + INTERVAL '9 days',
    NULL
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000007'),
    (SELECT id FROM items WHERE title = 'The Three Musketeers'),
    'active',
    NOW() - INTERVAL '8 days',
    NOW() + INTERVAL '13 days',
    NULL
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000009'),
    (SELECT id FROM items WHERE title = 'The Brothers Karamazov'),
    'active',
    NOW() - INTERVAL '6 days',
    NOW() + INTERVAL '15 days',
    NULL
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000011'),
    (SELECT id FROM items WHERE title = 'The Sign of the Four'),
    'active',
    NOW() - INTERVAL '4 days',
    NOW() + INTERVAL '17 days',
    NULL
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000013'),
    (SELECT id FROM items WHERE title = 'Alice''s Adventures in Wonderland'),
    'active',
    NOW() - INTERVAL '9 days',
    NOW() + INTERVAL '12 days',
    NULL
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000005'),
    (SELECT id FROM items WHERE title = 'Treasure Island'),
    'active',
    NOW() - INTERVAL '2 days',
    NOW() + INTERVAL '19 days',
    NULL
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000006'),
    (SELECT id FROM items WHERE title = 'Moby-Dick'),
    'active',
    NOW() - INTERVAL '11 days',
    NOW() + INTERVAL '10 days',
    NULL
);


-- ============================================================
-- Loans — overdue (3)
-- ============================================================
-- borrowed_at well in the past; due_at also in the past; status
-- explicitly 'overdue'; returned_at NULL. The items used below
-- all have status = 'on_loan'.

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000012'),
    (SELECT id FROM items WHERE title = 'The Memoirs of Sherlock Holmes'),
    'overdue',
    NOW() - INTERVAL '45 days',
    NOW() - INTERVAL '24 days',
    NULL
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000010'),
    (SELECT id FROM items WHERE title = 'Anna Karenina (audio)'),
    'overdue',
    NOW() - INTERVAL '35 days',
    NOW() - INTERVAL '14 days',
    NULL
);

INSERT INTO loans (patron_id, item_id, status, borrowed_at, due_at, returned_at)
VALUES (
    (SELECT id FROM patrons WHERE card_number = 'LIB000004'),
    (SELECT id FROM items WHERE title = 'Walden'),
    'overdue',
    NOW() - INTERVAL '28 days',
    NOW() - INTERVAL '7 days',
    NULL
);


-- ============================================================
-- Framework gap surfaced during D.4 investigation (2026-05-13)
-- ============================================================
-- One item intentionally NOT seeded here because the framework's
-- current public API does not expose a clean project-side hook:
--
--   * Custom bulk-action implementations.
--     `AdminOps` is pub(crate) (types.rs:232); a project cannot
--     override `execute_bulk_action`. Declaring metadata via
--     `ModelAdmin::bulk_actions()` without an implementation
--     would create dead buttons in the admin UI.
--
-- This is a framework decision for a future commit, not an
-- example workaround. The example demonstrates the framework's
-- depth via relational data + audit emission + RBAC defaults
-- only.
--
-- Permission-group seeding (previously listed here as a second
-- gap) became viable in 0.10.2 when `permissions::create_group`
-- learned ON CONFLICT semantics. Group rows still aren't seeded
-- from this SQL file because SQL migrations run before
-- `admin.seed_permissions()` (in Rust at boot), so the
-- permission rows the groups would bind to don't exist yet at
-- migration time. Seeding groups + bindings belongs in Rust
-- alongside the boot path; left out of this example to keep
-- main.rs linear and teaching-focused.
