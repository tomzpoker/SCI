# AUDIT_EXISTANT — SCI Family Rust

**Audit de reprise — base fournie : `SCI-family-rust(2).zip`**  
Date de l’audit : 2026-09-24  
Version déclarée du package : `0.4.0`  
Cible normative : SCI Manager v3 / backlog S00 → S23  
Statut : **STATIC AUDIT COMPLETED — REVIEW/DYNAMIC VALIDATION PENDING**

## 1. Objet

Cet audit applique la règle de reprise du cahier des charges maître : inventorier et observer l’existant avant toute réécriture.

Le périmètre observé est strictement le contenu de l’archive fournie. L’audit ne certifie pas les éléments qui nécessitent un environnement Windows/Git/PostgreSQL vivant.

Le cahier des charges impose pour une reprise d’un projet existant : inventaire, compilation, Git, architecture, dépendances, modèles de données, migrations, API, UI, tests, fonctionnalités présentes/absentes, dette technique, incompatibilités et risques, sans réécriture gratuite de l’existant.

## 2. Résumé exécutif

Le projet n’est pas vide. Il possède déjà un vertical slice SCI substantiel :

- application Rust + Dioxus 0.7 ;
- PostgreSQL 17 via Docker Compose ;
- 5 migrations SQL ;
- SCI : profil, associés, biens, lots, locataires, baux ;
- facturation, paiements, banque, import CSV et rapprochement ;
- TVA et échéances ;
- automatisation, tâches et audit ;
- documents avec stockage configurable ;
- intelligence documentaire avec comptes LLM/email, inbox, hash, extraction, classification, OCR et validation ;
- tests d’intégration couvrant 12 blocs métier.

L’écart principal n’est donc pas l’absence de métier : c’est la **continuité reproductible**. L’archive ne contient pas l’ensemble des fichiers d’état/release attendus et ne contient pas `.git`, donc la branche/commit/remote ne sont pas certifiables à partir du ZIP.

### Correction importante de cet audit

**La toolchain Windows cible est désormais MSVC, conformément à la décision actuelle du projet et au cahier des charges maître.**

La référence normative est :

- Rust stable/MSVC, avec la toolchain déclarée par `rust-toolchain.toml` : `1.98.1-x86_64-pc-windows-msvc` ;
- cible `wasm32-unknown-unknown` ;
- environnement de compilation natif Windows avec les composants MSVC / Windows SDK nécessaires ;
- Git for Windows ;
- PowerShell ;
- Docker Desktop pour PostgreSQL local.

La présence historique de réglages GNU/MSYS2 sur certaines machines n’est **pas** la cible du projet et ne doit plus être introduite dans les scripts de continuité.

## 3. Inventaire

### 3.1 Structure observée

Présents dans l’archive :

- `.cargo/config.toml`
- `.env`
- `.env.example`
- `.gitignore`
- `Cargo.toml`
- `Cargo.lock`
- `Dioxus.toml`
- `docker-compose.yml`
- `rust-toolchain.toml`
- `engines/manifest.toml`
- 5 migrations SQL (`0001` à `0005`)
- scripts de bootstrap/start/doctor/check/validation/sauvegarde/push
- sources Rust `application`, `domain`, `infrastructure`, `intelligence`, `server`, `ui`, `main`, `lib`
- sous-modules `src/assistant/*` et `src/documents/*`
- tests d’intégration dont `tests/integration/modules_12.rs`
- répertoires `assets`, `public`, `engines`, `versions`.

L’archive contient 46 fichiers visibles au niveau de l’inventaire réalisé.

### 3.2 Git

Le répertoire `.git` n’est pas présent dans l’archive.

Donc :

- branche : **NON CERTIFIABLE** depuis l’archive ;
- commit : **NON CERTIFIABLE** ;
- remotes : **NON CERTIFIABLES** ;
- état local/distant : **NON CERTIFIABLE** ;
- tags/releases : **NON CERTIFIABLES**.

Cela ne prouve pas que le dépôt Git local du développeur est absent ; cela signifie seulement qu’il n’est pas compris dans le livrable audité.

## 4. Build et environnement

### 4.1 Configuration déclarée

`Cargo.toml` déclare le package `sci-family-pilot` en version `0.4.0`, édition Rust 2024.

`rust-toolchain.toml` déclare :

- `1.98.1-x86_64-pc-windows-msvc` ;
- cible `wasm32-unknown-unknown`.

`docker-compose.yml` définit PostgreSQL 17-alpine :

- utilisateur : `sci` ;
- base : `sci_family` ;
- port hôte : `55432` ;
- volume : `sci_family_pgdata` ;
- healthcheck `pg_isready`.

### 4.2 État de certification build

La compilation réelle n’a pas pu être exécutée dans l’environnement d’audit utilisé pour analyser le ZIP, car Cargo/Rust n’y étaient pas disponibles.

**BUILD = TO_VERIFY sur Windows MSVC.**

