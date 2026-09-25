use super::{DocumentCandidate, DocumentType};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationInput {
    pub document_id: uuid::Uuid,
    pub filename: String,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub document_id: uuid::Uuid,
    pub document_type: DocumentType,
    pub confidence: f32,
    pub reasons: Vec<String>,
}
pub fn classify(input: ClassificationInput) -> ClassificationResult {
    let text = format!("{} {}", input.filename, input.text).to_lowercase();
    let (document_type, confidence, reasons) = if text.contains("relevé bancaire")
        || text.contains("releve bancaire")
        || text.contains("iban")
    {
        (
            DocumentType::BankStatement,
            0.90,
            vec!["Indices bancaires détectés".to_owned()],
        )
    } else if text.contains("facture") && (text.contains("fournisseur") || text.contains("siret")) {
        (
            DocumentType::InvoiceSupplier,
            0.88,
            vec!["Indices de facture fournisseur détectés".to_owned()],
        )
    } else if text.contains("loyer")
        || text.contains("quittance")
        || text.contains("appel de loyer")
    {
        (
            DocumentType::RentInvoice,
            0.85,
            vec!["Indices liés au loyer détectés".to_owned()],
        )
    } else if text.contains("avenant") || text.contains("amendement") {
        (
            DocumentType::LeaseAmendment,
            0.90,
            vec!["Indice d'avenant contractuel détecté".to_owned()],
        )
    } else if text.contains("bail") || text.contains("preneur") || text.contains("bailleur") {
        (
            DocumentType::Lease,
            0.84,
            vec!["Indices contractuels de location détectés".to_owned()],
        )
    } else if text.contains("assurance") {
        (
            DocumentType::Insurance,
            0.82,
            vec!["Indices d'assurance détectés".to_owned()],
        )
    } else if text.contains("impôt")
        || text.contains("taxe")
        || text.contains("déclaration fiscale")
    {
        (
            DocumentType::TaxDocument,
            0.80,
            vec!["Indices fiscaux détectés".to_owned()],
        )
    } else if text.contains("paiement")
        || text.contains("virement")
        || text.contains("preuve de paiement")
    {
        (
            DocumentType::PaymentProof,
            0.78,
            vec!["Indices de paiement détectés".to_owned()],
        )
    } else if text.contains("courrier") || text.contains("madame") || text.contains("monsieur") {
        (
            DocumentType::Correspondence,
            0.72,
            vec!["Indices de courrier détectés".to_owned()],
        )
    } else if text.contains("justificatif") || text.contains("piece justificative") || text.contains("pièce justificative") {
        (
            DocumentType::SupportingDocument,
            0.72,
            vec!["Indices de justificatif détectés".to_owned()],
        )
    } else {
        (
            DocumentType::Unknown,
            0.20,
            vec!["Aucun type suffisamment identifiable".to_owned()],
        )
    };
    ClassificationResult {
        document_id: input.document_id,
        document_type,
        confidence,
        reasons,
    }
}
pub fn to_candidate(
    input: &ClassificationInput,
    result: &ClassificationResult,
) -> DocumentCandidate {
    DocumentCandidate {
        id: result.document_id,
        filename: input.filename.clone(),
        document_type: result.document_type.clone(),
        classification_confidence: result.confidence,
        source: "ocr".to_owned(),
        requires_validation: true,
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies_bank_statement() {
        let r=classify(ClassificationInput{document_id:uuid::Uuid::nil(),filename:"releve.pdf".into(),text:"RELEVÉ BANCAIRE IBAN FR76".into()});
        assert_eq!(r.document_type, DocumentType::BankStatement);
        assert!(r.confidence>0.8);
    }
}
