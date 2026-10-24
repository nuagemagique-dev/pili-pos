//! The software identity of the POS, as the FDM and the ticket carry it.

use std::fmt;

/// Longest `posSwVersion` the FDM takes (POS description v2.2: 1 to 36
/// characters).
pub const MAX_LEN: usize = 36;

/// The `posSwVersion` of every event and ticket this terminal produces,
/// for example `pili-pos/1.0.0`. The shell that embeds the engine builds
/// it (it knows the product and its version); the fiscal core only
/// carries it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SoftwareIdentity(String);

/// A software identity the FDM would refuse.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdentityError {
    #[error("software identity is empty (POS spec v2.2: posSwVersion is 1 to 36 characters)")]
    Empty,
    #[error(
        "software identity has {0} characters (POS spec v2.2: posSwVersion is 1 to 36 characters)"
    )]
    TooLong(usize),
}

impl SoftwareIdentity {
    /// `s`, if it is 1 to 36 characters long.
    pub fn new(s: impl Into<String>) -> Result<Self, IdentityError> {
        let s = s.into();
        if s.is_empty() {
            return Err(IdentityError::Empty);
        }
        let n = s.chars().count();
        if n > MAX_LEN {
            return Err(IdentityError::TooLong(n));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SoftwareIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_refused() {
        assert_eq!(SoftwareIdentity::new(""), Err(IdentityError::Empty));
    }

    #[test]
    fn one_character_is_accepted() {
        assert_eq!(SoftwareIdentity::new("x").unwrap().as_str(), "x");
    }

    #[test]
    fn thirty_six_characters_are_accepted() {
        let s = "a".repeat(36);
        assert_eq!(SoftwareIdentity::new(s.clone()).unwrap().as_str(), s);
    }

    #[test]
    fn thirty_seven_characters_are_refused() {
        assert_eq!(
            SoftwareIdentity::new("a".repeat(37)),
            Err(IdentityError::TooLong(37))
        );
    }

    #[test]
    fn the_limit_counts_characters_not_bytes() {
        // 36 two-byte characters: 72 bytes, still 36 characters.
        assert!(SoftwareIdentity::new("é".repeat(36)).is_ok());
        assert!(SoftwareIdentity::new("é".repeat(37)).is_err());
    }
}
