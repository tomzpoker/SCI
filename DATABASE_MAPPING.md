# DATABASE_MAPPING — Rust simplifié → BDD de référence

Date : 2026-09-22
Statut : première matrice de convergence — aucune migration appliquée

## Légende

- **MERGE** : le concept courant existe dans la BDD de référence mais doit être fusionné avec son modèle cible.
- **REPLACE** : le modèle courant est trop pauvre et doit être remplacé fonctionnellement par plusieurs objets de référence.
- **NEW** : objet absent du runtime courant et requis par la BDD de référence/cahier des charges.
- **KEEP** : peut rester comme objet de façade/projection après validation.
- **TO_VERIFY** : correspondance sémantique insuffisamment certaine ; ne pas coder avant vérification.

## 1. Tables runtime Rust actuelles

| Runtime actuel | Cible de référence | Action | Commentaire |
|---|---|---|---|
| `workspaces` | Pas d'équivalent direct identifié | TO_VERIFY | Le modèle de référence paraît centré sur une entité SCI/configuration plutôt qu'un workspace utilisateur général. |
| `scis` | `sci_reference.sci_settings` + fonctions/références SCI | REPLACE | Le runtime mélange identité SCI, régime et configuration opérationnelle. Il faut séparer identité légale, configuration et référentiels. |
| `associates` | `sci_property.associates` | MERGE | Le modèle de référence doit être conservé ; les parts et historiques fiscaux nécessitent aussi `fiscal_ownership_snapshots`. |
| `properties` | `sci_property.properties` | MERGE | Conserver le modèle de patrimoine de référence. |
| `units` | `sci_property.units` | MERGE | Vérifier les catégories via `sci_reference.unit_types`. |
| `tenants` | `sci_property.tenants` | MERGE | Compléter avec les objets liés au recouvrement/facturation. |
| `leases` | `sci_property.leases` | MERGE | Compléter avec indexations, TVA et types/charges de bail. |
| `invoices` | `sci_billing.invoice_register` | REPLACE | Le registre cible contient direction, type, TVA, source event, paiement et statuts détaillés. |
| `payments` | `sci_finance.receivable_allocations` + journal/transactions | REPLACE | Un paiement ne doit pas être le seul objet financier ; distinguer encaissement bancaire, allocation et écriture. |
| `bank_transactions` | `sci_finance.bank_transactions` + `sci_banking.bank_accounts` + `bank_import_batches` + allocations | REPLACE | Ajouter compte bancaire, lot d'import, id externe, rapprochements partiels/multiples. |
| `automation_rules` | `sci_scheduler.event_obligation_rules` + `recurring_obligations` | REPLACE | Le moteur cible doit être piloté par événements/règles versionnées et obligations. |
| `tasks` | `sci_scheduler.scheduled_tasks` + workflow | REPLACE | Les tâches ne doivent pas porter seules la logique de workflow. |
| `tax_deadlines` | `sci_reference.fiscal_deadline_calendar` + scheduler | MERGE | Le référentiel fiscal et l'instance de tâche doivent être séparés. |
| `documents` | `sci_documents.documents` + `document_links` + `business_objects` | REPLACE | La référence distingue document, lien métier et objet fonctionnel. |
| `forecast_snapshots` | `sci_projections.treasury_scenarios` + vues de projection | REPLACE | Les prévisions doivent être scénarisées, probabilisées et recalculables. |
| `audit_events` | `sci_audit` + événements/transactions métier | REPLACE | Le journal d'audit doit être append-only et structuré ; ne pas le limiter à un payload libre. |
| `app_settings` | `sci_reference.sci_settings` + configuration métier | MERGE | Séparer configuration système, SCI, règles et références. |
| `storage_locations` | Aucun équivalent obligatoire direct | KEEP | Conserver comme couche de stockage externe si elle ne duplique pas les métadonnées documentaires. |

## 2. Event store à intégrer

| Objet | Action | Rôle |
|---|---|---|
| `sci_events.events` | NEW | Journal append-only des événements métier versionnés. |
| `sci_events.stream_heads` | NEW | Version courante de chaque aggregate/stream. |
| `sci_events.event_upcasters` | NEW | Compatibilité des payloads historiques. |

## 3. Workflow / saga

| Objet | Action | Rôle |
|---|---|---|
| `sci_workflow.process_states` | NEW | État courant d'un processus métier. |
| `sci_workflow.saga_transitions` | NEW | Transitions autorisées. |
| `sci_workflow.saga_transition_log` | NEW | Historique des transitions. |

## 4. Finance / fiscalité opérationnelle

