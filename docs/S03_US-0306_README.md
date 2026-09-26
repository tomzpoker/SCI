# US-0306 — Rendre les calculs reproductibles

Livrée avec le renforcement de `rule_calculation_runs` dans la migration `0011_reproducible_calculations.sql`.

Chaque run conserve :
- entrées ;
- règle et version ;
- snapshot de la définition ;
- source ;
- formule lisible ;
- résultat ;
- mode d'arrondi ;
- empreintes de traçabilité ;
- statut/rejeu.

`src/reproducibility.rs` expose `explain_calculation_run` pour restituer le dossier explicatif d'un résultat.
