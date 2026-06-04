-- seeds/shop_demo_extra.sql
-- Demo data for the four apps added after the initial seed:
--   addresses, product_images, cart_items, payments
--
-- Depends on shop_demo.sql having been run first (it needs real
-- customers, products and orders to point at). Data only: TRUNCATE +
-- INSERT exclusively, never touches schema. Safe to re-run.
--
-- Every foreign key is resolved by selecting a real id:
--   * customers / products -> subquery on their natural key (email / name)
--   * orders               -> selected directly from `orders`, so the row's
--                             own id flows into payments.order_id and the
--                             payment amount copies that order's total.
--
-- Run with:  psql "$DATABASE_URL" -f seeds/shop_demo_extra.sql

BEGIN;

TRUNCATE addresses, product_images, cart_items, payments
    RESTART IDENTITY CASCADE;

-- ---------------------------------------------------------------------------
-- Addresses (24) -- one default per customer + a few extra shipping addresses.
-- Country mix (US / Canada / UK / Germany / Australia) gives the
-- `country` + `is_default` list filters something to chew on.
-- ---------------------------------------------------------------------------
INSERT INTO addresses
    (customer_id, recipient_name, line1, line2, city, postal_code, country, is_default)
SELECT (SELECT id FROM customers c WHERE c.email = a.email),
       a.recipient_name, a.line1, a.line2, a.city, a.postal_code, a.country, a.is_default
FROM (VALUES
    ('james.anderson@gmail.com',  'James Anderson',  '123 Market St',              'Apt 4B',    'San Francisco', '94103',   'United States',  TRUE),
    ('maria.garcia@yahoo.com',    'Maria Garcia',    '456 5th Ave',                '',          'New York',      '10001',   'United States',  TRUE),
    ('david.chen@outlook.com',    'David Chen',      '789 Granville St',           'Unit 1203', 'Vancouver',     'V6B 1A1', 'Canada',         TRUE),
    ('sarah.johnson@gmail.com',   'Sarah Johnson',   '321 Ocean Dr',               '',          'Miami',         '33101',   'United States',  TRUE),
    ('michael.brown@hotmail.com', 'Michael Brown',   '654 Michigan Ave',           'Suite 200', 'Chicago',       '60601',   'United States',  TRUE),
    ('emily.davis@gmail.com',     'Emily Davis',     '987 Pine St',                'Apt 12',    'Seattle',       '98101',   'United States',  TRUE),
    ('daniel.martinez@proton.me', 'Daniel Martinez', '159 Main St',                '',          'Houston',       '77002',   'United States',  TRUE),
    ('jessica.wilson@gmail.com',  'Jessica Wilson',  '753 Beacon St',              'Apt 3',     'Boston',        '02108',   'United States',  TRUE),
    ('chris.lee@yahoo.com',       'Christopher Lee', '852 Bay St',                 'Unit 905',  'Toronto',       'M5H 2N2', 'Canada',         TRUE),
    ('ashley.taylor@gmail.com',   'Ashley Taylor',   '246 Burnside St',            '',          'Portland',      '97201',   'United States',  TRUE),
    ('matthew.thomas@outlook.com','Matthew Thomas',  '10 Downing St',              'Flat 2',    'London',        'EC1A 1BB','United Kingdom',  TRUE),
    ('amanda.white@gmail.com',    'Amanda White',    '135 Camelback Rd',           '',          'Phoenix',       '85001',   'United States',  TRUE),
    ('joshua.harris@hotmail.com', 'Joshua Harris',   '864 Scottsdale Rd',          'Apt 7',     'Scottsdale',    '85251',   'United States',  TRUE),
    ('olivia.martin@gmail.com',   'Olivia Martin',   '370 Fayetteville St',        '',          'Raleigh',       '27601',   'United States',  TRUE),
    ('andrew.thompson@yahoo.com', 'Andrew Thompson', '481 Broadway',               'Suite 5',   'Nashville',     '37201',   'United States',  TRUE),
    ('sophia.robinson@gmail.com', 'Sophia Robinson', '592 Peachtree St',           '',          'Atlanta',       '30303',   'United States',  TRUE),
    ('ryan.clark@outlook.com',    'Ryan Clark',      '703 16th St',                'Apt 9',     'Denver',        '80202',   'United States',  TRUE),
    ('isabella.lewis@gmail.com',  'Isabella Lewis',  '814 Congress Ave',           '',          'Austin',        '78701',   'United States',  TRUE),
    ('brandon.walker@hotmail.com','Brandon Walker',  'Unter den Linden 5',         '',          'Berlin',        '10115',   'Germany',        TRUE),
    ('mia.hall@gmail.com',        'Mia Hall',        '42 George St',               'Level 3',   'Sydney',        '2000',    'Australia',      TRUE),
    -- secondary (non-default) addresses for a handful of customers
    ('james.anderson@gmail.com',  'James Anderson',  '55 Broadway',                'Floor 8',   'Oakland',       '94607',   'United States',  FALSE),
    ('maria.garcia@yahoo.com',    'Maria Garcia',    '200 Atlantic Ave',           'Apt 6C',    'Brooklyn',      '11201',   'United States',  FALSE),
    ('david.chen@outlook.com',    'David Chen',      '1000 Rue Sainte-Catherine',  '',          'Montreal',      'H2Y 1C6', 'Canada',         FALSE),
    ('matthew.thomas@outlook.com','Matthew Thomas',  '25 Deansgate',               '',          'Manchester',    'M1 1AE',  'United Kingdom',  FALSE)
) AS a(email, recipient_name, line1, line2, city, postal_code, country, is_default);

