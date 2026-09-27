-- Ajoute les 3 champs "ancre + offset" pour les règles haut de gamme.
-- due_day_of_month : jour fixe du mois (ex: 24 pour la TVA). NULL = ancien système horizon.
-- urgency_lead_days : nombre de jours avant l'échéance où la tâche devient urgente.
-- prep_lead_days : nombre de jours avant l'échéance où la préparation démarre.

ALTER TABLE automation_rules
    ADD COLUMN IF NOT EXISTS due_day_of_month SMALLINT;

ALTER TABLE automation_rules
    ADD COLUMN IF NOT EXISTS urgency_lead_days SMALLINT NOT NULL DEFAULT 5;

ALTER TABLE automation_rules
    ADD COLUMN IF NOT EXISTS prep_lead_days SMALLINT NOT NULL DEFAULT 21;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'automation_rules_due_day_check'
    ) THEN
        ALTER TABLE automation_rules
            ADD CONSTRAINT automation_rules_due_day_check
            CHECK (due_day_of_month IS NULL OR due_day_of_month BETWEEN 1 AND 31);
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'automation_rules_urgency_check'
    ) THEN
        ALTER TABLE automation_rules
            ADD CONSTRAINT automation_rules_urgency_check
            CHECK (urgency_lead_days BETWEEN 0 AND 90);
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'automation_rules_prep_check'
    ) THEN
        ALTER TABLE automation_rules
            ADD CONSTRAINT automation_rules_prep_check
            CHECK (prep_lead_days BETWEEN 0 AND 180);
    END IF;
END $$;