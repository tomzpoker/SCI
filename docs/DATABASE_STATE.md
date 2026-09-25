# DATABASE_STATE — SCI Family Rust

## Moteur

- PostgreSQL : `17-alpine`
- Service Docker : `postgres`
- Base : `sci_family`
- Utilisateur : `sci`
- Port hôte : `55432`
- Historique : `_sqlx_migrations`

## Migrations disponibles

Nombre : **29**

- Première : `0001_foundation.sql`
- Dernière : `0029_multi_company_operating_profiles.sql`
- Séquence attendue : `0001..0029`

La liste complète est dans `migrations/`. Le gate `tests/migration_manifest.rs` vérifie la contiguïté et le caractère non destructif des migrations S15→S19.

## Modèle métier

Le schéma couvre notamment : entités juridiques, activités/profils d'exploitation, associés, biens/lots, locataires/baux, facturation/paiements, banque/rapprochement, TVA/fiscalité, trésorerie, workflows, documents/OCR, génération, e-facturation, IA, audit, authentification, sauvegardes et recovery.

## Migration runtime

L'application applique automatiquement les migrations au premier établissement de la connexion serveur via `sqlx::migrate!("./migrations")`. Le binaire explicite `sci-family-migrate` permet de les exécuter séparément.

## Sécurité

Les migrations sont additives. Aucun reset de données n'est effectué par l'application au démarrage. `docker compose down` conserve le volume PostgreSQL ; ne pas utiliser `down -v` sauf remise à zéro volontaire.

## Certification restante

L'état réellement appliqué à une base cible doit être vérifié avec `scripts\update-state.ps1` ou `scripts\migrate.ps1` après démarrage de Docker.