-- ---------------------------------------------------------------------------
-- Product images (42) -- one primary per product + secondary shots for a few.
-- URLs are built from the product's real id so each path is unique.
-- ---------------------------------------------------------------------------

-- Primary image for every product (position 1, is_primary = true).
INSERT INTO product_images (product_id, url, alt_text, position, is_primary)
SELECT p.id,
       'https://cdn.shop.example/products/' || p.id || '/main.jpg',
       p.name || ' — main product photo',
       1,
       TRUE
FROM products p;

-- Secondary images (position 2, is_primary = false) for a subset.
INSERT INTO product_images (product_id, url, alt_text, position, is_primary)
SELECT (SELECT id FROM products p WHERE p.name = v.pname),
       'https://cdn.shop.example/products/'
           || (SELECT id FROM products p WHERE p.name = v.pname) || '/' || v.file,
       v.alt_text,
       v.position,
       FALSE
FROM (VALUES
    ('Wireless Mouse',               'side.jpg',       'Wireless Mouse — side profile',        2),
    ('Mechanical Keyboard',          'angle.jpg',      'Mechanical Keyboard — angled view',    2),
    ('27-inch Monitor',              'back.jpg',       '27-inch Monitor — rear ports',         2),
    ('Noise-Cancelling Headphones',  'case.jpg',       'Noise-Cancelling Headphones — case',   2),
    ('Air Fryer',                    'open.jpg',       'Air Fryer — basket open',              2),
    ('Chef''s Knife',                'blade.jpg',      'Chef''s Knife — blade close-up',       2),
    ('Running Shoes',                'sole.jpg',       'Running Shoes — sole detail',          2),
    ('Winter Jacket',                'hood.jpg',       'Winter Jacket — hood up',              2),
    ('Camping Tent 4-Person',        'setup.jpg',      'Camping Tent — pitched setup',         2),
    ('Board Game: Settlers',         'components.jpg', 'Settlers — game components',           2),
    ('Remote Control Car',           'top.jpg',        'Remote Control Car — top view',        2),
    ('External SSD 1TB',             'ports.jpg',      'External SSD — USB-C port',            2)
) AS v(pname, file, alt_text, position);

