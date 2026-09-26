use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use sci_family_pilot::{engines, reproducibility, rules};

#[test]
fn legacy_vat_and_reference_calculations_remain_stable() {
    assert_eq!(engines::vat_from_gross_cents(120_000, 2000), 20_000);
    let definition = json!({"operation":"ADD","paths":["a","b"]});
    assert_eq!(rules::evaluate_definition(&definition, &json!({"a":12,"b":30})).unwrap(), json!(42));
    assert_eq!(reproducibility::describe_formula(&definition), "Somme(a + b)");
}

#[test]
fn known_financial_boundaries_remain_stable() {
    let d = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
    assert_eq!(d.succ_opt().unwrap(), NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());
    let pct = Decimal::new(25, 2);
    assert_eq!(pct * Decimal::new(400, 0), Decimal::new(100, 0));
    let _ = Utc::now();
}
