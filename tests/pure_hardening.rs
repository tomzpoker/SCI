use chrono::{Datelike, Duration, NaiveDate, TimeZone, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use sci_family_pilot::{billing, cash, engines, leases, reproducibility, rules, transactions};
use sci_family_pilot::transactions::{NormalizedTransactionInput, TransactionDirection};
use uuid::Uuid;

#[test]
fn vat_gross_conversion_is_bounded() {
    for rate in [500, 1000, 2000, 5500, 10000, 20000] {
        let vat = engines::vat_from_gross_cents(100_000, rate);
        assert!(vat >= 0);
        assert!(vat < 100_000);
    }
}

#[test]
fn cash_positions_remain_separate() {
    let p = cash::separate_cash_positions(10, 20, 30, 40);
    assert_eq!(p.map(|(_, v)| v), [10, 20, 30, 40]);
    assert_eq!(p[0].0.code(), "REAL");
    assert_eq!(p[3].0.code(), "FORECAST");
}

#[test]
fn indexed_rent_is_monotone_with_index() {
    let old = 100_000;
    let base = Decimal::new(100, 0);
    let mut previous = old;
    for index in [101, 102, 103, 104] {
        let current = leases::calculate_indexed_rent(old, base, Decimal::new(index, 0), None).unwrap();
        assert!(current >= previous);
        previous = current;
    }
}

#[test]
fn prorata_respects_contract_bounds() {
    let start = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2026, 2, 28).unwrap();
    let full = billing::prorated_amount(28_000, start, None, start, end);
    assert_eq!(full, 28_000);
    let partial = billing::prorated_amount(28_000, NaiveDate::from_ymd_opt(2026, 2, 15).unwrap(), None, start, end);
    assert!(partial > 0 && partial < full);
}

#[test]
fn due_date_stays_inside_month() {
    for month in 1..=12 {
        let first = NaiveDate::from_ymd_opt(2028, month, 1).unwrap();
        let due = billing::due_date_for_month(first, 31);
        assert_eq!(due.year(), 2028);
        assert_eq!(due.month(), month);
    }
}

#[test]
fn rule_engine_is_deterministic() {
    let definition = json!({"operation":"SUBTRACT","paths":["gross","vat"]});
    let inputs = json!({"gross": 1250, "vat": 250});
    let a = rules::evaluate_definition(&definition, &inputs).unwrap();
    let b = rules::evaluate_definition(&definition, &inputs).unwrap();
    assert_eq!(a, b);
    assert_eq!(a, json!(1000));
}

#[test]
fn formula_description_is_reproducible() {
    let definition = json!({"operation":"MULTIPLY_BPS","path":"amount","rate_bp":2000});
    assert_eq!(reproducibility::describe_formula(&definition), "amount × 2000 / 10 000");
}

#[test]
fn transaction_normalization_is_lossless_for_business_fields() {
    let input = NormalizedTransactionInput {
        occurred_at: Utc.with_ymd_and_hms(2026, 9, 25, 12, 0, 0).unwrap(),
        amount_cents: 125_000,
        direction: TransactionDirection::In,
        source_type: " bank ".into(),
        source_id: Some(Uuid::nil()),
        status: " pending ".into(),
        external_key: "REF-1".into(),
        counterparty: "  ACME  ".into(),
        label: "  LOYER  ".into(),
        metadata: json!({"x": 1}),
    };
    let out = transactions::normalize_transaction(input).unwrap();
    assert_eq!(out.source_type, "BANK");
    assert_eq!(out.status, "PENDING");
    assert_eq!(out.counterparty, "ACME");
    assert_eq!(out.label, "LOYER");
    assert_eq!(out.amount_cents, 125_000);
}

#[test]
fn temporal_boundary_31_dec_to_1_jan_is_exact() {
    let dec31 = Utc.with_ymd_and_hms(2026, 12, 31, 23, 59, 59).unwrap();
    let jan1 = dec31 + Duration::seconds(1);
    assert_eq!(jan1.date_naive(), NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());
}
