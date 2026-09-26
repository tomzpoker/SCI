-- S10 / Prévisionnel : flux récurrents et ponctuels configurables par l'utilisateur.
-- Migration additive, idempotente.

CREATE TABLE IF NOT EXISTS prevision_flows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    legal_entity_id UUID NOT NULL REFERENCES legal_entities(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    amount_cents BIGINT NOT NULL,
    color TEXT NOT NULL DEFAULT '#38bdf8',
    active BOOLEAN NOT NULL DEFAULT true,
    recurrence TEXT NOT NULL CHECK (recurrence IN ('ONCE', 'MONTHLY', 'QUARTERLY', 'YEARLY')),
    start_year INTEGER NOT NULL,
    start_month SMALLINT NOT NULL CHECK (start_month BETWEEN 1 AND 12),
    recurrence_day SMALLINT NOT NULL CHECK (recurrence_day BETWEEN 1 AND 31),
    payment_day SMALLINT NOT NULL CHECK (payment_day BETWEEN 1 AND 31),
    matched_tx_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_prevision_flows_entity
    ON prevision_flows(legal_entity_id, active, start_year, start_month);

CREATE INDEX IF NOT EXISTS idx_prevision_flows_recurrence
    ON prevision_flows(legal_entity_id, recurrence);

-- Contrainte d'unicité : un même libellé ne peut pas être dupliqué sur la même entité.
-- (Soft, on n'impose pas, juste un index pour recherche rapide par libellé.)
CREATE INDEX IF NOT EXISTS idx_prevision_flows_label
    ON prevision_flows(legal_entity_id, label);