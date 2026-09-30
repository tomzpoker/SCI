use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnpaidInvoiceItem {
    pub dolibarr_invoice_id: String,
    pub invoice_ref: String,
    pub client_id: String,
    pub client_name: String,
    pub client_email: String,
    pub issue_date_ts: i64,
    pub due_date_ts: i64,
    pub total_ttc: f64,
    pub paid: f64,
    pub outstanding: f64,
    pub days_overdue: i32,
    pub relance_level_sent: i32,
    pub last_relance_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnpaidTenantSummary {
    pub client_id: String,
    pub client_name: String,
    pub client_email: String,
    pub total_outstanding: f64,
    pub max_days_overdue: i32,
    pub invoice_count: i32,
    pub highest_level_sent: i32,
    pub invoices: Vec<UnpaidInvoiceItem>,
}

impl UnpaidTenantSummary {
    pub fn severity(&self) -> Severity {
        match self.max_days_overdue {
            d if d <= 0 => Severity::Ok,
            d if d <= 30 => Severity::Warning,
            d if d <= 60 => Severity::Late,
            _ => Severity::Critical,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Ok,
    Warning,
    Late,
    Critical,
}

impl Severity {
    pub fn color(self) -> &'static str {
        match self {
            Severity::Ok => "#22c55e",
            Severity::Warning => "#f59e0b",
            Severity::Late => "#ef4444",
            Severity::Critical => "#dc2626",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelanceHistoryItem {
    pub id: String,
    pub invoice_ref: String,
    pub client_name: String,
    pub level: i32,
    pub subject: String,
    pub sent_at_ts: i64,
    pub sent_via: String,
    pub status: String,
    pub error_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalBarItem {
    pub unit_id: String,
    pub unit_code: String,
    pub unit_label: String,
    pub property_name: String,
    pub tenant_name: Option<String>,
    pub tenant_email: Option<String>,
    pub dolibarr_client_id: Option<String>,
    pub total_outstanding: f64,
    pub max_days_overdue: i32,
    pub invoice_count: i32,
    pub highest_level_sent: i32,
    pub status: LocalStatus,
    pub invoices: Vec<UnpaidInvoiceItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LocalStatus {
    Vacant,
    UpToDate,
    Late,
    Critical,
}

impl LocalStatus {
    pub fn color(self) -> &'static str {
        match self {
            LocalStatus::Vacant => "#475569",
            LocalStatus::UpToDate => "#22c55e",
            LocalStatus::Late => "#f59e0b",
            LocalStatus::Critical => "#ef4444",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            LocalStatus::Vacant => "Libre",
            LocalStatus::UpToDate => "À jour",
            LocalStatus::Late => "Retard",
            LocalStatus::Critical => "Critique",
        }
    }
}

/// Prévisualisation d'une relance (sans envoi).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelancePreview {
    pub level_code: i32,
    pub level_label: String,
    pub subject: String,
    pub body: String,
    pub recipient_email: String,
    pub days_overdue: i32,
    pub outstanding: f64,
    pub penalties: f64,
    pub forfait: f64,
    pub total_due: f64,
}