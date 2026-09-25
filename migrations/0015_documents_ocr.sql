-- S05 — Documents & OCR
-- Files physiques remain outside PostgreSQL. The database stores metadata, paths,
-- hashes, statuses and auditable OCR/classification data only.

CREATE TABLE IF NOT EXISTS document_folder_catalog (
    code TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT true
);

INSERT INTO document_folder_catalog(code,label,relative_path) VALUES
 ('SOCIETE','Société','01_SOCIETE'),
 ('ASSOCIES','Associés','02_ASSOCIES'),
 ('IMMEUBLES','Immeubles','03_IMMEUBLES'),
 ('BAUX','Baux','04_BAUX'),
 ('LOCATAIRES','Locataires','05_LOCATAIRES'),
 ('FACTURES','Factures','06_FACTURES'),
 ('BANQUE','Banque','07_BANQUE'),
 ('FISCALITE','Fiscalité','08_FISCALITE'),
 ('TAXES','Taxes','09_TAXES'),
 ('TRAVAUX','Travaux','11_TRAVAUX'),
 ('JURIDIQUE','Juridique','12_JURIDIQUE'),
 ('ARCHIVES','Archives','99_ARCHIVES')
ON CONFLICT (code) DO NOTHING;

CREATE TABLE IF NOT EXISTS document_storage_config (
    legal_entity_id UUID PRIMARY KEY REFERENCES legal_entities(id) ON DELETE CASCADE,
    root_path TEXT NOT NULL,
    folder_overrides JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO document_storage_config(legal_entity_id,root_path)
SELECT id, 'documents' FROM legal_entities
ON CONFLICT (legal_entity_id) DO NOTHING;

ALTER TABLE documents ADD COLUMN IF NOT EXISTS origin TEXT NOT NULL DEFAULT 'MANUAL';
ALTER TABLE documents ADD COLUMN IF NOT EXISTS file_size_bytes BIGINT NOT NULL DEFAULT 0;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS mime_type TEXT NOT NULL DEFAULT 'application/octet-stream';
ALTER TABLE documents ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'IMPORTED';
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_status TEXT NOT NULL DEFAULT 'NOT_STARTED';
ALTER TABLE documents ADD COLUMN IF NOT EXISTS classification_confidence NUMERIC(6,5) NOT NULL DEFAULT 0;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS classification_reason TEXT NOT NULL DEFAULT '';
ALTER TABLE documents ADD COLUMN IF NOT EXISTS extraction_confidence NUMERIC(6,5) NOT NULL DEFAULT 0;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS duplicate_status TEXT NOT NULL DEFAULT 'NEW';
ALTER TABLE documents ADD COLUMN IF NOT EXISTS duplicate_of UUID REFERENCES documents(id);
ALTER TABLE documents ADD COLUMN IF NOT EXISTS validated_at TIMESTAMPTZ;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS validated_by TEXT;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS archived_at TIMESTAMPTZ;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS archive_reason TEXT;

CREATE INDEX IF NOT EXISTS idx_documents_hash ON documents(legal_entity_id,content_hash) WHERE content_hash IS NOT NULL AND content_hash<>'';
CREATE INDEX IF NOT EXISTS idx_documents_status ON documents(legal_entity_id,status,ocr_status);
CREATE INDEX IF NOT EXISTS idx_documents_duplicates ON documents(legal_entity_id,duplicate_status);

CREATE TABLE IF NOT EXISTS document_ocr_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    document_id UUID NOT NULL REFERENCES documents(id),
    engine TEXT NOT NULL,
    status TEXT NOT NULL,
    mean_confidence NUMERIC(6,5) NOT NULL DEFAULT 0,
    extracted_text TEXT NOT NULL DEFAULT '',
    error_message TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_document_ocr_runs_document ON document_ocr_runs(legal_entity_id,document_id,created_at DESC);

CREATE TABLE IF NOT EXISTS document_extractions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    document_id UUID NOT NULL REFERENCES documents(id),
    schema_version INTEGER NOT NULL DEFAULT 1,
    extracted_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    confidence NUMERIC(6,5) NOT NULL DEFAULT 0,
    validation_required BOOLEAN NOT NULL DEFAULT true,
    validated_at TIMESTAMPTZ,
    validated_by TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_document_extractions_document ON document_extractions(legal_entity_id,document_id,created_at DESC);

CREATE TABLE IF NOT EXISTS document_quality_checks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    document_id UUID NOT NULL REFERENCES documents(id),
    check_code TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'INFO',
    status TEXT NOT NULL,
    message TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_document_quality_checks ON document_quality_checks(legal_entity_id,document_id,created_at DESC);

CREATE TABLE IF NOT EXISTS document_duplicate_candidates (
    document_id UUID NOT NULL REFERENCES documents(id),
    candidate_document_id UUID NOT NULL REFERENCES documents(id),
    relation TEXT NOT NULL,
    confidence NUMERIC(6,5) NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY(document_id,candidate_document_id),
    CHECK(document_id<>candidate_document_id),
    CHECK(relation IN ('CERTAIN','PROBABLE','SIMILAR'))
);

CREATE TABLE IF NOT EXISTS document_events (
    id BIGSERIAL PRIMARY KEY,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    document_id UUID NOT NULL REFERENCES documents(id),
    event_type TEXT NOT NULL,
    actor TEXT NOT NULL DEFAULT 'SYSTEM',
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_document_events_document ON document_events(legal_entity_id,document_id,occurred_at DESC);

COMMENT ON TABLE documents IS 'Métadonnées documentaires uniquement; contenu physique hors BDD.';
COMMENT ON TABLE document_events IS 'Journal documentaire; aucune suppression silencieuse.';
