-- S02 / US-0204 — Historiser les changements critiques.
-- Migration additive : aucune donnée métier existante n'est supprimée.

CREATE TABLE IF NOT EXISTS entity_change_history (
    id BIGSERIAL PRIMARY KEY,
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
    effective_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    author TEXT NOT NULL DEFAULT 'SYSTEM',
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    reason TEXT,
    before_state JSONB NOT NULL DEFAULT '{}'::jsonb,
    after_state JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX IF NOT EXISTS idx_entity_change_history_scope
    ON entity_change_history(legal_entity_id, effective_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_entity_change_history_entity
    ON entity_change_history(legal_entity_id, entity_type, entity_id, id DESC);

-- Le journal critique est lui aussi rattaché explicitement à l'entité.
ALTER TABLE entity_change_history
    ADD CONSTRAINT ck_entity_change_history_author_nonempty
    CHECK (btrim(author) <> '');

ALTER TABLE entity_change_history
    ADD CONSTRAINT ck_entity_change_history_action_nonempty
    CHECK (btrim(action) <> '');

ALTER TABLE entity_change_history
    ADD CONSTRAINT ck_entity_change_history_type_nonempty
    CHECK (btrim(entity_type) <> '');
