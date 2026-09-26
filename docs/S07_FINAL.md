# S07 — Livraison finale — Facturation

Statut source : **READY FOR REVIEW**.

Le sprint couvre les 8 éléments demandés (les deux `US-0707` sont conservés, le second est nommé `US-0707 bis — e-facturation`).

### Fichiers principaux

- `D:\SCI\DEV\SCI-family-rust\migrations\0017_billing_invoicing.sql`
- `D:\SCI\DEV\SCI-family-rust\src\billing.rs`
- `D:\SCI\DEV\SCI-family-rust\src\server.rs`
- `D:\SCI\DEV\SCI-family-rust\src\ui.rs`
- `D:\SCI\DEV\SCI-family-rust\src\lib.rs`
- `D:\SCI\DEV\SCI-family-rust\scripts\verify-s07-billing.ps1`

### Validation

Les contrôles statiques S07 sont fournis dans `scripts\verify-s07-billing.ps1`.

La compilation Rust et l'exécution réelle de PostgreSQL n'ont pas pu être exécutées dans l'environnement de fabrication. La certification finale reste donc à faire sur le PC Windows du projet avec `cargo test --lib --features server` puis `cargo check --features server`, et avec la migration réellement appliquée.
