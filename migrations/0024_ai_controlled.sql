-- S14 / IA CONTRÔLÉE — providers optionnels, tools à risque, mémoire gouvernée,
-- conversations, invocations auditables et validations humaines.
-- Migration additive et non destructive.

CREATE TABLE IF NOT EXISTS ai_provider_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    provider_kind TEXT NOT NULL CHECK (provider_kind IN ('LOCAL_LLM','EXTERNAL_LLM','DISABLED')),
    name TEXT NOT NULL,
    model TEXT NOT NULL DEFAULT '',
    endpoint_reference TEXT NOT NULL DEFAULT '',
    credential_reference TEXT NOT NULL DEFAULT '',
    command_reference TEXT NOT NULL DEFAULT '',
    enabled BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(name) <> ''),
    UNIQUE(legal_entity_id,provider_kind,name)
);
CREATE INDEX IF NOT EXISTS idx_ai_provider_configs_scope_s14
    ON ai_provider_configs(legal_entity_id,enabled,provider_kind,name);

INSERT INTO ai_provider_configs(legal_entity_id,provider_kind,name,enabled)
SELECT e.id,'DISABLED','IA désactivée',true
FROM legal_entities e
WHERE NOT EXISTS (
    SELECT 1 FROM ai_provider_configs a
    WHERE a.legal_entity_id=e.id AND a.provider_kind='DISABLED'
);

CREATE TABLE IF NOT EXISTS ai_tool_catalog (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    tool_code TEXT NOT NULL,
    label_fr TEXT NOT NULL,
    risk_level TEXT NOT NULL CHECK (risk_level IN ('LOW','MEDIUM','HIGH','CRITICAL')),
    read_only BOOLEAN NOT NULL DEFAULT true,
    confirmation_required BOOLEAN NOT NULL DEFAULT false,
    authorized_roles JSONB NOT NULL DEFAULT '["MANAGER"]'::jsonb,
    description TEXT NOT NULL DEFAULT '',
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id,tool_code)
);
CREATE INDEX IF NOT EXISTS idx_ai_tool_catalog_scope_s14
    ON ai_tool_catalog(legal_entity_id,enabled,risk_level,tool_code);

INSERT INTO ai_tool_catalog(legal_entity_id,tool_code,label_fr,risk_level,read_only,confirmation_required,authorized_roles,description)
SELECT e.id,v.code,v.label,v.risk,v.read_only,v.confirm,v.roles::jsonb,v.description
FROM legal_entities e
CROSS JOIN (VALUES
 ('get_cash_balance','Consulter le solde théorique', 'LOW', true, false, '["MANAGER"]','Lit la position de trésorerie métier.'),
 ('get_real_bank_balance','Consulter le solde bancaire réel rapproché', 'LOW', true, false, '["MANAGER"]','Lit les soldes bancaires importés et rapprochés.'),
 ('get_forecast','Consulter les prévisions de trésorerie', 'LOW', true, false, '["MANAGER"]','Lit les prévisions et scénarios disponibles.'),
 ('get_unpaid_rents','Consulter les loyers impayés', 'LOW', true, false, '["MANAGER"]','Lit les créances et dossiers d''impayés.'),
 ('get_upcoming_deadlines','Consulter les prochaines échéances', 'LOW', true, false, '["MANAGER"]','Lit les échéances administratives et fiscales.'),
 ('calculate_rent_revision','Calculer une révision de loyer', 'MEDIUM', true, false, '["MANAGER"]','Effectue un calcul sans appliquer le nouveau loyer.'),
 ('calculate_vat','Calculer la TVA', 'MEDIUM', true, false, '["MANAGER"]','Interroge le moteur TVA, sans dépôt de déclaration.'),
 ('prepare_vat_return','Préparer une déclaration TVA', 'HIGH', false, true, '["MANAGER"]','Prépare une déclaration pour validation humaine.'),
 ('prepare_invoice','Préparer une facture', 'HIGH', false, true, '["MANAGER"]','Crée uniquement un brouillon explicitement marqué.'),
 ('prepare_reminder','Préparer une relance', 'HIGH', false, true, '["MANAGER"]','Prépare un document de relance sans envoi.'),
 ('search_documents','Rechercher dans les documents', 'LOW', true, false, '["MANAGER"]','Recherche les documents indexés.'),
 ('get_lease','Consulter un bail', 'LOW', true, false, '["MANAGER"]','Lit les données structurées d''un bail.'),
 ('get_tenant','Consulter un locataire', 'LOW', true, false, '["MANAGER"]','Lit les données d''un locataire.'),
 ('get_property','Consulter un bien', 'LOW', true, false, '["MANAGER"]','Lit les données d''un bien immobilier.'),
 ('simulate_tax','Simuler une imposition', 'MEDIUM', true, false, '["MANAGER"]','Produit exclusivement un scénario SIMULATION.'),
 ('create_draft','Créer un brouillon', 'HIGH', false, true, '["MANAGER"]','Crée un brouillon sans effet externe.'),
 ('request_user_validation','Demander une validation humaine', 'HIGH', false, true, '["MANAGER"]','Crée une demande dans l''inbox de validation.')
) AS v(code,label,risk,read_only,confirm,roles,description)
WHERE NOT EXISTS (
    SELECT 1 FROM ai_tool_catalog a WHERE a.legal_entity_id=e.id AND a.tool_code=v.code
);

