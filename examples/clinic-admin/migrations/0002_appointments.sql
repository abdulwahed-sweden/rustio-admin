-- Scheduling capability: appointments booked for patients.
-- Append-only: add a new numbered file to change the schema.

CREATE TABLE appointments (
    id           BIGSERIAL   PRIMARY KEY,
    patient_id   BIGINT      NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    scheduled_at TIMESTAMPTZ NOT NULL,
    reason       TEXT        NOT NULL DEFAULT '',
    status       TEXT        NOT NULL DEFAULT 'scheduled',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX appointments_patient_idx ON appointments (patient_id);
CREATE INDEX appointments_status_idx  ON appointments (status);
