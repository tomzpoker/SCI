-- S15 / UX / ZERO-SAISIE. Additif et non destructif.
CREATE TABLE IF NOT EXISTS ux_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    theme_code TEXT NOT NULL DEFAULT 'NORMAL' CHECK (theme_code IN ('NORMAL','FUN','DARK','LIGHT','HIGH_CONTRAST','CUSTOM')),
    fun_mode BOOLEAN NOT NULL DEFAULT false,
    explanation_level SMALLINT NOT NULL DEFAULT 1 CHECK (explanation_level BETWEEN 1 AND 4),
    custom_theme JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (legal_entity_id)
);
INSERT INTO ux_preferences(legal_entity_id)
SELECT e.id FROM legal_entities e
WHERE NOT EXISTS (SELECT 1 FROM ux_preferences p WHERE p.legal_entity_id=e.id);

CREATE TABLE IF NOT EXISTS data_quality_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    domain_code TEXT NOT NULL CHECK (domain_code IN ('SCI','BAUX','LOCATAIRES','BANQUE','FISCALITE','DOCUMENTS')),
    score SMALLINT NOT NULL CHECK (score BETWEEN 0 AND 100),
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (legal_entity_id,domain_code)
);
CREATE INDEX IF NOT EXISTS idx_data_quality_scope_s15 ON data_quality_snapshots(legal_entity_id,calculated_at DESC);

CREATE TABLE IF NOT EXISTS onboarding_steps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    step_no SMALLINT NOT NULL CHECK (step_no BETWEEN 1 AND 14),
    code TEXT NOT NULL,
    label_fr TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'TODO' CHECK (status IN ('TODO','IN_PROGRESS','DONE','SKIPPED')),
    notes TEXT NOT NULL DEFAULT '',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (legal_entity_id,step_no),
    UNIQUE (legal_entity_id,code)
);
INSERT INTO onboarding_steps(legal_entity_id,step_no,code,label_fr)
SELECT e.id,v.step_no,v.code,v.label FROM legal_entities e
CROSS JOIN (VALUES
 (1,'SCI','SCI'),(2,'REGIME_FISCAL','Régime fiscal'),(3,'TVA','TVA'),(4,'BIENS','Biens'),
 (5,'LOTS','Lots'),(6,'LOCATAIRES','Locataires'),(7,'BAUX','Baux'),(8,'BANQUE','Banque'),
 (9,'DOCUMENTS','Documents'),(10,'ASSOCIES','Associés'),(11,'IMPORTS','Imports'),(12,'AUTOMATISATIONS','Automatisations'),
 (13,'NOTIFICATIONS','Notifications'),(14,'IA','IA')
) AS v(step_no,code,label)
ON CONFLICT (legal_entity_id,step_no) DO NOTHING;
CREATE INDEX IF NOT EXISTS idx_onboarding_steps_scope_s15 ON onboarding_steps(legal_entity_id,step_no);
