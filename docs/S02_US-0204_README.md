# S02 — US-0204 — Historiser les changements

## Livré

La migration `0009_entity_change_history.sql` ajoute un historique dédié aux changements critiques. Chaque entrée conserve : date d'effet, date d'enregistrement, auteur, action, type/identifiant, motif optionnel, état avant et état après, métadonnées.

La couverture applicative porte sur la création, la modification et l'activation/désactivation des entités juridiques, la configuration SCI et les changements d'activités SARL. Les anciennes entrées de `audit_events` restent intactes.

La page Audit affiche désormais le journal existant et l'historique critique avec les états avant/après.

## Validation

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-change-history.ps1
```

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\migrate.ps1
```

Statut : `READY FOR REVIEW` jusqu'au test PostgreSQL/Windows réel.
