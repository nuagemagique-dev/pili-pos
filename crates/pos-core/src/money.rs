//! Money helpers, in cents.

/// Spread `amount_cents` (a whole-order change) over lines in
/// proportion to their `weights` (their totals), to the cent: the
/// shares add up exactly, the remainder going to the largest
/// fractional parts first. A line never gets more than its weight
/// (the shares keep the sign of `amount_cents`).
pub fn distribute(weights: &[i64], amount_cents: i64) -> Vec<i64> {
    let total: i64 = weights.iter().sum();
    if total <= 0 || weights.is_empty() {
        return vec![0; weights.len()];
    }
    let sign = amount_cents.signum();
    let amount = amount_cents.abs().min(total);
    let mut shares: Vec<i64> = Vec::with_capacity(weights.len());
    let mut remainders: Vec<(i64, usize)> = Vec::with_capacity(weights.len());
    for (i, &w) in weights.iter().enumerate() {
        let w = w.max(0);
        let exact = i128::from(amount) * i128::from(w);
        shares.push((exact / i128::from(total)) as i64);
        remainders.push(((exact % i128::from(total)) as i64, i));
    }
    let mut left = amount - shares.iter().sum::<i64>();
    // Largest remainder first; ties to the earlier line.
    remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, i) in remainders {
        if left == 0 {
            break;
        }
        if shares[i] < weights[i].max(0) {
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
    fn distribute_never_gives_a_line_more_than_its_weight() {
        let shares = distribute(&[1, 1000], -1000);
        assert!(shares[0] >= -1 && shares[1] >= -1000);
        assert_eq!(shares.iter().sum::<i64>(), -1000);
    }
}
