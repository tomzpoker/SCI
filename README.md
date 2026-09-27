# SCI Family — UI overlay v0.2.0

Patch ciblé sur le dépôt `tomzpoker/SCI` / `main`, destiné au projet local exact :

`D:\SCI\DEV\SCI-family-rust`

## Ce que le patch fait

- réduit la navigation quotidienne à : Accueil, À vérifier, Argent, Biens, Locataires, Documents, Échéances, Configuration ;
- conserve les modules techniques derrière « 🛠️ Outils avancés » ;
- refond l'accueil sans supprimer les moteurs existants ;
- ajoute un graphique de trésorerie avec deux courbes : **Réel** (mouvements bancaires) et **Prévu** (moteur de prévision) ;
- garde un axe mensuel commun et relie le réel au point courant du prévisionnel ;
- adapte le responsive et les thèmes existants ;
- crée des backups horodatés avant toute écriture.

## Installation locale

1. Extraire le ZIP.
2. Ouvrir PowerShell.
3. Se placer exactement dans `D:\SCI\DEV\SCI-family-rust`.
4. Faire un dry-run :

```powershell
powershell -ExecutionPolicy Bypass -File .\patches\apply-ui-overlay.ps1
```

5. Si le dry-run affiche `DRY RUN OK`, appliquer :

```powershell
powershell -ExecutionPolicy Bypass -File .\patches\apply-ui-overlay.ps1 -Apply
```

6. Tester :

```powershell
cargo check --features web
```

puis :

```powershell
dx serve --web
```

## Sécurité du patch

Le script refuse tout autre répertoire que `D:\SCI\DEV\SCI-family-rust`.
Il ne fait aucun appel GitHub et ne modifie aucun dépôt distant.
Il refuse d'appliquer le patch si les anchors attendus dans ton `ui.rs` ne sont pas présents.
