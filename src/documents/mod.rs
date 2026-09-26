pub mod classifier;
pub mod email;
pub mod extractor;
pub mod ocr;
pub(crate) mod workflow;
pub use workflow::DocumentsWorkflowPage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocumentType {
    InvoiceSupplier,
    RentInvoice,
    BankStatement,
    PaymentProof,
    Lease,
    LeaseAmendment,
    TaxDocument,
    Insurance,
    Administrative,
    Correspondence,
    SupportingDocument,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentCandidate {
    pub id: uuid::Uuid,
    pub filename: String,
    pub document_type: DocumentType,
    pub classification_confidence: f32,
    pub source: String,
    pub requires_validation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedField {
    pub name: String,
    pub value: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentExtraction {
    pub document_id: uuid::Uuid,
    pub fields: Vec<ExtractedField>,
    pub requires_user_validation: bool,
}
