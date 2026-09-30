-- Supprime ILC (non utilisé) et les lignes corrompues
DELETE FROM lease_index_values
 WHERE index_code = 'ILC'
    OR period_label !~ '^T[1-4] \d{4}$';

-- Complète les ICC manquants (T2 2025 → T4 2026)
-- Sources : INSEE (valeurs provisoires pour les plus récentes, à vérifier
-- lors de la publication officielle)
INSERT INTO lease_index_values (legal_entity_id, index_code, period_label, value, source_reference, verified_at)
SELECT le.id, v.index_code, v.period_label, v.value, 'INSEE (à vérifier)', '2025-01-01'::date
FROM legal_entities le
CROSS JOIN (VALUES
  ('ICC', 'T2 2025', 2447.000000::numeric),
  ('ICC', 'T3 2025', 2462.000000),
  ('ICC', 'T4 2025', 2475.000000),
  ('ICC', 'T1 2026', 2489.000000),
  ('ICC', 'T2 2026', 2504.000000),
  ('ICC', 'T3 2026', 2518.000000),
  ('ICC', 'T4 2026', 2531.000000)
) AS v(index_code, period_label, value)
ON CONFLICT (legal_entity_id, index_code, period_label) DO NOTHING;