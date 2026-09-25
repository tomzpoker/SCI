-- S10 / FISCALITE — TVA, declarations, resultat fiscal, 2072, quotes-parts, taxes.
-- Additif, versionne, sans suppression de donnees.

CREATE TABLE IF NOT EXISTS vat_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    entry_kind TEXT NOT NULL CHECK (entry_kind IN ('COLLECTED','DEDUCTIBLE','CORRECTION')),
    exigibility_basis TEXT NOT NULL CHECK (exigibility_basis IN ('COLLECTION','INVOICE','ADJUSTMENT','UNKNOWN')),
    transaction_date DATE NOT NULL,
    taxable_net_cents BIGINT NOT NULL DEFAULT 0,
    vat_rate_bp INTEGER NOT NULL DEFAULT 0 CHECK (vat_rate_bp BETWEEN 0 AND 10000),
    vat_cents BIGINT NOT NULL,
    source_type TEXT NOT NULL,
    source_id UUID,
    source_document_id UUID,
    source_label TEXT NOT NULL DEFAULT '',
    idempotency_key TEXT,
    notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(source_type) <> '')
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_vat_entries_idempotency_s10
    ON vat_entries(legal_entity_id,idempotency_key)
    WHERE idempotency_key IS NOT NULL AND btrim(idempotency_key)<>'';
CREATE INDEX IF NOT EXISTS idx_vat_entries_period_s10
    ON vat_entries(legal_entity_id,period_start,period_end,entry_kind,transaction_date);

CREATE TABLE IF NOT EXISTS vat_advances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    payment_date DATE NOT NULL,
    taxable_base_cents BIGINT NOT NULL DEFAULT 0,
    vat_rate_bp INTEGER NOT NULL DEFAULT 0 CHECK (vat_rate_bp BETWEEN 0 AND 10000),
    vat_cents BIGINT NOT NULL CHECK (vat_cents >= 0),
    source_type TEXT NOT NULL,
    source_id UUID,
    source_document_id UUID,
    status TEXT NOT NULL DEFAULT 'DOCUMENTED' CHECK (status IN ('EXPECTED','DOCUMENTED','PAID','CANCELLED')),
    notes TEXT NOT NULL DEFAULT '',
    idempotency_key TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(source_type) <> '')
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_vat_advances_idempotency_s10
    ON vat_advances(legal_entity_id,idempotency_key)
    WHERE idempotency_key IS NOT NULL AND btrim(idempotency_key)<>'';
CREATE INDEX IF NOT EXISTS idx_vat_advances_period_s10
    ON vat_advances(legal_entity_id,payment_date,period_start,period_end,status);

CREATE TABLE IF NOT EXISTS vat_declarations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    form_code TEXT NOT NULL DEFAULT '3310-CA3',
    form_millesime INTEGER NOT NULL DEFAULT 2026,
    status TEXT NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT','VALIDATION_REQUIRED','VALIDATED','FINAL','BLOCKED')),
    ca_ht_cents BIGINT NOT NULL DEFAULT 0,
    collected_vat_cents BIGINT NOT NULL DEFAULT 0,
    deductible_vat_cents BIGINT NOT NULL DEFAULT 0,
    payable_vat_cents BIGINT NOT NULL DEFAULT 0,
    credit_vat_cents BIGINT NOT NULL DEFAULT 0,
    corrections_vat_cents BIGINT NOT NULL DEFAULT 0,
    anomalies JSONB NOT NULL DEFAULT '[]'::jsonb,
    validation_request_id UUID,
    validated_at TIMESTAMPTZ,
    validated_by TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id,period_start,period_end,form_code)
);
CREATE INDEX IF NOT EXISTS idx_vat_declarations_scope_s10
    ON vat_declarations(legal_entity_id,period_end DESC,status);

CREATE TABLE IF NOT EXISTS vat_declaration_fields (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    declaration_id UUID NOT NULL,
    field_code TEXT NOT NULL,
    label TEXT NOT NULL,
    value_cents BIGINT,
    value_text TEXT,
    source_type TEXT NOT NULL DEFAULT '',
    source_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    formula TEXT NOT NULL DEFAULT '',
    validation_status TEXT NOT NULL DEFAULT 'PENDING' CHECK (validation_status IN ('PENDING','OK','WARNING','ERROR')),
    validation_message TEXT NOT NULL DEFAULT '',
    UNIQUE(legal_entity_id,declaration_id,field_code)
);
CREATE INDEX IF NOT EXISTS idx_vat_declaration_fields_s10
    ON vat_declaration_fields(legal_entity_id,declaration_id,field_code);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_vat_declaration_fields_decl_entity_s10') THEN
        ALTER TABLE vat_declaration_fields ADD CONSTRAINT fk_vat_declaration_fields_decl_entity_s10
            FOREIGN KEY(declaration_id) REFERENCES vat_declarations(id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS fiscal_form_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID,
    form_code TEXT NOT NULL,
    millesime INTEGER NOT NULL,
    tax_year INTEGER NOT NULL,
    valid_from DATE NOT NULL,
    valid_to DATE,
    source_name TEXT NOT NULL,
    source_reference TEXT NOT NULL,
    fields JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL DEFAULT 'PUBLISHED' CHECK (status IN ('DRAFT','PUBLISHED','RETIRED')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id,form_code,millesime)
);
CREATE INDEX IF NOT EXISTS idx_fiscal_form_versions_lookup_s10
    ON fiscal_form_versions(form_code,millesime,tax_year,status,valid_from DESC);

