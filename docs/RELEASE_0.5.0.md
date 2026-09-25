# Release 0.5.0

Statut : livraison finale du code, tests runtime à certifier sur l’environnement cible.

Date : générée par `scripts/release.ps1` au moment de la release effective.
Branche : générée par `scripts/release.ps1`.
Commit : généré par `scripts/release.ps1`.

Objectif : consolider S01→S19 dans un cycle reproductible de développement, test, release, recovery et validation métier de bout en bout.

Fonctionnalités : tests/hardening, installation vierge, upgrade, rollback, Git Sync, secret protection, cycle locatif/document/automation et handoff développeur.

Migrations : 0001 → 0029.

Correctifs release : cloisonnement multi-sociétés, validation OCR/extraction, snapshots indexés sur SQLx, gates de migration et scripts de recovery resynchronisés.
Breaking changes S17→S19 : aucune migration destructive introduite.
Compatibilité : Windows, Docker Compose, PostgreSQL 17, Rust toolchain déclarée.
Rollback : backup de sécurité préalable + restauration contrôlée.
Prochaine étape : exécuter la certification runtime sur Windows/Docker avant publication opérationnelle.
