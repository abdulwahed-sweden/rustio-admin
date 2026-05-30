-- Patient management capability: patients + their vitals.
-- Append-only: once applied, never edit this file — add a new numbered one.
-- Primary keys are BIGINT/BIGSERIAL (i64) — the admin runtime requires it.

CREATE TABLE patients (
    id          BIGSERIAL   PRIMARY KEY,
    full_name   TEXT        NOT NULL,
    email       TEXT        NOT NULL DEFAULT '',
    phone       TEXT        NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Full-text search column, maintained by Postgres. The admin opts
    -- into it via `Patient::search_index_column()`; nothing is indexed
    -- outside Postgres. Add columns to the expression as the model grows.
    search_vector tsvector GENERATED ALWAYS AS (
        to_tsvector(
            'english',
            coalesce(full_name, '') || ' ' ||
            coalesce(email, '')     || ' ' ||
            coalesce(phone, '')
        )
    ) STORED
);

CREATE INDEX patients_search_idx ON patients USING GIN (search_vector);

CREATE TABLE vitals (
    id          BIGSERIAL   PRIMARY KEY,
    patient_id  BIGINT      NOT NULL REFERENCES patients(id) ON DELETE CASCADE,
    heart_rate  BIGINT      NOT NULL DEFAULT 0,
    notes       TEXT        NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX vitals_patient_idx ON vitals (patient_id);
