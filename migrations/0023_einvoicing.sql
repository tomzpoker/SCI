-- S13 / E-FACTURATION — cœur indépendant des fournisseurs, adapters PDP,
-- réception/émission, statuts, accusés, erreurs, e-reporting et remplacement.
-- Migration additive et non destructive.

CREATE TABLE IF NOT EXISTS einvoice_providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    provider_code TEXT NOT NULL,
    provider_name TEXT NOT NULL,
    adapter_kind TEXT NOT NULL DEFAULT 'PDP_ADAPTER' CHECK (adapter_kind IN ('PDP_ADAPTER','LOCAL_ADAPTER','DISABLED')),
    format_code TEXT NOT NULL DEFAULT 'FACTUR_X',
    endpoint_reference TEXT NOT NULL DEFAULT '',
    credential_reference TEXT NOT NULL DEFAULT '',
    capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
    active BOOLEAN NOT NULL DEFAULT false,
    effective_from DATE NOT NULL DEFAULT CURRENT_DATE,
    effective_to DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(provider_code) <> ''),
    CHECK (btrim(provider_name) <> ''),
    CHECK (effective_to IS NULL OR effective_to >= effective_from),
    UNIQUE(legal_entity_id,provider_code,effective_from)
);
CREATE INDEX IF NOT EXISTS idx_einvoice_providers_scope_s13
    ON einvoice_providers(legal_entity_id,active,effective_from DESC,provider_code);

CREATE TABLE IF NOT EXISTS einvoice_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    invoice_id UUID,
    provider_code TEXT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('OUTBOUND','INBOUND')),
    event_type TEXT NOT NULL CHECK (event_type IN ('PREPARED','SUBMITTED','ACKNOWLEDGED','DELIVERED','RECEIVED','REJECTED','ERROR','CANCELLED','REPORT_PREPARED')),
    external_id TEXT NOT NULL DEFAULT '',
    status_code TEXT NOT NULL DEFAULT '',
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    error_code TEXT NOT NULL DEFAULT '',
    error_message TEXT NOT NULL DEFAULT '',
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_einvoice_events_scope_s13
    ON einvoice_events(legal_entity_id,invoice_id,occurred_at DESC,id DESC);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_einvoice_event_invoice_entity_s13') THEN
        ALTER TABLE einvoice_events ADD CONSTRAINT fk_einvoice_event_invoice_entity_s13
            FOREIGN KEY(invoice_id,legal_entity_id) REFERENCES invoices(id,legal_entity_id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS einvoice_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    invoice_id UUID,
    provider_code TEXT NOT NULL,
    flow TEXT NOT NULL CHECK (flow IN ('ISSUED','RECEIVED')),
    format_code TEXT NOT NULL,
    external_id TEXT NOT NULL DEFAULT '',
    payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    content_hash TEXT NOT NULL DEFAULT '',
    received_at TIMESTAMPTZ,
    emitted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_einvoice_documents_scope_s13
    ON einvoice_documents(legal_entity_id,invoice_id,created_at DESC);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_einvoice_document_invoice_entity_s13') THEN
        ALTER TABLE einvoice_documents ADD CONSTRAINT fk_einvoice_document_invoice_entity_s13
            FOREIGN KEY(invoice_id,legal_entity_id) REFERENCES invoices(id,legal_entity_id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS einvoice_ereporting (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    provider_code TEXT NOT NULL,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    report_type TEXT NOT NULL CHECK (report_type IN ('TRANSACTION','PAYMENT','VAT_EVENT','OTHER')),
    status TEXT NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT','READY','SUBMITTED','ACKNOWLEDGED','REJECTED','ERROR')),
    payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    external_id TEXT NOT NULL DEFAULT '',
    error_code TEXT NOT NULL DEFAULT '',
    error_message TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_einvoice_ereporting_scope_s13
    ON einvoice_ereporting(legal_entity_id,period_end DESC,status,report_type);

DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM pg_constraint WHERE conname='chk_invoices_electronic_status_s07') THEN
        ALTER TABLE invoices DROP CONSTRAINT chk_invoices_electronic_status_s07;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='chk_invoices_electronic_status_s13') THEN
        ALTER TABLE invoices ADD CONSTRAINT chk_invoices_electronic_status_s13
            CHECK (electronic_status IN ('NOT_READY','READY','QUEUED','SENT','RECEIVED','ACKNOWLEDGED','DELIVERED','REJECTED','ERROR','NOT_APPLICABLE'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_invoices_einvoice_external_s13
    ON invoices(legal_entity_id,electronic_external_id);

COMMENT ON TABLE einvoice_providers IS 'Configuration remplaçable des fournisseurs e-facture; le cœur facture ne dépend d''aucun prestataire.';
COMMENT ON TABLE einvoice_events IS 'Historique réception/émission, accusés et erreurs e-facturation.';
COMMENT ON TABLE einvoice_documents IS 'Enveloppes structurées e-facture conservées avec leur hash et fournisseur.';
COMMENT ON TABLE einvoice_ereporting IS 'Préparations e-reporting par période et type.';
