ALTER TABLE leases
    ADD COLUMN IF NOT EXISTS revision_period_months INTEGER NOT NULL DEFAULT 12
        CHECK (revision_period_months IN (12, 24, 36)),
    ADD COLUMN IF NOT EXISTS index_publication_day INTEGER NOT NULL DEFAULT 12
        CHECK (index_publication_day BETWEEN 1 AND 28),
    ADD COLUMN IF NOT EXISTS index_publication_month_offset INTEGER NOT NULL DEFAULT 3
        CHECK (index_publication_month_offset BETWEEN 1 AND 6);

UPDATE leases SET revision_period_months = 12
 WHERE revision_period_months IS NULL;