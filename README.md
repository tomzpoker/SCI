# SCI Family Pilot 0.4.0

Application full Rust pour le pilotage administratif d’une SCI familiale à l’IR avec TVA sur encaissement.

## Fonctionnel couvert

- Configuration SCI, siège, régime et coordonnées bancaires
- Associés et quote-parts avec contrôle du total à 100 %
- Patrimoine : biens + lots
- Locataires
- Baux et échéances de paiement
- Factures : brouillon, émission, suivi des paiements
- Encaissements et calcul TVA par période
- Banque : saisie, import CSV idempotent, rapprochement
- Calendrier fiscal et tâches bloquantes
- Référentiel documentaire et expirations
- Moteur d’anticipation idempotent
- Tâches et changements d’état
- Journal d’audit
- Prévision de trésorerie 12 mois

## Démarrage Windows

1. `docker compose up -d`
2. définir `DATABASE_URL=postgres://sci:sci@127.0.0.1:55432/sci_family`
3. `cargo check --features server`
4. `dx serve --web`

Le projet utilise PostgreSQL et applique les migrations au démarrage du backend.
