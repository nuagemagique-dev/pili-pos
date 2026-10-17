//! Staff badges: the reader setting and the badge number rules.
//!
//! Pure rules, shared by the engine and its tests. A badge is read as
//! a short hexadecimal token (an NFC tag id as uppercase hex, or the
//! characters a USB keyboard-wedge reader types); the engine keeps only
//! a keyed hash of it (`pos_engine::badge_key`), never the token.

use serde::{Deserialize, Serialize};

/// Where badges are read from. `Off` by default: nothing reads badges
/// and every badge command is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BadgeReader {
    #[default]
    Off,
    /// The NFC reader of the Android device.
    Nfc,
    /// A USB HID keyboard-wedge reader (types the number, then Enter).
    Usb,
}

impl BadgeReader {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Nfc => "nfc",
            Self::Usb => "usb",
        }
    }
}

impl std::str::FromStr for BadgeReader {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "off" => Ok(Self::Off),
            "nfc" => Ok(Self::Nfc),
            "usb" => Ok(Self::Usb),
            other => Err(format!("invalid badge reader: {other}")),
        }
    }
}

pub const WEDGE_GAP_MS_RANGE: (i64, i64) = (10, 200);
pub const WEDGE_MIN_LEN_RANGE: (i64, i64) = (4, 32);
pub const WEDGE_MAX_LEN_RANGE: (i64, i64) = (4, 64);

/// The per-installation badge setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BadgeSettings {
    pub reader: BadgeReader,
    /// Cashiers and waiters may log in with their badge (managers
    /// always type their PIN after it).
    pub login_enabled: bool,
    /// USB wedge timing: the longest pause between two characters of
    /// one burst, in milliseconds.
    pub wedge_max_gap_ms: i64,
    pub wedge_min_len: i64,
    pub wedge_max_len: i64,
}

impl Default for BadgeSettings {
    fn default() -> Self {
        Self {
            reader: BadgeReader::Off,
            login_enabled: false,
            wedge_max_gap_ms: 50,
            wedge_min_len: 6,
            wedge_max_len: 32,
        }
    }
}

impl BadgeSettings {
    /// The name of the first field out of range: gap 10 to 200 ms,
    /// minimum length 4 to 32, maximum length 4 to 64, minimum not above
    /// the maximum.
    pub fn validate(&self) -> Result<(), &'static str> {
        let within = |v: i64, (lo, hi): (i64, i64)| (lo..=hi).contains(&v);
        if !within(self.wedge_max_gap_ms, WEDGE_GAP_MS_RANGE) {
            return Err("wedge_max_gap_ms");
        }
        if !within(self.wedge_min_len, WEDGE_MIN_LEN_RANGE) {
            return Err("wedge_min_len");
        }
        if !within(self.wedge_max_len, WEDGE_MAX_LEN_RANGE) {
            return Err("wedge_max_len");
        }
        if self.wedge_min_len > self.wedge_max_len {
            return Err("wedge_min_len");
        }
        Ok(())
    }
}

/// A token that is not a badge number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidToken;

/// The badge token the engine works with: trimmed, uppercase, 4 to 64
/// hexadecimal digits.
pub fn normalize_token(raw: &str) -> Result<String, InvalidToken> {
    let token = raw.trim().to_ascii_uppercase();
    let ok = (4..=64).contains(&token.len()) && token.bytes().all(|b| b.is_ascii_hexdigit());
    if ok { Ok(token) } else { Err(InvalidToken) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trims_and_uppercases() {
        assert_eq!(normalize_token("  04a1b2c3 \r\n").unwrap(), "04A1B2C3");
        assert_eq!(normalize_token("04A1B2C3").unwrap(), "04A1B2C3");
    }

    #[test]
    fn normalize_accepts_decimal_and_hex_digits_only() {
        assert_eq!(normalize_token("0123456789").unwrap(), "0123456789");
        assert_eq!(normalize_token("abcdef").unwrap(), "ABCDEF");
        for bad in [
            "04G1B2",
            "04 A1B2",
            "04-A1B2",
            "04A1B2é",
            "\u{ff10}\u{ff11}\u{ff12}\u{ff13}",
        ] {
            assert_eq!(normalize_token(bad), Err(InvalidToken), "{bad}");
        }
    }

    #[test]
    fn normalize_enforces_4_to_64_characters() {
        assert_eq!(normalize_token(""), Err(InvalidToken));
        assert_eq!(normalize_token("   "), Err(InvalidToken));
        assert_eq!(normalize_token("ABC"), Err(InvalidToken));
        assert!(normalize_token("ABCD").is_ok());
        assert!(normalize_token(&"A".repeat(64)).is_ok());
        assert_eq!(normalize_token(&"A".repeat(65)), Err(InvalidToken));
    }

    #[test]
    fn normalize_does_not_strip_inner_whitespace() {
        assert_eq!(normalize_token("AB\nCD"), Err(InvalidToken));
    }

    #[test]
    fn reader_round_trips_its_names() {
        for (name, reader) in [
            ("off", BadgeReader::Off),
            ("nfc", BadgeReader::Nfc),
            ("usb", BadgeReader::Usb),
        ] {
            assert_eq!(name.parse::<BadgeReader>().unwrap(), reader);
            assert_eq!(reader.as_str(), name);
            assert_eq!(
                serde_json::to_string(&reader).unwrap(),
                format!("\"{name}\"")
            );
        }
        assert!("bluetooth".parse::<BadgeReader>().is_err());
        assert!("OFF".parse::<BadgeReader>().is_err());
        assert_eq!(BadgeReader::default(), BadgeReader::Off);
    }

    #[test]
    fn settings_defaults_are_valid_and_off() {
        let s = BadgeSettings::default();
        assert_eq!(s.reader, BadgeReader::Off);
        assert!(!s.login_enabled);
        assert_eq!(
            (s.wedge_max_gap_ms, s.wedge_min_len, s.wedge_max_len),
            (50, 6, 32)
        );
        assert_eq!(s.validate(), Ok(()));
    }

    #[test]
    fn settings_ranges_are_inclusive() {
        let ok = |f: &dyn Fn(&mut BadgeSettings)| {
            let mut s = BadgeSettings::default();
            f(&mut s);
            s.validate()
        };
        assert_eq!(ok(&|s| s.wedge_max_gap_ms = 10), Ok(()));
        assert_eq!(ok(&|s| s.wedge_max_gap_ms = 200), Ok(()));
        assert_eq!(ok(&|s| s.wedge_max_gap_ms = 9), Err("wedge_max_gap_ms"));
        assert_eq!(ok(&|s| s.wedge_max_gap_ms = 201), Err("wedge_max_gap_ms"));
        assert_eq!(ok(&|s| s.wedge_min_len = 4), Ok(()));
        assert_eq!(ok(&|s| s.wedge_min_len = 3), Err("wedge_min_len"));
        assert_eq!(ok(&|s| s.wedge_min_len = 33), Err("wedge_min_len"));
        assert_eq!(ok(&|s| s.wedge_max_len = 64), Ok(()));
        assert_eq!(ok(&|s| s.wedge_max_len = 3), Err("wedge_max_len"));
        assert_eq!(ok(&|s| s.wedge_max_len = 65), Err("wedge_max_len"));
    }

    #[test]
    fn settings_min_length_cannot_exceed_the_maximum() {
        let mut s = BadgeSettings::default();
        s.wedge_min_len = 10;
        s.wedge_max_len = 10;
        assert_eq!(s.validate(), Ok(()));
        s.wedge_max_len = 9;
        assert_eq!(s.validate(), Err("wedge_min_len"));
    }
}
