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
