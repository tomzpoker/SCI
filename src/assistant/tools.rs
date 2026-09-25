use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantToolSpec {
    pub code: String,
    pub label_fr: String,
    pub risk_level: String,
    pub read_only: bool,
    pub confirmation_required: bool,
    pub authorized_roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssistantTool {
    GetCashBalance,
    GetRealBankBalance,
    GetForecast { horizon_months: i32 },
    GetUnpaidRents,
    GetUpcomingDeadlines,
    CalculateRentRevision { lease_id: Uuid },
    CalculateVat { period_start: chrono::NaiveDate, period_end: chrono::NaiveDate },
    PrepareVatReturn { period_start: chrono::NaiveDate, period_end: chrono::NaiveDate },
    PrepareInvoice { lease_id: Uuid, issue_date: chrono::NaiveDate, due_date: chrono::NaiveDate },
    PrepareReminder { arrears_case_id: Uuid },
    SearchDocuments { query: String },
    GetLease { lease_id: Uuid },
    GetTenant { tenant_id: Uuid },
    GetProperty { property_id: Uuid },
    SimulateTax { label: String, taxable_base_cents: i64, rate_bp: i32 },
    CreateDraft { template_code: String, reference: String, recipient: String, body: String },
    RequestUserValidation { action_code: String, subject_type: String, subject_id: Option<Uuid>, proposed_payload: serde_json::Value, reason: String, risk_level: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolRequest {
    pub tool: AssistantTool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
    pub uncertainty: String,
}

pub fn default_tool_catalog() -> Vec<AssistantToolSpec> {
    vec![
        spec("get_cash_balance", "Consulter le solde théorique", "LOW", true, false),
        spec("get_real_bank_balance", "Consulter le solde bancaire réel", "LOW", true, false),
        spec("get_forecast", "Consulter les prévisions de trésorerie", "LOW", true, false),
        spec("get_unpaid_rents", "Consulter les loyers impayés", "LOW", true, false),
        spec("get_upcoming_deadlines", "Consulter les prochaines échéances", "LOW", true, false),
        spec("calculate_rent_revision", "Calculer une révision de loyer", "MEDIUM", true, false),
        spec("calculate_vat", "Calculer la TVA", "MEDIUM", true, false),
        spec("prepare_vat_return", "Préparer une déclaration TVA", "HIGH", false, true),
        spec("prepare_invoice", "Préparer une facture", "HIGH", false, true),
        spec("prepare_reminder", "Préparer une relance", "HIGH", false, true),
        spec("search_documents", "Rechercher des documents", "LOW", true, false),
        spec("get_lease", "Consulter un bail", "LOW", true, false),
        spec("get_tenant", "Consulter un locataire", "LOW", true, false),
        spec("get_property", "Consulter un bien", "LOW", true, false),
        spec("simulate_tax", "Simuler une imposition", "MEDIUM", true, false),
        spec("create_draft", "Créer un brouillon", "HIGH", false, true),
        spec("request_user_validation", "Demander une validation humaine", "HIGH", false, true),
    ]
}

fn spec(code: &str, label_fr: &str, risk_level: &str, read_only: bool, confirmation_required: bool) -> AssistantToolSpec {
    let authorized_roles = if code == "request_user_validation" { vec!["MANAGER".into(),"ACCOUNTANT".into(),"AI_AGENT".into()] } else if read_only { vec!["MANAGER".into(),"ACCOUNTANT".into(),"VIEWER".into(),"AI_AGENT".into()] } else { vec!["MANAGER".into(),"ACCOUNTANT".into(),"AI_AGENT".into()] }; AssistantToolSpec { code: code.into(), label_fr: label_fr.into(), risk_level: risk_level.into(), read_only, confirmation_required, authorized_roles }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_contains_requested_tools() {
        let c = default_tool_catalog();
        assert_eq!(c.len(), 17);
        assert!(c.iter().any(|x| x.code == "request_user_validation" && x.confirmation_required));
    }
}

#[cfg(test)]
mod hardening_tests {
    use super::*;
    #[test]
    fn all_requested_tool_codes_exist_exactly_once() {
        let requested = ["get_cash_balance","get_real_bank_balance","get_forecast","get_unpaid_rents","get_upcoming_deadlines","calculate_rent_revision","calculate_vat","prepare_vat_return","prepare_invoice","prepare_reminder","search_documents","get_lease","get_tenant","get_property","simulate_tax","create_draft","request_user_validation"];
        let catalog=default_tool_catalog();
        for code in requested { assert_eq!(catalog.iter().filter(|t| t.code==code).count(),1, "tool manquant ou dupliqué: {code}"); }
    }
}