Aucune certification de compilation ou de tests ne doit être déduite du seul audit statique.

### 4.3 Dépendances à confirmer par Cargo

`src/intelligence.rs` utilise notamment `futures_util`, `mail_parser`, `sha2`, `lopdf` et `ocr`. Ils ne sont pas tous visibles comme dépendances directes dans l’extrait de `Cargo.toml` observé.

Ce point est volontairement classé **TO_VERIFY** : le verdict doit venir d’un `cargo check` réel sur le workspace, pas d’une correction supposée sur la seule base d’un grep statique.

## 5. Base de données

### 5.1 Migrations présentes

1. `migrations/0001_foundation.sql`
2. `migrations/0002_onboarding_operations.sql`
3. `migrations/0003_operating_model.sql`
4. `migrations/0004_storage_locations.sql`
5. `migrations/0005_intelligence.sql`

### 5.2 Domaine déjà modélisé

La fondation contient notamment :

- `workspaces`
- `scis`
- `associates`
- `properties`
- `units`
- `tenants`
- `leases`
- `invoices`
- `payments`
- `bank_transactions`
- `automation_rules`
- `tasks`
- `tax_deadlines`
- `documents`
- `forecast_snapshots`
- `audit_events`
- `app_settings`.

Les migrations suivantes ajoutent le modèle opérationnel, les emplacements documentaires et la couche intelligence.

### 5.3 Sécurité des migrations

Les migrations observées ne contiennent pas de `DROP TABLE` ni de `TRUNCATE TABLE`.

L’état statique est donc compatible avec le principe anti-destructif. En revanche, les chemins complets `fresh / upgrade / restore / import / test` ne sont pas encore démontrés.

La démonstration de migrations sûres relève de **US-0102**, qui dépend de US-0101.

## 6. API / serveur

`src/server.rs` contient déjà un ensemble important de server functions couvrant notamment :

- dashboard et compteurs de modules ;
- profil SCI et onboarding ;
- associés, biens, lots, locataires, baux ;
- factures, paiements ;
- banque, import CSV, rapprochement ;
- TVA et échéances ;
- documents ;
- assistant ;
- automatisation, tâches, audit ;
- cycle d’anticipation.

Le vertical slice SCI est donc déjà largement amorcé et ne doit pas être remplacé par une nouvelle base métier sans justification.

## 7. Documents / intelligence

`src/intelligence.rs` implémente déjà un flux conceptuel de type :

**entrée document → hash → staging → extraction texte/OCR → classification → extraction de champs → inbox → validation → stockage final → audit**.

Le code comprend également les comptes LLM/email et des mécanismes de validation/approbation.

Cette base doit être conservée et durcie progressivement.

## 8. UI

`src/ui.rs` expose les principaux modules opérationnels et des actions de création, suppression, émission, rapprochement et validation.

Le frontend est Rust/Dioxus. L’archive ne montre pas de gros frontend JavaScript autonome.

Non certifié à ce stade :

- responsive smartphone ;
- PWA ;
- accessibilité complète ;
- Design System formel ;
- conformité UX exhaustive avec tous les contrats d’action.

Ces sujets restent du backlog et ne justifient pas une réécriture du vertical slice existant.

## 9. Tests

`tests/integration/modules_12.rs` couvre un parcours intégré en 12 blocs :

1. SCI / profil ;
2. associés ;
3. propriété ;
4. unité/lot ;
5. locataire ;
6. bail ;
7. facturation ;
8. paiements ;
9. banque ;
10. TVA / échéances ;
11. automatisation ;
12. documents / pilotage.

**TESTS = présents, mais exécution non certifiée dans l’environnement d’audit.**

Ce test ne constitue pas encore la couverture E2E multi-sociétés du futur S22.

## 10. Continuité projet

Le cahier des charges exige des fichiers d’état et de release permettant une reprise sans historique conversationnel.

Absents de l’archive fournie au moment de l’audit :

- `README.md`
- `CHANGELOG.md`
- `PROJECT_STATE.md`
- `ARCHITECTURE_STATE.md`
- `ROADMAP_STATE.md`
- `DATABASE_STATE.md`
- `VERSION_STATE.md`
- `DATABASE_MAPPING.md`
- `PHASE_1_STATUS.md`
- `versions/0.5.0.md`

`AUDIT_EXISTANT.md` est le livrable de US-0005 et est donc ajouté par le présent audit.

## 11. Incohérences de continuité observées

Le code applicatif déclare `0.4.0`.

Certains scripts évoquent déjà `0.5.0-dev` ou attendent un rapport `versions/0.5.0.md`, alors que ces artefacts ne sont pas fournis dans l’archive auditée.

Ce décalage doit être traité comme **un sujet de continuité/versioning**, sans augmenter artificiellement la version du package avant validation.

La version `0.4.0` reste donc la version déclarée de l’existant audité.

## 12. Toolchain Windows — décision finale pour la reprise

La décision de projet est maintenant explicite : **MSVC est la toolchain Windows canonique.**

