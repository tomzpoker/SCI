# US-0405 — Services métier sans SQL IA
S04 introduit `src/services.rs` comme frontière de service pour les opérations communes et les journalise dans `service_operations` avec validation, risque, statut et post-condition.

Les chemins IA existants ne disposent pas d'un outil SQL direct.
