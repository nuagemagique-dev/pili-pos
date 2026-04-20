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
}
