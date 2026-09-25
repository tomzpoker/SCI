# S02 — US-0205 — Réutiliser les moteurs communs

## Livré

Un contrat commun est introduit dans `src/engines.rs` pour les moteurs Calcul, Événements, Workflow, Banque, TVA, Documents, Trésorerie et Audit. Le contrat porte explicitement le `workspace_id` et le `legal_entity_id`, ainsi que la forme juridique, le régime fiscal et la configuration TVA/devise.

Les fonctions communes ne dépendent pas du code `SCI` historique. Le runtime charge le contexte de l'entité active et le réutilise pour le tableau de bord, la TVA, la banque, les documents, l'audit et le moteur d'anticipation.

La page Audit expose le catalogue des huit moteurs sur l'entité courante.

Aucune copie de moteur par société n'est créée.

## Validation

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-common-engines.ps1
```

```powershell
cd D:\SCI\DEV\SCI-family-rust;cargo test --lib --features server;cargo check --features server
```

Statut : `READY FOR REVIEW` jusqu'au build réel Windows/PostgreSQL.
