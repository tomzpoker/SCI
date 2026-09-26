use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskState {
    Planned,
    Ready,
    Running,
    Blocked,
    Done,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskItem {
    pub id: Uuid,
    pub code: String,
    pub title: String,
    pub description: String,
    pub due_at: DateTime<Utc>,
    pub state: TaskState,
    pub priority: i32,
    pub blocking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CashForecastPoint {
    pub date: NaiveDate,
    pub expected_inflows_cents: i64,
    pub expected_outflows_cents: i64,
    pub expected_vat_cents: i64,
    pub balance_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardSnapshot {
    pub sci_name: String,
    pub registered_office: String,
    pub tax_regime: String,
    pub vat_basis: String,
    pub cash_cents: i64,
    pub receivables_cents: i64,
    pub vat_to_prepare_cents: i64,
    pub tasks_due_30d: i64,
    pub overdue_tasks: i64,
    pub forecast_min_cash_cents: i64,
    pub risk_level: String,
    pub next_actions: Vec<TaskItem>,
    pub forecast: Vec<CashForecastPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleCounts {
    pub associates: i64,
    pub properties: i64,
    pub units: i64,
    pub tenants: i64,
    pub leases: i64,
    pub invoices: i64,
    pub payments: i64,
    pub bank_transactions: i64,
    pub unmatched_bank: i64,
    pub vat_receipts_cents: i64,
    pub tax_deadlines: i64,
    pub documents: i64,
    pub active_automation_rules: i64,
    pub open_tasks: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalEntityItem {
    pub id: Uuid,
    pub legal_name: String,
    pub legal_form_code: String,
    pub tax_regime: String,
    pub vat_status: String,
    pub vat_basis: String,
    pub siren: String,
    pub siret: String,
    pub registered_office: String,
    pub accounting_period_start: u8,
    pub fiscal_year_end: u8,
    pub currency_code: String,
    pub active: bool,
    pub bank_accounts_count: i64,
    pub primary_iban: String,
    pub primary_bic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalEntityBankAccountItem {
    pub id: Uuid,
    pub legal_entity_id: Uuid,
    pub label: String,
    pub iban: String,
    pub bic: String,
    pub is_primary: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalActivityCatalogItem {
    pub code: String,
    pub label: String,
    pub description: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalEntityActivityItem {
    pub legal_entity_id: Uuid,
    pub activity_code: String,
    pub label: String,
    pub description: String,
    pub is_primary: bool,
    pub active: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BusinessProfileCatalogItem {
    pub code: String,
    pub label: String,
    pub description: String,
    pub legal_form_code: String,
    pub default_tax_regime: String,
    pub module_key: String,
    pub capabilities: serde_json::Value,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalEntityBusinessProfileItem {
    pub legal_entity_id: Uuid,
    pub profile_code: String,
    pub label: String,
    pub description: String,
    pub legal_form_code: String,
    pub default_tax_regime: String,
    pub module_key: String,
    pub capabilities: serde_json::Value,
    pub is_primary: bool,
    pub active: bool,
    pub configuration: serde_json::Value,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalEntityBusinessProfileDraft {
    pub legal_entity_id: Uuid,
    pub profile_code: String,
    pub is_primary: bool,
    pub active: bool,
    pub configuration: serde_json::Value,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalEntityActivityDraft {
    pub legal_entity_id: Uuid,
    pub activity_code: String,
    pub is_primary: bool,
    pub active: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalEntityDraft {
    pub legal_name: String,
    pub legal_form_code: String,
    pub tax_regime: String,
    pub vat_status: String,
    pub vat_basis: String,
    pub siren: String,
    pub siret: String,
    pub registered_office: String,
    pub accounting_period_start: u8,
    pub fiscal_year_end: u8,
    pub currency_code: String,
    pub primary_iban: String,
    pub primary_bic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SciProfile {
    pub legal_name: String,
    pub siren: String,
    pub siret: String,
    pub registered_office: String,
    pub tax_regime: String,
    pub vat_status: String,
    pub vat_basis: String,
    pub accounting_period_start: u8,
    pub fiscal_year_end: u8,
    pub iban: String,
    pub bic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OnboardingStatus {
    pub profile_ready: bool,
    pub associates_ready: bool,
    pub property_ready: bool,
    pub tenant_ready: bool,
    pub lease_ready: bool,
    pub finance_ready: bool,
    pub automation_ready: bool,
    pub completed: bool,
    pub completion_pct: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssociateItem {
    pub id: Uuid,
    pub display_name: String,
    pub ownership_pct: Decimal,
    pub current_account_cents: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PropertyItem {
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub acquisition_date: Option<NaiveDate>,
    pub acquisition_cents: Option<i64>,
    pub units_count: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnitItem {
    pub id: Uuid,
    pub property_id: Uuid,
    pub property_name: String,
    pub code: String,
    pub label: String,
    pub unit_type: String,
    pub area_m2: Option<Decimal>,
    pub base_rent_cents: i64,
    pub vat_rate_bp: i32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TenantItem {
    pub id: Uuid,
    pub legal_name: String,
    pub siret: String,
    pub contact_email: String,
    pub contact_phone: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaseItem {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub unit_label: String,
    pub property_name: String,
    pub tenant_id: Uuid,
    pub tenant_name: String,
    pub reference: String,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub payment_day: i16,
    pub annual_review_month: Option<i16>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvoiceItem {
    pub id: Uuid,
    pub lease_id: Option<Uuid>,
    pub invoice_number: String,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    pub net_cents: i64,
    pub vat_cents: i64,
    pub gross_cents: i64,
    pub paid_cents: i64,
    pub status: String,
    pub tenant_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentItem {
    pub id: Uuid,
    pub invoice_id: Option<Uuid>,
    pub invoice_number: String,
    pub received_at: DateTime<Utc>,
    pub amount_cents: i64,
    pub reference: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BankTransactionItem {
    pub id: Uuid,
    pub booked_at: DateTime<Utc>,
    pub value_date: Option<NaiveDate>,
    pub amount_cents: i64,
    pub label: String,
    pub counterparty: String,
    pub external_id: String,
    pub reconciliation_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VatSummary {
    pub period_label: String,
    pub receipts_gross_cents: i64,
    pub taxable_net_cents: i64,
    pub vat_due_cents: i64,
    pub payments_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeadlineItem {
    pub id: Uuid,
    pub code: String,
    pub label: String,
    pub deadline_date: NaiveDate,
    pub period_label: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentItem {
    pub id: Uuid,
    pub category: String,
    pub title: String,
    pub file_name: String,
    pub storage_key: String,
    pub document_date: Option<NaiveDate>,
    pub expires_at: Option<NaiveDate>,
    pub origin: String,
    pub file_size_bytes: i64,
    pub mime_type: String,
    pub status: String,
    pub ocr_status: String,
    pub classification_confidence: Decimal,
    pub extraction_confidence: Decimal,
    pub duplicate_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaseDetailItem {
    pub id: Uuid,
    pub reference: String,
    pub unit_id: Uuid,
    pub unit_label: String,
    pub property_name: String,
    pub tenant_id: Uuid,
    pub tenant_name: String,
    pub signature_date: Option<NaiveDate>,
    pub effect_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub lease_type: String,
    pub destination: String,
    pub rent_amount_cents: i64,
    pub rent_frequency: String,
    pub payment_day: i16,
    pub vat_mode: String,
    pub index_code: String,
    pub index_base_value: Option<Decimal>,
    pub index_base_date: Option<NaiveDate>,
    pub index_cap_bp: Option<i32>,
    pub charges_mode: String,
    pub charges_amount_cents: i64,
    pub security_deposit_expected_cents: i64,
    pub entry_fee_expected_cents: i64,
    pub entry_fee_status: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaseClauseItem {
    pub id: Uuid,
    pub lease_id: Uuid,
    pub code: String,
    pub title: String,
    pub clause_type: String,
    pub body: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub version_no: i32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaseIndexItem {
    pub id: Uuid,
    pub index_code: String,
    pub period_label: String,
    pub value: Decimal,
    pub source_reference: String,
    pub verified_at: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaseChargeItem {
    pub id: Uuid,
    pub lease_id: Uuid,
    pub charge_type: String,
    pub mode: String,
    pub amount_cents: i64,
    pub variable_formula: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaseReductionItem {
    pub id: Uuid,
    pub lease_id: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub amount_cents: Option<i64>,
    pub percentage_bp: Option<i32>,
    pub reason: String,
    pub original_rent_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaseDepositItem {
    pub id: Uuid,
    pub lease_id: Uuid,
    pub movement_type: String,
    pub amount_cents: i64,
    pub movement_date: NaiveDate,
    pub justification: String,
    pub bank_transaction_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaseGuaranteeItem {
    pub id: Uuid,
    pub lease_id: Uuid,
    pub guarantee_type: String,
    pub guarantor_name: String,
    pub amount_cents: Option<i64>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub document_id: Option<Uuid>,
    pub notes: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaseRentRevisionItem {
    pub id: Uuid,
    pub lease_id: Uuid,
    pub calculation_date: NaiveDate,
    pub effective_date: NaiveDate,
    pub rule_text: String,
    pub index_code: String,
    pub index_period: String,
    pub old_rent_cents: i64,
    pub index_old: Option<Decimal>,
    pub index_new: Option<Decimal>,
    pub cap_bp: Option<i32>,
    pub new_rent_cents: i64,
    pub formula: String,
    pub result_status: String,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentFolderItem {
    pub code: String,
    pub label: String,
    pub relative_path: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentStorageConfigItem {
    pub legal_entity_id: Uuid,
    pub root_path: String,
    pub folder_overrides: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutomationRuleItem {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: String,
    pub trigger_kind: String,
    pub horizon_days: i32,
    pub priority: i32,
    pub auto_execute: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditItem {
    pub id: i64,
    pub occurred_at: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub entity_type: String,
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangeHistoryItem {
    pub id: i64,
    pub effective_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub author: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub reason: String,
    pub before_state: String,
    pub after_state: String,
    pub metadata: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutomationRunResult {
    pub created_tasks: usize,
    pub evaluated_rules: usize,
    pub ran_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VatPolicy {
    pub rate: Decimal,
    pub on_collection: bool,
}

impl VatPolicy {
    pub fn vat_from_gross(self, gross: Decimal) -> Decimal {
        if self.on_collection && self.rate > Decimal::ZERO {
            gross * self.rate / (Decimal::ONE + self.rate)
        } else {
            Decimal::ZERO
        }
    }
}
