use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssistantTool {
    GetDashboardSummary,
    GetTenantBalance { tenant_id: uuid::Uuid },
    GetLeaseStatus { lease_id: uuid::Uuid },
    GetUnpaidInvoices,
    GetUpcomingDeadlines,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub tool: AssistantTool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}
