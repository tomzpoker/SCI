# Architecture — Dolibarr Source de Vérité

**Version** : 1.0 (Sprint 1)
**Date** : 27/09/2026

## Principe fondamental

> **Une donnée = un seul écrivain.**

- **Dolibarr** écrit : compta, factures, paiements, tiers, TVA, GED, banque
- **App Rust** écrit : tâches, automatisations, audit, UX, baux, propriétés, lots

## Règle d'or

> **Les modules payants doivent toujours être remplacés par un équivalent gratuit et open source.**

Modules payants rejetés : Ultimateimmo (~360 €), eInvoicing (~200 €), scanpdf, XRechnung Mail-Import, FacturZen.

## Marquage des tables

### 🟢 DOLIBARR (source de vérité)

Ces tables locales font doublon avec Dolibarr. **À terme : suppression.**

| Table locale | Remplacée par (Dolibarr) |
|---|---|
| `invoices` | `llx_facture` |
| `billing_invoice_lines` | `llx_facturedet` |
| `billing_invoice_state_history` | `llx_facture_extrafields` + statut |
| `payments` | `llx_paiement` |
| `payment_promises` | `llx_paiement` (commentaires) |
| `tenants` | `llx_societe` (rôle client) |
| `associates` | `llx_societe` (catégorie "Associé") |
| `associate_share_history` | À migrer vers `llx_societe_extrafields` |
| `vat_declarations` | `llx_tva` (via module TVA) |
| `vat_entries` | `llx_tva` |
| `vat_advances` | `llx_tva` |
| `vat_declaration_fields` | Module TVA Dolibarr |
| `bank_transactions` | `llx_bank` |
| `bank_account_profiles` | `llx_bank_account` |
| `bank_import_batches` | Module Banque Dolibarr |
| `bank_reconciliation_matches` | Module Banque Dolibarr |
| `bank_reconciliation_events` | Module Banque Dolibarr |
| `email_accounts` | Module Email Dolibarr |
| `einvoice_documents` | Module eInvoicing (à évaluer gratuit) |
| `einvoice_events` | Module eInvoicing |
| `einvoice_providers` | Module eInvoicing |
| `einvoice_ereporting` | Module eInvoicing |

### 🔵 LOCAL (reste dans l'app Rust)

Ces tables sont spécifiques à la valeur ajoutée de l'app.

#### Moteur d'anticipation

| Table | Rôle |
|---|---|
| `tasks` | Tâches générées par le moteur |
| `automation_rules` | Règles d'anticipation |
| `automation_policies` | Politiques de workflow |
| `tax_deadlines` | Échéances fiscales |
| `fiscal_tax_obligations` | Obligations fiscales |
| `fiscal_2072_runs` | Calculs 2072 |
| `fiscal_2072_allocations` | Répartition 2072 |
| `fiscal_simulations` | Simulations fiscales |
| `fiscal_results` | Résultats fiscaux |
| `fiscal_dossiers` | Dossiers fiscaux |
| `fiscal_form_versions` | Versions formulaires |
| `prevision_flows` | Flux prévisionnels |
| `forecast_snapshots` | Snapshots prévisionnel |
| `treasury_forecast_events` | Événements prévision tréso |
| `treasury_forecast_snapshots` | Snapshots tréso |
| `treasury_recurring_patterns` | Patterns récurrents |
| `treasury_investments` | Placements |
| `treasury_investment_scenarios` | Scénarios |
| `treasury_investment_scenario_items` | Items scénarios |
| `cash_position_snapshots` | Positions de trésorerie |

#### Patrimoine SCI (spécifique SCI)

| Table | Rôle | Décision |
|---|---|---|
| `properties` | Biens immobiliers | **LOCAL** (Contrats Dolibarr trop pauvre) |
| `units` | Lots | **LOCAL** |
| `leases` | Baux | **LOCAL** |
| `lease_clauses` | Clauses bail | **LOCAL** |
| `lease_contract_events` | Événements bail | **LOCAL** |
| `lease_charge_rules` | Règles charges | **LOCAL** |
| `lease_deposits` | Dépôts garantie | **LOCAL** |
| `lease_entry_fees` | Frais d'entrée | **LOCAL** |
| `lease_guarantees` | Garanties | **LOCAL** |
| `lease_index_values` | Valeurs IRL | **LOCAL** |
| `lease_rent_revisions` | Révisions loyers | **LOCAL** |
| `lease_rent_reductions` | Réductions loyers | **LOCAL** |
| `lease_recoverable_taxes` | Taxes récupérables | **LOCAL** |
| `scis` | Fiche SCI | **LOCAL** |
| `legal_entities` | Entités juridiques | **LOCAL** |
| `legal_entity_activities` | Activités | **LOCAL** |
| `legal_entity_bank_accounts` | Comptes bancaires | **LOCAL** (lié à Dolibarr) |
| `legal_entity_business_profiles` | Profils métier | **LOCAL** |
| `legal_activity_catalog` | Catalogue activités | **LOCAL** |
| `arrears_cases` | Dossiers impayés | **LOCAL** |
| `arrears_events` | Événements impayés | **LOCAL** |
| `collection_actions` | Actions recouvrement | **LOCAL** |

