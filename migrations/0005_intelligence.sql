-- 0005_intelligence.sql
CREATE TABLE IF NOT EXISTS llm_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    name TEXT NOT NULL, base_url TEXT NOT NULL, model TEXT NOT NULL, api_key TEXT,
    is_free BOOLEAN NOT NULL DEFAULT TRUE, supports_voice BOOLEAN NOT NULL DEFAULT TRUE, enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), UNIQUE(sci_id,name)
);
CREATE TABLE IF NOT EXISTS email_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    label TEXT NOT NULL, address TEXT NOT NULL, imap_host TEXT NOT NULL, imap_port INTEGER NOT NULL DEFAULT 993 CHECK(imap_port BETWEEN 1 AND 65535),
    username TEXT NOT NULL, password TEXT NOT NULL, mailbox TEXT NOT NULL DEFAULT 'INBOX', tls BOOLEAN NOT NULL DEFAULT TRUE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE, auto_scan BOOLEAN NOT NULL DEFAULT TRUE, last_scan_at TIMESTAMPTZ, last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), UNIQUE(sci_id,address,imap_host,mailbox)
);
CREATE TABLE IF NOT EXISTS document_inbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL, source_ref TEXT NOT NULL, filename TEXT NOT NULL, content_type TEXT NOT NULL DEFAULT 'application/octet-stream',
    content_hash TEXT NOT NULL, staging_path TEXT NOT NULL, sender TEXT NOT NULL DEFAULT '', subject TEXT NOT NULL DEFAULT '', received_at TEXT NOT NULL DEFAULT '',
    document_date DATE, document_type TEXT NOT NULL DEFAULT 'A_CLASSER', classification_confidence NUMERIC(6,5) NOT NULL DEFAULT 0,
    classification_reasons JSONB NOT NULL DEFAULT '[]'::jsonb, ocr_text TEXT NOT NULL DEFAULT '', extracted_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    suggested_title TEXT NOT NULL, suggested_storage_key TEXT NOT NULL, selected_storage_location_id UUID REFERENCES storage_locations(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'PENDING', error_message TEXT, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(sci_id,source_type,source_ref,filename)
);
CREATE TABLE IF NOT EXISTS assistant_proposals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), sci_id UUID NOT NULL REFERENCES scis(id) ON DELETE CASCADE,
    action_type TEXT NOT NULL, payload JSONB NOT NULL DEFAULT '{}'::jsonb, explanation TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'PENDING',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(), decided_at TIMESTAMPTZ, UNIQUE(sci_id,id)
);
CREATE INDEX IF NOT EXISTS idx_llm_accounts_sci_enabled ON llm_accounts(sci_id,enabled,is_free);
CREATE INDEX IF NOT EXISTS idx_email_accounts_sci_enabled ON email_accounts(sci_id,enabled,auto_scan);
CREATE INDEX IF NOT EXISTS idx_document_inbox_sci_status ON document_inbox(sci_id,status,created_at DESC);
CREATE INDEX IF NOT EXISTS idx_document_inbox_hash ON document_inbox(sci_id,content_hash);
CREATE INDEX IF NOT EXISTS idx_assistant_proposals_sci_status ON assistant_proposals(sci_id,status,created_at DESC);
