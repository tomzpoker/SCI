# SCI Family Rust — SCI Family Pilot

Projet canonique : `D:\SCI\DEV\SCI-family-rust`

Package : `sci-family-pilot` `0.5.0`

## Derniers sprints

S07 Facturation · S08 Banque/Rapprochement · S09 Trésorerie · S10 Fiscalité · S11 Impayés/Recouvrement · S12 Courriers/PDF · S13 E-facturation · S14 IA contrôlée · S15 UX/Zéro-saisie · S16 Audit/Sécurité/Recovery · S17 Tests/Hardening · S18 Release · S19 Cycle métier.

## Récupération PC

1. Restaurer le dépôt dans `D:\SCI\DEV\SCI-family-rust`.
2. Vérifier Docker/PostgreSQL 17.
3. Exécuter `scripts\migrate.ps1`.
4. Exécuter `scripts\doctor.ps1`.
5. Exécuter `scripts\run-tests.ps1`.
6. Pour un contrôle DB/E2E explicite : `scripts\run-tests.ps1 -Database -E2E`.
7. Exécuter `scripts\release.ps1` avant une release.

L’environnement courant reste single-user/local. L’IA peut rester désactivée sans bloquer le métier. Les secrets provider/LLM ne sont jamais stockés en clair dans la base : seules des références sont enregistrées.


## S15 / S16

Le projet inclut désormais UX zéro-saisie, thèmes, onboarding 14 étapes, authentification locale, rôles/permissions, audit enrichi et recovery contrôlé. L’application peut fonctionner sans IA ; AI_AGENT n’est jamais OWNER implicitement et les tools à impact nécessitent une validation humaine.


## S17 → S19

- `TEST_MATRIX_S17.md` : matrice de couverture tests/hardening.
- `RELEASE_CHECKLIST_S18.md` : installation, upgrade, rollback, Git et release report.
- `S19_SYSTEM_CERTIFICATION.md` : cycles système et reprise développeur.
- `DEVELOPER_HANDOFF.md` : procédure de reprise sans historique de conversation.

Le script `scripts\run-tests.ps1` utilise les tests déterministes par défaut. Les tests DB/E2E protégés sont opt-in avec `server,test-auth` et `SCI_RUN_DB_TESTS=1` / `SCI_TEST_AUTH=1`.

## Étape 1 — socle multi-sociétés + garage sans SAV
Le projet ajoute des profils d’exploitation indépendants de la forme juridique. Une SARL à l’IS peut être configurée avec le profil `SARL_IS_GARAGE_SANS_SAV` (véhicules, stock de pièces, dépôt-vente, sans atelier/SAV). Une SCI peut rester `SCI_LOCATIVE_IR`. La base juridique reste `legal_entity_id` et la TVA est configurable par entité (`COLLECTION` ou `DEBIT`).

Voir `ETAPE_01_MULTI_SOCIETES_GARAGE.md` et `PLAN_5_ETAPES_GARAGE_SCI_SARL.md`.

## Démarrage recommandé — Windows

Depuis PowerShell :

`Set-Location D:\SCI\DEV\SCI-family-rust; .\scripts\start.ps1`

Le script crée `.env` depuis `.env.example` si nécessaire, installe la toolchain Rust déclarée, démarre PostgreSQL 17 et lance Dioxus. L’application applique automatiquement les migrations SQLx au démarrage serveur. Pour exécuter les migrations séparément : `Set-Location D:\SCI\DEV\SCI-family-rust; .\scripts\migrate.ps1`.

Tests déterministes : `Set-Location D:\SCI\DEV\SCI-family-rust; .\scripts\run-tests.ps1`

Tests DB/E2E : `Set-Location D:\SCI\DEV\SCI-family-rust; .\scripts\run-tests.ps1 -Database -E2E`

Arrêt sans supprimer les données : `Set-Location D:\SCI\DEV\SCI-family-rust; docker compose down`
