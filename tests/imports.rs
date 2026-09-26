use sci_family_pilot::documents::classifier::{classify, ClassificationInput};
use sci_family_pilot::documents::extractor::{extract, ExtractionInput};
use sci_family_pilot::documents::DocumentType;
use uuid::Uuid;

#[test]
fn pdf_ocr_import_chain_classifies_then_extracts() {
    let text = "RELEVÉ BANCAIRE IBAN FR76\nDate : 25/09/2026\nSolde 5000";
    let input = ClassificationInput { document_id: Uuid::new_v4(), filename: "releve.pdf".into(), text: text.into() };
    let classification = classify(input.clone());
    assert_eq!(classification.document_type, DocumentType::BankStatement);
    let extraction = extract(ExtractionInput { document_id: input.document_id, document_type: classification.document_type, text: input.text });
    assert!(extraction.requires_user_validation);
    assert!(extraction.fields.iter().any(|f| f.name == "reference"));
}

#[test]
fn unknown_import_remains_unknown() {
    let result=classify(ClassificationInput{document_id:Uuid::nil(),filename:"scan.dat".into(),text:"binary".into()});
    assert_eq!(result.document_type,DocumentType::Unknown);
}
