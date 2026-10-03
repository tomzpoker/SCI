-- ============================================================
-- Sprint 14 — Taxe foncière (refonte)
-- 1 avis TF = 1 bien. Minimum 2 feuillets (BASES + FEES).
-- ============================================================

BEGIN;

CREATE TABLE IF NOT EXISTS property_tax_notices (
    id                       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id          UUID NOT NULL,
    property_id              UUID NOT NULL,
    fiscal_year              INTEGER NOT NULL,
    notice_reference         TEXT,
    total_amount_cents       BIGINT,
    cotisations_amount_cents BIGINT,
    management_fees_cents    BIGINT,
    status                   TEXT NOT NULL DEFAULT 'AWAITING_DOCS'
                             CHECK (status IN ('AWAITING_DOCS','AWAITING_REVIEW','READY','INVOICED','ARCHIVED')),
    notes                    TEXT,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (legal_entity_id, property_id, fiscal_year)
);

CREATE TABLE IF NOT EXISTS property_tax_notice_documents (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    notice_id        UUID NOT NULL REFERENCES property_tax_notices(id) ON DELETE CASCADE,
    document_kind    TEXT NOT NULL CHECK (document_kind IN ('BASES','FEES','OTHER')),
    file_name        TEXT NOT NULL,
    storage_key      TEXT,
    ocr_raw_text     TEXT,
    ocr_extracted    JSONB,
    ocr_confidence   INTEGER,
    uploaded_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS property_tax_notice_addresses (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    notice_id         UUID NOT NULL REFERENCES property_tax_notices(id) ON DELETE CASCADE,
    address_label     TEXT NOT NULL,
    base_amount_cents BIGINT NOT NULL DEFAULT 0,
    tax_amount_cents  BIGINT NOT NULL DEFAULT 0,
    display_order     INTEGER NOT NULL DEFAULT 0,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS property_tax_notice_lines (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    address_id           UUID NOT NULL REFERENCES property_tax_notice_addresses(id) ON DELETE CASCADE,
    unit_id              UUID NOT NULL,
    share_bp             INTEGER NOT NULL DEFAULT 10000
                         CHECK (share_bp >= 0 AND share_bp <= 10000),
    amount_cents         BIGINT NOT NULL DEFAULT 0,
    is_vacant            BOOLEAN NOT NULL DEFAULT false,
    dolibarr_invoice_id  BIGINT,
    dolibarr_invoice_ref TEXT,
    status               TEXT NOT NULL DEFAULT 'pending'
                         CHECK (status IN ('pending','invoiced','failed','skipped_vacant')),
    error_message        TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (address_id, unit_id)
);

CREATE TABLE IF NOT EXISTS property_tax_notice_fees (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    notice_id         UUID NOT NULL REFERENCES property_tax_notices(id) ON DELETE CASCADE,
    fee_label         TEXT NOT NULL,
    fee_amount_cents  BIGINT NOT NULL,
    distribution_mode TEXT NOT NULL DEFAULT 'PRORATA_TAX'
                      CHECK (distribution_mode IN ('PRORATA_TAX','PRORATA_UNITS','MANUAL')),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ptn_property    ON property_tax_notices(property_id);
CREATE INDEX IF NOT EXISTS idx_ptnd_notice     ON property_tax_notice_documents(notice_id);
CREATE INDEX IF NOT EXISTS idx_ptna_notice     ON property_tax_notice_addresses(notice_id);
CREATE INDEX IF NOT EXISTS idx_ptnl_addr       ON property_tax_notice_lines(address_id);
CREATE INDEX IF NOT EXISTS idx_ptnf_notice     ON property_tax_notice_fees(notice_id);

CREATE OR REPLACE FUNCTION trg_ptn_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_ptn_set_updated_at ON property_tax_notices;
CREATE TRIGGER trg_ptn_set_updated_at
BEFORE UPDATE ON property_tax_notices
FOR EACH ROW EXECUTE FUNCTION trg_ptn_updated_at();

COMMIT;