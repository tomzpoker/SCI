use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdministrativeAction {
    GenerateRentInvoices {
        year: i32,
        month: u32,
    },
    ImportDocument {
        document_id: uuid::Uuid,
    },
    CreateTask {
        title: String,
        description: String,
    },
    RecordPayment {
        invoice_id: uuid::Uuid,
        amount_cents: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedAction {
    pub action: AdministrativeAction,
    pub explanation: String,
    pub requires_approval: bool,
}
