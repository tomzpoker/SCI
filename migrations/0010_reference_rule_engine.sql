-- S03 — Référentiels versionnés + Rule Engine.
-- Migration additive et append-only pour les versions publiées.
-- Aucune opération DROP/TRUNCATE/DELETE n'est utilisée.

CREATE TABLE IF NOT EXISTS versioned_references (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    namespace TEXT NOT NULL,
    code TEXT NOT NULL,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    valid_from DATE NOT NULL,
    valid_to DATE,
    source_kind TEXT NOT NULL DEFAULT 'OTHER' CHECK (source_kind IN ('OFFICIAL','OTHER')),
    source_name TEXT NOT NULL,
    source_url TEXT,
    source_reference TEXT,
    verified_at DATE,
    status TEXT NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT','PUBLISHED','RETIRED')),
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(workspace_id, namespace, code, version_no),
    CHECK (valid_to IS NULL OR valid_to >= valid_from),
    CHECK (btrim(namespace) <> ''),
    CHECK (btrim(code) <> ''),
    CHECK (btrim(source_name) <> '')
);

CREATE INDEX IF NOT EXISTS idx_versioned_references_lookup
    ON versioned_references(workspace_id, namespace, code, valid_from DESC, version_no DESC);
CREATE INDEX IF NOT EXISTS idx_versioned_references_status
    ON versioned_references(workspace_id, status, source_kind);

CREATE TABLE IF NOT EXISTS rule_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    code TEXT NOT NULL,
    label TEXT NOT NULL,
    engine_code TEXT NOT NULL DEFAULT 'RULE_ENGINE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(workspace_id, code),
    CHECK (btrim(code) <> ''),
    CHECK (btrim(label) <> '')
);

CREATE TABLE IF NOT EXISTS rule_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_definition_id UUID NOT NULL REFERENCES rule_definitions(id) ON DELETE RESTRICT,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_entity_id UUID REFERENCES legal_entities(id) ON DELETE RESTRICT,
    activity_code TEXT,
    jurisdiction_code TEXT,
    version_no INTEGER NOT NULL CHECK (version_no > 0),
    valid_from DATE NOT NULL,
    valid_to DATE,
    status TEXT NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT','PUBLISHED','RETIRED')),
    source_reference_id UUID REFERENCES versioned_references(id) ON DELETE RESTRICT,
    definition JSONB NOT NULL DEFAULT '{}'::jsonb,
    change_note TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (valid_to IS NULL OR valid_to >= valid_from),
    CHECK (activity_code IS NULL OR btrim(activity_code) <> ''),
    CHECK (jurisdiction_code IS NULL OR btrim(jurisdiction_code) <> ''),
    UNIQUE(rule_definition_id, legal_entity_id, activity_code, jurisdiction_code, version_no)
);

-- Une contrainte UNIQUE avec des NULL ne suffit pas à verrouiller les scopes génériques.
-- Cet index normalise les NULL pour garantir une seule version par scope/version.
CREATE UNIQUE INDEX IF NOT EXISTS uq_rule_versions_scope_version
    ON rule_versions(
        rule_definition_id,
        COALESCE(legal_entity_id, '00000000-0000-0000-0000-000000000000'::uuid),
        COALESCE(activity_code, ''),
        COALESCE(jurisdiction_code, ''),
        version_no
    );

CREATE INDEX IF NOT EXISTS idx_rule_versions_scope
    ON rule_versions(workspace_id, legal_entity_id, activity_code, jurisdiction_code, valid_from DESC, version_no DESC);
CREATE INDEX IF NOT EXISTS idx_rule_versions_rule
    ON rule_versions(rule_definition_id, status, valid_from DESC, version_no DESC);

