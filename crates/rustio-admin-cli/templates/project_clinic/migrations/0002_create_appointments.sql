CREATE TABLE appointments (
    id            BIGSERIAL    PRIMARY KEY,
    patient_id    BIGINT       NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    scheduled_for TIMESTAMPTZ  NOT NULL,
    reason        TEXT         NOT NULL DEFAULT '',
    status        TEXT         NOT NULL DEFAULT 'scheduled'
                  CHECK (status IN ('scheduled', 'completed', 'cancelled')),
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX appointments_patient_id_idx    ON appointments (patient_id);
CREATE INDEX appointments_scheduled_for_idx ON appointments (scheduled_for DESC);

-- Example appointments linked to the seeded patients by name, so the
-- foreign keys stay correct regardless of the generated ids.
INSERT INTO appointments (patient_id, scheduled_for, reason, status) VALUES
    ((SELECT id FROM patients WHERE full_name = 'Sarah Ahmed'), NOW() + INTERVAL '2 days', 'Annual checkup',  'scheduled'),
    ((SELECT id FROM patients WHERE full_name = 'John Okoro'),  NOW() + INTERVAL '3 days', 'Follow-up',       'scheduled'),
    ((SELECT id FROM patients WHERE full_name = 'Maria Lopez'), NOW() - INTERVAL '5 days', 'Lab results',     'completed');
