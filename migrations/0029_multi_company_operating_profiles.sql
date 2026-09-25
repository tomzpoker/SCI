-- ETAPE 1 — Multi-sociétés + profils d'exploitation.
-- Migration additive : aucune suppression de données métier.

ALTER TABLE legal_entities
    ADD COLUMN IF NOT EXISTS display_order INTEGER NOT NULL DEFAULT 100;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname='chk_legal_entities_vat_basis_operating_profiles'
    ) THEN
        ALTER TABLE legal_entities
            ADD CONSTRAINT chk_legal_entities_vat_basis_operating_profiles
            CHECK (vat_basis IN ('COLLECTION','DEBIT'));
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS business_profile_catalog (
    code TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    legal_form_code TEXT NOT NULL CHECK (legal_form_code IN ('SCI','SARL')),
    default_tax_regime TEXT CHECK (default_tax_regime IN ('IR','IS')),
    module_key TEXT NOT NULL,
    capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(code) <> ''),
    CHECK (btrim(label) <> ''),
    CHECK (btrim(module_key) <> '')
);

INSERT INTO business_profile_catalog(code,label,description,legal_form_code,default_tax_regime,module_key,capabilities)
VALUES
(
    'SCI_LOCATIVE_IR',
    'SCI locative à l’IR',
    'Profil pour une SCI détenant et louant des biens immobiliers.',
    'SCI',
    'IR',
    'SCI_LOCATIVE',
    '{"real_estate":true,"leases":true,"rent_invoicing":true,"vehicle_stock":false,"parts_stock":false,"consignment":false,"workshop":false}'::jsonb
),
(
    'SARL_IS_GARAGE_SANS_SAV',
    'SARL à l’IS — Garage automobile sans SAV / atelier',
    'Vente de véhicules, stock de voitures, stock de pièces et dépôt-vente, sans atelier ni SAV.',
    'SARL',
    'IS',
    'GARAGE_AUTO',
    '{"vehicle_sales":true,"vehicle_stock":true,"parts_stock":true,"consignment":true,"workshop":false,"after_sales":false,"repair":false,"rental":false,"real_estate":false}'::jsonb
),
(
    'SARL_IS_MARCHAND_DE_BIENS',
    'SARL à l’IS — Marchand de biens',
    'Profil pour activité d’achat, travaux et revente de biens immobiliers.',
    'SARL',
    'IS',
    'MARCHAND_DE_BIENS',
    '{"real_estate_trade":true,"property_acquisition":true,"property_resale":true,"vehicle_stock":false,"parts_stock":false,"consignment":false,"workshop":false}'::jsonb
)
ON CONFLICT(code) DO UPDATE SET
    label=EXCLUDED.label,
    description=EXCLUDED.description,
    legal_form_code=EXCLUDED.legal_form_code,
    default_tax_regime=EXCLUDED.default_tax_regime,
    module_key=EXCLUDED.module_key,
    capabilities=EXCLUDED.capabilities,
    active=true;

CREATE TABLE IF NOT EXISTS legal_entity_business_profiles (
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE RESTRICT,
    profile_code TEXT NOT NULL REFERENCES business_profile_catalog(code) ON DELETE RESTRICT,
    is_primary BOOLEAN NOT NULL DEFAULT false,
    active BOOLEAN NOT NULL DEFAULT true,
    configuration JSONB NOT NULL DEFAULT '{}'::jsonb,
    notes TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (legal_entity_id, profile_code)
);

CREATE INDEX IF NOT EXISTS idx_legal_entity_business_profiles_entity
    ON legal_entity_business_profiles(legal_entity_id, active, is_primary DESC);

CREATE UNIQUE INDEX IF NOT EXISTS uq_legal_entity_primary_business_profile
    ON legal_entity_business_profiles(legal_entity_id)
    WHERE active AND is_primary;

CREATE OR REPLACE FUNCTION validate_business_profile_assignment()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    entity_form TEXT;
    profile_form TEXT;
