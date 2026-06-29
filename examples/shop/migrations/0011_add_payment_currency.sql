-- Add the settlement currency to payment records. Sweden runs on SEK, so
-- that is the default — existing rows backfill to 'SEK' automatically.
-- Append-only: a new migration rather than an edit to 0009.

ALTER TABLE payments
    ADD COLUMN "currency" TEXT NOT NULL DEFAULT 'SEK';
