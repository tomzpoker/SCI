-- S04 / US-0407 — Idempotence transverse des jobs et opérations métier.
-- Migration additive et non destructive.

CREATE TABLE IF NOT EXISTS job_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    job_code TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'STARTED' CHECK (status IN ('STARTED','SUCCEEDED','FAILED')),
    attempt_count INTEGER NOT NULL DEFAULT 1 CHECK (attempt_count > 0),
    result_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    error_message TEXT NOT NULL DEFAULT '',
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(job_code) <> ''),
    CHECK (btrim(idempotency_key) <> ''),
    CHECK (btrim(request_hash) <> '')
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_job_executions_scope_key
    ON job_executions(legal_entity_id, job_code, idempotency_key);
CREATE INDEX IF NOT EXISTS idx_job_executions_scope_status
    ON job_executions(legal_entity_id, status, updated_at DESC);

ALTER TABLE invoices ADD COLUMN IF NOT EXISTS idempotency_key TEXT;
ALTER TABLE payments ADD COLUMN IF NOT EXISTS idempotency_key TEXT;

UPDATE invoices SET idempotency_key=COALESCE(NULLIF(idempotency_key,''),'legacy-invoice:'||id::text)
WHERE idempotency_key IS NULL OR btrim(idempotency_key)='';
UPDATE payments SET idempotency_key=COALESCE(NULLIF(idempotency_key,''),'legacy-payment:'||id::text)
WHERE idempotency_key IS NULL OR btrim(idempotency_key)='';

CREATE UNIQUE INDEX IF NOT EXISTS uq_invoices_entity_idempotency
    ON invoices(legal_entity_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL AND btrim(idempotency_key) <> '';
CREATE UNIQUE INDEX IF NOT EXISTS uq_payments_entity_idempotency
    ON payments(legal_entity_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL AND btrim(idempotency_key) <> '';
