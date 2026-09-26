use chrono::{Datelike, Duration, NaiveDate, Timelike, TimeZone, Utc};
use sci_family_pilot::billing::due_date_for_month;

#[test]
fn leap_year_and_year_change_boundaries() {
    assert_eq!(NaiveDate::from_ymd_opt(2028, 2, 29).unwrap().day(), 29);
    assert!(NaiveDate::from_ymd_opt(2027, 2, 29).is_none());
    let end = Utc.with_ymd_and_hms(2026,12,31,23,59,59).unwrap();
    let start = end + Duration::seconds(1);
    assert_eq!(start.year(),2027); assert_eq!(start.month(),1); assert_eq!(start.day(),1); assert_eq!(start.hour(),0); assert_eq!(start.minute(),0);
}

#[test]
fn invoice_due_day_is_clamped_each_month() {
    for month in 1..=12 {
        let first=NaiveDate::from_ymd_opt(2026,month,1).unwrap();
        let due=due_date_for_month(first,31);
        assert_eq!(due.month(),month);
    }
}
