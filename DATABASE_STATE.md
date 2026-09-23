# DATABASE_STATE

## Sources
- Runtime actuel : `migrations/0001..0004`
- BDD de référence : `example bdd/20260915060000_baseline.sql`
- Référentiels : `example bdd/20260915070000_reference_data_baseline.sql`

## Constat
BDD d'exemple : 86 tables.
Migrations Rust actuelles : 18 tables.

Le schéma runtime actuel n'est donc pas une implémentation directe du schéma de référence.

## Architecture de référence importante
- Event store : `sci_events.*`
- Finance : `sci_finance.*`
- Property : `sci_property.*`
- Projections : `sci_projections.*`
- Billing : `sci_billing.*`
- Documents : `sci_documents.*`
- Banking : `sci_banking.*`
- Reference/rules : `sci_reference.*`
- Scheduler : `sci_scheduler.*`
- Workflow : `sci_workflow.*`
- Audit : `sci_audit.*`

## Règle de migration
Aucune suppression ou remplacement destructif de la BDD source.
Toute convergence doit passer par une matrice de mapping, une migration testable, un rollback documenté et des fixtures.
