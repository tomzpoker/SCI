# S04 FINAL — Moteurs communs de données et gestion

## Statut
**COMPLETE côté source / READY FOR REVIEW**

Stories : US-0401, US-0402, US-0403, US-0404, US-0405.

## Migration
`0012_common_data_engines.sql`

## Tables / contrats
- `financial_transactions`
- `business_event_types`
- `business_events`
- `cash_position_snapshots`
- extension de `rule_calculation_runs` pour la reproductibilité
- `service_operations`

## Notes de compatibilité
Les tables métier historiques restent présentes. Les nouveaux moteurs communs se superposent au vertical slice sans suppression destructive.

## Validation
Le gate statique doit être exécuté avec `scripts/verify-s03-0306-s04.ps1`. Le build et la base PostgreSQL réels restent à certifier sur le PC Windows cible.


## US-0407
Idempotence transverse des jobs/opérations via `job_executions`, clés facture/paiement et reprises protégées.
