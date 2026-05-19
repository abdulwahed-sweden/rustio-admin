CREATE TABLE practitioners (
    id         BIGSERIAL   PRIMARY KEY,
    clinic_id  BIGINT      NOT NULL REFERENCES clinics(id) ON DELETE RESTRICT,
    full_name  TEXT        NOT NULL,
    specialty  TEXT        NOT NULL,
    license_no TEXT,
    status     TEXT        NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (specialty IN ('family_medicine', 'pediatrics', 'cardiology', 'dermatology', 'orthopedics', 'psychiatry')),
    CHECK (status    IN ('active', 'on_leave', 'retired', 'pending'))
);

CREATE INDEX practitioners_clinic_id_idx ON practitioners (clinic_id);
CREATE INDEX practitioners_status_idx    ON practitioners (status);
