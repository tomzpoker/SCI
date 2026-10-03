BEGIN;

ALTER TABLE property_tax_notices
    ADD COLUMN IF NOT EXISTS dgfip_invoice_id  BIGINT,
    ADD COLUMN IF NOT EXISTS dgfip_invoice_ref TEXT;

COMMIT;