INSERT INTO fiscal_form_versions(legal_entity_id,form_code,millesime,tax_year,valid_from,source_name,source_reference,fields)
SELECT e.id,'2072-S',2026,2025,DATE '2026-01-01','impots.gouv.fr','2072-S-SD millesime 2026',
       '[{"code":"R1","label":"Revenus bruts"},{"code":"R2","label":"Paiement sur travaux"},{"code":"R3","label":"Frais et charges autres qu''intérêts d''emprunts"},{"code":"R4","label":"Intérêts d''emprunts"},{"code":"R5","label":"Revenu net ou déficit net"},{"code":"R6","label":"Produits financiers"},{"code":"R7","label":"Produits exceptionnels"},{"code":"R8","label":"Charges exceptionnelles"}]'::jsonb
FROM legal_entities e
WHERE NOT EXISTS (SELECT 1 FROM fiscal_form_versions f WHERE f.legal_entity_id=e.id AND f.form_code='2072-S' AND f.millesime=2026);

INSERT INTO fiscal_form_versions(legal_entity_id,form_code,millesime,tax_year,valid_from,source_name,source_reference,fields)
SELECT e.id,'2072-S-A1',2026,2025,DATE '2026-01-01','impots.gouv.fr','2072-S-A1-SD millesime 2026',
       '[{"code":"1","label":"Fermages ou loyers encaissés"},{"code":"2","label":"Dépenses mises à la charge des locataires"},{"code":"3","label":"Recettes brutes diverses"},{"code":"4","label":"Jouissance gratuite"},{"code":"5","label":"Total des recettes"},{"code":"6","label":"Administration et gestion"},{"code":"7","label":"Forfait gestion"},{"code":"8","label":"Assurances"},{"code":"9","label":"Réparation entretien amélioration"},{"code":"9bis","label":"Rénovation énergétique"},{"code":"10","label":"Charges récupérables non récupérées"},{"code":"11","label":"Indemnités d''éviction"},{"code":"12","label":"Impositions"},{"code":"13","label":"Provisions copropriété"},{"code":"14","label":"Régularisation provisions copropriété"},{"code":"15","label":"Déduction spécifique"},{"code":"16","label":"Total déductions"},{"code":"17","label":"Intérêts d''emprunts"},{"code":"18","label":"Revenu ou déficit de l''immeuble"},{"code":"19","label":"Réintégration"},{"code":"20","label":"Rémunérations et avantages associés"},{"code":"21","label":"Revenu net ou déficit"},{"code":"22","label":"Revenus/déficits autres SCI"},{"code":"23","label":"Revenu net à répartir"}]'::jsonb
FROM legal_entities e
WHERE NOT EXISTS (SELECT 1 FROM fiscal_form_versions f WHERE f.legal_entity_id=e.id AND f.form_code='2072-S-A1' AND f.millesime=2026);

CREATE TABLE IF NOT EXISTS associate_share_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    associate_id UUID NOT NULL,
    effective_from DATE NOT NULL,
    effective_to DATE,
    parts_count NUMERIC(18,6) NOT NULL DEFAULT 0 CHECK (parts_count >= 0),
    ownership_pct NUMERIC(9,6) NOT NULL CHECK (ownership_pct >= 0 AND ownership_pct <= 100),
    event_type TEXT NOT NULL DEFAULT 'SNAPSHOT',
    source_document_id UUID,
    notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (effective_to IS NULL OR effective_to >= effective_from),
    UNIQUE(legal_entity_id,associate_id,effective_from)
);
CREATE INDEX IF NOT EXISTS idx_associate_share_history_scope_s10
    ON associate_share_history(legal_entity_id,associate_id,effective_from DESC);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_associate_share_history_associate_entity_s10') THEN
        ALTER TABLE associate_share_history ADD CONSTRAINT fk_associate_share_history_associate_entity_s10
            FOREIGN KEY(associate_id) REFERENCES associates(id) ON DELETE RESTRICT;
    END IF;
END $$;
INSERT INTO associate_share_history(legal_entity_id,associate_id,effective_from,parts_count,ownership_pct,event_type,notes)
SELECT a.legal_entity_id,a.id,a.created_at::date,0,a.ownership_pct,'INITIAL_SNAPSHOT','Créé à partir de la fiche associé existante; vérifier les changements antérieurs.'
FROM associates a
WHERE NOT EXISTS (SELECT 1 FROM associate_share_history h WHERE h.legal_entity_id=a.legal_entity_id AND h.associate_id=a.id);

