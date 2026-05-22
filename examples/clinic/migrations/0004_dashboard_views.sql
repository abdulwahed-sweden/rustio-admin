-- =============================================================================
-- Clinic dashboard view — unified open-action-items surface.
-- =============================================================================
--
-- One view, v_open_action_items, that UNIONs three categories of pending
-- work into a single uniform shape so a dashboard can render them as one
-- worklist and a cron / job runner can route by category. Each row is one
-- thing the clinic should do.
--
-- Categories (one row per actionable item):
--
--   uncollected_prescription
--     A prescription written ≥ 7 days ago that the pharmacy still
--     hasn't dispensed. Source: v_active_medications. Subject is the
--     patient; the related row is the prescription. Severity tiers
--     reflect how stale the script is.
--
--   patient_outstanding
--     An invoice with non-zero outstanding_from_patient. Source:
--     v_invoice_outstanding_by_source. Subject is the patient; the
--     related row is the invoice. Severity tiers reflect proximity
--     to the invoice due_date.
--
--   insurance_claim_pending
--     An invoice with non-zero outstanding_from_insurer. Source:
--     same view. Subject is the INSURER (not the patient — claims are
--     filed against the insurer); related row is the invoice. Severity
--     tiers reflect proximity to due_date.
--
-- Severity tiers:
--
--   Invoices: 'overdue'   → past due_date
--             'due_soon'  → within 7 days of due_date
--             'normal'    → further out, or no due_date set
--   Scripts:  'overdue'   → > 21 days uncollected
--             'due_soon'  → 14–21 days uncollected
--             'normal'    → 7–14 days uncollected (the filter floor)
--
-- Useful queries:
--
--   Today's worklist sorted by severity then age:
--     SELECT category, severity, subject_name, description, amount_sek
--     FROM   v_open_action_items
--     ORDER  BY
--       CASE severity WHEN 'overdue' THEN 1
--                     WHEN 'due_soon' THEN 2
--                     ELSE 3 END,
--       days_outstanding DESC;
--
--   Counts per category (header KPIs):
--     SELECT category, count(*) AS open
--     FROM   v_open_action_items
--     GROUP  BY category;
--
--   Front-desk-only items (exclude insurance claims):
--     SELECT * FROM v_open_action_items
--     WHERE  category <> 'insurance_claim_pending';
-- =============================================================================

BEGIN;

CREATE OR REPLACE VIEW v_open_action_items AS

  -- ---- Uncollected prescriptions (> 7 days since issued, not dispensed)
  SELECT
    'uncollected_prescription'::text                                 AS category,
    CASE
      WHEN am.issued_at < now() - interval '21 days' THEN 'overdue'
      WHEN am.issued_at < now() - interval '14 days' THEN 'due_soon'
      ELSE 'normal'
    END                                                              AS severity,
    am.patient                                                       AS subject_name,
    am.patient_id                                                    AS subject_id,
    am.medication || ' ' || am.strength
      || ' (take ' || am.dosage || ') uncollected since '
      || to_char(am.issued_at AT TIME ZONE 'Europe/Stockholm', 'YYYY-MM-DD')
                                                                     AS description,
    NULL::numeric(12,2)                                              AS amount_sek,
    am.issued_at::date                                               AS trigger_date,
    (CURRENT_DATE - am.issued_at::date)                              AS days_outstanding,
    am.prescription_id                                               AS related_id,
    'RX'::text                                                       AS related_kind
  FROM v_active_medications am
  WHERE am.is_dispensed = false
    AND am.issued_at < now() - interval '7 days'

  UNION ALL

  -- ---- Patient-owed outstanding amount on any invoice
  SELECT
    'patient_outstanding'::text                                      AS category,
    CASE
      WHEN v.due_date IS NOT NULL AND v.due_date < CURRENT_DATE       THEN 'overdue'
      WHEN v.due_date IS NOT NULL AND v.due_date <= CURRENT_DATE + 7  THEN 'due_soon'
      ELSE 'normal'
    END                                                              AS severity,
    p.full_name                                                      AS subject_name,
    v.patient_id                                                     AS subject_id,
    'Patient owes ' || v.outstanding_from_patient
      || ' SEK on ' || v.invoice_number
      || coalesce(' (due ' || v.due_date::text || ')', '')
                                                                     AS description,
    v.outstanding_from_patient                                       AS amount_sek,
    v.due_date                                                       AS trigger_date,
    (CURRENT_DATE - i.issued_at::date)                               AS days_outstanding,
    v.id                                                             AS related_id,
    'INV'::text                                                      AS related_kind
  FROM v_invoice_outstanding_by_source v
  JOIN invoices  i  ON i.id = v.id
  JOIN patients  p  ON p.id = v.patient_id
  WHERE v.outstanding_from_patient > 0

  UNION ALL

  -- ---- Insurance claims pending (subject = insurer)
  SELECT
    'insurance_claim_pending'::text                                  AS category,
    CASE
      WHEN v.due_date IS NOT NULL AND v.due_date < CURRENT_DATE       THEN 'overdue'
      WHEN v.due_date IS NOT NULL AND v.due_date <= CURRENT_DATE + 7  THEN 'due_soon'
      ELSE 'normal'
    END                                                              AS severity,
    v.insurance_provider                                             AS subject_name,
    v.insurance_policy_id                                            AS subject_id,
    'Claim ' || v.invoice_number
      || ' for ' || p.full_name
      || ' — pending ' || v.outstanding_from_insurer || ' SEK from '
      || v.insurance_provider
      || coalesce(' (due ' || v.due_date::text || ')', '')
                                                                     AS description,
    v.outstanding_from_insurer                                       AS amount_sek,
    v.due_date                                                       AS trigger_date,
    (CURRENT_DATE - i.issued_at::date)                               AS days_outstanding,
    v.id                                                             AS related_id,
    'INV'::text                                                      AS related_kind
  FROM v_invoice_outstanding_by_source v
  JOIN invoices i ON i.id = v.id
  JOIN patients p ON p.id = v.patient_id
  WHERE v.outstanding_from_insurer > 0;

COMMIT;
