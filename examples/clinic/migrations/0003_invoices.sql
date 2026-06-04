-- Billing capability: invoices raised against patients.
-- Append-only: add a new numbered file to change the schema.

CREATE TABLE invoices (
    id           BIGSERIAL   PRIMARY KEY,
    patient_id   BIGINT      NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    amount_cents BIGINT      NOT NULL DEFAULT 0,
    status       TEXT        NOT NULL DEFAULT 'unpaid',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX invoices_patient_idx ON invoices (patient_id);
CREATE INDEX invoices_status_idx  ON invoices (status);
