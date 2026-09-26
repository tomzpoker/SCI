-- S02 / US-0202 — Activités configurables d'une SARL.
-- Migration additive : aucune suppression de données ni d'objet existant.

CREATE TABLE IF NOT EXISTS legal_activity_catalog (
    code TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO legal_activity_catalog (code, label, description, active)
VALUES
    ('MARCHAND_DE_BIENS', 'Marchand de biens', 'Achat, rénovation et revente de biens immobiliers selon le périmètre réellement exploité.', true),
    ('GARAGE_AUTO', 'Garage automobile', 'Entretien, réparation et activités automobiles selon le périmètre réellement exploité.', true)
ON CONFLICT (code) DO NOTHING;

CREATE TABLE IF NOT EXISTS legal_entity_activities (
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    activity_code TEXT NOT NULL REFERENCES legal_activity_catalog(code),
    is_primary BOOLEAN NOT NULL DEFAULT false,
    active BOOLEAN NOT NULL DEFAULT true,
    notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (legal_entity_id, activity_code)
);

CREATE INDEX IF NOT EXISTS idx_legal_entity_activities_entity_active
    ON legal_entity_activities(legal_entity_id, active, is_primary DESC, activity_code);

CREATE UNIQUE INDEX IF NOT EXISTS uq_legal_entity_primary_activity
    ON legal_entity_activities(legal_entity_id)
    WHERE active AND is_primary;

CREATE OR REPLACE FUNCTION validate_sarl_activity_entity()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.activity_code := upper(trim(NEW.activity_code));

    IF NOT EXISTS (
        SELECT 1
        FROM legal_entities e
        WHERE e.id = NEW.legal_entity_id
          AND e.legal_form_code = 'SARL'
    ) THEN
        RAISE EXCEPTION 'Une activité de cette table doit être rattachée à une entité SARL.';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM legal_activity_catalog c
        WHERE c.code = NEW.activity_code
          AND c.active
    ) THEN
        RAISE EXCEPTION 'Activité inconnue ou inactive: %.', NEW.activity_code;
    END IF;

    NEW.updated_at := now();

    IF NEW.active AND NEW.is_primary THEN
        UPDATE legal_entity_activities
        SET is_primary = false,
            updated_at = now()
        WHERE legal_entity_id = NEW.legal_entity_id
          AND activity_code <> NEW.activity_code
          AND active
          AND is_primary;
    END IF;

    RETURN NEW;
END;
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_trigger
        WHERE tgname = 'trg_validate_sarl_activity_entity'
          AND tgrelid = 'legal_entity_activities'::regclass
    ) THEN
        CREATE TRIGGER trg_validate_sarl_activity_entity
        BEFORE INSERT OR UPDATE OF legal_entity_id, activity_code, active, is_primary
        ON legal_entity_activities
        FOR EACH ROW
        EXECUTE FUNCTION validate_sarl_activity_entity();
    END IF;
END
$$;

CREATE INDEX IF NOT EXISTS idx_legal_activity_catalog_active_label
    ON legal_activity_catalog(active, label);

COMMENT ON TABLE legal_entity_activities IS
    'Activités choisies par une SARL. Plusieurs activités peuvent être actives simultanément. Les activités d''une SCI ne transitent pas par cette table.';
