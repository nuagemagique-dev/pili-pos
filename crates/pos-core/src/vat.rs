use serde::{Deserialize, Serialize};

/// Belgian SCE 2.0 VAT codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VatCode {
    /// 21% - Beverages (alcohol, soft drinks)
    A,
    /// 12% - Restaurant services (dine-in food)
    B,
    /// 6% - Takeaway food
    C,
    /// 0% - Exempt
    D,
    /// N/A - Out of scope
    X,
}

impl VatCode {
    /// Returns the VAT rate as basis points (e.g. 2100 for 21%).
    #[must_use]
    pub fn rate_bps(self) -> u32 {
        match self {
            Self::A => 2100,
            Self::B => 1200,
            Self::C => 600,
            Self::D | Self::X => 0,
        }
    }

    /// The one-letter code printed on tickets and sent to the FDM.
    #[must_use]
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
}
