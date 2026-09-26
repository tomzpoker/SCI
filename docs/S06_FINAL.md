# S06 FINAL — BAUX / LOCATIONS

## Statut
COMPLETE CÔTÉ SOURCE / READY FOR REVIEW.

## Migrations
- 0016_leases_locations.sql

## Couverture
- Bail structuré : signature, effet, échéance, type, destination, loyer, fréquence, TVA, indexation,
charges, dépôt, droit d'entrée.
- Clauses individuelles et versionnées.
- Révisions de loyer calculées sans application silencieuse.
- Historique des indices ICC/ILC par période et source.
- Charges forfait/provision/provision variable/régularisation.
- Réductions temporaires conservées comme événements contractuels.
- Dépôts avec mouvements de réception/restitution/retenue et lien bancaire.
- Garanties documentables.
- Droit d'entrée qualifié `CONFIRMED`, `PENDING`, `UNCERTAIN` ou `NOT_APPLICABLE` ; `UNCERTAIN`
  génère une tâche de vérification.
