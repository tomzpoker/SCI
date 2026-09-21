# SCI Family Pilot 0.3.1

Base neuve, full Rust, destinée au pilotage administratif d'une SCI familiale à l'IR avec TVA gérée sur encaissements.

## Ce que cette version ajoute

- onboarding guidé avec progression 0–100 % ;
- configuration juridique/fiscale et coordonnées bancaires ;
- création et lecture des associés ;
- création et lecture des biens ;
- création et lecture des locataires ;
- migration opérationnelle 0002 ;
- compteurs PostgreSQL réels ;
- moteur d'anticipation idempotent + audit ;
- cockpit de trésorerie et vigilance ;
- architecture prête pour les flux baux → factures → encaissements → TVA.

## Démarrage Windows

```powershell
$env:Path="C:\msys64\ucrt64\bin;$env:USERPROFILE\.cargo\bin;$env:Path"
docker compose up -d
dx serve --web
```

Serveur local : http://127.0.0.1:8080

## Base de données

PostgreSQL local dans Docker, migrations appliquées automatiquement par SQLx.

La configuration locale (`.env`) reste hors Git.
