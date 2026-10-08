//! Synthetic fixture of the error registry.
use std::fmt::Display;

pub const ALPHA: &str = "alpha";
/// Second code.
pub const BETA: &str = "beta";
/// Not in ALL on purpose (an FDM code).
pub const FDM_PENDING: &str = "FDM_PENDING";

/// Every code, for the translation check.
pub const ALL: &[&str] = &[
    ALPHA,
    BETA,
];

#[cfg(feature = "hubrise")]
pub mod hubrise {
    pub const HUB_ONE: &str = "hub_one";

    pub const ALL: &[&str] = &[HUB_ONE];
}

pub fn coded(code: &str, values: &[(&str, &dyn Display)]) -> String {
    let mut out = code.to_string();
    for (name, value) in values {
        out.push('|');
        out.push_str(&format!("{name}={value}").replace('|', "/"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_is_the_code() {
        assert_eq!(coded(ALPHA, &[]), "alpha");
    }
}
