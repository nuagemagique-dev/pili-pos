//! Email address validation and PII masking.
//!
//! `validate` is a pragmatic, RFC 5321-ish check — not a full parser.
//! The point isn't standards purity, it's catching the mistakes an
//! operator typing an address on a touchscreen actually makes (missing
//! `@`, stray space, fat-fingered double `@`) before we ever hand the
//! address to Scaleway. Real-world addresses (including non-ASCII
//! local parts and internationalised domains) are accepted.

use std::fmt;

/// Why an address was rejected by [`validate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressError {
    MissingAt,
    MultipleAt,
    EmptyLocalPart,
    LocalPartTooLong,
    EmptyDomain,
    DomainMissingDot,
    InvalidDomainLabel,
    TooLong,
    ContainsWhitespace,
    ContainsControlChar,
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::MissingAt => "missing @",
            Self::MultipleAt => "more than one @",
            Self::EmptyLocalPart => "empty local part",
            Self::LocalPartTooLong => "local part longer than 64 characters",
            Self::EmptyDomain => "empty domain",
            Self::DomainMissingDot => "domain has no dot",
            Self::InvalidDomainLabel => "invalid domain label",
            Self::TooLong => "address longer than 254 characters",
            Self::ContainsWhitespace => "contains whitespace",
            Self::ContainsControlChar => "contains a control character",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for AddressError {}

const MAX_ADDRESS_LEN: usize = 254;
const MAX_LOCAL_LEN: usize = 64;
const MAX_LABEL_LEN: usize = 63;

/// Validate `email` well enough to accept it as a delivery target.
/// Does not attempt DNS/MX resolution — that's the outbox's job when
/// it actually tries to send.
pub fn validate(email: &str) -> Result<(), AddressError> {
    if email.chars().any(|c| c.is_control()) {
        return Err(AddressError::ContainsControlChar);
    }
    if email.chars().any(|c| c.is_whitespace()) {
        return Err(AddressError::ContainsWhitespace);
    }
    if email.chars().count() > MAX_ADDRESS_LEN {
        return Err(AddressError::TooLong);
    }

    let at_count = email.matches('@').count();
    if at_count == 0 {
        return Err(AddressError::MissingAt);
    }
    if at_count > 1 {
        return Err(AddressError::MultipleAt);
    }
    let (local, domain) = email.split_once('@').expect("exactly one @ checked above");

    if local.is_empty() {
        return Err(AddressError::EmptyLocalPart);
    }
    if local.chars().count() > MAX_LOCAL_LEN {
        return Err(AddressError::LocalPartTooLong);
    }

    if domain.is_empty() {
        return Err(AddressError::EmptyDomain);
    }
    if !domain.contains('.') {
        return Err(AddressError::DomainMissingDot);
    }
    // A trailing dot means the last label is empty, caught below, but
    // give it its own message since it's the most common typo.
    if domain.ends_with('.') || domain.starts_with('.') {
        return Err(AddressError::InvalidDomainLabel);
    }
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 || !labels.iter().all(|l| is_valid_label(l)) {
        return Err(AddressError::InvalidDomainLabel);
    }

    Ok(())
}

fn is_valid_label(label: &str) -> bool {
    if label.is_empty() || label.chars().count() > MAX_LABEL_LEN {
        return false;
    }
    if label.starts_with('-') || label.ends_with('-') {
        return false;
    }
    label.chars().all(|c| c.is_alphanumeric() || c == '-')
}

/// Mask an address for display/audit without keeping the plaintext
/// around: first char of the local part and of the domain's first
/// label are kept, everything else in those two spots becomes `***`;
/// remaining domain labels (the TLD, mostly) are left untouched so
/// the hint stays recognisable, e.g. `john@example.be` →
/// `j***@e***.be`.
///
/// Never fails: an unparseable input just gets masked wholesale so a
/// bad address can never leak into a stored hint.
pub fn mask(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return mask_part(email);
    };
    let masked_domain = match domain.split_once('.') {
        Some((first_label, rest)) => format!("{}.{rest}", mask_part(first_label)),
        None => mask_part(domain),
    };
    format!("{}@{masked_domain}", mask_part(local))
}

