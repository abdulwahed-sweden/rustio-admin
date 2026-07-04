-- Payment-method catalogue — the payment "systems" the shop accepts,
-- as a first-class admin entity. Sweden-first: Swish, Klarna and the
-- bank rails (Trustly / Bankgiro / Autogiro) lead, with the BNPL
-- challengers and the global wallets/cards rounding it out.
--
-- This is reference data, so it is seeded here (idempotently) rather than
-- in the demo seed files — the catalogue should exist on a fresh database
-- even before `seeds/` is loaded. Append-only by contract: edit nothing
-- here once applied; add a new numbered migration to evolve it.

CREATE TABLE payment_methods (
    id BIGSERIAL PRIMARY KEY,
    "code" TEXT NOT NULL UNIQUE,           -- stable slug, referenced by payments.method
    "display_name" TEXT NOT NULL,
    "category" TEXT NOT NULL CHECK ("category" IN
        ('mobile', 'invoice', 'card', 'bank_transfer', 'direct_debit', 'wallet')),
    "provider" TEXT NOT NULL,              -- the company / acquirer behind the rail
    "country" TEXT NOT NULL,               -- primary market: 'SE', 'Nordic', 'Global'
    "currency" TEXT NOT NULL DEFAULT 'SEK',
    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
    "display_order" BIGINT NOT NULL DEFAULT 0
);

-- The Swedish payment landscape, most-used first. `display_order` drives
-- the admin list ordering so the everyday rails sit at the top.
INSERT INTO payment_methods
    ("code", "display_name", "category", "provider", "country", "currency", "is_active", "display_order")
VALUES
    ('swish',      'Swish',                    'mobile',        'Getswish AB',          'SE',     'SEK', TRUE,  10),
    ('klarna',     'Klarna',                   'invoice',       'Klarna Bank AB',       'SE',     'SEK', TRUE,  20),
    ('card',       'Card (Visa / Mastercard)', 'card',          'Nets / Nexi',          'Global', 'SEK', TRUE,  30),
    ('trustly',    'Trustly',                  'bank_transfer', 'Trustly Group AB',     'SE',     'SEK', TRUE,  40),
    ('bankgiro',   'Bankgiro',                 'bank_transfer', 'Bankgirot',            'SE',     'SEK', TRUE,  50),
    ('autogiro',   'Autogiro',                 'direct_debit',  'Bankgirot',            'SE',     'SEK', TRUE,  60),
    ('qliro',      'Qliro',                    'invoice',       'Qliro AB',             'SE',     'SEK', TRUE,  70),
    ('walley',     'Walley',                   'invoice',       'Walley (Collector)',   'SE',     'SEK', TRUE,  80),
    ('apple_pay',  'Apple Pay',                'wallet',        'Apple',                'Global', 'SEK', TRUE,  90),
    ('google_pay', 'Google Pay',               'wallet',        'Google',               'Global', 'SEK', TRUE, 100),
    ('paypal',     'PayPal',                   'wallet',        'PayPal',               'Global', 'SEK', TRUE, 110)
ON CONFLICT ("code") DO NOTHING;
