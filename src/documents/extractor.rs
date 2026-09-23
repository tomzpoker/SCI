use serde::{Deserialize, Serialize};

use super::{DocumentExtraction, DocumentType, ExtractedField};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionInput {
    pub document_id: uuid::Uuid,
    pub document_type: DocumentType,
    pub text: String,
}

pub fn extract(input: ExtractionInput) -> DocumentExtraction {
    let fields = match input.document_type {
        DocumentType::InvoiceSupplier => extract_supplier_invoice(&input.text),
        DocumentType::RentInvoice => extract_rent_invoice(&input.text),
        DocumentType::BankStatement => extract_bank_statement(&input.text),
        _ => Vec::new(),
    };
    DocumentExtraction {
        document_id: input.document_id,
        fields,
        requires_user_validation: true,
    }
}

fn extract_supplier_invoice(text: &str) -> Vec<ExtractedField> {
    vec![
        field(
            "supplier",
            find_after(text, &["fournisseur", "vendor"]),
            0.65,
        ),
        field(
            "required_ttc",
            find_after(text, &["ttc", "toutes taxes"]),
            0.55,
        ),
    ]
}

fn extract_rent_invoice(text: &str) -> Vec<ExtractedField> {
    vec![
        field("tenant", find_after(text, &["locataire", "tenant"]), 0.65),
        field("amount_ttc", find_after(text, &["ttc", "total"]), 0.55),
    ]
}

fn extract_bank_statement(text: &str) -> Vec<ExtractedField> {
    vec![
        field(
            "reference",
            find_after(text, &["iban", "compte", "reference"]),
            0.65,
        ),
        field(
            "statement_date",
            find_after(text, &["date", "periode"]),
            0.55,
        ),
    ]
}

fn field(name: &str, value: String, confidence: f32) -> ExtractedField {
    ExtractedField {
        name: name.to_string(),
        value,
        confidence,
    }
}

fn find_after(text: &str, labels: &[&str]) -> String {
    for label in labels {
        if let Some(pos) = text.to_lowercase().find(&label.to_lowercase()) {
            let value = text[pos + label.len()..]
                .trim()
                .lines()
                .next()
                .unwrap_or("")
                .trim();
            if !value.is_empty() {
                return value.to_string();
            }
        }
    }
    String::new()
}
