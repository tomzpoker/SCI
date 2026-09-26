-- S14: IA contrôlée, providers optionnels, catalogue des tools, invocations, mémoire gouvernée.
-- Extension additive et non destructive.

-- Compatibilité: les structures principales peuvent avoir été amorcées par une livraison antérieure.
ALTER TABLE ai_provider_configs ADD COLUMN IF NOT EXISTS supports_text BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE ai_provider_configs ADD COLUMN IF NOT EXISTS supports_voice BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ai_provider_configs ADD COLUMN IF NOT EXISTS priority INTEGER NOT NULL DEFAULT 100;
CREATE INDEX IF NOT EXISTS idx_ai_provider_configs_priority_s14 ON ai_provider_configs(legal_entity_id,enabled,priority,updated_at DESC);

ALTER TABLE ai_tool_invocations ADD COLUMN IF NOT EXISTS error_message TEXT NOT NULL DEFAULT '';

-- La migration 0024 a déjà créé la fonction et le trigger de garde mémoire.
-- Ici on conserve seulement l extension de configuration provider.

COMMENT ON TABLE ai_tool_invocations IS 'Journal d exécution des outils IA; toute action à impact passe par validation et garde métier.';
COMMENT ON TABLE ai_memory IS 'Mémoire IA limitée aux préférences/habitudes/automatisations/style documentaire/UX; jamais autorité fiscale, légale ou financière.';
