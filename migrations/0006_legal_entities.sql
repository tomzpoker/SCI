-- S02 / US-0201 — Registre canonique des entités juridiques.
-- Migration additive : aucune suppression de données ni d'objet existant.

CREATE TABLE IF NOT EXISTS legal_entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_name TEXT NOT NULL,
    legal_form_code TEXT NOT NULL CHECK (legal_form_code IN ('SCI', 'SARL')),
    tax_regime TEXT NOT NULL CHECK (tax_regime IN ('IR', 'IS')),
    vat_status TEXT NOT NULL DEFAULT 'NONE',
    vat_basis TEXT NOT NULL DEFAULT 'COLLECTION',
    siren TEXT,
    siret TEXT,
    registered_office TEXT NOT NULL DEFAULT '',
    accounting_period_start SMALLINT NOT NULL DEFAULT 1 CHECK (accounting_period_start BETWEEN 1 AND 12),
    fiscal_year_end SMALLINT NOT NULL DEFAULT 12 CHECK (fiscal_year_end BETWEEN 1 AND 12),
    currency_code CHAR(3) NOT NULL DEFAULT 'EUR',
    settings JSONB NOT NULL DEFAULT '{}'::jsonb,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_legal_entities_workspace_siren
    ON legal_entities(workspace_id, siren)
    WHERE siren IS NOT NULL AND siren <> '';

CREATE INDEX IF NOT EXISTS idx_legal_entities_workspace_active
    ON legal_entities(workspace_id, active, legal_name);

CREATE TABLE IF NOT EXISTS legal_entity_bank_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE CASCADE,
    label TEXT NOT NULL DEFAULT 'Compte principal',
    iban TEXT NOT NULL DEFAULT '',
    bic TEXT NOT NULL DEFAULT '',
    is_primary BOOLEAN NOT NULL DEFAULT false,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_legal_entity_bank_accounts_entity
    ON legal_entity_bank_accounts(legal_entity_id, active, is_primary DESC);

CREATE UNIQUE INDEX IF NOT EXISTS uq_legal_entity_primary_bank_account
    ON legal_entity_bank_accounts(legal_entity_id)
    WHERE is_primary AND active;

INSERT INTO legal_entities (
    id,
    workspace_id,
    legal_name,
    legal_form_code,
    tax_regime,
    vat_status,
    vat_basis,
    siren,
    siret,
    registered_office,
    accounting_period_start,
    fiscal_year_end,
    currency_code,
    active
)
SELECT
    s.id,
    s.workspace_id,
    s.legal_name,
    'SCI',
    CASE WHEN s.tax_regime IN ('IR', 'IS') THEN s.tax_regime ELSE 'IR' END,
    s.vat_status,
    s.vat_basis,
    s.siren,
    s.siret,
    COALESCE(s.registered_office, ''),
    s.accounting_period_start,
    s.fiscal_year_end,
    'EUR',
    true
FROM scis s
ON CONFLICT (id) DO NOTHING;

ALTER TABLE scis
    ADD COLUMN IF NOT EXISTS legal_entity_id UUID;

UPDATE scis
SET legal_entity_id = id
WHERE legal_entity_id IS NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_scis_legal_entity'
    ) THEN
        ALTER TABLE scis
            ADD CONSTRAINT fk_scis_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS uq_scis_legal_entity
    ON scis(legal_entity_id)
    WHERE legal_entity_id IS NOT NULL;

INSERT INTO legal_entity_bank_accounts (
    legal_entity_id,
    label,
    iban,
    bic,
    is_primary,
    active
)
SELECT
    s.id,
    'Compte principal',
    COALESCE(s.iban, ''),
    COALESCE(s.bic, ''),
    true,
    true
FROM scis s
WHERE COALESCE(s.iban, '') <> ''
  AND NOT EXISTS (
      SELECT 1
      FROM legal_entity_bank_accounts a
      WHERE a.legal_entity_id = s.id
        AND a.is_primary
        AND a.active
  );

ALTER TABLE audit_events
    ADD COLUMN IF NOT EXISTS legal_entity_id UUID;

UPDATE audit_events
SET legal_entity_id = sci_id
WHERE legal_entity_id IS NULL
  AND sci_id IS NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_audit_events_legal_entity'
    ) THEN
        ALTER TABLE audit_events
            ADD CONSTRAINT fk_audit_events_legal_entity
            FOREIGN KEY (legal_entity_id) REFERENCES legal_entities(id);
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_audit_events_legal_entity_time
    ON audit_events(legal_entity_id, occurred_at DESC);
