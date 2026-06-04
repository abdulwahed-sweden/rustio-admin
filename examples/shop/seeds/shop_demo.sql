-- seeds/shop_demo.sql
-- Realistic demo data for the rustio-admin shop database.
-- Data only: uses TRUNCATE + INSERT exclusively, never touches schema.
-- Safe to re-run: TRUNCATE ... RESTART IDENTITY CASCADE resets everything first.
--
-- Foreign keys are resolved by subqueries on natural keys (email / name),
-- never by hard-coded id values. Each order's total is computed from its
-- order_items (quantity * unit_price) at insert time, so totals are exact
-- without any post-insert UPDATE.
--
-- Run with:  psql "$DATABASE_URL" -f seeds/shop_demo.sql

BEGIN;

TRUNCATE order_items, orders, products, customers, categories
    RESTART IDENTITY CASCADE;

-- ---------------------------------------------------------------------------
-- Categories (6)
-- ---------------------------------------------------------------------------
INSERT INTO categories (name, slug) VALUES
    ('Electronics',         'electronics'),
    ('Home & Kitchen',      'home-kitchen'),
    ('Books',               'books'),
    ('Clothing',            'clothing'),
    ('Sports & Outdoors',   'sports-outdoors'),
    ('Toys & Games',        'toys-games');

-- ---------------------------------------------------------------------------
-- Customers (20)
-- ---------------------------------------------------------------------------
INSERT INTO customers (full_name, email, phone) VALUES
    ('James Anderson',   'james.anderson@gmail.com',     '+1 (415) 555-0182'),
    ('Maria Garcia',     'maria.garcia@yahoo.com',       '+1 (212) 555-0143'),
    ('David Chen',       'david.chen@outlook.com',       '+1 (646) 555-0119'),
    ('Sarah Johnson',    'sarah.johnson@gmail.com',       '+1 (305) 555-0177'),
    ('Michael Brown',    'michael.brown@hotmail.com',     '+1 (312) 555-0164'),
    ('Emily Davis',      'emily.davis@gmail.com',         '+1 (206) 555-0138'),
    ('Daniel Martinez',  'daniel.martinez@proton.me',     '+1 (702) 555-0155'),
    ('Jessica Wilson',   'jessica.wilson@gmail.com',      '+1 (617) 555-0192'),
    ('Christopher Lee',  'chris.lee@yahoo.com',           '+1 (408) 555-0126'),
    ('Ashley Taylor',    'ashley.taylor@gmail.com',       '+1 (503) 555-0171'),
    ('Matthew Thomas',   'matthew.thomas@outlook.com',    '+1 (713) 555-0148'),
    ('Amanda White',     'amanda.white@gmail.com',        '+1 (602) 555-0133'),
    ('Joshua Harris',    'joshua.harris@hotmail.com',     '+1 (480) 555-0188'),
    ('Olivia Martin',    'olivia.martin@gmail.com',       '+1 (919) 555-0157'),
    ('Andrew Thompson',  'andrew.thompson@yahoo.com',     '+1 (615) 555-0162'),
    ('Sophia Robinson',  'sophia.robinson@gmail.com',     '+1 (404) 555-0179'),
    ('Ryan Clark',       'ryan.clark@outlook.com',        '+1 (303) 555-0114'),
    ('Isabella Lewis',   'isabella.lewis@gmail.com',      '+1 (512) 555-0196'),
    ('Brandon Walker',   'brandon.walker@hotmail.com',    '+1 (916) 555-0121'),
    ('Mia Hall',         'mia.hall@gmail.com',            '+1 (858) 555-0185');

-- ---------------------------------------------------------------------------
-- Products (30) -- prices between 5.00 and 999.00, mixed stock
-- ---------------------------------------------------------------------------
INSERT INTO products (name, price, in_stock) VALUES
    ('Wireless Mouse',                 24.99,  TRUE),
    ('Mechanical Keyboard',            89.99,  TRUE),
    ('27-inch Monitor',               299.00,  TRUE),
    ('USB-C Hub',                      39.50,  TRUE),
    ('Noise-Cancelling Headphones',   199.99,  TRUE),
    ('Bluetooth Speaker',              59.99,  FALSE),
    ('Laptop Stand',                   34.95,  TRUE),
    ('Webcam 1080p',                   69.99,  TRUE),
    ('External SSD 1TB',              129.00,  TRUE),
    ('Smartphone Charger',             19.99,  TRUE),
    ('Stainless Steel Cookware Set',  249.00,  TRUE),
    ('Chef''s Knife',                  79.00,  TRUE),
    ('Cutting Board',                  22.50,  FALSE),
    ('French Press',                   29.99,  TRUE),
    ('Air Fryer',                     119.99,  TRUE),
    ('The Pragmatic Programmer',       45.00,  TRUE),
    ('Clean Code',                     39.99,  TRUE),
    ('SQL Performance Explained',      32.50,  FALSE),
    ('Designing Data-Intensive Apps',  54.99,  TRUE),
    ('Cotton T-Shirt',                 14.99,  TRUE),
    ('Running Shoes',                  89.95,  TRUE),
    ('Winter Jacket',                 159.00,  FALSE),
    ('Wool Socks 3-Pack',              18.00,  TRUE),
    ('Yoga Mat',                       27.99,  TRUE),
    ('Dumbbell Set 20kg',              89.00,  TRUE),
    ('Camping Tent 4-Person',         199.00,  TRUE),
    ('Water Bottle 1L',                12.99,  TRUE),
    ('Board Game: Settlers',           49.99,  TRUE),
    ('Building Blocks 500pc',          34.99,  FALSE),
    ('Remote Control Car',             64.99,  TRUE);