fn mask_part(s: &str) -> String {
    match s.chars().next() {
        Some(c) => format!("{c}***"),
        None => "***".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate: valid addresses ──────────────────────────────

    #[test]
    fn accepts_ordinary_addresses() {
        for ok in [
            "john@example.be",
            "j.doe+ticket@sub.example.com",
            "a@b.co",
            "user_name@example-domain.fr",
        ] {
            assert!(validate(ok).is_ok(), "{ok} should be valid");
        }
    }

    #[test]
    fn accepts_unicode_local_part_and_domain() {
        // Real customers have accented names, and IDN domains are
        // real (café.fr is a registrable .fr domain).
        for ok in ["jérôme@café.fr", "日本語@例え.jp"] {
            assert!(validate(ok).is_ok(), "{ok} should be valid");
        }
    }

    #[test]
    fn accepts_max_length_local_part() {
        let local = "a".repeat(64);
        let email = format!("{local}@example.com");
        assert!(validate(&email).is_ok());
    }

    // ── validate: invalid addresses ────────────────────────────

    #[test]
    fn rejects_missing_at() {
        assert_eq!(validate("john.example.be"), Err(AddressError::MissingAt));
    }

    #[test]
    fn rejects_double_at() {
        assert_eq!(validate("john@@example.be"), Err(AddressError::MultipleAt));
        assert_eq!(validate("jo@hn@example.be"), Err(AddressError::MultipleAt));
    }

    #[test]
    fn rejects_empty_local_part() {
        assert_eq!(validate("@example.be"), Err(AddressError::EmptyLocalPart));
    }

    #[test]
    fn rejects_local_part_over_64_chars() {
        let local = "a".repeat(65);
        let email = format!("{local}@example.com");
        assert_eq!(validate(&email), Err(AddressError::LocalPartTooLong));
    }

    #[test]
    fn rejects_empty_domain() {
        assert_eq!(validate("john@"), Err(AddressError::EmptyDomain));
    }

    #[test]
    fn rejects_domain_without_a_dot() {
        assert_eq!(
            validate("john@localhost"),
            Err(AddressError::DomainMissingDot)
        );
    }

    #[test]
    fn rejects_trailing_dot() {
        assert_eq!(
            validate("john@example.be."),
            Err(AddressError::InvalidDomainLabel)
        );
    }

    #[test]
    fn rejects_leading_dot() {
        assert_eq!(
            validate("john@.example.be"),
            Err(AddressError::InvalidDomainLabel)
        );
    }

    #[test]
    fn rejects_empty_label_in_the_middle() {
        assert_eq!(
            validate("john@example..be"),
            Err(AddressError::InvalidDomainLabel)
        );
    }

    #[test]
    fn rejects_label_starting_or_ending_with_hyphen() {
        assert_eq!(
            validate("john@-example.be"),
            Err(AddressError::InvalidDomainLabel)
        );
        assert_eq!(
            validate("john@example-.be"),
            Err(AddressError::InvalidDomainLabel)
        );
    }

    #[test]
    fn rejects_spaces_anywhere() {
        assert_eq!(
            validate("john doe@example.be"),
            Err(AddressError::ContainsWhitespace)
        );
        assert_eq!(
            validate(" john@example.be"),
            Err(AddressError::ContainsWhitespace)
        );
        assert_eq!(
            validate("john@example.be "),
            Err(AddressError::ContainsWhitespace)
        );
    }

    #[test]
    fn rejects_control_characters() {
        assert_eq!(
            validate("john@example.be\u{0}"),
            Err(AddressError::ContainsControlChar)
        );
        assert_eq!(
            validate("jo\nhn@example.be"),
            Err(AddressError::ContainsControlChar)
        );
    }

    #[test]
    fn rejects_address_over_254_chars() {
        // Build a domain of an exact char length out of valid <=63-char
        // labels, so the boundary test doesn't depend on hand-counted
        // arithmetic.
        fn domain_of_len(n: usize) -> String {
            let mut labels = Vec::new();
            let mut remaining = n;
            loop {
                let take = remaining.min(63);
                labels.push("a".repeat(take));
                if take == remaining {
                    break;
                }
                remaining -= take + 1; // +1 for the dot joining the next label
            }
            let domain = labels.join(".");
            assert_eq!(domain.chars().count(), n);
            domain
        }

        let local = "a".repeat(MAX_LOCAL_LEN);
        let domain_len = MAX_ADDRESS_LEN - local.len() - 1; // -1 for '@'
        let email_254 = format!("{local}@{}", domain_of_len(domain_len));
        assert_eq!(email_254.chars().count(), MAX_ADDRESS_LEN);
        assert!(validate(&email_254).is_ok());

        let email_255 = format!("{email_254}x");
        assert_eq!(email_255.chars().count(), MAX_ADDRESS_LEN + 1);
        assert_eq!(validate(&email_255), Err(AddressError::TooLong));
    }

    // ── mask ────────────────────────────────────────────────────

    #[test]
    fn masks_the_documented_example() {
        assert_eq!(mask("john@example.be"), "j***@e***.be");
    }

    #[test]
    fn masks_preserve_the_tld_when_domain_has_more_labels() {
        assert_eq!(mask("john@mail.example.co.uk"), "j***@m***.example.co.uk");
    }

    #[test]
    fn masks_single_char_local_part() {
        assert_eq!(mask("j@example.be"), "j***@e***.be");
    }

    #[test]
    fn masks_never_include_the_full_original_address() {
        let email = "sensitive.name@private-domain.example";
        let masked = mask(email);
        assert_ne!(masked, email);
        assert!(!masked.contains("sensitive"));
        assert!(!masked.contains("private-domain"));
    }

    #[test]
    fn masks_input_without_at_defensively() {
        // Not a real code path (validate() would reject this first),
        // but mask() must never panic on garbage input.
        assert_eq!(mask("not-an-email"), "n***");
    }
}