Le bootstrap S01 doit donc :

- détecter Rustup/Cargo ;
- vérifier la toolchain MSVC épinglée par le projet ;
- vérifier `wasm32-unknown-unknown` ;
- détecter les composants natifs MSVC/Windows SDK nécessaires ;
- ne pas ajouter de chemin MSYS2/GNU ;
- détecter Git ;
- détecter Docker et son daemon ;
- démarrer PostgreSQL local ;
- créer `.env` depuis `.env.example` seulement s’il est absent ;
- produire un rapport de santé versionné dans `snapshots/`.

Le bootstrap ne doit pas exécuter les migrations de US-0102, afin de préserver la séparation des User Stories.

## 13. Sécurité / secrets

L’archive contient `.env`, mais son contenu n’est pas reproduit dans cet audit.

`.gitignore` exclut `.env`.

Avant un push/release réel, il reste obligatoire de vérifier que des secrets réels n’ont jamais été commités dans l’historique Git distant.

## 14. Dette / risques

### BLOCKING pour certification de continuité

1. Git/branche/commit non certifiables depuis l’archive.
2. Fichiers d’état/release absents.
3. Build et tests non exécutés dans l’environnement d’audit.
4. Bootstrap actuel trop déclaratif pour constituer à lui seul un health-check reproductible.
5. Certains scripts de continuité contenaient encore des hypothèses GNU/MSYS2 : elles doivent être supprimées ou rendues neutres pour la cible MSVC.

### HIGH

1. Valider les dépendances Cargo de l’intelligence documentaire.
2. Valider les 5 migrations sur PostgreSQL 17.
3. Tester clone → bootstrap sur une machine Windows MSVC propre.
4. Vérifier les données et contrats multi-entités avant S02.

### MEDIUM

1. `src/server.rs` est concentré ; un découpage progressif pourra être fait sans réécriture massive.
2. Design System formel à matérialiser.
3. Moteurs de règles/fiscalité versionnés à approfondir.
4. E-invoicing, SARL/IS, stock, consolidation et multi-sociétés complète restent à construire.

## 15. Ce qui doit être conservé

Ne pas repartir de zéro sur :

- domaine SCI existant ;
- migrations existantes ;
- server functions ;
- facturation/paiement/banque ;
- automatisation ;
- documents/intelligence ;
- test intégré 12 modules ;
- Docker PostgreSQL.

La stratégie de reprise est l’amélioration incrémentale et vérifiable.

## 16. Statut US-0005

**US-0005 — Auditer l’existant : AUDIT STATIC COMPLETED.**

Le prochain livrable est **US-0101 — Bootstrap reproductible**.

Le cycle obligatoire reste :

**Research → Design → Plan → Execute → Review → Ship**.

## 17. Prochain livrable exact

### US-0101 — Bootstrap reproductible

Objectif concret : rendre le poste reconstructible à partir du dépôt, sans dépendre de l’historique de conversation.

Sortie attendue :

**clone → bootstrap → environnement MSVC → Docker/PostgreSQL → rapport santé → READY**

Les migrations et la vérification exhaustive de recovery restent dans les stories S01 suivantes.

## 18. Commande de validation dynamique

Sur Windows, depuis :

`D:\SCI\DEV\SCI-family-rust`

Commande diagnostic :

```powershell
cd D:\SCI\DEV\SCI-family-rust;git status --short --branch;git remote -v;rustc --version;cargo --version;docker compose ps;cargo check --features server
```

Puis le bootstrap S01 :

```powershell
cd D:\SCI\DEV\SCI-family-rust;.\scripts\bootstrap.ps1
```

Après bootstrap, le health-report produit dans `snapshots/` constitue l’artefact de contrôle US-0101.

## 19. Verdict d’état

| Domaine | État | Commentaire |
|---|---|---|
| Rust | PRESENT | Projet Rust/Dioxus structuré |
| Toolchain Windows | PRESENT / TARGET MSVC | MSVC est la cible canonique |
| DB | PRESENT | PostgreSQL 17 + 5 migrations |
| Domaine SCI | PRESENT | Vertical slice déjà large |
| Banque | PRESENT | CSV + rapprochement |
| TVA | PRESENT | Résumé TVA + échéances |
| Automatisation | PRESENT | Règles + cycle d’anticipation |
| Documents | PRESENT | Registre + stockage |
| OCR/IA | PRESENT | Couche intelligence amorcée |
| Tests | PRESENT | Intégration 12 modules |
| Git | NON CERTIFIABLE | `.git` absent de l’archive |
| Build | TO_VERIFY | Pas exécuté dans l’environnement d’audit |
| Continuité | INCOMPLETE | Fichiers d’état absents |
| Bootstrap | PARTIAL → S01 | À rendre vérifiable et producteur de rapport |
| Multi-sociétés | PARTIAL | Socle actuel principalement SCI |

**AUDIT_EXISTANT = COMPLETED / DYNAMIC VALIDATION REQUIRED**
