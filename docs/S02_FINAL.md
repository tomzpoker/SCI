# S02 FINAL — Core multi-sociétés

**Projet :** SCI Family Rust  
**Chemin canonique :** `D:\SCI\DEV\SCI-family-rust`  
**Date :** 2026-09-24  
**Statut source :** **SPRINT COMPLETE — READY FOR REVIEW**

## Stories livrées

| Story | Sujet | Résultat |
|---|---|---|
| US-0201 | Gérer les entités juridiques | Entités SCI/SARL, fiscalité, TVA, banques |
| US-0202 | Gérer les activités d'une SARL | Plusieurs activités dans une même SARL, activité principale |
| US-0203 | Cloisonner les données | `legal_entity_id` obligatoire + contexte actif + FKs de portée |
| US-0204 | Historiser les changements | Date d'effet, auteur, avant/après, motif, métadonnées |
| US-0205 | Réutiliser les moteurs communs | Contrat commun Calcul/Event/Workflow/Bank/VAT/Document/Cash/Audit |

## Migrations ajoutées pendant S02

- `0006_legal_entities.sql`
- `0007_sarl_activities.sql`
- `0008_entity_data_isolation.sql`
- `0009_entity_change_history.sql`

## Modules structurants S02

- `src/entity_scope.rs`
- `src/sarl_activities.rs`
- `src/history.rs`
- `src/engines.rs`

## Point de vérité

Le sprint est terminé côté conception et code source. La certification `SHIPPED` est volontairement laissée au test réel sur le PC Windows/PostgreSQL cible : migration, `cargo test --lib --features server`, `cargo check --features server` et scénario multi-sociétés.

## Commandes de fin de sprint

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-entity-isolation.ps1
```

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-change-history.ps1
```

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-common-engines.ps1
```

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\migrate.ps1
```

```powershell
cd D:\SCI\DEV\SCI-family-rust;cargo test --lib --features server;cargo check --features server
```