-- ---------------------------------------------------------------------------
-- Orders (40) + Order items (89), built in one statement.
--
-- order_meta  : one row per order (customer + status)
-- item_spec   : one row per order line (order_no, product, quantity)
-- order_totals: per-order total derived from item_spec * real product price
-- ins_orders  : insert orders (total already exact) ordered by order_no,
--               so BIGSERIAL ids ascend in step with order_no
-- orders_mapped: pair each new order id back to its order_no via row_number
-- final INSERT : order_items, unit_price copied straight from products.price
-- ---------------------------------------------------------------------------
WITH order_meta(order_no, cust_email, status) AS (
    VALUES
        ( 1, 'james.anderson@gmail.com',  'shipped'),
        ( 2, 'maria.garcia@yahoo.com',    'paid'),
        ( 3, 'david.chen@outlook.com',    'shipped'),
        ( 4, 'sarah.johnson@gmail.com',   'paid'),
        ( 5, 'michael.brown@hotmail.com', 'cancelled'),
        ( 6, 'emily.davis@gmail.com',     'shipped'),
        ( 7, 'daniel.martinez@proton.me', 'paid'),
        ( 8, 'jessica.wilson@gmail.com',  'pending'),
        ( 9, 'chris.lee@yahoo.com',       'shipped'),
        (10, 'ashley.taylor@gmail.com',   'paid'),
        (11, 'matthew.thomas@outlook.com','shipped'),
        (12, 'amanda.white@gmail.com',    'paid'),
        (13, 'joshua.harris@hotmail.com', 'cancelled'),
        (14, 'olivia.martin@gmail.com',   'shipped'),
        (15, 'andrew.thompson@yahoo.com', 'paid'),
        (16, 'sophia.robinson@gmail.com', 'pending'),
        (17, 'ryan.clark@outlook.com',    'shipped'),
        (18, 'isabella.lewis@gmail.com',  'paid'),
        (19, 'brandon.walker@hotmail.com','shipped'),
        (20, 'mia.hall@gmail.com',        'paid'),
        (21, 'james.anderson@gmail.com',  'shipped'),
        (22, 'maria.garcia@yahoo.com',    'paid'),
        (23, 'david.chen@outlook.com',    'pending'),
        (24, 'sarah.johnson@gmail.com',   'shipped'),
        (25, 'emily.davis@gmail.com',     'cancelled'),
        (26, 'daniel.martinez@proton.me', 'paid'),
        (27, 'jessica.wilson@gmail.com',  'shipped'),
        (28, 'chris.lee@yahoo.com',       'paid'),
        (29, 'ashley.taylor@gmail.com',   'shipped'),
        (30, 'amanda.white@gmail.com',    'pending'),
        (31, 'olivia.martin@gmail.com',   'paid'),
        (32, 'sophia.robinson@gmail.com', 'shipped'),
        (33, 'ryan.clark@outlook.com',    'paid'),
        (34, 'isabella.lewis@gmail.com',  'shipped'),
        (35, 'brandon.walker@hotmail.com','cancelled'),
        (36, 'mia.hall@gmail.com',        'paid'),
        (37, 'james.anderson@gmail.com',  'shipped'),
        (38, 'maria.garcia@yahoo.com',    'paid'),
        (39, 'matthew.thomas@outlook.com','shipped'),
        (40, 'andrew.thompson@yahoo.com', 'paid')
),
item_spec(order_no, prod_name, quantity) AS (
    VALUES
        ( 1, 'Wireless Mouse',                2),
        ( 1, 'Mechanical Keyboard',           1),
        ( 1, 'USB-C Hub',                     1),
        ( 2, '27-inch Monitor',               1),
        ( 2, 'USB-C Hub',                     2),
        ( 3, 'Noise-Cancelling Headphones',   1),
        ( 4, 'Bluetooth Speaker',             1),
        ( 4, 'Smartphone Charger',            3),
        ( 4, 'Wireless Mouse',                1),
        ( 5, 'Air Fryer',                     1),
        ( 6, 'Chef''s Knife',                 1),
        ( 6, 'Cutting Board',                 2),
        ( 6, 'French Press',                  1),
        ( 7, 'The Pragmatic Programmer',      1),
        ( 7, 'Clean Code',                    1),
        ( 7, 'Designing Data-Intensive Apps', 1),
        ( 7, 'SQL Performance Explained',     1),
        ( 8, 'Cotton T-Shirt',                3),
        ( 9, 'Running Shoes',                 1),
        ( 9, 'Wool Socks 3-Pack',             2),
        (10, 'Yoga Mat',                      1),
        (10, 'Water Bottle 1L',               2),
        (10, 'Cotton T-Shirt',                2),
        (11, 'External SSD 1TB',              1),
        (11, 'USB-C Hub',                     1),
        (12, 'Laptop Stand',                  2),
        (12, 'Webcam 1080p',                  1),
        (12, 'Smartphone Charger',            2),
        (13, 'Winter Jacket',                 1),
        (14, 'Camping Tent 4-Person',         1),
        (14, 'Water Bottle 1L',               3),
        (14, 'Wool Socks 3-Pack',             1),
        (15, 'Board Game: Settlers',          2),
        (16, 'Building Blocks 500pc',         1),
        (16, 'Remote Control Car',            1),
        (17, 'Stainless Steel Cookware Set',  1),
        (18, 'Mechanical Keyboard',           1),
        (18, 'Wireless Mouse',                1),
        (18, '27-inch Monitor',               1),
        (18, 'USB-C Hub',                     1),
        (19, 'Smartphone Charger',            2),
        (19, 'USB-C Hub',                     1),
        (20, 'Dumbbell Set 20kg',             1),
        (20, 'Yoga Mat',                      1),
        (20, 'Water Bottle 1L',               2),
        (21, 'Clean Code',                    2),
        (22, 'Noise-Cancelling Headphones',   1),
        (22, 'Bluetooth Speaker',             1),
        (22, 'Smartphone Charger',            1),
        (23, 'French Press',                  1),
        (23, 'Chef''s Knife',                 1),
        (24, 'Running Shoes',                 1),
        (25, 'Air Fryer',                     1),
        (25, 'Cutting Board',                 1),
        (26, 'External SSD 1TB',              2),
        (27, 'Webcam 1080p',                  1),
        (27, 'Laptop Stand',                  1),
        (27, 'USB-C Hub',                     2),
        (27, 'Wireless Mouse',                1),
        (28, 'Cotton T-Shirt',                4),
        (28, 'Wool Socks 3-Pack',             1),
        (29, 'Water Bottle 1L',               5),
        (30, 'The Pragmatic Programmer',      1),
        (31, 'Designing Data-Intensive Apps', 1),
        (31, 'SQL Performance Explained',     1),
        (31, 'Clean Code',                    1),
        (32, 'Smartphone Charger',            1),
        (32, 'Wireless Mouse',                1),
        (33, 'Camping Tent 4-Person',         1),
        (34, 'Board Game: Settlers',          1),
        (34, 'Remote Control Car',            1),
        (34, 'Building Blocks 500pc',         1),
        (34, 'Cotton T-Shirt',                2),
        (35, 'Winter Jacket',                 1),
        (35, 'Wool Socks 3-Pack',             1),
        (36, 'Mechanical Keyboard',           2),
        (37, '27-inch Monitor',               2),
        (37, 'Laptop Stand',                  1),
        (37, 'USB-C Hub',                     2),
        (38, 'Dumbbell Set 20kg',             1),
        (38, 'Water Bottle 1L',               3),
        (38, 'Yoga Mat',                      1),
        (39, 'Noise-Cancelling Headphones',   1),
        (39, 'USB-C Hub',                     1),
        (39, 'Smartphone Charger',            1),
        (40, 'Chef''s Knife',                 1),
        (40, 'French Press',                  1),
        (40, 'Cutting Board',                 1),
        (40, 'Wool Socks 3-Pack',             1)
),
order_totals AS (
    SELECT om.order_no,
           om.cust_email,
           om.status,
           SUM(it.quantity * p.price) AS total
    FROM order_meta om
    JOIN item_spec it ON it.order_no = om.order_no
    JOIN products  p  ON p.name = it.prod_name
    GROUP BY om.order_no, om.cust_email, om.status
),
ins_orders AS (
    INSERT INTO orders (customer_id, total, status)
    SELECT (SELECT c.id FROM customers c WHERE c.email = ot.cust_email),
           ot.total,
           ot.status
    FROM order_totals ot
    ORDER BY ot.order_no
    RETURNING id
),
orders_mapped AS (
    SELECT id AS order_id,
           row_number() OVER (ORDER BY id) AS order_no
    FROM ins_orders
)
INSERT INTO order_items (order_id, product_id, quantity, unit_price)
SELECT om.order_id,
       p.id,
       it.quantity,
       p.price
FROM item_spec it
JOIN orders_mapped om ON om.order_no = it.order_no
JOIN products      p  ON p.name = it.prod_name;

COMMIT;

-- Refresh planner statistics (pg_class.reltuples) so the admin
-- dashboard's approximate row counts are correct immediately, instead
-- of reading 0 until autovacuum next runs ANALYZE. Outside the
-- transaction so it sees the committed rows.
ANALYZE categories, customers, products, orders, order_items;
