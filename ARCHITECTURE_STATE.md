# ARCHITECTURE_STATE

## Architecture actuelle
Dioxus fullstack + Rust + SQLx + PostgreSQL.

## Couches constatées
- domain
- application
- infrastructure
- server functions
- UI
- assistant
- documents

## Architecture cible à converger
- Core Domain
- Configuration Engine
- Reference Data Engine
- Rules Engine
- Calculation Engine
- Event Engine
- Workflow Engine
- Automation Engine
- Approval Engine
- Document/OCR Engine
- Bank/Reconciliation Engine
- Tax/Fiscal Engine
- Cash Forecast Engine
- Audit/Security
- API/UI/CLI

## Décision
Conserver les composants Rust existants et converger progressivement vers cette architecture. Pas de réécriture globale.
