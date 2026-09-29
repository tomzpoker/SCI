-- Ajout du taux de TVA par transaction bancaire (points de base).
-- NULL ou 0 = pas de TVA récupérable (assurance exonérée, etc.)
-- 2000 = 20%  |  1000 = 10%  |  550 = 5.5%
ALTER TABLE bank_transactions
    ADD COLUMN IF NOT EXISTS vat_rate_bp integer;

COMMENT ON COLUMN bank_transactions.vat_rate_bp IS
    'Taux de TVA en points de base (2000 = 20%, 1000 = 10%, NULL = aucune TVA)';