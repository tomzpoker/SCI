# ============================================================================
#  Seed : donnees de test pour la base Rust
#  Idempotent : peut etre relance sans creer de doublons.
# ============================================================================

$projectRoot = "D:\SCI\DEV\SCI-family-rust"
$sqlFile = Join-Path $env:TEMP "seed-test-data.sql"

$sql = @'
DO $$
DECLARE
    v_sci_id uuid;
    v_legal_entity_id uuid;
    v_property_id uuid;
    v_unit_a_id uuid;
    v_unit_b_id uuid;
    v_tenant_1_id uuid;
    v_tenant_2_id uuid;
BEGIN
    -- Recupere la SCI existante (creee au bootstrap de l'app)
    SELECT id, legal_entity_id INTO v_sci_id, v_legal_entity_id
    FROM scis LIMIT 1;

    IF v_sci_id IS NULL THEN
        RAISE NOTICE 'Aucune SCI trouvee. Lance d''abord l''app pour creer le bootstrap.';
        RETURN;
    END IF;

    RAISE NOTICE 'SCI trouvee : % (entity %)', v_sci_id, v_legal_entity_id;

    -- Propriete
    SELECT id INTO v_property_id FROM properties
    WHERE legal_entity_id = v_legal_entity_id AND name = 'Immeuble Rue de la Paix' LIMIT 1;

    IF v_property_id IS NULL THEN
        INSERT INTO properties (sci_id, legal_entity_id, name, address, acquisition_date, acquisition_cents)
        VALUES (v_sci_id, v_legal_entity_id, 'Immeuble Rue de la Paix', '12 rue de la Paix, 75002 Paris', '2020-06-15', 45000000)
        RETURNING id INTO v_property_id;
        RAISE NOTICE 'Propriete creee : %', v_property_id;
    ELSE
        RAISE NOTICE 'Propriete existante : %', v_property_id;
    END IF;

    -- Lot A
    SELECT id INTO v_unit_a_id FROM units WHERE property_id = v_property_id AND code = 'LOCAL-A' LIMIT 1;
    IF v_unit_a_id IS NULL THEN
        INSERT INTO units (property_id, legal_entity_id, code, label, unit_type, area_m2, base_rent_cents, vat_rate_bp)
        VALUES (v_property_id, v_legal_entity_id, 'LOCAL-A', 'Local commercial A', 'COMMERCIAL', 45.0, 80000, 2000)
        RETURNING id INTO v_unit_a_id;
        RAISE NOTICE 'Lot A cree : %', v_unit_a_id;
    ELSE
        RAISE NOTICE 'Lot A existant : %', v_unit_a_id;
    END IF;

    -- Lot B
    SELECT id INTO v_unit_b_id FROM units WHERE property_id = v_property_id AND code = 'LOCAL-B' LIMIT 1;
    IF v_unit_b_id IS NULL THEN
        INSERT INTO units (property_id, legal_entity_id, code, label, unit_type, area_m2, base_rent_cents, vat_rate_bp)
        VALUES (v_property_id, v_legal_entity_id, 'LOCAL-B', 'Local commercial B', 'COMMERCIAL', 60.0, 120000, 2000)
        RETURNING id INTO v_unit_b_id;
        RAISE NOTICE 'Lot B cree : %', v_unit_b_id;
    ELSE
        RAISE NOTICE 'Lot B existant : %', v_unit_b_id;
    END IF;

    -- Locataire 1
    SELECT id INTO v_tenant_1_id FROM tenants WHERE legal_entity_id = v_legal_entity_id AND legal_name = 'test' LIMIT 1;
    IF v_tenant_1_id IS NULL THEN
        INSERT INTO tenants (sci_id, legal_entity_id, legal_name, siret, contact_email, contact_phone)
        VALUES (v_sci_id, v_legal_entity_id, 'test', '12345678900011', 'test@example.com', '0612345678')
        RETURNING id INTO v_tenant_1_id;
        RAISE NOTICE 'Locataire test cree : %', v_tenant_1_id;
    ELSE
        RAISE NOTICE 'Locataire test existant : %', v_tenant_1_id;
    END IF;

    -- Locataire 2
    SELECT id INTO v_tenant_2_id FROM tenants WHERE legal_entity_id = v_legal_entity_id AND legal_name = 'Locataire Test Sprint2' LIMIT 1;
    IF v_tenant_2_id IS NULL THEN
        INSERT INTO tenants (sci_id, legal_entity_id, legal_name, siret, contact_email, contact_phone)
        VALUES (v_sci_id, v_legal_entity_id, 'Locataire Test Sprint2', '98765432100022', 'test2@example.com', '0687654321')
        RETURNING id INTO v_tenant_2_id;
        RAISE NOTICE 'Locataire Sprint2 cree : %', v_tenant_2_id;
    ELSE
        RAISE NOTICE 'Locataire Sprint2 existant : %', v_tenant_2_id;
    END IF;

    -- Bail A
    IF NOT EXISTS (SELECT 1 FROM leases WHERE unit_id = v_unit_a_id AND legal_entity_id = v_legal_entity_id) THEN
        INSERT INTO leases (legal_entity_id, unit_id, tenant_id, reference, start_date, end_date, notice_months, payment_day, annual_review_month, active, signature_date, lease_type, rent_amount_cents, rent_frequency, vat_mode, charges_mode)
        VALUES (v_legal_entity_id, v_unit_a_id, v_tenant_1_id, 'BAIL-2024-A', '2024-01-01', '2027-01-01', 3, 5, 1, true, '2023-12-15', 'BAIL_COMMERCIAL', 80000, 'MONTHLY', 'FROM_UNIT', 'NONE');
        RAISE NOTICE 'Bail A cree.';
    ELSE
        RAISE NOTICE 'Bail A existant.';
    END IF;

    -- Bail B
    IF NOT EXISTS (SELECT 1 FROM leases WHERE unit_id = v_unit_b_id AND legal_entity_id = v_legal_entity_id) THEN
        INSERT INTO leases (legal_entity_id, unit_id, tenant_id, reference, start_date, end_date, notice_months, payment_day, annual_review_month, active, signature_date, lease_type, rent_amount_cents, rent_frequency, vat_mode, charges_mode)
        VALUES (v_legal_entity_id, v_unit_b_id, v_tenant_2_id, 'BAIL-2024-B', '2024-03-01', '2027-03-01', 3, 5, 1, true, '2024-02-15', 'BAIL_COMMERCIAL', 120000, 'MONTHLY', 'FROM_UNIT', 'NONE');
        RAISE NOTICE 'Bail B cree.';
    ELSE
        RAISE NOTICE 'Bail B existant.';
    END IF;

    RAISE NOTICE '=== Seed termine ===';
END $$;
'@

$utf8 = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($sqlFile, $sql, $utf8)

Write-Host "Fichier SQL cree : $sqlFile" -ForegroundColor Cyan
Write-Host "Execution dans le conteneur..." -ForegroundColor Yellow

docker cp $sqlFile sci-family-postgres:/tmp/seed-test-data.sql | Out-Null
docker exec -i sci-family-postgres psql -U sci -d sci_family -f /tmp/seed-test-data.sql

Write-Host ""
Write-Host "Verification :" -ForegroundColor Cyan
docker exec -i sci-family-postgres psql -U sci -d sci_family -c "SELECT 'legal_entities' AS t, COUNT(*) FROM legal_entities UNION ALL SELECT 'scis', COUNT(*) FROM scis UNION ALL SELECT 'properties', COUNT(*) FROM properties UNION ALL SELECT 'units', COUNT(*) FROM units UNION ALL SELECT 'tenants', COUNT(*) FROM tenants UNION ALL SELECT 'leases', COUNT(*) FROM leases;"

Remove-Item $sqlFile -Force -ErrorAction SilentlyContinue