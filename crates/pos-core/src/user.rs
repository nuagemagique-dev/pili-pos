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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub insz: String,
    pub name: String,
    pub role: UserRole,
    pub active: bool,
    /// True when a PIN hash is stored in the DB — the frontend uses
    /// this to decide whether to show the PIN numpad at login.
    pub has_pin: bool,
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
}
