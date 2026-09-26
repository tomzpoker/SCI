# Audit final statique — SCI Family Pilot 0.5.0

## Périmètre contrôlé

- Package Rust : `sci-family-pilot` 0.5.0
- Migrations : `0001..0029` (29 fichiers, séquence contiguë)
- Sprints couverts : S01→S19
- Cibles documentées : Windows + Docker Compose + PostgreSQL 17

## Correctifs intégrés

1. **Gate migrations** : le test de manifeste accepte la migration `0029`; le gate S15/S16 conserve volontairement `0001..0028` comme baseline historique.
2. **État release** : `PROJECT_STATE.md`, `ARCHITECTURE_STATE.md`, `ROADMAP_STATE.md`, `DATABASE_STATE.md` et `VERSION_STATE.md` sont réalignés sur 0.5.0 / S19 / migration 0029.
3. **Snapshot** : `create_snapshot` lit la dernière migration réellement appliquée via `_sqlx_migrations` au lieu d’une valeur codée en dur ou d’un document d’état.
4. **Cloisonnement multi-sociétés** : le changement d’entité exige une appartenance `auth_user_roles`, une entité active et `DATA_READ`; la session DB est mise à jour avec la même entité.
5. **Sessions sur entités désactivées** : une session liée à une entité inactive n’est plus reconnue; la connexion sélectionne uniquement une entité active accessible.
6. **Administration des entités** : création, modification, activation/désactivation et lecture des comptes bancaires vérifient l’accès de l’utilisateur à l’entité cible.
7. **Validation documentaire** : la validation recherche l’extraction par `document_id` et utilise le rôle authentifié pour l’audit/validation.
8. **Sélecteur d’entité UI** : il s’initialise sur `AuthStatusItem.legal_entity_id` au lieu de toujours revenir à l’ancienne SCI.
9. **Audit changement d’entité** : les paramètres SQL sont cohérents et l’événement enregistre l’utilisateur courant.
10. **Audit entités juridiques** : le faux acteur `MANAGER` codé en dur est remplacé par le rôle réellement authentifié.
11. **Scripts** : release/recovery/migrate/start et gates statiques sont réalignés sur la migration et la version actuelles.

## Vérifications effectuées ici

- inventaire et contiguïté des 29 migrations ;
- contrôle des références actives 0.5.0 / 0029 ;
- contrôle statique des requêtes de session, permissions, scope entité et validation documentaire ;
- contrôle des scripts de démarrage/test/release ;
- ajout de tests source-level couvrant les correctifs de sécurité et de scope.

## Limite de certification

L’environnement d’analyse ne contient pas `cargo`, `rustc`, `rustfmt`, Docker Desktop ni PowerShell. Aucun build Rust, test Cargo, démarrage PostgreSQL ou cycle E2E réel ne peut donc être revendiqué comme exécuté ici. La certification runtime doit être faite sur Windows avec les prérequis du projet.

## Commandes de certification sur la machine cible

```powershell
Set-Location D:\SCI\DEV\SCI-family-rust; .\scripts\start.ps1
Set-Location D:\SCI\DEV\SCI-family-rust; .\scripts\run-tests.ps1
Set-Location D:\SCI\DEV\SCI-family-rust; .\scripts\run-tests.ps1 -Database -E2E
Set-Location D:\SCI\DEV\SCI-family-rust; .\scripts\doctor.ps1
```

La commande `start.ps1` crée `.env` depuis `.env.example` si nécessaire, démarre PostgreSQL et lance `dx serve --web`; le serveur applique les migrations SQLx au démarrage.

## R1 — Correctifs après compilation Windows

Corrections appliquées à la suite du retour de compilation `dx serve --web` du 25/09/2026 :

1. `src/assistant/control.rs` : la requête SQL de création de validation IA utilise désormais une raw string Rust afin que le JSON `["AI"]` / `{"status":"EXECUTED"}` ne casse plus le parseur Rust.
2. `src/documents/workflow.rs` : le composant `DocumentsWorkflowPage` a été réécrit avec des fermetures RSX explicites et équilibrées.
3. Contrôle structurel complémentaire : corrections des fermetures manquantes dans `src/workflow.rs`, `src/leases.rs` et `src/generation/mod.rs`.
4. `src/fiscal.rs` : composant `FiscalPage` réécrit proprement pour supprimer les fermetures artificielles et garantir une structure RSX lisible et équilibrée.
5. Audit lexical de tous les fichiers `src/**/*.rs` : aucun délimiteur `{}`, `()`, `[]` déséquilibré détecté après les corrections.

### Limite R1

Le compilateur Rust/Dioxus n'est pas disponible dans l'environnement de préparation de cette archive. Le retour Windows fourni par l'utilisateur est donc la preuve runtime des deux erreurs initiales ; les contrôles R1 supplémentaires sont statiques. La certification finale reste à exécuter sur la machine Windows cible.
