# AUDIT_EXISTANT — SCI Manager Rust

Date d'audit : 2026-09-22
Version déclarée : 0.4.0
Archive auditée : `SCI-feat-rust-family-storage`
Source BDD comparée : `example bdd/20260915060000_baseline.sql`
Référentiel comparé : `example bdd/20260915070000_reference_data_baseline.sql`

## 1. État de contrôle

| Domaine | État | Qualité | Risque | Action |
|---|---|---|---|---|
| Rust | Présent | Moyen | Élevé | Restaurer une validation locale `cargo check/test` et verrouiller l'architecture |
| UI | Dioxus fullstack | Moyen | Moyen | Conserver tant que l'équivalence avec l'exigence Rust/WASM est documentée |
| DB runtime | PostgreSQL + 4 migrations | Moyen | Élevé | Reconcevoir la cible autour du modèle de données de référence sans migration destructive |
| BDD d'exemple | 86 tables de référence opérationnelle détectées | Élevé | Critique si ignorée | Construire une matrice de correspondance/import |
| API | Server functions Rust | Moyen | Élevé | Séparer services métier, transactions, autorisations et effets de bord |
| Événements | Abstraction applicative absente | Faible | Critique | Implémenter Event/Stream/Handler avant automatisations avancées |
| Workflow | Modèle partiel via tâches | Faible | Élevé | Introduire Process/State/Transition/Approval |
| Documents/OCR | Squelettes présents | Faible | Élevé | Compléter pipeline hash → doublon → type → OCR → extraction → validation |
| Banque | Import et rapprochement simplifiés | Moyen | Élevé | Réintroduire comptes, lots d'import et allocations de rapprochement |
| Fiscalité | TVA simplifiée | Faible | Critique | Introduire Rules/RuleVersion/EffectivePeriod et calculs purs |
| Trésorerie | Prévision 12 mois simplifiée | Faible | Élevé | Séparer certain/probable/hypothèse/scénario et flux réel/prévisionnel |
| Audit | `audit_events` simple | Moyen | Élevé | Passer à un journal append-only avec before/after/source/reason/validation |
| Tests | Aucun répertoire de tests détecté | Faible | Critique | Créer unit/integration/workflow/regression/property/import/security tests |
| Continuité | Fichiers d'état absents | Faible | Élevé | Créer PROJECT_STATE / DATABASE_STATE / ARCHITECTURE_STATE / ROADMAP_STATE / VERSION_STATE |
| Git | Métadonnées `.git` absentes de l'archive | Inconnu | Élevé | Reconnecter le dépôt et documenter branche/commit réels |

## 2. Compilation et tests

La validation automatique n'a pas pu être exécutée dans l'environnement d'audit : le binaire `cargo` n'est pas installé ici.

Conséquence : aucune affirmation de type `cargo check PASS` ou `cargo test PASS` n'est faite dans cet audit.

Commande de reprise attendue sous Windows :

```powershell
cargo check --features server
cargo test --features server
```

## 3. Structure Rust constatée

Le projet contient notamment :

- `src/domain.rs`
- `src/application.rs`
- `src/server.rs`
- `src/infrastructure.rs`
- `src/ui.rs`
- `src/documents/*`
- `src/assistant/*`
- `migrations/0001..0004`
- `scripts/bootstrap.ps1`
- `scripts/check-project.ps1`
- `scripts/push-to-github.ps1`
- `scripts/start.ps1`

Le projet est déclaré en Rust 2024, version 0.4.0, avec Dioxus 0.7 et PostgreSQL/SQLx.

## 4. Écart critique avec la BDD d'exemple

La BDD d'exemple contient 86 tables réparties notamment entre :

- `sci_events`
- `sci_finance`
- `sci_property`
- `sci_projections`
- `sci_billing`
- `sci_documents`
- `sci_banking`
- `sci_reference`
- `sci_scheduler`
- `sci_workflow`
- `sci_audit`

Les migrations Rust actuelles portent 18 tables simplifiées non qualifiées par schéma.

Conclusion : la migration actuelle n'est pas une migration technique de la BDD d'exemple. C'est un modèle parallèle simplifié.