BEGIN
    SELECT legal_form_code INTO entity_form
    FROM legal_entities WHERE id=NEW.legal_entity_id;

    SELECT legal_form_code INTO profile_form
    FROM business_profile_catalog WHERE code=NEW.profile_code AND active;

    IF entity_form IS NULL THEN
        RAISE EXCEPTION 'Entité juridique introuvable pour le profil %', NEW.profile_code;
    END IF;
    IF profile_form IS NULL THEN
        RAISE EXCEPTION 'Profil d''exploitation inconnu ou inactif: %', NEW.profile_code;
    END IF;
    IF entity_form <> profile_form THEN
        RAISE EXCEPTION 'Le profil % exige une entité % ; entité actuelle: %', NEW.profile_code, profile_form, entity_form;
    END IF;

    IF NEW.profile_code='SARL_IS_GARAGE_SANS_SAV' AND EXISTS(
        SELECT 1 FROM legal_entities WHERE id=NEW.legal_entity_id AND tax_regime <> 'IS'
    ) THEN
        RAISE EXCEPTION 'Le profil GARAGE_SANS_SAV est réservé à une SARL à l''IS.';
    END IF;

    NEW.updated_at := now();

    IF NEW.active AND NEW.is_primary THEN
        UPDATE legal_entity_business_profiles
        SET is_primary=false, updated_at=now()
        WHERE legal_entity_id=NEW.legal_entity_id
          AND profile_code<>NEW.profile_code
          AND active
          AND is_primary;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_validate_business_profile_assignment ON legal_entity_business_profiles;
CREATE TRIGGER trg_validate_business_profile_assignment
BEFORE INSERT OR UPDATE OF legal_entity_id, profile_code, active, is_primary
ON legal_entity_business_profiles
FOR EACH ROW EXECUTE FUNCTION validate_business_profile_assignment();

-- Liaison explicite des trois profils canoniques aux entités déjà connues.
INSERT INTO legal_entity_business_profiles(legal_entity_id,profile_code,is_primary,active,configuration,notes)
SELECT e.id,
       CASE
         WHEN e.legal_form_code='SCI' THEN 'SCI_LOCATIVE_IR'
         WHEN e.legal_form_code='SARL' AND e.tax_regime='IS' AND EXISTS(
             SELECT 1 FROM legal_entity_activities a WHERE a.legal_entity_id=e.id AND a.activity_code='GARAGE_AUTO' AND a.active
         ) THEN 'SARL_IS_GARAGE_SANS_SAV'
         WHEN e.legal_form_code='SARL' AND e.tax_regime='IS' AND EXISTS(
             SELECT 1 FROM legal_entity_activities a WHERE a.legal_entity_id=e.id AND a.activity_code='MARCHAND_DE_BIENS' AND a.active
         ) THEN 'SARL_IS_MARCHAND_DE_BIENS'
         ELSE NULL
       END,
       CASE WHEN e.legal_form_code='SCI' THEN true ELSE false END,
       true,
       CASE WHEN e.legal_form_code='SARL' THEN '{"configuration_status":"A_CONFIGURER"}'::jsonb ELSE '{}'::jsonb END,
       'Affectation initiale du profil opérationnel.'
FROM legal_entities e
WHERE (e.legal_form_code='SCI')
   OR (e.legal_form_code='SARL' AND EXISTS(
       SELECT 1 FROM legal_entity_activities a WHERE a.legal_entity_id=e.id AND a.active AND a.activity_code IN ('GARAGE_AUTO','MARCHAND_DE_BIENS')
   ))
ON CONFLICT(legal_entity_id,profile_code) DO NOTHING;

-- Activité métier plus précise pour le garage sans SAV. L'ancienne activité GARAGE_AUTO
-- reste disponible afin de ne pas casser les données existantes.
INSERT INTO legal_activity_catalog(code,label,description,active)
VALUES(
    'GARAGE_AUTO_SANS_SAV',
    'Garage automobile — vente / stock / dépôt-vente sans SAV',
    'Vente de véhicules, stock de véhicules, stock de pièces et dépôt-vente. Aucun atelier ni SAV.',
    true
)
ON CONFLICT(code) DO UPDATE SET
    label=EXCLUDED.label,
    description=EXCLUDED.description,
    active=true;

CREATE INDEX IF NOT EXISTS idx_legal_entities_display_order
    ON legal_entities(active,display_order,legal_name);

COMMENT ON TABLE legal_entity_business_profiles IS
    'Profil opérationnel configurable d’une entité juridique. Il permet de piloter plusieurs sociétés dans la même application et de partager les moteurs communs.';
