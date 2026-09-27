# Architecture Decision Records (ADR)

## ADR-001 : Dolibarr = source de vérité comptable

**Date** : 27/09/2026
**Statut** : Accepté

### Contexte

L'application Rust `sci-family-rust` gérait initialement sa propre comptabilité (factures, paiements, TVA) en parallèle de Dolibarr. Cela créait une double saisie et un risque de désynchronisation.

### Décision

Dolibarr devient la source de vérité unique pour toutes les données comptables et fiscales. L'application Rust devient une couche d'anticipation, d'UX et d'intelligence métier.

### Conséquences

- ✅ Une seule source de vérité → pas de désynchronisation
- ✅ Conformité fiscale garantie par Dolibarr
- ✅ L'app Rust se concentre sur sa valeur ajoutée
- ⚠️ Dépendance à l'API Dolibarr
- ⚠️ Migration des données existantes à planifier

---

## ADR-002 : Pas de modules payants

**Date** : 27/09/2026
**Statut** : Accepté

### Contexte

Dolibarr propose de nombreux modules payants sur le Dolistore (Ultimateimmo ~360 €, eInvoicing ~200 €, etc.). Ces modules pourraient couvrir certains besoins métier.

### Décision

Aucun module payant ne sera utilisé. Chaque besoin couvert par un module payant doit être remplacé par :
- Un équivalent gratuit et open source
- Une implémentation maison dans l'app Rust
- Une alternative fonctionnelle (workflow manuel, etc.)

### Conséquences

- ✅ Coût zéro en licences
- ✅ Maîtrise complète du code
- ⚠️ Plus de développement à faire
- ⚠️ Certaines fonctionnalités avancées peuvent être absentes

---

## ADR-003 : Les baux restent en local (app Rust)

**Date** : 27/09/2026
**Statut** : Accepté

### Contexte

Le module natif Contrats/Abonnements de Dolibarr pourrait être utilisé pour gérer les baux. Des modules payants (Ultimateimmo) existent aussi.

### Décision

Les tables `leases`, `properties`, `units` et toutes les tables liées aux baux restent dans l'app Rust.

### Justification

1. **Le module Contrats Dolibarr est trop générique** — pas de notion de préavis, IRL, dépôt de garantie, révision, charges locatives[reference:5].
2. **Les modules spécialisés sont payants** — Ultimateimmo (~360 €), rejeté par ADR-002.
3. **Les modules payants ont des bugs** — Ultimateimmo est signalé comme *"unusable for production"*[reference:6].
4. **Les tables locales ont déjà les champs spécifiques SCI** — `payment_day`, `annual_review_month`, `notice_months`, etc.
5. **Le module Contrats reste activé** pour lier un bail à une facture récurrente Dolibarr si besoin.

### Conséquences

- ✅ Données métier SCI maîtrisées
- ✅ Pas de dépendance à un module externe
- ⚠️ La facturation récurrente devra être générée par l'app Rust vers Dolibarr