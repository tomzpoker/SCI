-- S05 — Workflow + Validation + Autonomie.
-- Migration additive et non destructive.

CREATE TABLE IF NOT EXISTS workflow_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    trigger_kind TEXT NOT NULL,
    timeout_seconds INTEGER NOT NULL DEFAULT 3600 CHECK (timeout_seconds BETWEEN 1 AND 604800),
    output_schema JSONB NOT NULL DEFAULT '{}'::jsonb,
    enabled BOOLEAN NOT NULL DEFAULT true,
    version_no INTEGER NOT NULL DEFAULT 1 CHECK (version_no > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id, code, version_no),
    CHECK (btrim(code) <> ''),
    CHECK (btrim(name) <> ''),
    CHECK (btrim(trigger_kind) <> '')
);

CREATE TABLE IF NOT EXISTS workflow_steps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_definition_id UUID NOT NULL REFERENCES workflow_definitions(id) ON DELETE RESTRICT,
    step_no INTEGER NOT NULL CHECK (step_no > 0),
    code TEXT NOT NULL,
    action_kind TEXT NOT NULL,
    retry_limit INTEGER NOT NULL DEFAULT 3 CHECK (retry_limit BETWEEN 0 AND 100),
    timeout_seconds INTEGER NOT NULL DEFAULT 900 CHECK (timeout_seconds BETWEEN 1 AND 604800),
    requires_validation BOOLEAN NOT NULL DEFAULT false,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE(workflow_definition_id, step_no),
    UNIQUE(workflow_definition_id, code),
    CHECK (btrim(code) <> ''),
    CHECK (btrim(action_kind) <> '')
);

CREATE TABLE IF NOT EXISTS workflow_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    workflow_definition_id UUID NOT NULL REFERENCES workflow_definitions(id) ON DELETE RESTRICT,
    idempotency_key TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'RUNNING' CHECK (status IN ('RUNNING','WAITING_VALIDATION','SUCCEEDED','FAILED','CANCELLED')),
    current_step_no INTEGER NOT NULL DEFAULT 1,
    input_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    output_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    error_message TEXT NOT NULL DEFAULT '',
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id, idempotency_key)
);

CREATE TABLE IF NOT EXISTS automation_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    activity_code TEXT NOT NULL DEFAULT '',
    object_type TEXT NOT NULL DEFAULT '',
    risk_level TEXT NOT NULL DEFAULT 'MEDIUM' CHECK (risk_level IN ('LOW','MEDIUM','HIGH','CRITICAL')),
    automation_level SMALLINT NOT NULL CHECK (automation_level BETWEEN 0 AND 5),
    validation_required BOOLEAN NOT NULL DEFAULT true,
    enabled BOOLEAN NOT NULL DEFAULT true,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id, activity_code, object_type, risk_level)
);

CREATE TABLE IF NOT EXISTS validation_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    workflow_run_id UUID REFERENCES workflow_runs(id) ON DELETE RESTRICT,
    action_code TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id UUID,
    before_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    proposed_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    reason TEXT NOT NULL DEFAULT '',
    source_payload JSONB NOT NULL DEFAULT '[]'::jsonb,
    documents_payload JSONB NOT NULL DEFAULT '[]'::jsonb,
    expected_result JSONB NOT NULL DEFAULT '{}'::jsonb,
    risk_level TEXT NOT NULL DEFAULT 'MEDIUM' CHECK (risk_level IN ('LOW','MEDIUM','HIGH','CRITICAL')),
    status TEXT NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING','APPROVED','REJECTED','EXPIRED')),
    requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    reviewed_at TIMESTAMPTZ,
    reviewed_by TEXT,
    review_reason TEXT NOT NULL DEFAULT '',
    UNIQUE(workflow_run_id, action_code),
    CHECK (btrim(action_code) <> ''),
    CHECK (btrim(subject_type) <> '')
);
CREATE INDEX IF NOT EXISTS idx_validation_requests_inbox
    ON validation_requests(legal_entity_id, status, risk_level, requested_at DESC);

CREATE TABLE IF NOT EXISTS postcondition_checks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    validation_request_id UUID REFERENCES validation_requests(id) ON DELETE RESTRICT,
    workflow_run_id UUID REFERENCES workflow_runs(id) ON DELETE RESTRICT,
    check_code TEXT NOT NULL,
    expected_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    actual_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING','MATCHED','MISMATCH','ERROR')),
    checked_at TIMESTAMPTZ,
    error_message TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(check_code) <> '')
);
CREATE INDEX IF NOT EXISTS idx_postcondition_scope
    ON postcondition_checks(legal_entity_id, status, created_at DESC);

COMMENT ON TABLE job_executions IS 'Clé d''idempotence transverse : une même opération peut être reprise sans duplication.';
COMMENT ON TABLE workflow_definitions IS 'Définitions versionnées de workflows communs.';
COMMENT ON TABLE workflow_runs IS 'Exécutions idempotentes et reprenables des workflows.';
COMMENT ON TABLE automation_policies IS 'Niveau 0..5 par entité, activité, type et risque.';
COMMENT ON TABLE validation_requests IS 'Inbox de validation avec avant/après, sources, documents et résultat attendu.';
COMMENT ON TABLE postcondition_checks IS 'Vérification du résultat réel contre le résultat attendu.';

-- Un workflow minimal est disponible pour chaque entité existante, sans inventer de règles métier.
INSERT INTO workflow_definitions(workspace_id,legal_entity_id,code,name,trigger_kind,timeout_seconds,output_schema)
SELECT e.workspace_id,e.id,'ADMIN_ACTION','Action administrative générique','MANUAL',3600,'{}'::jsonb
FROM legal_entities e
WHERE NOT EXISTS (
    SELECT 1 FROM workflow_definitions w
    WHERE w.legal_entity_id=e.id AND w.code='ADMIN_ACTION' AND w.version_no=1
);
INSERT INTO workflow_steps(workflow_definition_id,step_no,code,action_kind,retry_limit,timeout_seconds,requires_validation,config)
SELECT w.id,1,'VALIDATE','SERVICE_VALIDATION',3,900,true,'{}'::jsonb
FROM workflow_definitions w
WHERE w.code='ADMIN_ACTION' AND w.version_no=1
  AND NOT EXISTS (SELECT 1 FROM workflow_steps s WHERE s.workflow_definition_id=w.id AND s.step_no=1);
INSERT INTO workflow_steps(workflow_definition_id,step_no,code,action_kind,retry_limit,timeout_seconds,requires_validation,config)
SELECT w.id,2,'EXECUTE','SERVICE_EXECUTION',3,1800,false,'{}'::jsonb
FROM workflow_definitions w
WHERE w.code='ADMIN_ACTION' AND w.version_no=1
  AND NOT EXISTS (SELECT 1 FROM workflow_steps s WHERE s.workflow_definition_id=w.id AND s.step_no=2);
INSERT INTO workflow_steps(workflow_definition_id,step_no,code,action_kind,retry_limit,timeout_seconds,requires_validation,config)
SELECT w.id,3,'POSTCONDITION','POSTCONDITION_CHECK',2,600,false,'{}'::jsonb
FROM workflow_definitions w
WHERE w.code='ADMIN_ACTION' AND w.version_no=1
  AND NOT EXISTS (SELECT 1 FROM workflow_steps s WHERE s.workflow_definition_id=w.id AND s.step_no=3);

COMMENT ON COLUMN invoices.idempotency_key IS 'Identité de création/reprise d''une facture ; même opération = même clé.';
COMMENT ON COLUMN payments.idempotency_key IS 'Identité de création/reprise d''un encaissement ; même opération = même clé.';
