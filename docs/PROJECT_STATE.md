# PROJECT_STATE — SCI Family Rust

> État courant de la livraison 0.5.0. Les valeurs Git/DB volatiles peuvent être rafraîchies par `scripts\update-state.ps1`.

## Identité

- Projet : SCI Family Rust / SCI Family Pilot
- Chemin canonique : `D:\SCI\DEV\SCI-family-rust`
- Langage : Rust 2024
- UI : Dioxus 0.7 / fullstack web
- Base locale : PostgreSQL 17 via Docker Compose
- Version déclarée : `0.5.0`
- Phase : `Étape 1 — socle multi-sociétés + profils d’exploitation`
- Story courante : `S19 — Cycle métier consolidé / Release 0.5.0`

## Fonctionnalités présentes

- SCI / onboarding et zéro-saisie ;
- associés, biens, lots, locataires, baux ;
- facturation, paiements, TVA et échéances ;
- banque, import CSV, rapprochement et trésorerie ;
- automatisation, tâches, audit, recovery ;
- documents, stockage externe, OCR, classification, extraction et validation ;
- génération de documents / PDF et e-facturation ;
- assistant IA contrôlé avec outils à impact soumis à validation ;
- profils d’exploitation multi-sociétés, dont `SCI_LOCATIVE_IR` et `SARL_IS_GARAGE_SANS_SAV`.

## Base de données

- Migrations disponibles : `0001` → `0029` (29 fichiers).
- Migration finale actuelle : `0029_multi_company_operating_profiles.sql`.
- Exécution : SQLx `sqlx::migrate!("./migrations")`.

## Sécurité / correctifs appliqués

- Le changement d’entité est limité aux entités auxquelles l’utilisateur possède un rôle actif autorisé ; la session est resynchronisée avec l’entité sélectionnée.
- La validation documentaire recherche désormais l’extraction par `document_id` et utilise le rôle authentifié comme auteur de validation.
- Les opérations documentaires d’écriture exigent `DATA_WRITE`.
- Les snapshots déterminent la dernière migration depuis `_sqlx_migrations`, pas depuis un fichier d’état potentiellement obsolète.

## Tests

- Les tests déterministes sont présents dans `tests/` et exécutés par `scripts\run-tests.ps1`.
- La certification runtime Windows/MSVC + Docker/PostgreSQL doit être exécutée sur le PC cible ; elle n'a pas été exécutée dans l'environnement de fabrication de cette archive.

## Prochain objectif exact

`Certification runtime Windows/MSVC + Docker/PostgreSQL`, puis publication de la release stable.
