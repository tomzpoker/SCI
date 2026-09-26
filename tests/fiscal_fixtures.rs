use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Fixture {
    id: String,
    rule_version: String,
    date: String,
    inputs: Value,
    expected_output: Value,
    source: FixtureSource,
}
#[derive(Debug, Deserialize)]
struct FixtureSource { name: String, reference: String }

fn load(name: &str) -> Fixture {
    let s = match name {
        "vat_basic" => include_str!("../fixtures/fiscal/vat_basic.json"),
        "vat_credit" => include_str!("../fixtures/fiscal/vat_credit.json"),
        "quote_transition" => include_str!("../fixtures/fiscal/quote_transition.json"),
        _ => panic!("fixture inconnue"),
    };
    serde_json::from_str(s).expect("fixture JSON valide")
}

#[test]
fn every_fiscal_fixture_contains_provenance() {
    for name in ["vat_basic", "vat_credit", "quote_transition"] {
        let f = load(name);
        assert!(!f.id.is_empty());
        assert!(!f.rule_version.is_empty());
        assert!(!f.date.is_empty());
        assert!(!f.inputs.is_null());
        assert!(!f.expected_output.is_null());
        assert!(!f.source.name.is_empty());
        assert!(!f.source.reference.is_empty());
    }
}

#[test]
fn fiscal_fixture_outputs_are_stable() {
    let f = load("vat_basic");
    assert_eq!(f.expected_output["vat_cents"], 20000);
    assert_eq!(f.expected_output["payable_cents"], 20000);
}
