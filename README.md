# SCI Family Pilot

> Pilotage administratif automatisé d'une SCI familiale à l'IR.
> Dolibarr est la source de vérité comptable. L'app Rust apporte l'UX, l'anticipation et l'intelligence métier.

## Stack

- **Rust 2024** · **Dioxus 0.7** (fullstack web) · **SQLx 0.9** · **PostgreSQL 17** · **Docker**
- Cible : Windows/MSVC (serveur) + wasm32-unknown-unknown (UI)
- OCR : Tesseract + ImageMagick dans le conteneur Dolibarr

## Démarrage rapide (Windows)

```powershell
Set-Location D:\SCI\DEV\sci-family-rust
.\scripts\start.ps1