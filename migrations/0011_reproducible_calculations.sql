-- S03 — US-0306 : calculs reproductibles.
-- Migration additive et non destructive.

ALTER TABLE rule_calculation_runs
    ADD COLUMN IF NOT EXISTS formula_text TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS rule_snapshot JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS source_snapshot JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS rounding_mode TEXT NOT NULL DEFAULT 'INTEGER_CENTS',
    ADD COLUMN IF NOT EXISTS input_hash TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS output_hash TEXT NOT NULL DEFAULT '';

COMMENT ON COLUMN rule_calculation_runs.formula_text IS 'Formule lisible utilisée au moment du calcul.';
COMMENT ON COLUMN rule_calculation_runs.rule_snapshot IS 'Snapshot immuable de la RuleVersion utilisée.';
COMMENT ON COLUMN rule_calculation_runs.source_snapshot IS 'Snapshot de la source rattachée à la règle.';

-- Reprise des runs historiques existants : enrichissement sans réécriture du résultat.
UPDATE rule_calculation_runs r
SET formula_text = CASE
        WHEN COALESCE(rv.definition->>'operation','') <> '' THEN 'Opération ' || (rv.definition->>'operation')
        ELSE 'Définition historique'
    END,
    rule_snapshot = jsonb_build_object(
        'rule_version_id', rv.id,
        'version_no', rv.version_no,
        'definition', rv.definition
    ) || COALESCE(jsonb_build_object('source_reference_id', rv.source_reference_id), '{}'::jsonb),
    source_snapshot = COALESCE((
        SELECT jsonb_build_object(
            'source_name', vr.source_name,
            'source_reference', COALESCE(vr.source_reference,''),
            'source_url', COALESCE(vr.source_url,'')
        ) FROM versioned_references vr WHERE vr.id=rv.source_reference_id
    ), '{}'::jsonb),
    rounding_mode = COALESCE(NULLIF(r.rounding_mode,''),'INTEGER_CENTS'),
    input_hash = CASE WHEN r.input_hash='' THEN 'LEGACY_UNHASHED' ELSE r.input_hash END,
    output_hash = CASE WHEN r.output_hash='' THEN 'LEGACY_UNHASHED' ELSE r.output_hash END
FROM rule_versions rv
WHERE rv.id=r.rule_version_id
  AND (r.formula_text='' OR r.rule_snapshot='{}'::jsonb OR r.source_snapshot='{}'::jsonb);
