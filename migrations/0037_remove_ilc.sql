-- Supprime ILC (non utilisé)
DELETE FROM lease_index_values WHERE index_code = 'ILC';

-- Nettoie d'éventuels doublons ILC résiduels dans les baux
UPDATE leases SET index_code = 'ICC'
 WHERE index_code = 'ILC';