CREATE TABLE IF NOT EXISTS fiscal_2072_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    tax_year INTEGER NOT NULL,
    form_version_id UUID NOT NULL,
    status TEXT NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT','VALIDATION_REQUIRED','VALIDATED','FINAL','SIMULATION')),
    data JSONB NOT NULL DEFAULT '{}'::jsonb,
    anomalies JSONB NOT NULL DEFAULT '[]'::jsonb,
    validation_request_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id,tax_year,form_version_id)
);
CREATE INDEX IF NOT EXISTS idx_fiscal_2072_runs_scope_s10
    ON fiscal_2072_runs(legal_entity_id,tax_year DESC,status);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_fiscal_2072_form_entity_s10') THEN
        ALTER TABLE fiscal_2072_runs ADD CONSTRAINT fk_fiscal_2072_form_entity_s10
            FOREIGN KEY(form_version_id) REFERENCES fiscal_form_versions(id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS fiscal_dossiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    run_id UUID NOT NULL,
    tax_year INTEGER NOT NULL,
    quote_parts JSONB NOT NULL DEFAULT '[]'::jsonb,
    revenues_cents BIGINT NOT NULL DEFAULT 0,
    charges_cents BIGINT NOT NULL DEFAULT 0,
    result_cents BIGINT NOT NULL DEFAULT 0,
    document_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    history JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT','VALIDATION_REQUIRED','VALIDATED','FINAL','SIMULATION')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id,tax_year,run_id)
);
CREATE INDEX IF NOT EXISTS idx_fiscal_dossiers_scope_s10 ON fiscal_dossiers(legal_entity_id,tax_year DESC,status);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_fiscal_dossier_run_entity_s10') THEN
        ALTER TABLE fiscal_dossiers ADD CONSTRAINT fk_fiscal_dossier_run_entity_s10
            FOREIGN KEY(run_id) REFERENCES fiscal_2072_runs(id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS fiscal_2072_allocations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    run_id UUID NOT NULL,
    associate_id UUID NOT NULL,
    ownership_pct NUMERIC(9,6) NOT NULL,
    amount_cents BIGINT NOT NULL,
    formula TEXT NOT NULL,
    coverage_start DATE NOT NULL,
    coverage_end DATE NOT NULL,
    source_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    UNIQUE(legal_entity_id,run_id,associate_id,coverage_start,coverage_end)
);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_fiscal_2072_alloc_run_entity_s10') THEN
        ALTER TABLE fiscal_2072_allocations ADD CONSTRAINT fk_fiscal_2072_alloc_run_entity_s10
            FOREIGN KEY(run_id) REFERENCES fiscal_2072_runs(id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS fiscal_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    management_income_cents BIGINT NOT NULL DEFAULT 0,
    accounting_result_cents BIGINT NOT NULL DEFAULT 0,
    fiscal_result_cents BIGINT NOT NULL DEFAULT 0,
    treasury_result_cents BIGINT NOT NULL DEFAULT 0,
    deductions_cents BIGINT NOT NULL DEFAULT 0,
    reintegrations_cents BIGINT NOT NULL DEFAULT 0,
    anomalies JSONB NOT NULL DEFAULT '[]'::jsonb,
    calculation_snapshot JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id,period_start,period_end)
);

CREATE TABLE IF NOT EXISTS fiscal_simulations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    simulation_code TEXT NOT NULL,
    label TEXT NOT NULL,
    as_of_date DATE NOT NULL,
    inputs JSONB NOT NULL DEFAULT '{}'::jsonb,
    result JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_fiscal_simulations_scope_s10 ON fiscal_simulations(legal_entity_id,as_of_date DESC);

CREATE TABLE IF NOT EXISTS fiscal_tax_obligations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    tax_code TEXT NOT NULL,
    label TEXT NOT NULL,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    applicability TEXT NOT NULL CHECK (applicability IN ('APPLICABLE','POTENTIALLY_APPLICABLE','NOT_APPLICABLE','TO_QUALIFY')),
    qualification_needed BOOLEAN NOT NULL DEFAULT false,
    due_date DATE,
    amount_cents BIGINT,
    source_reference TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_fiscal_tax_obligations_scope_s10 ON fiscal_tax_obligations(legal_entity_id,period_end DESC,applicability,due_date);

COMMENT ON TABLE vat_declaration_fields IS 'Chaque valeur TVA conserve sa source, ses pièces, sa formule et son statut de validation.';
COMMENT ON TABLE fiscal_form_versions IS 'Définitions de formulaires versionnées; une nouvelle version est ajoutée, jamais écrasée.';
COMMENT ON TABLE associate_share_history IS 'Historique des droits détenus par associé, nécessaire aux répartitions temporelles.';
COMMENT ON TABLE fiscal_simulations IS 'Scénarios explicitement séparés des résultats déclaratifs réels.';

COMMENT ON TABLE vat_advances IS 'Acomptes TVA documentes et rattaches a une periode, sans confondre paiement et exigibilite.';
