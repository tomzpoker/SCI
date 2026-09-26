-- ============================================================
-- 0004_storage_locations.sql
-- Stockage documentaire configurable
--
-- Migration additive :
-- - ne modifie pas 0001_foundation.sql
-- - conserve documents.storage_key
-- - ne casse aucun document existant
-- - permet progressivement de rattacher un document
--   à un emplacement de stockage
-- ============================================================

CREATE TABLE storage_locations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    sci_id UUID NOT NULL
        REFERENCES scis(id)
        ON DELETE CASCADE,

    name TEXT NOT NULL,

    storage_type TEXT NOT NULL DEFAULT 'LOCAL',

    configuration JSONB NOT NULL DEFAULT '{}'::jsonb,

    is_active BOOLEAN NOT NULL DEFAULT TRUE,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT storage_locations_name_not_empty
        CHECK (btrim(name) <> ''),

    CONSTRAINT storage_locations_type_not_empty
        CHECK (btrim(storage_type) <> '')
);

CREATE INDEX idx_storage_locations_sci_id
    ON storage_locations(sci_id);

CREATE INDEX idx_storage_locations_sci_active
    ON storage_locations(sci_id, is_active);

ALTER TABLE documents
    ADD COLUMN storage_location_id UUID
        REFERENCES storage_locations(id)
        ON DELETE SET NULL;

CREATE INDEX idx_documents_storage_location_id
    ON documents(storage_location_id);