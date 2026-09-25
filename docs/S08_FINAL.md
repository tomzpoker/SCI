# S08 FINAL — Banque & Rapprochement

S08 est livré comme migration additive `0018`, module Rust `src/banking.rs`, intégration UI et script de contrôle.

La déduplication repose sur une empreinte SHA-256 et sur un identifiant source scindé par compte lorsque nécessaire. Les transactions restent immuables dans leur historique fonctionnel ; la suppression physique n’est pas introduite par ce sprint.

Le moteur distingue cinq niveaux de rapprochement et journalise les changements. Une correspondance de facture sans signaux suffisants reste `À valider` plutôt que de créer automatiquement un encaissement.

Pré-requis runtime : PostgreSQL 17 du projet et, pour PDF/OCR, un moteur local accessible par `SCI_OCR_COMMAND`.
