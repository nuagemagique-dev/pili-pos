//! Money helpers, in cents.

use serde_json::Value;

/// An amount in cents as an `f64` (exact below 2^53, far above any real
/// amount), for the few float formulas the FDM wire format imposes.
#[must_use]
pub fn to_f64(cents: i64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "cent amounts stay far below 2^53, so the i64 to f64 conversion is exact"
    )]
    let v = cents as f64;
    v
}

/// Rounds a float to the nearest whole number (ties away from zero) as
/// an `i64`. Saturates on out-of-range input; NaN is `0`.
#[must_use]
pub fn round_to_i64(v: f64) -> i64 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "f64 to i64 `as` saturates and maps NaN to 0, which is the documented behaviour"
    )]
    let r = v.round() as i64;
    r
}

/// Cents as the euro amount the FDM wire format carries (`1250` is `12.5`).
#[must_use]
pub fn eur(cents: i64) -> f64 {
    to_f64(cents) / 100.0
}

/// A euro amount rounded to whole cents (`12.5` is `1250`). Saturates on
/// out-of-range or non-finite input (NaN is `0`).
#[must_use]
pub fn cents_from_eur(euros: f64) -> i64 {
    round_to_i64(euros * 100.0)
}

/// Reads an archived JSON euro amount as cents. `None` when the value is
/// not a number, so a malformed archive row is never silently read as 0.
#[must_use]
pub fn cents_of(v: &Value) -> Option<i64> {
    v.as_f64().map(cents_from_eur)
}

/// Spread `amount_cents` (a whole-order change) over lines in
/// proportion to their `weights` (their totals), to the cent: the
/// shares add up exactly, the remainder going to the largest
/// fractional parts first. A line never gets more than its weight
/// (the shares keep the sign of `amount_cents`). Negative weights count
/// as zero.
#[must_use]
pub fn distribute(weights: &[i64], amount_cents: i64) -> Vec<i64> {
    let weights: Vec<i64> = weights.iter().map(|w| (*w).max(0)).collect();
    let total: i64 = weights.iter().sum();
    if total <= 0 {
        return vec![0; weights.len()];
    }
    let sign = amount_cents.signum();
    let amount = amount_cents.abs().min(total);
    let mut shares: Vec<i64> = Vec::with_capacity(weights.len());
    let mut remainders: Vec<(i64, usize)> = Vec::with_capacity(weights.len());
    for (i, &w) in weights.iter().enumerate() {
        let exact = i128::from(amount) * i128::from(w);
        // `amount <= total` and `w <= total`, so both fit in an i64.
        shares.push(i64::try_from(exact / i128::from(total)).expect("share is at most the amount"));
        remainders.push((
            i64::try_from(exact % i128::from(total)).expect("remainder is below the total"),
            i,
        ));
    }
    let mut left = amount - shares.iter().sum::<i64>();
    // Largest remainder first; ties to the earlier line.
    remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, i) in remainders {
        if left == 0 {
            break;
        }
        if shares[i] < weights[i] {
            shares[i] += 1;
            left -= 1;
        }
    }
    shares.into_iter().map(|s| s * sign).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distribute_adds_up_to_the_cent() {
        assert_eq!(distribute(&[4000, 1200], -2600), vec![-2000, -600]);
        let shares = distribute(&[333, 333, 334], -100);
        assert_eq!(shares.iter().sum::<i64>(), -100);
        assert_eq!(shares, vec![-33, -33, -34]);
        assert_eq!(
            distribute(&[100, 50], -1000),
            vec![-100, -50],
            "capped at the total"
        );
        assert_eq!(distribute(&[], -5), Vec::<i64>::new());
        assert_eq!(distribute(&[0, 0], -5), vec![0, 0]);
    }

    #[test]
    fn distribute_ignores_negative_weights() {
        // The -50 line counts as 0: the whole -50 goes to the first line.
        assert_eq!(distribute(&[100, -50], -50), vec![-50, 0]);
        let shares = distribute(&[100, -50, 100], -50);
        assert_eq!(shares.iter().sum::<i64>(), -50);
        assert_eq!(shares[1], 0);
    }

    #[test]
    fn euro_cent_conversions_round_trip_and_read_json() {
        assert_eq!(eur(1250), 12.5);
        assert_eq!(cents_from_eur(12.5), 1250);
        assert_eq!(cents_from_eur(0.295), 30);
        assert_eq!(cents_from_eur(-0.005), -1);
        assert_eq!(cents_from_eur(f64::NAN), 0);
        assert_eq!(cents_of(&serde_json::json!(3.4)), Some(340));
        assert_eq!(cents_of(&serde_json::json!(7)), Some(700));
        assert_eq!(cents_of(&serde_json::json!("3.4")), None);
        assert_eq!(cents_of(&serde_json::Value::Null), None);
    }

    #[test]
    fn distribute_never_gives_a_line_more_than_its_weight() {
        let shares = distribute(&[1, 1000], -1000);
        assert!(shares[0] >= -1 && shares[1] >= -1000);
        assert_eq!(shares.iter().sum::<i64>(), -1000);
    }
}
