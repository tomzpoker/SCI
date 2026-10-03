# CURRENT_STATE — SCI Family Rust

> **Source de vérité unique.** Mis à jour à chaque fin de sprint.
> Voir aussi : `architecture.md` (plan de migration Dolibarr) et `decisions.md` (ADRs).

---

## Identité

| Champ | Valeur |
|---|---|
| Projet | SCI Family Pilot |
| Package | `sci-family-pilot` |
| Version | `0.5.0` |
| Repo | https://github.com/tomzpoker/SCI |
| Chemin local | `D:\SCI\DEV\sci-family-rust` |
| Stack | Rust 2024 · Dioxus 0.7 fullstack · SQLx 0.9 · PostgreSQL 17-alpine · Docker |
| Cible | Windows/MSVC (serveur) · wasm32-unknown-unknown (UI web) |

---

## Dernier sprint terminé

**Sprint Taxe Foncière** — commit `df55e2d`, poussé sur `main`.

> ⚠️ **Deux numérotations coexistent dans l'historique du projet** :
> - `docs/S01..S19` = sprints d'origine du projet (avant migration Dolibarr), tous archivés dans `docs/_archive/`
> - Plan de migration `SPRINTS.docx` = S1..S16, dont le S14 = Répartition Taxe Foncière (celui-ci)

### Fonctionnalités livrées

1. **Upload 2 feuillets TF** (PDF/JPG/PNG/HEIC) avec OCR auto (Tesseract + ImageMagick)
2. **Extraction OCR** : texte complet + crop colonne droite ciblé
3. **Répartition par lot** : % en UI (1-100), stocké en bp (×100) en base
4. **Détection lots vacants** : pas de facture locataire (reste à charge SCI)
5. **Factures locataires Dolibarr** : ligne "Taxe foncière" + ligne "Frais de gestion"
   - Frais **uniquement sur adresses scindées** (>1 lot dans l'adresse)
6. **Facture fournisseur DGFiP** : référence `TF<année>-<BIEN>` (ex: `TF2027-GRANDE_BRETAGNE`)
   - Feuillets archivés dans la GED (tiers Patrimoine SCI + facture fournisseur)
7. **Remplacement / suppression** de feuillets à tout moment
8. **Barre de progression** lors de l'OCR et la facturation

### Fichiers créés

- `src/tax_fonciere/mod.rs`, `models.rs`, `calculation.rs`, `parser.rs`, `tax_service.rs`
- `src/ui/tax_fonciere.rs`
- `migrations/0042_property_tax_notices.sql`
- `migrations/0044_tax_notice_documents_bytes.sql`
- `migrations/0045_tax_notice_supplier_invoice.sql`

### Fichiers modifiés

- `src/dolibarr/server_fns.rs` (+368 lignes : OCR colonne droite + OCR frais)
- `src/ui.rs` (route `Page::TaxFonciere`)
- `src/lib.rs` (module `tax_fonciere`)
- `src/main.rs` (`DefaultBodyLimit` 32 Mo pour uploads)
- `assets/main.css` (barre de progression)
- `Cargo.toml` (`rust_decimal_macros` en dev-deps)

---

## Schéma base de données

**Migrations : 43 fichiers** (0001 à 0045, sans 0041 ni 0043 supprimés).
Dernière : `0045_tax_notice_supplier_invoice.sql`.

### Tables module Taxe Foncière

| Table | Rôle |
|---|---|
| `property_tax_notices` | Avis TF : bien, année, montant total, frais, statut, ID/ref facture fournisseur |
| `property_tax_notice_documents` | Feuillets : type BASES/FEES, bytes, mime, OCR brut, ECM id Dolibarr |
| `property_tax_notice_addresses` | Adresses extraites + cotisation |
| `property_tax_notice_lines` | Répartition : lot, % (bp), montant, `is_vacant`, ID facture locataire |
| `property_tax_notice_fees` | Frais de gestion à répartir |

Statuts `property_tax_notices` : `AWAITING_DOCS`, `AWAITING_REVIEW`, `READY`, `INVOICED`, `ARCHIVED`.

---

## Règles métier critiques (Sprint TF)

1. **1 avis TF = 1 bien** (jamais multi-bien)
2. **1 feuillet BASES + 1 feuillet FEES** = minimum pour facturer
3. **Frais de gestion répartis au prorata** des cotisations, **uniquement sur les adresses scindées** (>1 lot)
4. **Lots vacants** = pas de facture locataire, reste à charge SCI
5. **1 lot ne peut être attribué qu'à une seule adresse** par avis
6. **Total des % d'une adresse = 100 %** obligatoire pour activer la facturation
7. **Idempotence** : ref facture fournisseur `TF<année>-<BIEN>` → clic 2x ne crée pas de doublon
8. **Suppression bloquée** si factures déjà émises

---

## Commandes utiles

```powershell
# Lancer l'app
dx serve

# Compiler (backend)
cargo build --features server

# Nettoyer le cache Dioxus
Remove-Item -Recurse -Force .dioxus, target\dioxus -ErrorAction SilentlyContinue

# Migrations
cargo run --bin sci-family-migrate --features server

# Vérifier la base
docker exec -it sci-family-postgres psql -U sci -d sci_family -c "SELECT version FROM _sqlx_migrations ORDER BY version DESC LIMIT 5;"

# Lister les conteneurs
docker ps --format "{{.Names}}"