CREATE TABLE IF NOT EXISTS ai_conversations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    title TEXT NOT NULL DEFAULT 'Assistant SCI',
    input_mode TEXT NOT NULL DEFAULT 'TEXT' CHECK (input_mode IN ('TEXT','TRANSCRIBED_VOICE')),
    output_mode TEXT NOT NULL DEFAULT 'TEXT' CHECK (output_mode IN ('TEXT','SYNTHESIZED_VOICE')),
    provider_kind TEXT NOT NULL DEFAULT 'DISABLED' CHECK (provider_kind IN ('LOCAL_LLM','EXTERNAL_LLM','DISABLED')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_ai_conversations_scope_s14 ON ai_conversations(legal_entity_id,updated_at DESC);

CREATE TABLE IF NOT EXISTS ai_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    conversation_id UUID NOT NULL REFERENCES ai_conversations(id) ON DELETE RESTRICT,
    role TEXT NOT NULL CHECK (role IN ('SYSTEM','USER','ASSISTANT','TOOL')),
    content TEXT NOT NULL,
    grounded BOOLEAN NOT NULL DEFAULT true,
    uncertainty TEXT NOT NULL DEFAULT '',
    tool_code TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_ai_messages_scope_s14 ON ai_messages(legal_entity_id,conversation_id,created_at);

CREATE TABLE IF NOT EXISTS ai_tool_invocations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    conversation_id UUID,
    tool_code TEXT NOT NULL,
    risk_level TEXT NOT NULL CHECK (risk_level IN ('LOW','MEDIUM','HIGH','CRITICAL')),
    read_only BOOLEAN NOT NULL,
    confirmation_required BOOLEAN NOT NULL,
    actor_role TEXT NOT NULL DEFAULT 'MANAGER',
    status TEXT NOT NULL DEFAULT 'PROPOSED' CHECK (status IN ('PROPOSED','VALIDATION_REQUIRED','EXECUTED','REJECTED','FAILED')),
    arguments JSONB NOT NULL DEFAULT '{}'::jsonb,
    result JSONB NOT NULL DEFAULT '{}'::jsonb,
    uncertainty TEXT NOT NULL DEFAULT '',
    validation_request_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    executed_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_ai_tool_invocations_scope_s14
    ON ai_tool_invocations(legal_entity_id,created_at DESC,status,risk_level);
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_ai_tool_invocation_conversation_s14') THEN
        ALTER TABLE ai_tool_invocations ADD CONSTRAINT fk_ai_tool_invocation_conversation_s14
            FOREIGN KEY(conversation_id) REFERENCES ai_conversations(id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname='fk_ai_tool_invocation_validation_s14') THEN
        ALTER TABLE ai_tool_invocations ADD CONSTRAINT fk_ai_tool_invocation_validation_s14
            FOREIGN KEY(validation_request_id) REFERENCES validation_requests(id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS ai_memory (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    category TEXT NOT NULL CHECK (category IN ('PREFERENCE','HABIT','AUTOMATION','DOCUMENT_STYLE','UX')),
    memory_key TEXT NOT NULL,
    value JSONB NOT NULL DEFAULT '{}'::jsonb,
    source TEXT NOT NULL DEFAULT 'USER',
    user_approved BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(legal_entity_id,category,memory_key)
);
CREATE INDEX IF NOT EXISTS idx_ai_memory_scope_s14 ON ai_memory(legal_entity_id,category,updated_at DESC);

CREATE OR REPLACE FUNCTION guard_ai_memory_s14() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT (NEW.category IN ('PREFERENCE','HABIT','AUTOMATION','DOCUMENT_STYLE','UX')) THEN
        RAISE EXCEPTION 'Catégorie de mémoire IA interdite';
    END IF;
    IF NEW.memory_key ~* '(fiscal|legal|jurid|tax|tva|vat|finance|financial|bank|iban|account)' THEN
        RAISE EXCEPTION 'Une mémoire IA ne peut pas contenir de règle fiscale, légale ou financière critique';
    END IF;
    IF NEW.user_approved IS DISTINCT FROM true THEN
        NEW.user_approved := false;
    END IF;
    NEW.updated_at := now();
    RETURN NEW;
END; $$;
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname='trg_guard_ai_memory_s14') THEN
        CREATE TRIGGER trg_guard_ai_memory_s14 BEFORE INSERT OR UPDATE ON ai_memory
        FOR EACH ROW EXECUTE FUNCTION guard_ai_memory_s14();
    END IF;
END $$;

INSERT INTO audit_events(sci_id,legal_entity_id,actor,action,entity_type,entity_id,payload)
SELECT NULL,e.id,'SYSTEM','AI_CONTROL_POLICY_INITIALIZED','AI_POLICY',e.id,json_build_object('ai_optional',true,'direct_sql_forbidden',true)
FROM legal_entities e
WHERE NOT EXISTS (
    SELECT 1 FROM audit_events a WHERE a.legal_entity_id=e.id AND a.action='AI_CONTROL_POLICY_INITIALIZED'
);

COMMENT ON TABLE ai_provider_configs IS 'Fournisseurs IA optionnels. DISABLED permet un fonctionnement sans IA.';
COMMENT ON TABLE ai_tool_catalog IS 'Catalogue de tools avec risque, lecture seule, confirmation et rôles autorisés.';
COMMENT ON TABLE ai_tool_invocations IS 'Journal de chaque tool : arguments, résultat, validation et incertitude.';
COMMENT ON TABLE ai_memory IS 'Mémoire UX/documentaire explicitement limitée; jamais de règles fiscales/légales/financières critiques.';
