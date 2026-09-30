-- Mode d'application de la révision : immédiate ou à partir du mois suivant
ALTER TABLE leases
    ADD COLUMN IF NOT EXISTS revision_application_mode TEXT NOT NULL DEFAULT 'NEXT_MONTH'
        CHECK (revision_application_mode IN ('IMMEDIATE', 'NEXT_MONTH'));

-- Retrait du biennal : on ne garde que 12 et 36 mois
UPDATE leases SET revision_period_months = 12
 WHERE revision_period_months NOT IN (12, 36);

ALTER TABLE leases DROP CONSTRAINT IF EXISTS leases_revision_period_months_check;
ALTER TABLE leases ADD CONSTRAINT leases_revision_period_months_check
    CHECK (revision_period_months IN (12, 36));