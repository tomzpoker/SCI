# S01-US-0104 — Guide d'application

Chemin cible : `D:\SCI\DEV\SCI-family-rust`

## Fichiers à remplacer/créer

- `Cargo.toml`
- `src/lib.rs`
- `src/main.rs`
- `src/observability.rs`
- `src/intelligence.rs`
- `src/bin/sci-family-migrate.rs`
- `scripts/verify-observability.ps1`
- `versions/S01_US-0104.md`
- `PROJECT_STATE.md`
- `ARCHITECTURE_STATE.md`
- `ROADMAP_STATE.md`
- `VERSION_STATE.md`
- `S01.md`

## Validation unique recommandée

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-observability.ps1;cargo test --lib --features server;cargo check --features server
```

Le changement ne modifie aucune migration SQL ni le schéma de données.

## Résultat attendu

`OBSERVABILITY_SELF_TESTS=PASS`

Puis un `cargo test` et un `cargo check` verts.

Le statut reste `READY FOR REVIEW` jusqu'à la validation réelle Windows/MSVC.
