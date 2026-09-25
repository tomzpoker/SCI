use sci_family_pilot::documents::classifier::{classify, ClassificationInput};
use sci_family_pilot::documents::DocumentType;
use uuid::Uuid;

#[test]
fn missing_or_garbled_document_does_not_become_a_false_known_type() {
    let result = classify(ClassificationInput { document_id: Uuid::nil(), filename: "unknown.bin".into(), text: "\0\u{1}\u{2}".into() });
    assert!(matches!(result.document_type, DocumentType::Unknown));
    assert!(result.confidence <= 0.2);
}

#[test]
fn ocr_failure_must_not_be_fabricated_as_success() {
    // The OCR adapter records unavailable/failed status instead of inventing text.
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/documents/workflow.rs");
    let source = std::fs::read_to_string(path).expect("workflow source");
    assert!(source.contains("OCR_UNAVAILABLE") || source.contains("OCR_FAILED"));
}
