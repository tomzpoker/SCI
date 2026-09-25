# VERSION_STATE — SCI Family Rust

## Version projet

- Package : `sci-family-pilot`
- Version : `0.5.0`
- Sprint : `S19`
- Étape : `Étape 1 — multi-sociétés + profils d’exploitation`

## Versions techniques

- Dioxus : `0.7` (CLI attendue : `0.7.10` selon `scripts\start.ps1`)
- PostgreSQL : `17-alpine`
- SQLx : `0.9`
- Toolchain Rust : `1.98.1-x86_64-pc-windows-msvc`
- Target WASM : `wasm32-unknown-unknown`

## Schéma

- Dernière migration : `0029_multi_company_operating_profiles.sql`
- Nombre disponible : `29`

## État de la présente livraison

- Code source : corrigé pour les blockers identifiés dans l'audit statique.
- Tests déterministes : prêts à être exécutés.
- Runtime Windows/MSVC + Docker/PostgreSQL : à certifier sur le PC cible.

## Prochaine étape

`Certification runtime Windows/MSVC + Docker/PostgreSQL`
