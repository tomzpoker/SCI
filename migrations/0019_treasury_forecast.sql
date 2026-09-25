-- S09 — TRÉSORERIE
-- Prévisions, flux récurrents, matérialisation d'événements et scénarios d'investissement.
-- Migration additive et non destructive.

CREATE TABLE IF NOT EXISTS treasury_recurring_patterns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    pattern_key TEXT NOT NULL,
    label_pattern TEXT NOT NULL,
    counterparty TEXT NOT NULL DEFAULT '',
    direction TEXT NOT NULL CHECK (direction IN ('IN','OUT')),
    classification TEXT NOT NULL CHECK (classification IN ('FIXED','VARIABLE','SEASONAL','PUNCTUAL','PROBABLY_RECURRING','UNKNOWN')),
    average_amount_cents BIGINT NOT NULL,
    min_amount_cents BIGINT NOT NULL,
    max_amount_cents BIGINT NOT NULL,
    average_gap_days NUMERIC(12,2),
    confidence_bp INTEGER NOT NULL DEFAULT 0 CHECK (confidence_bp BETWEEN 0 AND 10000),
    occurrence_count INTEGER NOT NULL DEFAULT 0,
    seasonal_months JSONB NOT NULL DEFAULT '[]'::jsonb,
    last_seen_date DATE,
    next_expected_date DATE,
    active BOOLEAN NOT NULL DEFAULT true,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id,pattern_key)
);
CREATE INDEX IF NOT EXISTS idx_treasury_recurring_entity_s09 ON treasury_recurring_patterns(legal_entity_id,active,classification);

CREATE TABLE IF NOT EXISTS treasury_forecast_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    forecast_date DATE NOT NULL,
    label TEXT NOT NULL,
    amount_cents BIGINT NOT NULL,
    qualification TEXT NOT NULL CHECK (qualification IN ('CERTAIN','PROBABLE','HYPOTHESIS','SCENARIO')),
    state TEXT NOT NULL DEFAULT 'SCHEDULED' CHECK (state IN ('SCHEDULED','PENDING','AWAITING_BANK_MATCH','MATCHED','COMPLETED','CANCELLED')),
    source_type TEXT NOT NULL,
    source_id UUID,
    recurrence_id UUID,
    matched_bank_transaction_id UUID,
    materialization_key TEXT,
    notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_treasury_events_entity_date_s09 ON treasury_forecast_events(legal_entity_id,forecast_date,state);
CREATE UNIQUE INDEX IF NOT EXISTS uq_treasury_events_materialization_s09
    ON treasury_forecast_events(legal_entity_id,materialization_key)
    WHERE materialization_key IS NOT NULL AND btrim(materialization_key)<>'';
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_treasury_event_recurrence_s09') THEN
        ALTER TABLE treasury_forecast_events ADD CONSTRAINT fk_treasury_event_recurrence_s09
            FOREIGN KEY (recurrence_id) REFERENCES treasury_recurring_patterns(id) ON DELETE SET NULL;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_treasury_event_bank_tx_s09') THEN
        ALTER TABLE treasury_forecast_events ADD CONSTRAINT fk_treasury_event_bank_tx_s09
            FOREIGN KEY (matched_bank_transaction_id) REFERENCES bank_transactions(id) ON DELETE SET NULL;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS treasury_forecast_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    scenario_code TEXT NOT NULL DEFAULT 'BASE',
    horizon_months INTEGER NOT NULL CHECK (horizon_months IN (1,2,3,6,9,12,24,36,60,120)),
    generated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    as_of_date DATE NOT NULL,
    min_balance_cents BIGINT NOT NULL,
    max_balance_cents BIGINT NOT NULL,
    ending_balance_cents BIGINT NOT NULL,
    points JSONB NOT NULL DEFAULT '[]'::jsonb,
    UNIQUE(legal_entity_id,scenario_code,horizon_months,as_of_date)
);
CREATE INDEX IF NOT EXISTS idx_treasury_snapshots_entity_s09 ON treasury_forecast_snapshots(legal_entity_id,generated_at DESC);

CREATE TABLE IF NOT EXISTS treasury_investments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    category TEXT NOT NULL CHECK (category IN ('TRAVAUX','EQUIPEMENT','RENOVATION','COPROPRIETE','SECURITE','MISE_AUX_NORMES','RENOUVELLEMENT','AUTRE')),
    label TEXT NOT NULL,
    planned_date DATE NOT NULL,
    amount_cents BIGINT NOT NULL CHECK (amount_cents >= 0),
    probability_bp INTEGER NOT NULL DEFAULT 5000 CHECK (probability_bp BETWEEN 0 AND 10000),
    vat_rate_bp INTEGER NOT NULL DEFAULT 0 CHECK (vat_rate_bp BETWEEN 0 AND 10000),
    fiscal_impact_cents BIGINT NOT NULL DEFAULT 0,
    rental_impact_cents BIGINT NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'PLANNED' CHECK (status IN ('PLANNED','CONFIRMED','CANCELLED')),
    notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_treasury_investments_entity_s09 ON treasury_investments(legal_entity_id,status,planned_date);

CREATE TABLE IF NOT EXISTS treasury_investment_scenarios (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    scenario_probability_bp INTEGER NOT NULL DEFAULT 5000 CHECK (scenario_probability_bp BETWEEN 0 AND 10000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id,code)
);

CREATE TABLE IF NOT EXISTS treasury_investment_scenario_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    scenario_id UUID NOT NULL,
    investment_id UUID NOT NULL,
    override_date DATE,
    override_amount_cents BIGINT,
    override_probability_bp INTEGER CHECK (override_probability_bp BETWEEN 0 AND 10000),
    notes TEXT NOT NULL DEFAULT '',
    UNIQUE(legal_entity_id,scenario_id,investment_id)
);
CREATE INDEX IF NOT EXISTS idx_treasury_scenario_items_s09 ON treasury_investment_scenario_items(legal_entity_id,scenario_id);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_treasury_scenario_item_scenario_s09') THEN
        ALTER TABLE treasury_investment_scenario_items ADD CONSTRAINT fk_treasury_scenario_item_scenario_s09
            FOREIGN KEY (scenario_id) REFERENCES treasury_investment_scenarios(id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_treasury_scenario_item_investment_s09') THEN
        ALTER TABLE treasury_investment_scenario_items ADD CONSTRAINT fk_treasury_scenario_item_investment_s09
            FOREIGN KEY (investment_id) REFERENCES treasury_investments(id) ON DELETE RESTRICT;
    END IF;
END $$;

COMMENT ON TABLE treasury_forecast_events IS 'Événements prévisionnels distincts des encaissements/paiements réels.';
COMMENT ON TABLE treasury_forecast_snapshots IS 'Prévisions à horizons multiples, historisées et reproductibles.';
COMMENT ON TABLE treasury_investments IS 'Investissements futurs identifiés avec probabilité, TVA, fiscalité et impact locatif.';
