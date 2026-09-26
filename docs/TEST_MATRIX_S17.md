# S17 — Matrice de tests & hardening

| US | Couverture | Artefact |
|---|---|---|
| 1701 | calculs purs + bornes | `tests/pure_hardening.rs`, tests unitaires modules |
| 1702 | DB + services | `tests/integration/modules_12.rs`, `tests/migrations_runtime.rs` |
| 1703 | événement → automation → validation → résultat | `tests/workflow_s17.rs`, `src/workflow.rs` |
| 1704 | régression | `tests/regression.rs` |
| 1705 | tests property-like déterministes | `tests/pure_hardening.rs`, `tests/temporal_boundaries.rs` |
| 1706 | templates + PDF | `tests/snapshot_documents.rs`, `fixtures/snapshots/` |
| 1707 | CSV / OFX / OCR / classification | `src/banking.rs`, `tests/imports.rs`, `tests/resilience.rs` |
| 1708 | permissions / IA | `tests/security.rs`, `src/security.rs` |
| 1709 | fixtures fiscales versionnées | `fixtures/fiscal/`, `tests/fiscal_fixtures.rs` |
| 1710 | frontières temporelles | `tests/temporal_boundaries.rs`, `tests/pure_hardening.rs` |
| 1711 | migrations | `tests/migration_manifest.rs`, `tests/migrations_runtime.rs`, script isolé |
| 1712 | résilience | `tests/resilience.rs`, `scripts/verify-s17-s19-final.ps1` |

Les tests DB/e2e sont explicitement opt-in avec `SCI_RUN_DB_TESTS=1` et, pour les Server Functions protégées, la feature `test-auth`.
