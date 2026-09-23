# SCI Family Pilot — Checkpoint technique du 22/09/2026

## Statut
CHECKPOINT VALIDÉ — état local fonctionnel et testable

## Version
0.4.0

## Branche source
feat/rust-family-storage

## Commit source
32ec80a — Fix working dashboard overview

## Environnement validé
- Windows
- Rust stable x86_64-pc-windows-gnu
- rustc 1.98.1
- Dioxus 0.7.10
- PostgreSQL 17 Alpine
- Base : sci_family
- Port applicatif : 127.0.0.1:8080

## Validation
- cargo check --features server : PASS
- cargo test --features server --test integration -- --nocapture : PASS — 12/12 modules
- dx build --platform web : PASS — client et serveur construits
- serveur Dioxus : démarre correctement
- application web : accessible et fonctionnelle sur http://127.0.0.1:8080

## Fonctionnel actuellement validé
- socle Rust/Dioxus
- PostgreSQL et infrastructure de données
- configuration SCI
- associés
- patrimoine / biens / unités
- locataires
- baux
- facturation
- paiements
- banque et rapprochement
- TVA
- échéances
- automatisations
- documents
- tableau de bord
- onboarding
- anticipation
- tests d'intégration des 12 modules

## Architecture
Le projet actif est D:\SCI\DEV\SCI-family-rust. Le code applicatif est organisé autour des modules domain, application, infrastructure, server, assistant, documents et ui.

## État UI
L'application possède désormais une interface SCI dédiée avec navigation métier, tableau de bord, métriques, modules et prévision de trésorerie. La prochaine étape est l'amélioration UI/UX et la finition visuelle, pas une reconstruction du socle.

## Warnings connus
Le build reste vert malgré plusieurs warnings Rust non bloquants et un warning du bundler Dioxus concernant la désérialisation d'un asset. Aucun de ces warnings n'empêche actuellement le build ou le démarrage de l'application.

## Documents antérieurs
Les fichiers PHASE_1_STATUS.md, PROJECT_STATE.md et CHANGELOG.md contiennent encore des formulations correspondant à des états antérieurs de l'audit. Ce checkpoint constitue la photographie technique validée du 22/09/2026.

## Règle pour la suite
Ne pas repartir de zéro et ne pas reconstruire les fichiers existants sans inspection ciblée. Toute évolution doit partir de ce checkpoint et préserver les tests 12/12.



## NEXT DEVELOPER HANDOFF

### Objectif immédiat
Poursuivre la finition UI/UX de SCI Family Pilot à partir de l'interface Rust/Dioxus actuellement fonctionnelle. Ne pas reconstruire le socle technique.

### Première action
Inspecter assets/main.css avant toute modification visuelle. L'interface existante dans src/ui.rs utilise notamment app-shell, sidebar, topbar, hero-card, metric-row, metric-card, module-card, forecast-grid, forecast-card, setup-banner et page-intro.

### Ordre de travail recommandé
1. Vérifier l'état visuel actuel avec l'application lancée.
2. Inspecter src/ui.rs et assets/main.css ensemble pour identifier les écarts UI/UX.
3. Améliorer d'abord le Dashboard / Accueil SCI.
4. Harmoniser ensuite les pages métier et les composants réutilisables.
5. Vérifier le responsive desktop/mobile.
6. Ne modifier le backend ou le modèle de données que si un besoin UI réel le justifie.

### Contraintes de stabilité
- Ne pas remplacer ou reconstruire massivement src/ui.rs.
- Ne pas remplacer tests/integration/modules_12.rs par une version simplifiée.
- Préserver les 12 modules d'intégration et leur couverture existante.
- Toute modification fonctionnelle doit conserver cargo test --features server --test integration -- --nocapture à 12/12.
- Après une modification significative, exécuter cargo check --features server, puis les tests d'intégration et dx build --platform web.
- Ne pas supprimer les migrations ou les données existantes pour résoudre un problème applicatif.
- Inspecter le code existant avant toute correction ciblée.

### État fonctionnel à préserver
Le serveur, PostgreSQL, le dashboard, la configuration SCI, associés, patrimoine, unités, locataires, baux, facturation, paiements, banque/rapprochement, TVA, échéances, automatisations, documents, onboarding et anticipation sont actuellement validés par les tests et/ou l'application.

### Warnings connus
Les warnings Rust et le warning du bundler Dioxus sont connus et non bloquants à ce stade. Ne pas entreprendre une mise à niveau de Dioxus ou une modification du pipeline de build uniquement pour ces warnings sans reproduire d'abord un problème fonctionnel.

### Git / sauvegarde
Le checkpoint est publié sur origin/checkpoint/rust-family-phase1-stable-20260922. Le script scripts/push-to-github.ps1 force actuellement la branche main : ne pas l'utiliser pour pousser ce checkpoint ou une branche de travail sans l'avoir préalablement adapté.

### Définition du prochain jalon
Le prochain jalon doit conserver le socle fonctionnel actuel, améliorer l'interface SCI dédiée et produire une nouvelle validation complète : tests d'intégration 12/12 + build web réussi + application démarrable.
