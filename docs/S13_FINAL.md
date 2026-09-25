# S13 FINAL — E-FACTURATION

Migrations principales: `0023_einvoicing.sql`, `0025_einvoicing_runtime.sql`.

Points clés: provider-neutral, réception/émission structurées, statuts/accusés/erreurs, e-reporting, remplacement à date d’effet, aucun secret fournisseur en clair.

Limite explicite: le transport réseau vers une PDP réelle reste un adaptateur externe à brancher; le cœur ne l’exécute pas implicitement.
