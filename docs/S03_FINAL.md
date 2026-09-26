# S03 FINAL — Référentiels + Rule Engine

Statut source : **COMPLETE / READY FOR REVIEW**

Stories :
- US-0301 — Référentiels versionnés
- US-0302 — Rulesets par périmètre
- US-0303 — Rejouer un calcul historique
- US-0304 — Sources fiscales officielles
- US-0305 — Gérer une évolution de loi
- US-0306 — Rendre les calculs reproductibles

## Architecture livrée

`versioned_references` conserve source, version, période de validité, vérification et statut.

`rule_definitions` porte l'identité d'une règle. `rule_versions` porte chaque version avec scope entité/activité/juridiction, période, source et définition JSON.

`rule_calculation_runs` conserve les entrées et le résultat d'un calcul, ainsi que formule, snapshots de règle/source, arrondi et empreintes de traçabilité. Le replay recharge la RuleVersion et réévalue la définition historique.

Le moteur de résolution privilégie le scope le plus spécifique compatible avec l'entité active, l'activité, la juridiction et la date demandées.

Une RuleVersion ou référence publiée est immuable. Une évolution se fait par nouvelle version. Les versions publiées qui se chevauchent sur un même scope sont refusées.

## Définition Rule Engine

Le DSL minimal fourni est volontairement borné et déterministe :
- `PASS_THROUGH`
- `ADD`
- `SUBTRACT`
- `MULTIPLY_BPS`

Cela fournit le socle d'historisation/replay sans préempter le moteur de calcul générique de S04.

## Certification

Le conteneur de fabrication n'est pas équipé du toolchain Windows/PostgreSQL du PC cible. La migration 0010, le build Rust et les scénarios dynamiques doivent donc être exécutés sur le PC cible avant `SHIPPED`.
