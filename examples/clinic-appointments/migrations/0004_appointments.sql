CREATE TABLE appointments (
    id              BIGSERIAL   PRIMARY KEY,
    patient_id      BIGINT      NOT NULL REFERENCES patients(id)      ON DELETE RESTRICT,
    practitioner_id BIGINT      NOT NULL REFERENCES practitioners(id) ON DELETE RESTRICT,
    status          TEXT        NOT NULL DEFAULT 'scheduled',
    scheduled_at    TIMESTAMPTZ NOT NULL,
    checked_in_at   TIMESTAMPTZ,
    ended_at        TIMESTAMPTZ,
    CHECK (status IN ('scheduled', 'completed', 'cancelled', 'no_show'))
);

CREATE INDEX appointments_patient_id_idx      ON appointments (patient_id);
CREATE INDEX appointments_practitioner_id_idx ON appointments (practitioner_id);
CREATE INDEX appointments_status_idx          ON appointments (status);
CREATE INDEX appointments_scheduled_at_idx    ON appointments (scheduled_at);
