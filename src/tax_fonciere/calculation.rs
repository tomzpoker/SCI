use rust_decimal::Decimal;

pub const HUNDRED_PCT_BP: i32 = 10_000;

pub fn apply_share_bp(amount_cents: i64, share_bp: i32) -> i64 {
    if amount_cents <= 0 || share_bp <= 0 {
        return 0;
    }
    let amount = Decimal::from(amount_cents);
    let share  = Decimal::from(share_bp);
    let hundred = Decimal::from(HUNDRED_PCT_BP);
    ((amount * share) / hundred)
        .round_dp(0)
        .to_string()
        .parse::<i64>()
        .unwrap_or(0)
}

pub fn shares_total_ok(shares_bp: &[i32]) -> bool {
    shares_bp.iter().sum::<i32>() == HUNDRED_PCT_BP
}

pub fn prorate_fees(fee_cents: i64, weights_cents: &[i64]) -> Vec<i64> {
    let n = weights_cents.len();
    if n == 0 || fee_cents <= 0 {
        return vec![0; n];
    }
    let total_weight: i64 = weights_cents.iter().sum();
    if total_weight <= 0 {
        let each = fee_cents / (n as i64);
        let mut out = vec![each; n];
        let mut rest = fee_cents - each * (n as i64);
        let mut i = 0;
        while rest > 0 {
            out[i] += 1;
            rest -= 1;
            i = (i + 1) % n;
        }
        return out;
    }
    let mut out = Vec::with_capacity(n);
    let mut allocated: i64 = 0;
    for w in weights_cents.iter() {
        let part = ((Decimal::from(fee_cents) * Decimal::from(*w))
            / Decimal::from(total_weight))
            .round_dp(0)
            .to_string()
            .parse::<i64>()
            .unwrap_or(0);
        out.push(part);
        allocated += part;
    }
    let diff = fee_cents - allocated;
    if diff != 0 {
        if let Some(last) = out.last_mut() {
            *last += diff;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_share() {
        assert_eq!(apply_share_bp(807_000, 10_000), 807_000);
        assert_eq!(apply_share_bp(807_000, 5_000), 403_500);
        assert_eq!(apply_share_bp(807_000, 2_500), 201_750);
    }

    #[test]
    fn test_prorate_fees() {
        let fees = prorate_fees(39_600, &[807_000, 128_700]);
        let sum: i64 = fees.iter().sum();
        assert_eq!(sum, 39_600);
        assert!(fees[0] > fees[1]);
    }

    #[test]
    fn test_prorate_fees_equal_weights() {
        let fees = prorate_fees(10_000, &[0, 0]);
        assert_eq!(fees.iter().sum::<i64>(), 10_000);
        assert_eq!(fees[0], 5_000);
        assert_eq!(fees[1], 5_000);
    }
}