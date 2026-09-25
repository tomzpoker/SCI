# S14 FINAL — IA CONTRÔLÉE

Migrations principales: `0024_ai_controlled.sql`, `0026_ai_controlled.sql`.

Le provider par défaut est DISABLED. Aucun secret LLM n’est stocké: seules des références d’endpoint/credential/commande peuvent être conservées. Les actions non read-only avec confirmation exigent une validation humaine.
