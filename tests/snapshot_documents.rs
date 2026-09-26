use chrono::NaiveDate;
use sci_family_pilot::generation::{kind_requires_validation, render_template_text};
use sci_family_pilot::generation::pdf::{write_pdf, PdfSpec};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[test]
fn template_snapshot_matches_fixture() {
    let template = include_str!("../fixtures/snapshots/letter_template.txt");
    let expected = include_str!("../fixtures/snapshots/letter_expected.txt").trim_end();
    let actual = render_template_text(template, &json!({"name":"SCI TEST","amount":"1250,00 €"}));
    assert_eq!(actual, expected);
}

#[test]
fn pdf_snapshot_has_a4_and_metadata_markers() {
    let mut path = PathBuf::from(std::env::temp_dir());
    path.push(format!("sci-snapshot-{}.pdf", Uuid::new_v4()));
    let spec = PdfSpec {
        header: "SCI TEST".into(), footer: "{reference} • {status} • page {page}/{pages}".into(),
        reference: "FAC-TEST".into(), status: "BROUILLON".into(), date: NaiveDate::from_ymd_opt(2026,9,25).unwrap(),
        lines: vec!["Document de test".into()], watermark: "BROUILLON".into(), logo_path: None, margins_mm: (10.0,10.0,10.0,10.0),
    };
    write_pdf(&path, &spec).expect("write pdf");
    let bytes = fs::read(&path).expect("read pdf");
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.starts_with("%PDF-1.4"));
    assert!(text.contains("595 842"));
    assert!(text.contains("4641432D54455354")); // FAC-TEST en WinAnsi hex
    assert!(text.contains("42524F55524C4C4F4E")); // BROUILLON en WinAnsi hex
    fs::remove_file(path).ok();
    assert!(kind_requires_validation("LEASE"));
}
