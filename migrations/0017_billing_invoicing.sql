-- S07 / FACTURATION — génération, charges, régularisations, taxes récupérables,
-- avoirs, états, brouillons et préparation à la facturation électronique.
-- Migration additive et non destructive.

ALTER TABLE invoices ADD COLUMN IF NOT EXISTS document_kind TEXT NOT NULL DEFAULT 'INVOICE';
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS source_invoice_id UUID;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS billing_period_start DATE;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS billing_period_end DATE;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS issued_at TIMESTAMPTZ;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS pdf_generated_at TIMESTAMPTZ;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS pdf_draft_watermark BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS electronic_status TEXT NOT NULL DEFAULT 'NOT_READY';
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS electronic_provider_code TEXT;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS electronic_format TEXT;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS electronic_external_id TEXT;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS electronic_sent_at TIMESTAMPTZ;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS electronic_received_at TIMESTAMPTZ;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS generation_key TEXT;

UPDATE invoices SET status='PAID_PARTIAL' WHERE status='PARTIAL';
UPDATE invoices SET document_kind='INVOICE' WHERE document_kind IS NULL OR btrim(document_kind)='';

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='chk_invoices_status_s07') THEN
        ALTER TABLE invoices ADD CONSTRAINT chk_invoices_status_s07
            CHECK (status IN ('DRAFT','VALIDATED','ISSUED','PAID_PARTIAL','PAID','OVERDUE','CANCELLED','CREDITED'));
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='chk_invoices_document_kind_s07') THEN
        ALTER TABLE invoices ADD CONSTRAINT chk_invoices_document_kind_s07
            CHECK (document_kind IN ('INVOICE','CREDIT_NOTE'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='chk_invoices_electronic_status_s07') THEN
        ALTER TABLE invoices ADD CONSTRAINT chk_invoices_electronic_status_s07
            CHECK (electronic_status IN ('NOT_READY','READY','QUEUED','SENT','RECEIVED','REJECTED','NOT_APPLICABLE'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_invoices_entity_period_s07
    ON invoices(legal_entity_id, billing_period_start, billing_period_end, document_kind);
CREATE INDEX IF NOT EXISTS idx_invoices_entity_electronic_s07
    ON invoices(legal_entity_id, electronic_status, electronic_provider_code);
CREATE UNIQUE INDEX IF NOT EXISTS uq_invoices_entity_generation_key_s07
    ON invoices(legal_entity_id, generation_key)
    WHERE generation_key IS NOT NULL AND btrim(generation_key) <> '';

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_invoices_source_invoice_entity_s07') THEN
        ALTER TABLE invoices ADD CONSTRAINT fk_invoices_source_invoice_entity_s07
            FOREIGN KEY (source_invoice_id, legal_entity_id)
            REFERENCES invoices(id, legal_entity_id);
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS billing_invoice_lines (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    invoice_id UUID NOT NULL,
    line_no INTEGER NOT NULL CHECK (line_no > 0),
    line_type TEXT NOT NULL CHECK (line_type IN ('RENT','CHARGE','REGULARISATION','RECOVERABLE_TAX','OTHER','CREDIT')),
    description TEXT NOT NULL,
    quantity NUMERIC(18,6) NOT NULL DEFAULT 1,
    unit_net_cents BIGINT NOT NULL,
    net_cents BIGINT NOT NULL,
    vat_rate_bp INTEGER NOT NULL DEFAULT 0 CHECK (vat_rate_bp >= 0),
    vat_cents BIGINT NOT NULL DEFAULT 0,
    gross_cents BIGINT NOT NULL,
    recoverable_tax BOOLEAN NOT NULL DEFAULT false,
    service_period_start DATE,
    service_period_end DATE,
    source_type TEXT,
    source_id UUID,
    source_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id, invoice_id, line_no),
    CHECK (gross_cents = net_cents + vat_cents),
    CHECK (btrim(description) <> '')
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_billing_invoice_lines_id_entity_s07
    ON billing_invoice_lines(id, legal_entity_id);
CREATE INDEX IF NOT EXISTS idx_billing_invoice_lines_invoice_s07
    ON billing_invoice_lines(legal_entity_id, invoice_id, line_no);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_billing_lines_invoice_entity_s07') THEN
        ALTER TABLE billing_invoice_lines ADD CONSTRAINT fk_billing_lines_invoice_entity_s07
            FOREIGN KEY (invoice_id, legal_entity_id)
            REFERENCES invoices(id, legal_entity_id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS billing_invoice_state_history (
    id BIGSERIAL PRIMARY KEY,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    invoice_id UUID NOT NULL,
    from_status TEXT,
    to_status TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    actor TEXT NOT NULL DEFAULT 'MANAGER',
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_billing_invoice_state_history_s07
    ON billing_invoice_state_history(legal_entity_id, invoice_id, changed_at DESC);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_billing_invoice_state_invoice_entity_s07') THEN
        ALTER TABLE billing_invoice_state_history ADD CONSTRAINT fk_billing_invoice_state_invoice_entity_s07
            FOREIGN KEY (invoice_id, legal_entity_id)
            REFERENCES invoices(id, legal_entity_id) ON DELETE RESTRICT;
    END IF;
END $$;

INSERT INTO billing_invoice_state_history(legal_entity_id,invoice_id,from_status,to_status,reason)
SELECT i.legal_entity_id,i.id,NULL,i.status,'Reprise de l''historique S07'
FROM invoices i
WHERE NOT EXISTS (
    SELECT 1 FROM billing_invoice_state_history h WHERE h.legal_entity_id=i.legal_entity_id AND h.invoice_id=i.id
);

CREATE TABLE IF NOT EXISTS billing_charge_actuals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    lease_id UUID NOT NULL,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    charge_code TEXT NOT NULL,
    actual_cents BIGINT NOT NULL CHECK (actual_cents >= 0),
    source_reference TEXT NOT NULL DEFAULT '',
    assessed_at DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id, lease_id, period_start, period_end, charge_code)
);
CREATE INDEX IF NOT EXISTS idx_billing_charge_actuals_s07
    ON billing_charge_actuals(legal_entity_id, lease_id, period_start);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_billing_charge_actuals_lease_entity_s07') THEN
        ALTER TABLE billing_charge_actuals ADD CONSTRAINT fk_billing_charge_actuals_lease_entity_s07
            FOREIGN KEY (lease_id, legal_entity_id)
            REFERENCES leases(id, legal_entity_id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS billing_regularizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    lease_id UUID NOT NULL,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    charge_code TEXT NOT NULL,
    provision_cents BIGINT NOT NULL DEFAULT 0,
    actual_cents BIGINT NOT NULL DEFAULT 0,
    difference_cents BIGINT NOT NULL,
    treatment TEXT NOT NULL CHECK (treatment IN ('NO_DIFFERENCE','ADDITIONAL_INVOICE','CREDIT_NOTE')),
    invoice_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id, lease_id, period_start, period_end, charge_code)
);
CREATE INDEX IF NOT EXISTS idx_billing_regularizations_s07
    ON billing_regularizations(legal_entity_id, lease_id, period_start, treatment);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_billing_regularizations_lease_entity_s07') THEN
        ALTER TABLE billing_regularizations ADD CONSTRAINT fk_billing_regularizations_lease_entity_s07
            FOREIGN KEY (lease_id, legal_entity_id)
            REFERENCES leases(id, legal_entity_id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_billing_regularizations_invoice_entity_s07') THEN
        ALTER TABLE billing_regularizations ADD CONSTRAINT fk_billing_regularizations_invoice_entity_s07
            FOREIGN KEY (invoice_id, legal_entity_id)
            REFERENCES invoices(id, legal_entity_id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS lease_recoverable_taxes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    lease_id UUID NOT NULL,
    tax_code TEXT NOT NULL,
    label TEXT NOT NULL,
    legal_basis TEXT NOT NULL DEFAULT '',
    recovery_rate_bp INTEGER NOT NULL DEFAULT 10000 CHECK (recovery_rate_bp BETWEEN 0 AND 10000),
    effective_from DATE NOT NULL,
    effective_to DATE,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (effective_to IS NULL OR effective_to >= effective_from),
    UNIQUE(legal_entity_id, lease_id, tax_code, effective_from)
);
CREATE INDEX IF NOT EXISTS idx_lease_recoverable_taxes_s07
    ON lease_recoverable_taxes(legal_entity_id, lease_id, active, effective_from);
CREATE UNIQUE INDEX IF NOT EXISTS uq_lease_recoverable_taxes_id_entity_s07
    ON lease_recoverable_taxes(id, legal_entity_id);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_lease_recoverable_taxes_lease_entity_s07') THEN
        ALTER TABLE lease_recoverable_taxes ADD CONSTRAINT fk_lease_recoverable_taxes_lease_entity_s07
            FOREIGN KEY (lease_id, legal_entity_id)
            REFERENCES leases(id, legal_entity_id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS billing_tax_assessments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    lease_id UUID NOT NULL,
    tax_rule_id UUID NOT NULL,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    assessed_cents BIGINT NOT NULL CHECK (assessed_cents >= 0),
    assessed_at DATE NOT NULL,
    source_reference TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id, tax_rule_id, period_start, period_end)
);
CREATE INDEX IF NOT EXISTS idx_billing_tax_assessments_s07
    ON billing_tax_assessments(legal_entity_id, lease_id, period_start);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_billing_tax_assessments_lease_entity_s07') THEN
        ALTER TABLE billing_tax_assessments ADD CONSTRAINT fk_billing_tax_assessments_lease_entity_s07
            FOREIGN KEY (lease_id, legal_entity_id)
            REFERENCES leases(id, legal_entity_id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_billing_tax_assessments_rule_entity_s07') THEN
        ALTER TABLE billing_tax_assessments ADD CONSTRAINT fk_billing_tax_assessments_rule_entity_s07
            FOREIGN KEY (tax_rule_id, legal_entity_id)
            REFERENCES lease_recoverable_taxes(id, legal_entity_id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS billing_einvoice_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    provider_code TEXT NOT NULL,
    provider_name TEXT NOT NULL,
    format_code TEXT NOT NULL DEFAULT 'MIXED' CHECK (format_code IN ('UBL','CII','MIXED')),
    endpoint_reference TEXT NOT NULL DEFAULT '',
    credentials_reference TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT true,
    effective_from DATE NOT NULL DEFAULT CURRENT_DATE,
    effective_to DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (effective_to IS NULL OR effective_to >= effective_from),
    CHECK (btrim(provider_code) <> ''),
    CHECK (btrim(provider_name) <> '')
);
CREATE INDEX IF NOT EXISTS idx_billing_einvoice_connections_s07
    ON billing_einvoice_connections(legal_entity_id, active, effective_from DESC);
CREATE UNIQUE INDEX IF NOT EXISTS uq_billing_einvoice_connections_id_entity_s07
    ON billing_einvoice_connections(id, legal_entity_id);
CREATE UNIQUE INDEX IF NOT EXISTS uq_billing_einvoice_active_s07
    ON billing_einvoice_connections(legal_entity_id)
    WHERE active;

CREATE TABLE IF NOT EXISTS billing_einvoice_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    invoice_id UUID NOT NULL,
    connection_id UUID,
    direction TEXT NOT NULL CHECK (direction IN ('OUTBOUND','INBOUND')),
    event_type TEXT NOT NULL,
    provider_status TEXT NOT NULL DEFAULT '',
    external_id TEXT NOT NULL DEFAULT '',
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_billing_einvoice_events_s07
    ON billing_einvoice_events(legal_entity_id, invoice_id, occurred_at DESC);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_billing_einvoice_event_invoice_entity_s07') THEN
        ALTER TABLE billing_einvoice_events ADD CONSTRAINT fk_billing_einvoice_event_invoice_entity_s07
            FOREIGN KEY (invoice_id, legal_entity_id)
            REFERENCES invoices(id, legal_entity_id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_billing_einvoice_event_connection_entity_s07') THEN
        ALTER TABLE billing_einvoice_events ADD CONSTRAINT fk_billing_einvoice_event_connection_entity_s07
            FOREIGN KEY (connection_id, legal_entity_id) REFERENCES billing_einvoice_connections(id, legal_entity_id) ON DELETE SET NULL;
    END IF;
END $$;

COMMENT ON TABLE billing_einvoice_connections IS 'Abstraction de plateforme : le prestataire peut changer sans migrer les factures historiques ni dépendre d''un fournisseur unique.';
COMMENT ON COLUMN billing_einvoice_connections.credentials_reference IS 'Référence opaque vers un secret externe ; jamais le secret lui-même.';
COMMENT ON COLUMN invoices.generation_key IS 'Clé d''idempotence S07 pour éviter les doublons de génération de factures.';
COMMENT ON COLUMN invoices.pdf_draft_watermark IS 'TRUE si le PDF a été généré comme brouillon et porte la mention BROUILLON.';
