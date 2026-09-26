# S02 — US-0203 — Cloisonner les données

## Ce qui est livré

US-0203 fait de `legal_entity_id` la portée canonique des données métier existantes, sans supprimer `sci_id` ni casser le vertical slice historique.

Les tables opérationnelles désormais obligatoirement rattachées à une entité juridique sont :

`associates`, `properties`, `units`, `tenants`, `leases`, `invoices`, `payments`, `bank_transactions`, `automation_rules`, `tasks`, `tax_deadlines`, `documents`, `forecast_snapshots`, `app_settings`, `storage_locations`, `llm_accounts`, `email_accounts`, `document_inbox`, `assistant_proposals`.

Les comptes bancaires de société restent gérés directement par `legal_entity_bank_accounts` depuis US-0201.

## Cloisonnement

- toutes les lignes de ces tables portent `legal_entity_id NOT NULL` ;
- les données historiques sont reprises automatiquement depuis `scis.legal_entity_id` ou depuis leur parent déjà cloisonné ;
- les relations structurantes vérifient la paire `(objet, legal_entity_id)` pour empêcher une référence croisée entre sociétés ;
- les règles, tâches, échéances, banque, documents, stockage, e-mail et IA sont filtrés par l'entité active ;
- les écritures Rust utilisent le contexte `current_legal_entity_id()` ;
- l'interface expose un sélecteur d'« Entité active » et bascule la vue métier sur cette portée ;
- `sci_id` reste présent comme compatibilité historique lorsque la table en possédait un ; il devient nullable uniquement pour les tables génériques qui doivent pouvoir servir une SARL sans fabriquer de seconde SCI.

## Limite explicitement conservée

Le schéma historique ne contient actuellement aucune table de stock. US-0203 ne crée donc pas de faux module de stock. Toute future table de stock devra obligatoirement porter `legal_entity_id` et suivre le même modèle de cloisonnement.

## Runtime

Le projet reste un poste local mono-utilisateur. L'entité active est stockée dans le processus serveur et initialisée sur l'entité historique `00000000-0000-0000-0000-000000000010`. Ce choix évite de transmettre une portée arbitraire depuis le navigateur et pourra être remplacé par un contexte de session/authentification lorsque celui-ci sera introduit.

## Migration

`migrations/0008_entity_data_isolation.sql` est additive : aucune table, colonne, index ou schéma n'est supprimé et aucune donnée n'est effacée.

La migration :

1. ajoute `legal_entity_id` ;
2. reprend les données historiques ;
3. bloque la suite si une ligne reste sans entité ;
4. impose `NOT NULL` ;
5. ajoute les FK d'entité ;
6. ajoute les FK composites nécessaires aux relations métier ;
7. ajoute les index et unicités par entité.

## Validation

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-entity-isolation.ps1
```

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\migrate.ps1
```

```powershell
cd D:\SCI\DEV\SCI-family-rust;cargo test --lib --features server;cargo check --features server
```

Puis tester au minimum :

- créer deux entités juridiques différentes ;
- sélectionner l'entité A et constater que ses factures, banque, tâches, documents et règles sont seules visibles ;
- sélectionner l'entité B et constater que la vue bascule sans mélange ;
- créer une banque/règle/document dans B puis revenir sur A ;
- vérifier dans Audit les changements de périmètre.

## Statut

`READY FOR REVIEW` — le gate statique est vérifiable ici ; la migration PostgreSQL et le build Windows doivent encore être exécutés sur le PC cible avant `SHIPPED`.

Prochaine story : `US-0204 — Historiser les changements`.