#### Audit & traçabilité

| Table | Rôle |
|---|---|
| `audit_events` | Journal des actions |
| `entity_change_history` | Historique changements |
| `business_events` | Événements métier |
| `business_event_types` | Types d'événements |
| `business_profile_catalog` | Catalogue profils |
| `data_quality_snapshots` | Qualité données |
| `postcondition_checks` | Vérifications post-conditions |
| `validation_requests` | Demandes validation |

#### Documents & GED (en attendant migration Dolibarr)

| Table | Rôle | Décision |
|---|---|---|
| `documents` | Documents | **À migrer vers GED Dolibarr** |
| `document_events` | Événements documents | **À migrer** |
| `document_inbox` | Boîte réception | **LOCAL** (workflow) |
| `document_ocr_runs` | Runs OCR | **LOCAL** (OCR tiers) |
| `document_extractions` | Extractions OCR | **LOCAL** |
| `document_folder_catalog` | Catégories | **LOCAL** |
| `document_duplicate_candidates` | Doublons | **LOCAL** |
| `document_quality_checks` | Qualité | **LOCAL** |
| `document_storage_config` | Config stockage | **LOCAL** |
| `document_templates` | Modèles | **LOCAL** |
| `document_generation_events` | Générations | **LOCAL** |
| `generated_documents` | Docs générés | **LOCAL** |
| `pdf_profiles` | Profils PDF | **LOCAL** |
| `storage_locations` | Emplacements | **LOCAL** |

#### IA & Assistant

| Table | Rôle |
|---|---|
| `ai_conversations` | Conversations |
| `ai_memory` | Mémoire |
| `ai_messages` | Messages |
| `ai_provider_configs` | Config LLM |
| `ai_tool_catalog` | Outils IA |
| `ai_tool_invocations` | Invocations |
| `llm_accounts` | Comptes LLM |
| `assistant_proposals` | Propositions |

#### Sécurité & Config

| Table | Rôle |
|---|---|
| `auth_users` | Utilisateurs |
| `auth_sessions` | Sessions |
| `auth_user_roles` | Rôles |
| `security_permission_catalog` | Permissions |
| `security_role_permissions` | Permissions rôles |
| `ux_preferences` | Préférences UX |
| `app_settings` | Paramètres app |

#### Workflow & Jobs

| Table | Rôle |
|---|---|
| `workflow_definitions` | Définitions workflow |
| `workflow_runs` | Exécutions |
| `workflow_steps` | Étapes |
| `job_executions` | Exécutions jobs |
| `service_operations` | Opérations service |
| `rule_definitions` | Définitions règles |
| `rule_versions` | Versions règles |
| `rule_calculation_runs` | Calculs |
| `onboarding_steps` | Étapes onboarding |
| `versioned_references` | Références versionnées |

#### Divers

| Table | Rôle |
|---|---|
| `workspaces` | Espaces travail |
| `system_backups` | Sauvegardes |
| `system_snapshots` | Snapshots |
| `_sqlx_migrations` | Migrations sqlx |

### ⚪ À SUPPRIMER (fonctionnalités non utilisées ou redondantes)

Ces tables n'ont pas de raison d'être conservées.

| Table | Raison |
|---|---|
| `billing_charge_actuals` | Fonctionnalité non utilisée |
| `billing_einvoice_connections` | Doublon avec `einvoice_providers` |
| `billing_einvoice_events` | Doublon avec `einvoice_events` |
| `billing_regularizations` | Fonctionnalité non utilisée |
| `billing_tax_assessments` | Fonctionnalité non utilisée |
| `recovery_runs` | Doublon avec `arrears_cases` |
| `vat_entries` | Doublon avec `vat_declarations` |

## Plan de migration

| Phase | Sprint | Action |
|---|---|---|
| 1 | S1 | Audit & marquage (ce document) |
| 2 | S2 | Étendre le client API Dolibarr |
| 3 | S3-S4 | Basculer facturation sur Dolibarr (lecture + écriture) |
| 4 | S5 | Basculer paiements sur Dolibarr |
| 5 | S6 | Basculer tiers sur Dolibarr |
| 6 | S7 | Basculer banque sur Dolibarr |
| 7 | S8 | Basculer GED sur Dolibarr (module ECM) |
| 8 | S9 | Supprimer les tables doublons locales |

## Points de vigilance

1. **Ne pas supprimer les tables doublons avant d'avoir migré les données vers Dolibarr.**
2. **La GED Dolibarr (ECM) est en version beta** — on garde les tables locales pour l'instant[reference:4].
3. **Le module Contrats Dolibarr ne convient pas aux baux** — on garde les tables locales de baux.