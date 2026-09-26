CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE workspaces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    timezone TEXT NOT NULL DEFAULT 'Europe/Paris',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE scis (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_name TEXT NOT NULL,
    siren TEXT,
    siret TEXT,
    tax_regime TEXT NOT NULL DEFAULT 'IR',
    vat_status TEXT NOT NULL DEFAULT 'OPTION_LOYERS',
    vat_basis TEXT NOT NULL DEFAULT 'COLLECTION',
    accounting_period_start SMALLINT NOT NULL DEFAULT 1 CHECK (accounting_period_start BETWEEN 1 AND 12),
    fiscal_year_end SMALLINT NOT NULL DEFAULT 12 CHECK (fiscal_year_end BETWEEN 1 AND 12),
    registered_office TEXT,
    iban TEXT,
    bic TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE associates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    display_name TEXT NOT NULL,
    tax_identifier TEXT,
    ownership_pct NUMERIC(7,4) NOT NULL CHECK (ownership_pct >= 0 AND ownership_pct <= 100),
    current_account_cents BIGINT NOT NULL DEFAULT 0,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE properties (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    address TEXT NOT NULL,
    acquisition_date DATE,
    acquisition_cents BIGINT,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE units (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    property_id UUID NOT NULL REFERENCES properties(id) ON DELETE CASCADE,
    code TEXT NOT NULL,
    label TEXT NOT NULL,
    unit_type TEXT NOT NULL DEFAULT 'COMMERCIAL',
    area_m2 NUMERIC(12,2),
    base_rent_cents BIGINT NOT NULL DEFAULT 0,
    vat_rate_bp INTEGER NOT NULL DEFAULT 2000,
    active BOOLEAN NOT NULL DEFAULT true,
    UNIQUE(property_id, code)
);

CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    legal_name TEXT NOT NULL,
    siret TEXT,
    contact_email TEXT,
    contact_phone TEXT,
    billing_address TEXT,
    active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE leases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    unit_id UUID NOT NULL REFERENCES units(id) ON DELETE RESTRICT,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    reference TEXT NOT NULL,
    start_date DATE NOT NULL,
    end_date DATE,
    notice_months INTEGER NOT NULL DEFAULT 3,
    indexation_rule TEXT,
    payment_day SMALLINT NOT NULL DEFAULT 5 CHECK (payment_day BETWEEN 1 AND 31),
    annual_review_month SMALLINT CHECK (annual_review_month BETWEEN 1 AND 12),
    active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE invoices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    lease_id UUID REFERENCES leases(id) ON DELETE SET NULL,
    invoice_number TEXT,
    issue_date DATE NOT NULL,
    service_period_start DATE,
    service_period_end DATE,
    due_date DATE NOT NULL,
    net_cents BIGINT NOT NULL,
    vat_cents BIGINT NOT NULL DEFAULT 0,
    gross_cents BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'DRAFT',
    pdf_path TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE payments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    invoice_id UUID REFERENCES invoices(id) ON DELETE SET NULL,
    received_at TIMESTAMPTZ NOT NULL,
    amount_cents BIGINT NOT NULL CHECK (amount_cents >= 0),
    reference TEXT,
    source TEXT NOT NULL DEFAULT 'BANK',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE bank_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    booked_at TIMESTAMPTZ NOT NULL,
    value_date DATE,
    amount_cents BIGINT NOT NULL,
    label TEXT NOT NULL,
    counterparty TEXT,
    external_id TEXT,
    reconciliation_status TEXT NOT NULL DEFAULT 'UNMATCHED',
    imported_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(sci_id, external_id)
);

CREATE TABLE automation_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    trigger_kind TEXT NOT NULL,
    schedule_expr TEXT,
    horizon_days INTEGER NOT NULL DEFAULT 90,
    priority INTEGER NOT NULL DEFAULT 50,
    auto_execute BOOLEAN NOT NULL DEFAULT false,
    enabled BOOLEAN NOT NULL DEFAULT true,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE(sci_id, code)
);

CREATE TABLE tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    automation_rule_id UUID REFERENCES automation_rules(id) ON DELETE SET NULL,
    code TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    due_at TIMESTAMPTZ NOT NULL,
    state TEXT NOT NULL DEFAULT 'PLANNED',
    priority INTEGER NOT NULL DEFAULT 50,
    source TEXT NOT NULL DEFAULT 'RULE_ENGINE',
    blocking BOOLEAN NOT NULL DEFAULT false,
    completed_at TIMESTAMPTZ,
    entity_type TEXT,
    entity_id UUID,
    occurrence_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(sci_id, code, occurrence_key)
);