-- ---------------------------------------------------------------------------
-- Cart items (15) -- a few customers with live, un-checked-out carts.
-- ---------------------------------------------------------------------------
INSERT INTO cart_items (customer_id, product_id, quantity)
SELECT (SELECT id FROM customers c WHERE c.email = v.email),
       (SELECT id FROM products  p WHERE p.name  = v.pname),
       v.quantity
FROM (VALUES
    ('james.anderson@gmail.com',  'USB-C Hub',                 1),
    ('james.anderson@gmail.com',  'Laptop Stand',              1),
    ('maria.garcia@yahoo.com',    'Cotton T-Shirt',            2),
    ('maria.garcia@yahoo.com',    'Wool Socks 3-Pack',         1),
    ('emily.davis@gmail.com',     'French Press',              1),
    ('emily.davis@gmail.com',     'Chef''s Knife',             1),
    ('daniel.martinez@proton.me', 'Clean Code',                1),
    ('daniel.martinez@proton.me', 'SQL Performance Explained', 1),
    ('ashley.taylor@gmail.com',   'Yoga Mat',                  1),
    ('ashley.taylor@gmail.com',   'Water Bottle 1L',           3),
    ('isabella.lewis@gmail.com',  'Mechanical Keyboard',       1),
    ('sophia.robinson@gmail.com', 'Running Shoes',             1),
    ('sophia.robinson@gmail.com', 'Wool Socks 3-Pack',         2),
    ('mia.hall@gmail.com',        'Board Game: Settlers',      1),
    ('ryan.clark@outlook.com',    'External SSD 1TB',          1)
) AS v(email, pname, quantity);

-- ---------------------------------------------------------------------------
-- Payments (40) -- one per order, status derived from the order's own status:
--   paid / shipped -> completed      pending -> pending
--   cancelled      -> refunded / failed (alternating, 2 each)
-- amount copies the order's total; provider / method vary by order id so the
-- `status` / `provider` / `method` list filters all have spread.
-- ---------------------------------------------------------------------------

-- Completed payments for fulfilled and paid orders.
INSERT INTO payments (order_id, provider, method, transaction_id, amount, status)
SELECT o.id,
       (ARRAY['stripe','paypal','adyen','braintree'])[1 + (o.id % 4)],
       (ARRAY['card','paypal','apple_pay','google_pay'])[1 + (o.id % 4)],
       'txn_2026_' || to_char(o.id, 'FM00000'),
       o.total,
       'completed'
FROM orders o
WHERE o.status IN ('paid', 'shipped');

-- Pending payments for orders still awaiting payment.
INSERT INTO payments (order_id, provider, method, transaction_id, amount, status)
SELECT o.id,
       (ARRAY['stripe','paypal','adyen','braintree'])[1 + (o.id % 4)],
       (ARRAY['card','paypal','apple_pay','google_pay'])[1 + (o.id % 4)],
       'txn_2026_' || to_char(o.id, 'FM00000'),
       o.total,
       'pending'
FROM orders o
WHERE o.status = 'pending';

-- Cancelled orders: alternate refunded / failed so both statuses appear.
INSERT INTO payments (order_id, provider, method, transaction_id, amount, status)
SELECT c.id,
       (ARRAY['stripe','paypal','adyen','braintree'])[1 + (c.id % 4)],
       (ARRAY['card','paypal','apple_pay','google_pay'])[1 + (c.id % 4)],
       'txn_2026_' || to_char(c.id, 'FM00000'),
       c.total,
       CASE WHEN c.rn % 2 = 0 THEN 'refunded' ELSE 'failed' END
FROM (
    SELECT o.id, o.total, row_number() OVER (ORDER BY o.id) AS rn
    FROM orders o
    WHERE o.status = 'cancelled'
) AS c;

COMMIT;

-- Refresh planner statistics (pg_class.reltuples) so the admin
-- dashboard's approximate row counts are correct immediately, instead
-- of reading 0 until autovacuum next runs ANALYZE. Outside the
-- transaction so it sees the committed rows.
ANALYZE addresses, product_images, cart_items, payments;
