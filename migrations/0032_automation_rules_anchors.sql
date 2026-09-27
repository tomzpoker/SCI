-- Capture du drift : modifications faites manuellement dans la base
-- pour ancrer les règles d'automisation sur des dates calendaires fixes
-- plutôt que sur l'ancien système "horizon_days".
--
-- Référence logique : voir automation_tick_with() dans src/server.rs

-- TVA sur encaissements : ancrage au 24 du mois courant
-- (déclaration de la TVA du mois précédent)
UPDATE automation_rules
SET due_day_of_month = 24,
    prep_lead_days = 21,
    urgency_lead_days = 5,
    name = 'Préparer la TVA sur encaissements',
    description = 'Déclaration de TVA sur les encaissements du mois précédent. À préparer avant le 24 du mois courant.'
WHERE code = 'VAT_COLLECTION';

-- Clôture annuelle : ancrage au 1er mars (dépôt des comptes annuels)
-- due_day_of_month = NULL car l'ancrage est géré en code (pas un jour du mois courant)
UPDATE automation_rules
SET due_day_of_month = NULL,
    prep_lead_days = 90,
    urgency_lead_days = 30,
    name = 'Préparer la clôture annuelle',
    description = 'Préparer les comptes annuels avant le 1er mars (exercice clos le 31/12).'
WHERE code = 'ANNUAL_CLOSE';