# Audit de départ — avant extension garage

## Déjà présent
- registre `legal_entities` avec formes `SCI` et `SARL` et régimes `IR` / `IS` ;
- comptes bancaires rattachés à chaque entité ;
- portée canonique `legal_entity_id` utilisée par les modules financiers, facturation, banque, trésorerie, fiscalité, documents, IA et audit ;
- sélecteur d'entité dans l'UI ;
- activités SARL existantes (`MARCHAND_DE_BIENS`, `GARAGE_AUTO`) ;
- moteur de règles et données communs ;
- TVA déjà exposée dans la configuration d'entité en `COLLECTION` ou `DEBIT`.

## Manque constaté pour le garage
- aucun registre métier dédié aux véhicules en stock ;
- aucun registre de stock de pièces ;
- aucun cycle de dépôt-vente structuré ;
- aucun cycle achats/ventes automobile dédié ;
- aucun moteur de valorisation/rotation de stock automobile ;
- aucune couverture spécifique des obligations fiscales propres au commerce automobile ;
- trésorerie consolidée multi-entités non exposée comme vue métier dédiée ;
- connecteurs externes réels encore séparés des adapters internes.

## Décision de conception
L'étape 1 ne prétend pas résoudre ces manques : elle pose le cloisonnement et les profils qui permettront de les ajouter sans transformer la SCI en faux garage ni dupliquer les moteurs communs.
