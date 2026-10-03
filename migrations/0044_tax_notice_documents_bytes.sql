BEGIN;

ALTER TABLE property_tax_notice_documents
    ADD COLUMN IF NOT EXISTS file_bytes      BYTEA,
    ADD COLUMN IF NOT EXISTS mime_type       TEXT,
    ADD COLUMN IF NOT EXISTS dolibarr_ecm_id BIGINT;

COMMIT;