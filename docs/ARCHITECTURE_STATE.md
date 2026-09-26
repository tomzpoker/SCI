# ARCHITECTURE_STATE — SCI Family Rust

## Stack

- Rust 2024
- Dioxus 0.7 fullstack
- SQLx 0.9
- PostgreSQL 17-alpine
- Docker Compose
- PowerShell pour l'exploitation Windows
- Cible Windows canonique : MSVC
- Cible WASM : `wasm32-unknown-unknown`

## Flux

```text
Dioxus UI
   ↓
Server functions / services
   ↓
Domain + infrastructure Rust
   ↓
SQLx
   ↓
PostgreSQL
```

## Migrations

Le projet contient 29 migrations SQL contiguës, de `0001_foundation.sql` à `0029_multi_company_operating_profiles.sql`.
L'application serveur et le binaire `sci-family-migrate` utilisent le même `sqlx::migrate!("./migrations")`.

## Modules métier

SCI/onboarding, associés, biens, lots, locataires, baux, facturation, paiements, banque, TVA, fiscalité, trésorerie, automatisation, documents/OCR, génération/PDF, e-facturation, IA contrôlée, audit et recovery.

## Cloisonnement multi-sociétés

`legal_entity_id` est la portée métier canonique. Le sélecteur d'entité vérifie désormais l'appartenance de l'utilisateur à l'entité active et resynchronise `auth_sessions.legal_entity_id` avant de modifier la portée locale.

## Principes

- évolutions additives ;
- pas de Supabase runtime ;
- fichiers documentaires hors PostgreSQL ;
- validation humaine pour les actions à impact ;
- migrations destructives protégées par les gates ;
- simulations séparées des données réelles.

## Certification restante

Compilation Windows/MSVC, tests DB/E2E réels, démarrage Docker/Dioxus, responsive/PWA et validation opérationnelle complète restent à exécuter sur le poste cible.
