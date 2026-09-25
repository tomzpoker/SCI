# S18 — Checklist Release

## Installation vierge

`clone → bootstrap.ps1 → migrate.ps1 → run-tests.ps1 → launch`

## Upgrade

- backup avant changement
- migrations SQLx additives
- vérification de la séquence
- tests après migration
- Doctor après mise à jour

## Rollback

- backup de sécurité préalable
- backup explicitement choisi
- restauration contrôlée
- Doctor obligatoire ensuite
- aucune sélection implicite d'un backup

## Git

- `verify-git-hygiene.ps1`
- `git-sync.ps1`
- pas de push forcé
- pas de secrets, dumps ou données réelles

## Release report

`release.ps1` génère automatiquement version, date, branche, commit, fonctionnalités, migrations, tests, compatibilité et rollback.
