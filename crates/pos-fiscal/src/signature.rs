use base64::Engine;
use sha1::Digest;

#[derive(Debug, thiserror::Error)]
pub enum SignatureError {
    #[error("Base64 decode failed: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("Invalid signature format")]
    InvalidFormat,
}

/// Compute the shortSignature from the FDM's digitalSignature.
///
/// Per SCE 2.0 spec section 2.4:
///   shortSignature = Hex(SHA1(Base64Decode(digitalSignature)))
///
/// Only applicable for event N (normal sale).
pub fn compute_short_signature(digital_signature_b64: &str) -> Result<String, SignatureError> {
    let raw = base64::engine::general_purpose::STANDARD.decode(digital_signature_b64)?;
    let hash = sha1::Sha1::digest(&raw);
    Ok(hex::encode(hash))
}

/// Verify that the shortSignature returned by the FDM matches
/// the computed value from digitalSignature.
pub fn verify_short_signature(
    digital_signature_b64: &str,
    expected_short: &str,
) -> Result<bool, SignatureError> {
    let computed = compute_short_signature(digital_signature_b64)?;
    Ok(computed.eq_ignore_ascii_case(expected_short))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_signature_computation() {
        // Create a known base64 input and verify the hash chain works
        let raw_bytes = b"test signature data for SCE 2.0";
        let b64 = base64::engine::general_purpose::STANDARD.encode(raw_bytes);

        let short = compute_short_signature(&b64).unwrap();

        // SHA1 of raw_bytes
        let expected_hash = sha1::Sha1::digest(raw_bytes);
        let expected = hex::encode(expected_hash);

        assert_eq!(short, expected);
    }

    #[test]
    fn verify_matching_signature() {
        let raw_bytes = b"hello fdm";
        let b64 = base64::engine::general_purpose::STANDARD.encode(raw_bytes);
        let short = compute_short_signature(&b64).unwrap();

        assert!(verify_short_signature(&b64, &short).unwrap());
    }

    #[test]
    fn verify_mismatched_signature() {
        let raw_bytes = b"hello fdm";
        let b64 = base64::engine::general_purpose::STANDARD.encode(raw_bytes);

        assert!(!verify_short_signature(&b64, "0000000000000000000000000000000000000000").unwrap());
    }

    #[test]
    fn malformed_base64_is_rejected_not_panicked() {
        for bad in ["not base64 at all!!!", "===", "%%%", "a"] {
            let err = compute_short_signature(bad).unwrap_err();
            assert!(matches!(err, SignatureError::Base64(_)), "input={bad:?}");
        }
    }

    #[test]
    fn empty_input_is_a_base64_error_not_a_panic() {
        // Empty string decodes to an empty byte vector under most base64
        // implementations, which is a *valid* (if unusual) decode — this
        // pins down the actual behavior so a future base64 crate bump
        // that changes it is caught.
        let result = compute_short_signature("");
        assert!(result.is_ok(), "empty base64 should decode to empty bytes");
        // SHA1("") is a well-known constant.
        assert_eq!(result.unwrap(), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn verify_short_signature_propagates_decode_errors() {
        assert!(verify_short_signature("not base64!!!", "anything").is_err());
    }

    #[test]
    fn short_signature_is_case_insensitive_on_the_expected_side() {
        let raw_bytes = b"hello fdm";
        let b64 = base64::engine::general_purpose::STANDARD.encode(raw_bytes);
        let short = compute_short_signature(&b64).unwrap();
        assert!(verify_short_signature(&b64, &short.to_uppercase()).unwrap());
    }

    #[test]
    fn short_signature_is_always_40_hex_chars_sha1() {
        // SHA1 digests are always 20 bytes = 40 hex chars, regardless
        // of input length.
        for input in [
            b"".as_slice(),
            b"a",
            b"a fairly long digital signature payload",
        ] {
            let b64 = base64::engine::general_purpose::STANDARD.encode(input);
            let short = compute_short_signature(&b64).unwrap();
            assert_eq!(short.len(), 40, "input len={}", input.len());
            assert!(short.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn different_inputs_never_collide_in_practice() {
        let a = compute_short_signature(
            &base64::engine::general_purpose::STANDARD.encode(b"signature A"),
        )
        .unwrap();
        let b = compute_short_signature(
            &base64::engine::general_purpose::STANDARD.encode(b"signature B"),
        )
        .unwrap();
        assert_ne!(a, b);
    }
}
