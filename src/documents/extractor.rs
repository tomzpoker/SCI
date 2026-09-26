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
        DocumentType::Lease | DocumentType::LeaseAmendment => extract_lease(&input.text),
        DocumentType::TaxDocument => extract_tax(&input.text),
        DocumentType::Correspondence => extract_correspondence(&input.text),
        DocumentType::SupportingDocument | DocumentType::Insurance | DocumentType::Administrative | DocumentType::PaymentProof | DocumentType::Unknown => Vec::new(),
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

fn extract_lease(text: &str) -> Vec<ExtractedField> {
    vec![
        field("reference", find_after(text, &["référence", "reference", "n°"]), 0.60),
        field("tenant", find_after(text, &["preneur", "locataire"]), 0.70),
        field("rent", find_after(text, &["loyer", "loyer mensuel"]), 0.65),
        field("index", find_after(text, &["indice", "ILC", "ICC"]), 0.60),
    ]
}

fn extract_tax(text: &str) -> Vec<ExtractedField> {
    vec![
        field("tax_type", find_after(text, &["taxe", "impôt"]), 0.65),
        field("reference", find_after(text, &["référence", "reference"]), 0.55),
        field("amount", find_after(text, &["montant", "total"]), 0.55),
    ]
}

fn extract_correspondence(text: &str) -> Vec<ExtractedField> {
    vec![
        field("sender", find_after(text, &["de", "expéditeur"]), 0.45),
        field("date", find_after(text, &["date"]), 0.45),
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


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_lease_fields() {
        let r=extract(ExtractionInput{document_id:uuid::Uuid::nil(),document_type:DocumentType::Lease,text:"Référence B-12\nPreneur ACME\nLoyer 1500\nIndice ILC".into()});
        assert!(r.fields.iter().any(|f| f.name=="rent" && f.value.contains("1500")));
        assert!(r.fields.iter().any(|f| f.name=="index" && f.value.contains("ILC")));
    }
}