CREATE TABLE IF NOT EXISTS rule_calculation_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    rule_version_id UUID NOT NULL REFERENCES rule_versions(id) ON DELETE RESTRICT,
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    as_of_date DATE NOT NULL,
    input_values JSONB NOT NULL DEFAULT '{}'::jsonb,
    output_value JSONB NOT NULL DEFAULT 'null'::jsonb,
    replayed_from UUID REFERENCES rule_calculation_runs(id) ON DELETE RESTRICT,
    status TEXT NOT NULL DEFAULT 'COMPLETED' CHECK (status IN ('COMPLETED','REPLAYED','FAILED')),
    actor TEXT NOT NULL DEFAULT 'SYSTEM',
    CHECK (btrim(actor) <> '')
);

CREATE INDEX IF NOT EXISTS idx_rule_calculation_runs_scope
    ON rule_calculation_runs(legal_entity_id, calculated_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_rule_calculation_runs_rule
    ON rule_calculation_runs(rule_version_id, calculated_at DESC, id DESC);

-- Référentiels maîtres de sources : aucun contenu fiscal chiffré n'est inventé ici.
INSERT INTO versioned_references(
    workspace_id, namespace, code, version_no, valid_from, source_kind,
    source_name, source_url, verified_at, status, payload
)
SELECT w.id, 'OFFICIAL_SOURCE', v.code, 1, DATE '2026-01-01', 'OFFICIAL', v.source_name,
       v.source_url, DATE '2026-09-24', 'PUBLISHED', '{}'::jsonb
FROM workspaces w
CROSS JOIN (VALUES
    ('IMPOTS_GOUV_FR', 'impots.gouv.fr', 'https://www.impots.gouv.fr'),
    ('LEGIFRANCE', 'Légifrance', 'https://www.legifrance.gouv.fr'),
    ('SERVICE_PUBLIC', 'service-public.fr', 'https://www.service-public.fr')
) AS v(code, source_name, source_url)
WHERE w.id='00000000-0000-0000-0000-000000000001'
ON CONFLICT (workspace_id, namespace, code, version_no) DO NOTHING;

CREATE OR REPLACE FUNCTION validate_reference_version()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP='DELETE' THEN
        IF OLD.status='PUBLISHED' THEN
            RAISE EXCEPTION 'Une référence publiée est immuable : créer une nouvelle version';
        END IF;
        RETURN OLD;
    END IF;
    NEW.updated_at := now();
    IF NEW.status='PUBLISHED' AND NEW.verified_at IS NULL THEN
        RAISE EXCEPTION 'Une référence publiée doit posséder verified_at';
    END IF;
    IF TG_OP IN ('UPDATE','DELETE') AND OLD.status='PUBLISHED' THEN
        RAISE EXCEPTION 'Une référence publiée est immuable : créer une nouvelle version';
    END IF;
    RETURN NEW;
END;
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgname='trg_validate_reference_version'
          AND tgrelid='versioned_references'::regclass
    ) THEN
        CREATE TRIGGER trg_validate_reference_version
        BEFORE INSERT OR UPDATE OR DELETE ON versioned_references
        FOR EACH ROW EXECUTE FUNCTION validate_reference_version();
    END IF;
END
$$;

CREATE OR REPLACE FUNCTION validate_rule_version()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    entity_form TEXT;
    entity_workspace UUID;
