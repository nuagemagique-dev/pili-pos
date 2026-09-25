use serde::{Deserialize, Serialize};

/// Belgian SCE 2.0 VAT codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VatCode {
    /// 21% — Beverages (alcohol, soft drinks)
    A,
    /// 12% — Restaurant services (dine-in food)
    B,
    /// 6% — Takeaway food
    C,
    /// 0% — Exempt
    D,
    /// N/A — Out of scope
    X,
}

impl VatCode {
    /// Returns the VAT rate as basis points (e.g. 2100 for 21%).
    pub fn rate_bps(self) -> u32 {
        match self {
            Self::A => 2100,
            Self::B => 1200,
            Self::C => 600,
            Self::D => 0,
            Self::X => 0,
        }
    }

    /// Calculate VAT amount in eurocents from an inclusive price.
    /// Formula: vat = price - (price * 10000) / (10000 + rate_bps)
    pub fn vat_from_inclusive(self, price_cents: i64) -> i64 {
        let rate = self.rate_bps() as i64;
        if rate == 0 {
            return 0;
        }
        price_cents - (price_cents * 10_000) / (10_000 + rate)
    }

    /// Calculate the price excluding VAT from an inclusive price.
    pub fn excl_from_inclusive(self, price_cents: i64) -> i64 {
        price_cents - self.vat_from_inclusive(price_cents)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::X => "X",
        }
    }
}

