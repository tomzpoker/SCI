# S01 — US-0105 — Doctor / Recovery

**Projet :** SCI Family Rust  
**Chemin canonique :** `D:\SCI\DEV\SCI-family-rust`  
**Sprint :** S01 — Socle technique + continuité  
**Statut livrable :** **IMPLEMENTED — REVIEW WINDOWS/MSVC REQUIRED**

## Objectif

Détecter les pannes récupérables et fournir des réparations sûres sans effacer de données, sans modifier Git et sans introduire de chemin GNU/MSYS2.

## Doctor

Commande :

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\doctor.ps1
```

Le doctor est **read-only**. Il produit :

`D:\SCI\DEV\SCI-family-rust\snapshots\doctor-YYYYMMDD-HHMMSS.md`

Il vérifie :

- Rust / Cargo / Rustup et cible MSVC ;
- Git, branche, commit, remotes sans afficher les URLs ;
- Docker / PostgreSQL ;
- historique SQLx `_sqlx_migrations` et nombre de migrations ;
- documents et racines de stockage actives ;
- manifeste des moteurs ;
- `.env`, `.env.example` ;
- PROJECT_STATE / ARCHITECTURE_STATE / ROADMAP_STATE / DATABASE_STATE / VERSION_STATE.

Les avertissements et erreurs de diagnostic reçoivent un `error_id=ERR-...`.

## Recovery

Le recovery fonctionne en deux modes.

### Dry-run

Aucune réparation n'est appliquée sans `-Apply`.

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\recovery.ps1 -AllSafeRepairs
```

### Réparation sûre de l'environnement

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\recovery.ps1 -Apply -AllSafeRepairs
```

Cette réparation peut :

- créer `.env` seulement s'il est absent, à partir de `.env.example` ;
- créer les répertoires techniques manquants ;
- créer la racine d'inbox documentaire locale si elle manque ;
- démarrer/attendre PostgreSQL avec Docker Compose.

Elle ne peut pas :

- supprimer une base ou un volume Docker ;
- supprimer des documents ;
- réinitialiser Git ;
- écraser un `.env` ;
- écraser un fichier d'état ;
- inventer un manifeste moteur ;
- modifier une migration existante.

## Migration en récupération

La migration est volontairement séparée du mode « toutes les réparations sûres » car elle peut modifier la structure de la base.

Elle doit être explicitement demandée :

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\recovery.ps1 -Apply -RepairPostgres -ApplyMigrations
```

Le recovery délègue à `scripts\migrate.ps1`, donc au gate US-0102, avec son backup préalable et sa politique anti-destructive.

## Validation statique

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-doctor-recovery.ps1
```

## Principe important pour la continuité

Le doctor peut constater qu'un fichier d'état manque, mais le recovery ne crée pas automatiquement un état inventé. Un fichier `PROJECT_STATE.md` doit toujours refléter un état observé, jamais une supposition.

## Definition of Done US-0105

- doctor et recovery présents ;
- diagnostic Git sans fuite d'URL de remote ;
- DB / migrations vérifiées ;
- documents / moteurs / configuration vérifiés ;
- réparations explicitement bornées et non destructives ;
- rapports `snapshots/` ;
- gate statique inclus ;
- documentation et états mis à jour ;
- tests Windows/MSVC à exécuter avant `SHIPPED`.

## Prochaine story

Après validation de S01, le backlog normatif passe à **S02 — Core multi-sociétés**, en commençant par `US-0201 — Gérer les entités juridiques`.
