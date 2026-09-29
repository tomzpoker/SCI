-- Métadonnées locales pour les lignes bancaires Dolibarr.
-- Dolibarr est la source de vérité pour les mouvements ; on stocke ici
-- uniquement ce que Dolibarr ne sait pas : le taux de TVA (points de base)
-- pour le calcul de la TVA déductible sur encaissements.
--
-- `dolibarr_line_id` = ID de la ligne dans `llx_bank` (clé stable).
CREATE TABLE IF NOT EXISTS bank_line_metadata (
    dolibarr_line_id text PRIMARY KEY,
    vat_rate_bp      integer,
    updated_at       timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_bank_line_metadata_rate
    ON bank_line_metadata (vat_rate_bp)
    WHERE vat_rate_bp IS NOT NULL;

COMMENT ON TABLE bank_line_metadata IS
    'Attributs locaux non stockés dans Dolibarr, joints par dolibarr_line_id';