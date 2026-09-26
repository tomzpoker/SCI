# ÉTAPE 01 — Multi-sociétés + profil SARL garage sans SAV

Cette étape ajoute le socle de configuration nécessaire pour piloter dans la même application :

- une SCI à l’IR ;
- une SARL à l’IS ;
- un profil d’exploitation `SARL_IS_GARAGE_SANS_SAV`.

Le profil garage décrit explicitement : vente de véhicules, stock de voitures, stock de pièces, dépôt-vente, sans atelier et sans SAV.

La TVA de chaque entité reste configurable indépendamment en `COLLECTION` (encaissement) ou `DEBIT` (débits).

Les moteurs métier communs continuent d’utiliser `legal_entity_id` comme portée canonique.

Cette étape ne prétend pas encore couvrir les opérations détaillées du garage (achats véhicules, stock, dépôt-vente, facturation automobile, TVA sur marge, etc.) : elles constituent les étapes suivantes.
