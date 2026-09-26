-- S06 — Baux / Locations
-- Extension progressive of the existing lease model. Historical lease identity is preserved.

ALTER TABLE leases ADD COLUMN IF NOT EXISTS signature_date DATE;
ALTER TABLE leases ADD COLUMN IF NOT EXISTS lease_type TEXT NOT NULL DEFAULT 'BAIL_COMMERCIAL';
ALTER TABLE leases ADD COLUMN IF NOT EXISTS destination TEXT NOT NULL DEFAULT '';
ALTER TABLE leases ADD COLUMN IF NOT EXISTS rent_amount_cents BIGINT NOT NULL DEFAULT 0;
ALTER TABLE leases ADD COLUMN IF NOT EXISTS rent_frequency TEXT NOT NULL DEFAULT 'MONTHLY';
ALTER TABLE leases ADD COLUMN IF NOT EXISTS vat_mode TEXT NOT NULL DEFAULT 'FROM_UNIT';
ALTER TABLE leases ADD COLUMN IF NOT EXISTS index_code TEXT;
ALTER TABLE leases ADD COLUMN IF NOT EXISTS index_base_value NUMERIC(14,6);
ALTER TABLE leases ADD COLUMN IF NOT EXISTS index_base_date DATE;
ALTER TABLE leases ADD COLUMN IF NOT EXISTS index_cap_bp INTEGER;
ALTER TABLE leases ADD COLUMN IF NOT EXISTS charges_mode TEXT NOT NULL DEFAULT 'NONE';
ALTER TABLE leases ADD COLUMN IF NOT EXISTS charges_amount_cents BIGINT NOT NULL DEFAULT 0;
ALTER TABLE leases ADD COLUMN IF NOT EXISTS security_deposit_expected_cents BIGINT NOT NULL DEFAULT 0;
ALTER TABLE leases ADD COLUMN IF NOT EXISTS entry_fee_expected_cents BIGINT NOT NULL DEFAULT 0;
ALTER TABLE leases ADD COLUMN IF NOT EXISTS entry_fee_status TEXT NOT NULL DEFAULT 'NOT_SET';
ALTER TABLE leases ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

UPDATE leases l
SET rent_amount_cents = CASE WHEN l.rent_amount_cents=0 THEN COALESCE((SELECT u.base_rent_cents FROM units u WHERE u.id=l.unit_id),0) ELSE l.rent_amount_cents END
WHERE l.rent_amount_cents=0;

CREATE TABLE IF NOT EXISTS lease_clauses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    lease_id UUID NOT NULL REFERENCES leases(id),
    code TEXT NOT NULL,
    title TEXT NOT NULL,
    clause_type TEXT NOT NULL,
    body TEXT NOT NULL,
    effective_from DATE NOT NULL,
    effective_to DATE,
    version_no INTEGER NOT NULL DEFAULT 1,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(lease_id,code,version_no)
);

CREATE TABLE IF NOT EXISTS lease_index_values (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    index_code TEXT NOT NULL,
    period_label TEXT NOT NULL,
    value NUMERIC(14,6) NOT NULL,
    source_reference TEXT NOT NULL DEFAULT '',
    verified_at DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id,index_code,period_label)
);
CREATE INDEX IF NOT EXISTS idx_lease_indices_lookup ON lease_index_values(legal_entity_id,index_code,period_label);

CREATE TABLE IF NOT EXISTS lease_rent_revisions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    lease_id UUID NOT NULL REFERENCES leases(id),
    calculation_date DATE NOT NULL,
    effective_date DATE NOT NULL,
    rule_text TEXT NOT NULL,
    index_code TEXT,
    index_period TEXT,
    old_rent_cents BIGINT NOT NULL,
    index_old NUMERIC(14,6),
    index_new NUMERIC(14,6),
    cap_bp INTEGER,
    new_rent_cents BIGINT NOT NULL,
    formula TEXT NOT NULL,
    result_status TEXT NOT NULL DEFAULT 'CALCULATED',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS lease_charge_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    lease_id UUID NOT NULL REFERENCES leases(id),
    charge_type TEXT NOT NULL,
    mode TEXT NOT NULL,
    amount_cents BIGINT NOT NULL DEFAULT 0,
    variable_formula TEXT NOT NULL DEFAULT '',
    effective_from DATE NOT NULL,
    effective_to DATE,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS lease_rent_reductions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    lease_id UUID NOT NULL REFERENCES leases(id),
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    amount_cents BIGINT,
    percentage_bp INTEGER,
    reason TEXT NOT NULL,
    original_rent_cents BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK(end_date>=start_date),
    CHECK((amount_cents IS NOT NULL) OR (percentage_bp IS NOT NULL)),
    CHECK((percentage_bp IS NULL) OR (percentage_bp BETWEEN 0 AND 10000))
);

CREATE TABLE IF NOT EXISTS lease_deposits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    lease_id UUID NOT NULL REFERENCES leases(id),
    movement_type TEXT NOT NULL,
    amount_cents BIGINT NOT NULL CHECK(amount_cents>=0),
    movement_date DATE NOT NULL,
    justification TEXT NOT NULL DEFAULT '',
    bank_transaction_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK(movement_type IN ('EXPECTED','RECEIVED','RESTITUTION','RETAINED','ADJUSTMENT'))
);

CREATE TABLE IF NOT EXISTS lease_guarantees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    lease_id UUID NOT NULL REFERENCES leases(id),
    guarantee_type TEXT NOT NULL,
    guarantor_name TEXT NOT NULL,
    amount_cents BIGINT,
    start_date DATE,
    end_date DATE,
    document_id UUID REFERENCES documents(id),
    notes TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS lease_entry_fees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    lease_id UUID NOT NULL REFERENCES leases(id),
    expected_amount_cents BIGINT NOT NULL DEFAULT 0,
    qualification_status TEXT NOT NULL DEFAULT 'UNCERTAIN',
    justification TEXT NOT NULL DEFAULT '',
    alert_required BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS lease_contract_events (
    id BIGSERIAL PRIMARY KEY,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    lease_id UUID NOT NULL REFERENCES leases(id),
    event_type TEXT NOT NULL,
    effective_date DATE NOT NULL,
    actor TEXT NOT NULL DEFAULT 'MANAGER',
    before_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    after_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    reason TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_lease_clauses ON lease_clauses(legal_entity_id,lease_id,active);
CREATE INDEX IF NOT EXISTS idx_lease_charges ON lease_charge_rules(legal_entity_id,lease_id,active);
CREATE INDEX IF NOT EXISTS idx_lease_reductions ON lease_rent_reductions(legal_entity_id,lease_id,start_date,end_date);
CREATE INDEX IF NOT EXISTS idx_lease_deposits ON lease_deposits(legal_entity_id,lease_id,movement_date);
CREATE INDEX IF NOT EXISTS idx_lease_events ON lease_contract_events(legal_entity_id,lease_id,effective_date,created_at);

COMMENT ON TABLE lease_index_values IS 'Historique des indices ICC/ILC et autres séries nécessaires aux révisions.';
COMMENT ON TABLE lease_rent_reductions IS 'Réductions temporaires contractuelles; le loyer historique n’est pas écrasé.';
COMMENT ON TABLE lease_entry_fees IS 'Qualification potentielle du droit d’entrée; les cas incertains déclenchent une alerte.';

CREATE UNIQUE INDEX IF NOT EXISTS ux_lease_entry_fee_per_lease ON lease_entry_fees(legal_entity_id,lease_id);
