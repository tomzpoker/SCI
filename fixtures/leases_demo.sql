-- ============================================================
--  Fixture : test des règles de révision (annuel, triennal, long historique)
--
--  DEMO-A : annuel, start 2024-06-01
--    2025-06-01 → T4 2024 → 1025.48 €
--    2026-06-01 → T4 2025 → 1050.95 €
--    prochaine 2027-06-01 → T4 2026 (jaune)
--
--  DEMO-B : triennal, start 2020-06-01
--    2023-06-01 → T4 2022 → 1019.32 €
--    2026-06-01 → T4 2025 → 1047.90 €
--    prochaine 2029-06-01 → T4 2028 (à importer, gris)
--
--  DEMO-C : annuel sur 20 ans, start 2005-06-01
--    20 révisions (2006-06-01 à 2025-06-01) → loyer cumulé ~×1.62
--    prochaine 2026-06-01 → T4 2025 (à vérifier)
-- ============================================================

\set ON_ERROR_STOP on

DO $$
DECLARE
    v_entity  uuid;
    v_unit    uuid;
    v_tenant  uuid;
    v_lease_a uuid;
    v_lease_b uuid;
    v_lease_c uuid;
    v_rent    bigint;
    v_old_rent bigint;
    v_old_idx numeric;
    v_new_idx numeric;
    i         int;
    v_year    int;
    v_prev_idx numeric;