impl std::fmt::Display for VatCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for VatCode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "A" => Ok(Self::A),
            "B" => Ok(Self::B),
            "C" => Ok(Self::C),
            "D" => Ok(Self::D),
            "X" => Ok(Self::X),
            other => Err(format!("invalid VAT code: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_CODES: [VatCode; 5] = [VatCode::A, VatCode::B, VatCode::C, VatCode::D, VatCode::X];

    #[test]
    fn vat_from_inclusive_21_percent() {
        // 1210 cents inclusive at 21% → VAT = 1210 - (1210 * 10000 / 12100) = 1210 - 1000 = 210
        assert_eq!(VatCode::A.vat_from_inclusive(1210), 210);
    }

    #[test]
    fn vat_from_inclusive_12_percent() {
        // 1120 cents inclusive at 12% → VAT = 1120 - (1120 * 10000 / 11200) = 1120 - 1000 = 120
        assert_eq!(VatCode::B.vat_from_inclusive(1120), 120);
    }

    #[test]
    fn vat_from_inclusive_6_percent() {
        // 1060 cents inclusive at 6% → VAT = 1060 - (1060*10000/10600) = 1060-1000 = 60
        assert_eq!(VatCode::C.vat_from_inclusive(1060), 60);
    }

    #[test]
    fn vat_from_inclusive_zero() {
        assert_eq!(VatCode::D.vat_from_inclusive(1000), 0);
        assert_eq!(VatCode::X.vat_from_inclusive(1000), 0);
    }

    #[test]
    fn vat_from_inclusive_zero_price() {
        for code in ALL_CODES {
            assert_eq!(code.vat_from_inclusive(0), 0);
            assert_eq!(code.excl_from_inclusive(0), 0);
        }
    }

    #[test]
    fn excl_plus_vat_equals_inclusive_for_known_cases() {
        // The identity is true by construction (excl = price - vat), but
        // pin down concrete numbers so a refactor that breaks the
        // relationship is caught immediately, not just by the property
        // test below.
        for (code, price) in [
            (VatCode::A, 1210),
            (VatCode::B, 1120),
            (VatCode::C, 1060),
            (VatCode::D, 1000),
            (VatCode::X, 999),
        ] {
            let vat = code.vat_from_inclusive(price);
            let excl = code.excl_from_inclusive(price);
            assert_eq!(excl + vat, price, "code={code:?} price={price}");
        }
    }

    #[test]
    fn vat_from_inclusive_rounds_down_on_non_exact_division() {
        // 1211 cents at 21%: 1211*10000/12100 = 1000.826... → integer
        // division truncates to 1000, so VAT = 211, not 210.
        assert_eq!(VatCode::A.vat_from_inclusive(1211), 211);
    }

    #[test]
    fn negative_price_refund_mirrors_positive_price() {
        // A refund line carries a negative price; the formula must be
        // exactly antisymmetric — no separate sign-handling branch to
        // drift out of sync with the sale-side formula.
        for code in ALL_CODES {
            for price in [1, 100, 1210, 999_999] {
                assert_eq!(
                    code.vat_from_inclusive(-price),
                    -code.vat_from_inclusive(price),
                    "code={code:?} price={price}"
                );
                assert_eq!(
                    code.excl_from_inclusive(-price),
                    -code.excl_from_inclusive(price),
                    "code={code:?} price={price}"
                );
            }
        }
    }

    #[test]
    fn large_value_does_not_overflow_or_panic() {
        // Well beyond any realistic restaurant bill (1 billion EUR),
        // still far under the i64 * 10_000 overflow boundary
        // (i64::MAX / 10_000 ≈ 9.2e14).
        let price = 100_000_000_000i64; // 1,000,000,000.00 EUR in cents
        for code in ALL_CODES {
            let vat = code.vat_from_inclusive(price);
            let excl = code.excl_from_inclusive(price);
            assert_eq!(excl + vat, price);
            assert!(vat >= 0);
            assert!(excl >= 0);
        }
    }

    #[test]
    fn rate_bps_matches_the_sce_2_0_table() {
        assert_eq!(VatCode::A.rate_bps(), 2100);
        assert_eq!(VatCode::B.rate_bps(), 1200);
        assert_eq!(VatCode::C.rate_bps(), 600);
        assert_eq!(VatCode::D.rate_bps(), 0);
        assert_eq!(VatCode::X.rate_bps(), 0);
    }

    #[test]
    fn as_str_and_from_str_round_trip() {
        for code in ALL_CODES {
            assert_eq!(code.as_str().parse::<VatCode>().unwrap(), code);
        }
    }

    #[test]
    fn from_str_rejects_unknown_codes() {
        for bad in ["a", "Z", "", "AB", "0"] {
            assert!(bad.parse::<VatCode>().is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn display_matches_as_str() {
        for code in ALL_CODES {
            assert_eq!(code.to_string(), code.as_str());
        }
    }

    // ── Property tests: arithmetic invariants over realistic ranges ──
    //
    // Money is i64 eurocents, never floats (CLAUDE.md). These pin down
    // the two invariants every downstream consumer (event.rs,
    // ticket.rs, the reports aggregation) relies on implicitly:
    // excl + vat == incl, and VAT never exceeds the inclusive price.

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        /// Realistic-and-then-some restaurant bill-line range: 0 to
        /// 100,000,000.00 EUR, well clear of the i64*10_000 overflow
        /// boundary (i64::MAX / 10_000 ≈ 9.22 * 10^14).
        const REALISTIC_CENTS: std::ops::RangeInclusive<i64> = 0..=10_000_000_000i64;

        fn any_code() -> impl Strategy<Value = VatCode> {
            prop_oneof![
                Just(VatCode::A),
                Just(VatCode::B),
                Just(VatCode::C),
                Just(VatCode::D),
                Just(VatCode::X),
            ]
        }

        proptest! {
            #[test]
            fn excl_plus_vat_always_equals_inclusive(
                code in any_code(),
                price in REALISTIC_CENTS,
            ) {
                let vat = code.vat_from_inclusive(price);
                let excl = code.excl_from_inclusive(price);
                prop_assert_eq!(excl + vat, price);
            }

            #[test]
            fn vat_never_exceeds_the_inclusive_price(
                code in any_code(),
                price in REALISTIC_CENTS,
            ) {
                let vat = code.vat_from_inclusive(price);
                prop_assert!(vat >= 0);
                prop_assert!(vat <= price);
            }

            #[test]
            fn excl_is_never_negative_for_a_non_negative_price(
                code in any_code(),
                price in REALISTIC_CENTS,
            ) {
                prop_assert!(code.excl_from_inclusive(price) >= 0);
            }

            #[test]
            fn negation_is_exactly_antisymmetric(
                code in any_code(),
                price in REALISTIC_CENTS,
            ) {
                prop_assert_eq!(code.vat_from_inclusive(-price), -code.vat_from_inclusive(price));
                prop_assert_eq!(code.excl_from_inclusive(-price), -code.excl_from_inclusive(price));
            }

            /// Many lines, same VAT code: whether you compute VAT once
            /// on the summed inclusive price, or line by line and add
            /// the results (integer cents — always exact addition,
            /// never floats), both stay within [0, total] and both
            /// satisfy excl+vat==their own total exactly.
            #[test]
            fn per_rate_totals_stay_consistent_across_many_lines(
                code in any_code(),
                prices in proptest::collection::vec(0..=100_000_000i64, 1..20),
            ) {
                let total: i64 = prices.iter().sum();
                let vat_of_total = code.vat_from_inclusive(total);
                prop_assert!((0..=total).contains(&vat_of_total));
                prop_assert_eq!(vat_of_total + code.excl_from_inclusive(total), total);

                let vat_summed_per_line: i64 =
                    prices.iter().map(|&p| code.vat_from_inclusive(p)).sum();
                prop_assert!((0..=total).contains(&vat_summed_per_line));
            }
        }
    }
}
