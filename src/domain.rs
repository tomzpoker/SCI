use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskState { Planned, Ready, Running, Blocked, Done, Skipped }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskItem {
    pub id: Uuid,
    pub title: String,
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
    pub properties: i64,
    pub units: i64,
    pub tenants: i64,
    pub leases: i64,
    pub invoices: i64,
    pub payments: i64,
    pub bank_transactions: i64,
    pub unmatched_bank: i64,
    pub documents: i64,
    pub automation_rules: i64,
    pub enabled_automation_rules: i64,
    pub tax_deadlines: i64,
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
    pub iban: String,
    pub bic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    pub lead_days: Option<i64>,
    pub tolerance_cents: Option<i64>,
    pub basis: Option<String>,
    pub check_documents: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
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