Il ne faut donc pas continuer en ajoutant des champs au modèle simplifié sans décision d'architecture explicite. La prochaine étape doit être une matrice de correspondance et une stratégie de convergence.

## 5. Architecture déjà présente dans la BDD d'exemple

La BDD de référence possède déjà des briques structurantes importantes :

- événements versionnés : `sci_events.events`, `stream_heads`, `event_upcasters` ;
- workflow : `sci_workflow.process_states`, `saga_transitions`, `saga_transition_log` ;
- finance : comptes, journaux, allocations, fournisseurs, TVA, flux fiscaux ;
- banque : comptes, lots d'import, transactions, allocations de rapprochement ;
- facturation : registre des factures, séquences, historique de statuts, relances, e-facturation ;
- documents : documents et liens métier ;
- projections : soldes, risques, patrimoine, fiscalité, trésorerie ;
- référentiel : règles, indices, calendriers, types, méthodes ;
- scheduler : obligations et tâches récurrentes.

Cette architecture est cohérente avec le principe du cahier des charges demandant un système événementiel, versionné, auditable et évolutif.

## 6. Points forts à conserver dans le Rust actuel

- cœur applicatif Rust ;
- séparation `domain/application/infrastructure/server/ui` ;
- PostgreSQL via SQLx ;
- migrations versionnées et additives ;
- import CSV bancaire idempotent au niveau applicatif ;
- transaction lors du rapprochement bancaire ;
- journal d'audit existant ;
- abstraction minimale pour assistant, OCR et documents ;
- configuration de stockage documentaire ajoutée sans supprimer `documents.storage_key`.

## 7. Risques fonctionnels constatés

### 7.1 Historique
Plusieurs endpoints utilisent des suppressions physiques (`DELETE`) sur les objets métier. Cela est incompatible avec la cible d'historisation forte du projet lorsqu'une suppression est juridiquement ou fonctionnellement sensible.

### 7.2 Événementiel
Les automatisations actuelles créent directement des tâches depuis des règles planifiées. Le chaînage événement → handler → workflow → validation → résultat n'est pas encore le moteur central.

### 7.3 Fiscalité
Le calcul TVA actuellement présent est un calcul opérationnel simplifié basé sur les paiements et factures. Il ne constitue pas encore le moteur fiscal versionné demandé par le cahier des charges.

### 7.4 Prévision
La prévision actuelle projette principalement les loyers et initialise les sorties à zéro. Elle ne couvre donc pas encore le modèle certain/probable/hypothèse/scénario exigé.

### 7.5 Assistant
L'assistant actuel est essentiellement un routeur lexical de demandes. Il ne dispose pas encore d'une chaîne complète `Intent → Permission Check → Business Tool → Result → Explanation`.

### 7.6 Documents
Les modules OCR/classification/extraction existent comme contrats de données, mais le pipeline opérationnel complet et la persistance des états de validation restent à construire.

## 8. Décision d'architecture recommandée par l'audit

Ne pas réécrire immédiatement le projet Rust.

Ne pas transformer non plus la BDD d'exemple en migrations massives aveugles.

Procédure retenue :

1. stabiliser le socle Rust et sa reproductibilité ;
2. produire la matrice BDD source → modèle cible ;
3. définir le modèle canonique Rust/SQL ;
4. intégrer en premier les sous-systèmes structurels `events`, `workflow`, `reference/rules`, `audit` ;
5. migrer progressivement les fonctionnalités métier ;
6. conserver des migrations additives et réversibles ;
7. seulement ensuite enrichir UI, OCR avancé et IA.

## 9. Priorité immédiate

**Phase 0 : audit — terminée au niveau statique/documentaire.**

**Blocage de validation :** compilation/tests non exécutables dans l'environnement actuel faute de `cargo`.

**Prochaine tâche exacte :** créer la matrice de correspondance complète entre les 86 tables de la BDD d'exemple et les modèles actuels, en classant chaque objet `KEEP / MERGE / REPLACE / NEW / OBSOLETE / TO_VERIFY`, sans modifier les données sources.
