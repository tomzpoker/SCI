# SCI Family Pilot 0.1.0

Application neuve en Rust pour piloter une SCI familiale à l'IR avec TVA sur encaissements.

## Ce qui existe dans cette version

- cockpit de pilotage ;
- configuration de la SCI ;
- indicateurs connectés à PostgreSQL ;
- espaces Patrimoine, Locations, Facturation, Banque, TVA, Calendrier fiscal ;
- moteur d'anticipation idempotent ;
- règles d'automatisation ;
- journal d'audit ;
- prévision de trésorerie ;
- architecture Dioxus Fullstack + PostgreSQL/SQLx.

La base est volontairement neuve : aucune donnée patrimoniale ou locative fictive n'est injectée.

## Prérequis Windows

- Rust stable GNU ;
- MSYS2 UCRT64 avec GCC/binutils ;
- Docker Desktop.

Le projet contient `.cargo/config.toml` pour pointer les builds GNU vers `C:\msys64\ucrt64\bin`, ce qui évite de modifier manuellement le PATH pour Cargo.

## Démarrage

```powershell
docker compose up -d
.\scripts\start.ps1
```

Ouvrir `http://127.0.0.1:8080`.

## Base de données

PostgreSQL écoute sur `127.0.0.1:55432`.
La migration `migrations/0001_foundation.sql` est appliquée automatiquement au premier accès serveur.

## Philosophie

Les automatisations préparent et signalent. Les opérations sensibles restent soumises à validation du gérant. Chaque cycle automatique laisse une trace dans le journal d'audit.
