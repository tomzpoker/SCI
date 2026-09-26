# SCI Family Rust — reprise développeur sans historique de conversation

1. Décompresser le projet dans `D:\SCI\DEV\SCI-family-rust`.
2. Vérifier `PROJECT_STATE.md`, `ROADMAP_STATE.md`, `DATABASE_STATE.md`, `ARCHITECTURE_STATE.md` et `VERSION_STATE.md`.
3. Exécuter `scripts\start.ps1` pour démarrer PostgreSQL et Dioxus.
4. Exécuter `scripts\doctor.ps1` pour le diagnostic.
5. Exécuter `scripts\run-tests.ps1` pour les tests déterministes.
6. Pour les tests DB/E2E : `scripts\run-tests.ps1 -Database -E2E`.
7. Pour une release : `scripts\release.ps1` après validation runtime et dépôt Git propre.

Le dépôt contient les migrations, fixtures, scripts de vérification, recovery, états et procédures nécessaires à une reprise indépendante.
