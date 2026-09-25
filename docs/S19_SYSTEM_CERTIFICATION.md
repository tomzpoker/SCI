# S19 — Validation système de bout en bout

## US-1901 — Cycle locatif complet

`SCI → Immeuble → Lot → Locataire → Bail → Loyer → Facture → Encaissement → Banque → Rapprochement → TVA → Fiscalité → Calendrier → Document → Archive`

Le contrat de cycle est versionné dans `fixtures/e2e/locative_cycle.json`.

## US-1902 — Cycle document automatique

`Document entrant → Détection → Hash → Doublon → OCR → Classification → Extraction → Contrôles → Validation → Intégration`

La chaîne réelle existe dans `src/documents/workflow.rs`; la validation humaine précède l'intégration structurée.

## US-1903 — Cycle automation

`Événement → Règle → Calcul → Suggestion → Validation → Exécution → Post-condition → Audit`

Les workflows utilisent idempotence, validation et contrôle de post-condition.

## US-1904 — Reprise développeur

Le dépôt contient état, migrations, fixtures, tests, Doctor, recovery et procédure de release afin qu'un nouveau développeur puisse reprendre sans historique de conversation.

## Certification runtime

La livraison finale fournit les tests et scripts reproductibles. La certification runtime complète doit encore être exécutée sur la machine cible Windows avec PostgreSQL/Docker réels ; cet environnement d’analyse ne dispose ni de Cargo/Rust ni de Docker/PowerShell et ne permet donc pas de certifier l’exécution locale.
