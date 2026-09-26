# S02 — US-0202 — Gérer les activités d’une SARL

## Ce qui est livré

Cette story permet de rattacher plusieurs activités au même enregistrement `legal_entities` lorsque sa forme juridique est `SARL`. Le catalogue normatif livré contient :

- `MARCHAND_DE_BIENS` — Marchand de biens
- `GARAGE_AUTO` — Garage automobile

Une SARL peut activer les deux simultanément. Une seule activité peut être marquée comme principale à un instant donné.

## Protection

- contrôle côté Rust ;
- garde côté PostgreSQL pour interdire le rattachement à une SCI ;
- activité absente/inactive refusée ;
- audit des changements ;
- migration additive et sans suppression de données métier ;
- aucun basculement des tables métiers historiques vers `legal_entity_id` : ce travail est US-0203.

## Validation

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-sarl-activities.ps1
```

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\migrate.ps1
```

```powershell
cd D:\SCI\DEV\SCI-family-rust;cargo test --lib --features server;cargo check --features server
```

## Fonctionnel

Créer une SARL dans **Entités**, ouvrir **Activités SARL**, activer `Marchand de biens`, puis `Garage automobile`. Les deux doivent apparaître actives en même temps.

## Statut

`READY FOR REVIEW` — la compilation et la validation PostgreSQL doivent être exécutées sur Windows/MSVC avant Ship.

Prochaine story : `US-0203 — Cloisonner les données`.
