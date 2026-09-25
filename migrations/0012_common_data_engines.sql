-- S04 — Moteurs communs de données et gestion.
-- Migration additive et non destructive.

CREATE TABLE IF NOT EXISTS financial_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    occurred_at TIMESTAMPTZ NOT NULL,
    amount_cents BIGINT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('IN','OUT')),
    source_type TEXT NOT NULL,
    source_id UUID,
    status TEXT NOT NULL DEFAULT 'RECORDED' CHECK (status IN ('RECORDED','PENDING','RECONCILED','CANCELLED')),
    external_key TEXT,
    counterparty TEXT NOT NULL DEFAULT '',
    label TEXT NOT NULL DEFAULT '',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(source_type) <> '')
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_financial_transactions_external
    ON financial_transactions(legal_entity_id, external_key)
    WHERE external_key IS NOT NULL AND btrim(external_key) <> '';
CREATE INDEX IF NOT EXISTS idx_financial_transactions_scope_date
    ON financial_transactions(legal_entity_id, occurred_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_financial_transactions_source
    ON financial_transactions(legal_entity_id, source_type, source_id);

CREATE TABLE IF NOT EXISTS business_event_types (
    code TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(code) <> '')
);

INSERT INTO business_event_types(code,label) VALUES
 ('LeaseCreated','Bail créé'),
 ('InvoiceIssued','Facture émise'),
 ('PaymentDetected','Paiement détecté'),
 ('StockReceived','Stock reçu'),
 ('StockSold','Stock vendu'),
 ('TaxDeadlineReached','Échéance fiscale atteinte'),
 ('PropertyCreated','Bien créé'),
 ('TenantCreated','Locataire créé'),
 ('DocumentRegistered','Document enregistré'),
 ('BankImported','Mouvement bancaire importé'),
 ('TaskCreated','Tâche créée')
ON CONFLICT (code) DO NOTHING;

CREATE TABLE IF NOT EXISTS business_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    event_type TEXT NOT NULL REFERENCES business_event_types(code),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    source_type TEXT NOT NULL,
    source_id UUID,
    idempotency_key TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'RAISED' CHECK (status IN ('RAISED','PROCESSED','FAILED')),
    processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(source_type) <> ''),
    CHECK (btrim(idempotency_key) <> '')
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_business_events_idempotency
    ON business_events(legal_entity_id, idempotency_key);
CREATE INDEX IF NOT EXISTS idx_business_events_scope
    ON business_events(legal_entity_id, occurred_at DESC, id DESC);

CREATE TABLE IF NOT EXISTS cash_position_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    as_of_date DATE NOT NULL,
    position_type TEXT NOT NULL CHECK (position_type IN ('REAL','RECONCILED','THEORETICAL','FORECAST')),
    amount_cents BIGINT NOT NULL,
    source_run_id UUID,
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id, as_of_date, position_type)
);
CREATE INDEX IF NOT EXISTS idx_cash_position_scope
    ON cash_position_snapshots(legal_entity_id, as_of_date DESC, position_type);

CREATE TABLE IF NOT EXISTS service_operations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    service_code TEXT NOT NULL,
    operation_code TEXT NOT NULL,
    request_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    validation_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    postcondition_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    risk_level TEXT NOT NULL DEFAULT 'MEDIUM' CHECK (risk_level IN ('LOW','MEDIUM','HIGH','CRITICAL')),
    status TEXT NOT NULL DEFAULT 'SUCCEEDED' CHECK (status IN ('STARTED','SUCCEEDED','FAILED')),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(service_code) <> ''),
    CHECK (btrim(operation_code) <> '')
);
CREATE INDEX IF NOT EXISTS idx_service_operations_scope
    ON service_operations(legal_entity_id, occurred_at DESC, id DESC);

COMMENT ON TABLE financial_transactions IS 'Flux financiers normalisés, communs à tous les profils juridiques.';
COMMENT ON TABLE business_events IS 'Événements métier standardisés et idempotents.';
COMMENT ON TABLE cash_position_snapshots IS 'Sépare explicitement réel, rapproché, théorique et prévisionnel.';
COMMENT ON TABLE service_operations IS 'Journal du passage par la frontière service métier : validation, transaction, post-condition.';
