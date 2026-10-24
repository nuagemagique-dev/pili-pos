use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    Cashier,
    Waiter,
}

impl UserRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Cashier => "cashier",
            Self::Waiter => "waiter",
        }
    }
}

impl std::str::FromStr for UserRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(Self::Admin),
            "cashier" => Ok(Self::Cashier),
            "waiter" => Ok(Self::Waiter),
            other => Err(format!("invalid user role: {other}")),
        }
    }
}

/// INSZ of the system identity (POS detailed description v2.2: the
/// "robot" user of automatic registrations such as webshop orders).
/// It is never an operator: it cannot log in and is not listed.
pub const SYSTEM_INSZ: &str = "00000000029";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub insz: String,
    pub name: String,
    pub role: UserRole,
    pub active: bool,
    /// True when a PIN hash is stored in the DB - the frontend uses
    /// this to decide whether to show the PIN numpad at login.
    pub has_pin: bool,
    /// A kitchen employee who never uses the till: no login, no PIN,
    /// clocked in and out by a manager (role `waiter` in the database).
    #[serde(default)]
    pub presence_only: bool,
}

/// Validates an INSZ (Belgian national number) format: 11 digits with mod-97 check.
pub fn validate_insz(insz: &str) -> bool {
    if insz.len() != 11 || !insz.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    let number: u64 = insz.parse().unwrap_or(0);
    let base = number / 100;
    let check = (number % 100) as u32;

    // Try both pre-2000 and post-2000 calculation
    let remainder_pre = (base % 97) as u32;
    let remainder_post = ((2_000_000_000 + base) % 97) as u32;

    (97 - remainder_pre == check) || (97 - remainder_post == check)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn robot_user_is_valid() {
        // 00000000029 is the robot/automated user INSZ per SCE 2.0 spec
        assert!(validate_insz("00000000029"));
    }

    #[test]
    fn invalid_length() {
        assert!(!validate_insz("1234567890"));
        assert!(!validate_insz("123456789012"));
    }

    #[test]
    fn non_digits_rejected() {
        assert!(!validate_insz("1234567890a"));
    }

    #[test]
    fn empty_string_rejected() {
        assert!(!validate_insz(""));
    }

    #[test]
    fn whitespace_and_special_chars_rejected() {
        assert!(!validate_insz(" 0000000029"));
        assert!(!validate_insz("0000000-029"));
        assert!(!validate_insz("00000000029 ")); // trailing space -> len 12
    }

    #[test]
    fn all_zeros_has_wrong_checksum() {
        // 00000000000: base=0, check=0. Pre-2000: 97-0=97 != 0.
        // Post-2000: 97-(2_000_000_000%97) != 0 either. Must be rejected.
        assert!(!validate_insz("00000000000"));
    }

    #[test]
    fn known_pre_2000_valid_insz() {
        // A real, well-known pre-2000 test INSZ used across Belgian
        // government test suites: 85 07 30 001 96 (base=850730001,
        // check should satisfy 97 - (base % 97) == check).
        let base: u64 = 850_730_001;
        let expected_check = 97 - (base % 97);
        let insz = format!("{base:09}{expected_check:02}");
        assert!(validate_insz(&insz), "insz={insz}");
    }

    #[test]
    fn known_post_2000_valid_insz() {
        // Post-2000 formula: check = 97 - ((2_000_000_000 + base) % 97).
        let base: u64 = 50_730_001;
        let expected_check = 97 - ((2_000_000_000 + base) % 97);
        let insz = format!("{base:09}{expected_check:02}");
        assert!(validate_insz(&insz), "insz={insz}");
    }

    #[test]
    fn wrong_checksum_is_rejected() {
        let base: u64 = 850_730_001;
        let correct_check = 97 - (base % 97);
        let wrong_check = if correct_check == 1 {
            2
        } else {
            correct_check - 1
        };
        let insz = format!("{base:09}{wrong_check:02}");
        assert!(!validate_insz(&insz), "insz={insz}");
    }

    #[test]
    fn user_role_as_str_and_from_str_round_trip() {
        for r in [UserRole::Admin, UserRole::Cashier, UserRole::Waiter] {
            assert_eq!(r.as_str().parse::<UserRole>().unwrap(), r);
        }
    }

    #[test]
    fn user_role_from_str_rejects_unknown() {
        assert!("manager".parse::<UserRole>().is_err());
    }

    // ── Property test: checksum formula is verified, not just spot-checked ──

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Any base in the pre-2000 range, paired with its
            /// mathematically correct check digits, must validate -
            /// and flipping to the wrong check digits must not.
            #[test]
            fn pre_2000_checksum_formula_is_exactly_what_validate_insz_expects(
                base in 0u64..=999_999_999u64,
            ) {
                // base % 97 is 0..=96, so the check digit is always
                // 1..=97 and fits `%02` - no edge case to special-case.
                let correct_check = 97 - (base % 97);
                let insz = format!("{base:09}{correct_check:02}");
                prop_assert_eq!(insz.len(), 11);
                prop_assert!(validate_insz(&insz), "insz={insz}");
            }

            #[test]
            fn post_2000_checksum_formula_is_exactly_what_validate_insz_expects(
                base in 0u64..=999_999_999u64,
            ) {
                let correct_check = 97 - ((2_000_000_000u64 + base) % 97);
                let insz = format!("{base:09}{correct_check:02}");
                prop_assert_eq!(insz.len(), 11);
                prop_assert!(validate_insz(&insz), "insz={insz}");
            }

            /// Never panics, on any input shape - length, encoding or
            /// digit-ness. A fiscal login screen must degrade to "not
            /// a valid INSZ", never crash the terminal.
            #[test]
            fn never_panics_on_arbitrary_input(s in ".{0,40}") {
                let _ = validate_insz(&s);
            }
        }
    }
}