BEGIN
    SELECT id INTO v_entity FROM legal_entities LIMIT 1;
    IF v_entity IS NULL THEN RAISE EXCEPTION 'Aucune legal_entity'; END IF;

    -- ============================================================
    -- 1. Générer tous les ICC utiles (2004 → 2026), formule linéaire
    -- ============================================================
    INSERT INTO lease_index_values
        (legal_entity_id, index_code, period_label, value, source_reference, verified_at)
    SELECT v_entity, 'ICC', 'T' || q || ' ' || y,
           (1500 + (y - 2005) * 45 + (q - 1) * 4)::numeric,
           'INSEE (fixture)', NULL
    FROM generate_series(2004, 2026) y,
         generate_series(1, 4) q
    ON CONFLICT (legal_entity_id, index_code, period_label)
    DO UPDATE SET value = EXCLUDED.value,
                  source_reference = EXCLUDED.source_reference,
                  verified_at = EXCLUDED.verified_at;

    -- ============================================================
    -- 2. Nettoyage des anciens baux de test
    -- ============================================================
    DELETE FROM lease_rent_revisions
     WHERE legal_entity_id = v_entity
       AND lease_id IN (SELECT id FROM leases
                         WHERE legal_entity_id = v_entity
                           AND reference IN ('DEMO-A','DEMO-B','DEMO-C'));
    DELETE FROM leases
     WHERE legal_entity_id = v_entity
       AND reference IN ('DEMO-A','DEMO-B','DEMO-C');

    -- ============================================================
    -- 3. Récupérer un lot + un locataire
    -- ============================================================
    SELECT u.id INTO v_unit
      FROM units u JOIN properties p ON p.id = u.property_id
     WHERE p.legal_entity_id = v_entity LIMIT 1;
    IF v_unit IS NULL THEN RAISE EXCEPTION 'Aucun lot'; END IF;

    SELECT t.id INTO v_tenant
      FROM tenants t WHERE t.legal_entity_id = v_entity LIMIT 1;
    IF v_tenant IS NULL THEN RAISE EXCEPTION 'Aucun locataire'; END IF;

    -- Corriger loyers à 0
    UPDATE leases
       SET rent_amount_cents  = 100000,
           current_rent_cents = 100000,
           updated_at         = now()
     WHERE legal_entity_id = v_entity
       AND COALESCE(NULLIF(current_rent_cents, 0),
                    NULLIF(rent_amount_cents, 0), 0) = 0;

    -- ============================================================
    -- 4. DEMO-A : annuel, start 2024-06-01, base T4 2023
    -- ============================================================
    -- T4 2023 = 1500 + (2023-2005)*45 + 3*4 = 1500 + 810 + 12 = 2322
    INSERT INTO leases (
        legal_entity_id, unit_id, tenant_id, reference, start_date, end_date,
        notice_months, payment_day, annual_review_month, active, signature_date,
        lease_type, destination, rent_amount_cents, current_rent_cents,
        rent_frequency, vat_mode, index_code, index_base_value, index_base_date,
        index_cap_bp, charges_mode, charges_amount_cents,
        security_deposit_expected_cents, entry_fee_expected_cents,
        entry_fee_status, updated_at,
        revision_period_months, index_publication_day, index_publication_month_offset
    ) VALUES (
        v_entity, v_unit, v_tenant, 'DEMO-A', '2024-06-01', NULL,
        3, 1, NULL, true, '2024-05-15',
        'BAIL_COMMERCIAL', 'Bureaux', 100000, 104719,
        'MONTHLY', 'FROM_UNIT', 'ICC', 2322.0, '2023-12-31',
        NULL, 'NONE', 0,
        0, 0, 'NOT_SET', now(),
        12, 12, 3
    ) RETURNING id INTO v_lease_a;

    -- T4 2024 = 1500 + 19*45 + 12 = 2367 → 100000 * 2367 / 2322 = 101938
    -- T4 2025 = 1500 + 20*45 + 12 = 2412 → 101938 * 2412 / 2367 = 103876
    INSERT INTO lease_rent_revisions (
        legal_entity_id, lease_id, calculation_date, effective_date,
        rule_text, index_code, index_period, old_rent_cents,
        index_old, index_new, cap_bp, new_rent_cents, formula,
        result_status, status, note, validated_at, override_rent_cents
    ) VALUES
    (v_entity, v_lease_a, '2025-06-01', '2025-06-01',
     'Indexation contractuelle', 'ICC', 'T4 2024', 100000,
     2322.0, 2367.0, NULL, 101938, '100000 x 2367 / 2322',
     'VALIDATED', 'VALIDATED', '', '2025-06-01 09:00:00+00', NULL),
    (v_entity, v_lease_a, '2026-06-01', '2026-06-01',
     'Indexation contractuelle', 'ICC', 'T4 2025', 101938,
     2367.0, 2412.0, NULL, 103876, '101938 x 2412 / 2367',
     'VALIDATED', 'VALIDATED', '', '2026-06-01 09:00:00+00', NULL);

    UPDATE leases SET current_rent_cents = 103876 WHERE id = v_lease_a;

    -- ============================================================
    -- 5. DEMO-B : triennal, start 2020-06-01, base T4 2019
    -- ============================================================
    -- T4 2019 = 1500 + 14*45 + 12 = 2142
    INSERT INTO leases (
        legal_entity_id, unit_id, tenant_id, reference, start_date, end_date,
        notice_months, payment_day, annual_review_month, active, signature_date,
        lease_type, destination, rent_amount_cents, current_rent_cents,
        rent_frequency, vat_mode, index_code, index_base_value, index_base_date,
        index_cap_bp, charges_mode, charges_amount_cents,
        security_deposit_expected_cents, entry_fee_expected_cents,
        entry_fee_status, updated_at,
        revision_period_months, index_publication_day, index_publication_month_offset
    ) VALUES (
        v_entity, v_unit, v_tenant, 'DEMO-B', '2020-06-01', NULL,
        3, 1, NULL, true, '2020-05-15',
        'BAIL_COMMERCIAL', 'Entrepôt', 100000, 104890,
        'MONTHLY', 'FROM_UNIT', 'ICC', 2142.0, '2019-12-31',
        NULL, 'NONE', 0,
        0, 0, 'NOT_SET', now(),
        36, 12, 3
    ) RETURNING id INTO v_lease_b;

    -- T4 2022 = 1500 + 17*45 + 12 = 2277 → 100000 * 2277 / 2142 = 106303
    -- T4 2025 = 2412 → 106303 * 2412 / 2277 = 112587
    INSERT INTO lease_rent_revisions (
        legal_entity_id, lease_id, calculation_date, effective_date,
        rule_text, index_code, index_period, old_rent_cents,
        index_old, index_new, cap_bp, new_rent_cents, formula,
        result_status, status, note, validated_at, override_rent_cents
    ) VALUES
    (v_entity, v_lease_b, '2023-06-01', '2023-06-01',
     'Indexation contractuelle', 'ICC', 'T4 2022', 100000,
     2142.0, 2277.0, NULL, 106303, '100000 x 2277 / 2142',
     'VALIDATED', 'VALIDATED', '', '2023-06-01 09:00:00+00', NULL),
    (v_entity, v_lease_b, '2026-06-01', '2026-06-01',
     'Indexation contractuelle', 'ICC', 'T4 2025', 106303,
     2277.0, 2412.0, NULL, 112587, '106303 x 2412 / 2277',
     'VALIDATED', 'VALIDATED', '', '2026-06-01 09:00:00+00', NULL);

    UPDATE leases SET current_rent_cents = 112587 WHERE id = v_lease_b;

    -- ============================================================
    -- 6. DEMO-C : annuel sur 20 ans, start 2005-06-01, base T4 2004
    -- ============================================================
    -- T4 2004 = 1500 + 0*45 + 12 = 1512
    INSERT INTO leases (
        legal_entity_id, unit_id, tenant_id, reference, start_date, end_date,
        notice_months, payment_day, annual_review_month, active, signature_date,
        lease_type, destination, rent_amount_cents, current_rent_cents,
        rent_frequency, vat_mode, index_code, index_base_value, index_base_date,
        index_cap_bp, charges_mode, charges_amount_cents,
        security_deposit_expected_cents, entry_fee_expected_cents,
        entry_fee_status, updated_at,
        revision_period_months, index_publication_day, index_publication_month_offset
    ) VALUES (
        v_entity, v_unit, v_tenant, 'DEMO-C', '2005-06-01', NULL,
        3, 1, NULL, true, '2005-05-15',
        'BAIL_COMMERCIAL', 'Local historique', 100000, 100000,
        'MONTHLY', 'FROM_UNIT', 'ICC', 1512.0, '2004-12-31',
        NULL, 'NONE', 0,
        0, 0, 'NOT_SET', now(),
        12, 12, 3
    ) RETURNING id INTO v_lease_c;

    -- Boucle : 20 révisions annuelles, de 2006-06-01 à 2025-06-01
    -- À chaque année y : indice = T4 de (y-1)
    v_rent := 100000;
    v_prev_idx := 1512;  -- T4 2004
    FOR i IN 1..20 LOOP
        v_year := 2005 + i;  -- 2006, 2007, ..., 2025
        -- Indice T4 de (v_year - 1) = 1500 + ((v_year-1) - 2005)*45 + 12
        v_new_idx := 1500 + ((v_year - 1) - 2005) * 45 + 12;
        v_old_rent := v_rent;
        v_rent := round(v_rent::numeric * v_new_idx / v_prev_idx)::bigint;

        INSERT INTO lease_rent_revisions (
            legal_entity_id, lease_id, calculation_date, effective_date,
            rule_text, index_code, index_period, old_rent_cents,
            index_old, index_new, cap_bp, new_rent_cents, formula,
            result_status, status, note, validated_at, override_rent_cents
        ) VALUES (
            v_entity, v_lease_c,
            (v_year || '-06-01')::date, (v_year || '-06-01')::date,
            'Indexation contractuelle', 'ICC', 'T4 ' || (v_year - 1),
            v_old_rent, v_prev_idx, v_new_idx, NULL, v_rent,
            v_old_rent || ' x ' || v_new_idx || ' / ' || v_prev_idx,
            'VALIDATED', 'VALIDATED', '',
            (v_year || '-06-01 09:00:00+00')::timestamptz, NULL
        );

        v_prev_idx := v_new_idx;
    END LOOP;

    UPDATE leases SET current_rent_cents = v_rent WHERE id = v_lease_c;

    RAISE NOTICE 'Fixture OK — A=% B=% C=%', v_lease_a, v_lease_b, v_lease_c;
END $$;