BEGIN
    IF TG_OP='DELETE' THEN
        IF OLD.status='PUBLISHED' THEN
            RAISE EXCEPTION 'Une RuleVersion publiée est immuable : créer une nouvelle version';
        END IF;
        RETURN OLD;
    END IF;

    NEW.updated_at := now();

    SELECT e.legal_form_code, e.workspace_id
      INTO entity_form, entity_workspace
      FROM legal_entities e
     WHERE e.id=NEW.legal_entity_id;

    IF NEW.legal_entity_id IS NOT NULL THEN
        IF entity_workspace IS NULL THEN
            RAISE EXCEPTION 'Entité juridique introuvable pour la RuleVersion';
        END IF;
        IF entity_workspace <> NEW.workspace_id THEN
            RAISE EXCEPTION 'RuleVersion hors workspace de son entité';
        END IF;
        IF NEW.activity_code IS NOT NULL THEN
            IF entity_form <> 'SARL' THEN
                RAISE EXCEPTION 'Une activité ne peut cibler que des règles SARL';
            END IF;
            IF NOT EXISTS (
                SELECT 1 FROM legal_activity_catalog c
                WHERE c.code=NEW.activity_code
            ) THEN
                RAISE EXCEPTION 'Activité inconnue pour la RuleVersion: %', NEW.activity_code;
            END IF;
        END IF;
    END IF;

    IF NEW.status='PUBLISHED' AND NEW.source_reference_id IS NULL THEN
        RAISE EXCEPTION 'Une RuleVersion publiée doit référencer une source';
    END IF;

    IF NEW.status='PUBLISHED' AND EXISTS (
        SELECT 1
          FROM rule_versions r
         WHERE r.id <> NEW.id
           AND r.rule_definition_id=NEW.rule_definition_id
           AND COALESCE(r.legal_entity_id,'00000000-0000-0000-0000-000000000000')
               = COALESCE(NEW.legal_entity_id,'00000000-0000-0000-0000-000000000000')
           AND COALESCE(r.activity_code,'*')=COALESCE(NEW.activity_code,'*')
           AND COALESCE(r.jurisdiction_code,'*')=COALESCE(NEW.jurisdiction_code,'*')
           AND r.status='PUBLISHED'
           AND r.valid_from <= COALESCE(NEW.valid_to,'9999-12-31'::date)
           AND COALESCE(r.valid_to,'9999-12-31'::date) >= NEW.valid_from
    ) THEN
        RAISE EXCEPTION 'Deux RuleVersions publiées se chevauchent sur le même périmètre';
    END IF;

    IF TG_OP='UPDATE' AND OLD.status='PUBLISHED' THEN
        RAISE EXCEPTION 'Une RuleVersion publiée est immuable : créer une nouvelle version';
    END IF;

    RETURN NEW;
END;
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgname='trg_validate_rule_version'
          AND tgrelid='rule_versions'::regclass
    ) THEN
        CREATE TRIGGER trg_validate_rule_version
        BEFORE INSERT OR UPDATE OR DELETE ON rule_versions
        FOR EACH ROW EXECUTE FUNCTION validate_rule_version();
    END IF;
END
$$;

CREATE OR REPLACE FUNCTION validate_rule_calculation_run()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    rule_workspace UUID;
    rule_entity UUID;
BEGIN
    SELECT rv.workspace_id, rv.legal_entity_id
      INTO rule_workspace, rule_entity
      FROM rule_versions rv
     WHERE rv.id=NEW.rule_version_id;

    IF rule_workspace IS NULL THEN
        RAISE EXCEPTION 'RuleVersion introuvable';
    END IF;
    IF rule_workspace <> NEW.workspace_id THEN
        RAISE EXCEPTION 'Run de calcul hors workspace de sa RuleVersion';
    END IF;
    IF rule_entity IS NOT NULL AND rule_entity <> NEW.legal_entity_id THEN
        RAISE EXCEPTION 'Run de calcul hors entité de sa RuleVersion';
    END IF;
    RETURN NEW;
END;
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgname='trg_validate_rule_calculation_run'
          AND tgrelid='rule_calculation_runs'::regclass
    ) THEN
        CREATE TRIGGER trg_validate_rule_calculation_run
        BEFORE INSERT OR UPDATE ON rule_calculation_runs
        FOR EACH ROW EXECUTE FUNCTION validate_rule_calculation_run();
    END IF;
END
$$;

-- Les versions publiées doivent rester lisibles même quand une règle future est ajoutée.
COMMENT ON TABLE versioned_references IS 'Référentiels versionnés avec validité, source et statut. Les versions publiées sont immuables.';
COMMENT ON TABLE rule_versions IS 'Versions de règles par entité, activité, juridiction et période. Publication append-only.';
COMMENT ON TABLE rule_calculation_runs IS 'Entrées et résultat d''un calcul permettant le replay historique avec sa RuleVersion.';
