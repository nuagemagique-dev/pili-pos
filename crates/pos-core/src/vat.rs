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
    fn vat_from_inclusive_zero() {
        assert_eq!(VatCode::D.vat_from_inclusive(1000), 0);
        assert_eq!(VatCode::X.vat_from_inclusive(1000), 0);
    }
}