CREATE TABLE tax_deadlines (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    code TEXT NOT NULL,
    label TEXT NOT NULL,
    deadline_date DATE NOT NULL,
    period_label TEXT,
    source_name TEXT,
    source_url TEXT,
    verified_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'PLANNED',
    UNIQUE(sci_id, code, deadline_date)
);

CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    category TEXT NOT NULL,
    title TEXT NOT NULL,
    file_name TEXT NOT NULL,
    storage_key TEXT NOT NULL,
    content_hash TEXT,
    document_date DATE,
    expires_at DATE,
    extracted_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE forecast_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    horizon_days INTEGER NOT NULL,
    expected_inflows_cents BIGINT NOT NULL DEFAULT 0,
    expected_outflows_cents BIGINT NOT NULL DEFAULT 0,
    expected_vat_due_cents BIGINT NOT NULL DEFAULT 0,
    expected_tax_cents BIGINT NOT NULL DEFAULT 0,
    minimum_cash_cents BIGINT NOT NULL DEFAULT 0,
    risk_level TEXT NOT NULL DEFAULT 'NORMAL',
    details JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE audit_events (
    id BIGSERIAL PRIMARY KEY,
    sci_id UUID REFERENCES scis(id) ON DELETE CASCADE,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    actor TEXT NOT NULL DEFAULT 'SYSTEM',
    action TEXT NOT NULL,
    entity_type TEXT,
    entity_id UUID,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE app_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(sci_id, key)
);

CREATE INDEX idx_tasks_due ON tasks(sci_id, state, due_at);
CREATE INDEX idx_tasks_occurrence ON tasks(sci_id, code, occurrence_key);
CREATE INDEX idx_payments_received ON payments(sci_id, received_at);
CREATE INDEX idx_bank_booked ON bank_transactions(sci_id, booked_at);
CREATE INDEX idx_deadlines_date ON tax_deadlines(sci_id, deadline_date);
CREATE INDEX idx_audit_time ON audit_events(sci_id, occurred_at DESC);

INSERT INTO workspaces (id, name) VALUES ('00000000-0000-0000-0000-000000000001', 'SCI familiale — espace principal');
INSERT INTO scis (id, workspace_id, legal_name, tax_regime, vat_status, vat_basis, registered_office)
VALUES ('00000000-0000-0000-0000-000000000010', '00000000-0000-0000-0000-000000000001', 'SCI À CONFIGURER', 'IR', 'OPTION_LOYERS', 'COLLECTION', 'À configurer');

INSERT INTO automation_rules (sci_id, code, name, description, trigger_kind, horizon_days, priority, auto_execute, config) VALUES
('00000000-0000-0000-0000-000000000010', 'RENT_INVOICE', 'Préparer les loyers', 'Prépare les brouillons de factures locatives avant l échéance prévue.', 'SCHEDULE', 45, 80, false, '{"lead_days":7}'),
('00000000-0000-0000-0000-000000000010', 'PAYMENT_RECONCILIATION', 'Rapprocher les encaissements', 'Propose automatiquement le rapprochement entre banque et factures ouvertes.', 'EVENT', 30, 90, false, '{"tolerance_cents":2}'),
('00000000-0000-0000-0000-000000000010', 'VAT_COLLECTION', 'Préparer la TVA sur encaissements', 'Calcule la TVA exigible à partir des encaissements effectivement identifiés.', 'PERIOD_END', 90, 100, false, '{"basis":"collection"}'),
('00000000-0000-0000-0000-000000000010', 'ANNUAL_CLOSE', 'Préparer la clôture', 'Constitue la checklist de clôture, pièces manquantes et contrôles de cohérence.', 'ANNUAL', 180, 95, false, '{"check_documents":true}'),
('00000000-0000-0000-0000-000000000010', 'LEASE_REVIEW', 'Anticiper les baux', 'Détecte les dates de révision, échéances et préavis des baux.', 'DAILY', 180, 70, false, '{"lead_days":120}'),
('00000000-0000-0000-0000-000000000010', 'INSURANCE_EXPIRY', 'Surveiller les assurances', 'Crée une alerte avant expiration des attestations et contrats.', 'DAILY', 180, 70, false, '{"lead_days":45}');