| Objet | Action | Rôle |
|---|---|---|
| `sci_finance.accounts` | NEW | Plan/référentiel de comptes opérationnels. |
| `sci_finance.journal_entries` | NEW | Écritures sources. |
| `sci_finance.journal_lines` | NEW | Lignes d'écritures. |
| `sci_finance.receivable_allocations` | NEW | Allocation d'encaissements aux créances. |
| `sci_finance.supplier_invoices` | NEW | Factures fournisseurs. |
| `sci_finance.supplier_payment_allocations` | NEW | Paiements fournisseurs. |
| `sci_finance.expense_allocations` | NEW | Répartition de dépenses. |
| `sci_finance.tax_cash_flows` | NEW | Flux fiscaux réels. |
| `sci_finance.associate_distributions` | NEW | Répartition du résultat. |
| `sci_finance.fiscal_ownership_snapshots` | NEW | Historisation des quote-parts par période. |
| `sci_finance.fiscal_deficit_lots` | NEW | Suivi des déficits reportables. |
| `sci_finance.loan_installments` | NEW | Échéanciers de prêts lorsque le module est activé. |
| `sci_finance.loans` | NEW / MODULE OFF PAR DÉFAUT | Emprunts. |
| `sci_finance.posting_rules` | NEW | Règles de génération d'écritures. |
| `sci_finance.tva_declarations` | NEW | Déclarations TVA préparées. |
| `sci_finance.tva_declaration_transmissions` | NEW | Historique des transmissions. |

## 5. Banque

| Objet | Action |
|---|---|
| `sci_banking.bank_accounts` | NEW |
| `sci_banking.bank_import_batches` | NEW |
| `sci_banking.bank_reconciliation_allocations` | NEW |
| `sci_finance.bank_transactions` | MERGE / REPLACE du runtime `bank_transactions` |

## 6. Facturation / recouvrement / e-facturation

| Objet | Action |
|---|---|
| `sci_billing.invoice_register` | MERGE / CIBLE PRINCIPALE |
| `sci_billing.invoice_sequences` | NEW |
| `sci_billing.invoice_status_history` | NEW |
| `sci_billing.reminder_actions` | NEW |
| `sci_billing.reminder_scenarios` | NEW |
| `sci_billing.reminder_templates` | NEW |
| `sci_billing.e_invoice_endpoints` | NEW / MODULE E-INVOICING |
| `sci_billing.e_invoice_messages` | NEW / MODULE E-INVOICING |

## 7. Documents

| Objet | Action |
|---|---|
| `sci_documents.documents` | MERGE / CIBLE PRINCIPALE |
| `sci_documents.document_links` | NEW |
| `sci_documents.business_objects` | NEW |

## 8. Property / location

| Objet | Action |
|---|---|
| `sci_property.associates` | MERGE |
| `sci_property.properties` | MERGE |
| `sci_property.units` | MERGE |
| `sci_property.tenants` | MERGE |
| `sci_property.leases` | MERGE |
| `sci_property.lease_indexations` | NEW |
| `sci_property.lease_vat_treatments` | NEW |
| `sci_property.recurring_contracts` | NEW |
| `sci_property.sci_capital` | NEW |
| `sci_property.capital_movements` | NEW |
| `sci_property.capital_command_registry` | NEW |
| `sci_property.tenant_invoice_type_terms` | NEW |
| `sci_property.vat_options` | NEW |

## 9. Référentiels

Les tables suivantes doivent être traitées comme des données de référence versionnées, pas comme des constantes Rust :

- `account_classes`
- `allocation_methods`
- `capital_movement_types`
- `client_invoice_types`
- `contract_types`
- `event_type_to_invoice_type`
- `event_types`
- `expense_natures`
- `fiscal_deadline_calendar`
- `fiscal_rules`
- `insee_indices`
- `lease_charge_types`
- `lease_statuses`
- `obligation_types`
- `process_types`
- `receivable_allocation_policies`
- `receivable_priorities`
- `schema_versions`
- `sci_settings`
- `stream_types`
- `treasury_scenario_modes`
- `unit_types`
- `vat_declaration_workflow_statuses`

Action : **NEW / CONSERVER DANS LEUR RESPONSABILITÉ DE RÉFÉRENTIEL**, avec version et période de validité au niveau du modèle Rust cible.

## 10. Projections

| Objet | Action |
|---|---|
| `property_economic_view` | NEW / VIEW |
| `property_tax_view` | NEW / VIEW |
| `read_lease_balances` | NEW / READ MODEL |
| `read_tenant_balances` | NEW / READ MODEL |
| `read_tenant_risk_indicators` | NEW / READ MODEL |
| `tax_declaration_associate_shares` | NEW / READ MODEL |
| `tax_declarations_2072` | NEW / READ MODEL |
| `treasury_scenarios` | NEW |
| `treasury_scenario_lines` | NEW |

## 11. Scheduler / audit

| Objet | Action |
|---|---|
| `sci_scheduler.event_obligation_rules` | NEW |
| `sci_scheduler.recurring_obligations` | NEW |
| `sci_scheduler.scheduled_tasks` | MERGE de `tasks` |
| `sci_audit.integrity_checks` | NEW |

## 12. Décision de convergence

Le prochain schéma Rust ne doit **pas** être construit en ajoutant 68 tables au modèle simplifié au hasard.

Il faut :

1. stabiliser les domaines et identifiants ;
2. introduire les schémas PostgreSQL de référence par domaine ;
3. connecter le code Rust aux services métier, pas directement aux tables dans chaque endpoint ;
4. introduire Event/Workflow/Reference/Rules ;
5. fournir des projections de lecture pour l'UI ;
6. maintenir une compatibilité de migration et de rollback ;
7. importer les données via une procédure séparée et testable.

Aucune donnée de la BDD source n'est modifiée par cette matrice.
