use image::{GrayImage, Luma};
use qrcode::QrCode;

#[derive(Debug, thiserror::Error)]
pub enum QrError {
    #[error("QR code generation failed: {0}")]
    Generate(String),
    #[error("PNG encoding failed: {0}")]
    Encode(String),
}

/// Generate a QR code PNG image (bytes) from the FDM verification URL.
///
/// POS description v2.2 §6.5 asks for a Model 2 ("type 2") code with
/// medium ECC holding the FDM's 38-character URL. It also names a 25x25
/// symbol (version 2), which cannot hold that URL: the mixed-case
/// Base-62 value forces byte mode, and version 2 tops out at 34 bytes.
/// The encoder therefore picks the smallest version that fits, which is
/// version 3 (29x29) for every real verification URL.
///
/// Returns PNG image bytes suitable for printing or display.
pub fn generate_qr_png(verification_url: &str, module_size: u32) -> Result<Vec<u8>, QrError> {
    let code = QrCode::with_error_correction_level(verification_url, qrcode::EcLevel::M)
        .map_err(|e| QrError::Generate(e.to_string()))?;

    let modules = code.to_colors();
    let width = code.width() as u32;

    let quiet_zone: u32 = 4;
    let img_size = (width + quiet_zone * 2) * module_size;

    let mut img = GrayImage::from_pixel(img_size, img_size, Luma([255u8]));

    for (y, row) in modules.chunks(width as usize).enumerate() {
        for (x, &module) in row.iter().enumerate() {
            if module == qrcode::Color::Dark {
                let px = (x as u32 + quiet_zone) * module_size;
                let py = (y as u32 + quiet_zone) * module_size;
                for dy in 0..module_size {
                    for dx in 0..module_size {
                        img.put_pixel(px + dx, py + dy, Luma([0u8]));
                    }
                }
            }
        }
    }

    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(
        encoder,
        img.as_raw(),
        img_size,
        img_size,
        image::ColorType::L8.into(),
    )
    .map_err(|e| QrError::Encode(e.to_string()))?;

    Ok(buf)
}

/// Generate a QR code as a matrix of booleans (true = dark module).
/// Useful for ESC/POS raster printing.
pub fn generate_qr_matrix(verification_url: &str) -> Result<Vec<Vec<bool>>, QrError> {
    let code = QrCode::with_error_correction_level(verification_url, qrcode::EcLevel::M)
        .map_err(|e| QrError::Generate(e.to_string()))?;

    let width = code.width();
    let modules = code.to_colors();

    let matrix: Vec<Vec<bool>> = modules
        .chunks(width)
        .map(|row| row.iter().map(|&c| c == qrcode::Color::Dark).collect())
        .collect();

    Ok(matrix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_qr_from_sample_url() {
        // Sample 38-char verification URL
        let url = "https://www.gs2.be/1234567890abcdefghi";
        let png = generate_qr_png(url, 4).unwrap();
        assert!(!png.is_empty());
        // PNG magic bytes
        assert_eq!(&png[..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn generate_matrix() {
        let url = "https://www.gs2.be/1234567890abcdefghi";
        let matrix = generate_qr_matrix(url).unwrap();
        assert!(!matrix.is_empty());
        // All rows same length
        let width = matrix[0].len();
        assert!(matrix.iter().all(|row| row.len() == width));
    }

    /// FDM detailed description v1.4 §Chapitre 3 gives the worked
    /// example verification URL: fixed lowercase prefix
    /// `https://www.gs2.be/` (19 chars) + a 19-char mixed-case Base-62
    /// encoded value (§3.1 "LONGUEUR FIXE": always 19 base-62 chars) -
    /// 38 characters total, matching POS description v2.2 §6.5's "38
    /// caractères alphanumériques".
    ///
    /// NOTE - a real verification URL like this one is NOT encodable
    /// in QR "Alphanumeric" mode (ISO/IEC 18004): that mode's charset
    /// is uppercase-only (0-9, A-Z, space, $%*+-./:), but the fixed
    /// prefix is lowercase and the Base-62 alphabet's values 36-61 are
    /// also lowercase (see the spec's own table). So this always falls
    /// back to QR "Byte" mode, whose capacity at Version 2 (the
    /// 25x25-module symbol POS description v2.2 §6.5 names, "type 2 QR"
    /// = ISO Model 2) tops out at 34 bytes even at the lowest EC level,
    /// below the 38 bytes needed. This test therefore does NOT assert
    /// a specific module count (that would just be asserting today's
    /// `qrcode`-crate behaviour, not the spec) - it pins down that the
    /// spec's own worked example encodes without panicking or erroring
    /// and stays a valid square symbol. See the test-run report for the
    /// concrete module count this currently produces and why it is
    /// worth a certification-readiness follow-up.
    #[test]
    fn spec_example_url_encodes_to_a_valid_square_qr_code() {
        let url = format!("https://www.gs2.be/{}", "viXnmc1vvqGaMAS8MJV");
        assert_eq!(url.len(), 38, "prefix (19) + encoded value (19)");

        let matrix = generate_qr_matrix(&url).unwrap();
        assert_eq!(matrix.len(), matrix[0].len(), "QR modules are square");
        assert!(matrix.iter().all(|row| row.len() == matrix.len()));

        let png = generate_qr_png(&url, 4).unwrap();
        assert_eq!(&png[..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    /// Every real verification URL has the same shape (19-char lowercase
    /// prefix + 19 Base-62 chars), so it always lands on version 3.
    #[test]
    fn spec_example_url_needs_version_3_with_ecc_m() {
        let url = format!("https://www.gs2.be/{}", "viXnmc1vvqGaMAS8MJV");
        let code = QrCode::with_error_correction_level(&url, qrcode::EcLevel::M).unwrap();
        assert_eq!(code.version(), qrcode::Version::Normal(3));
        assert_eq!(code.error_correction_level(), qrcode::EcLevel::M);
        assert_eq!(generate_qr_matrix(&url).unwrap().len(), 29);
        assert!(
            QrCode::with_version(&url, qrcode::Version::Normal(2), qrcode::EcLevel::L).is_err(),
            "version 2 (25x25) cannot hold the URL even at the lowest ECC"
        );
    }

    /// Sanity check for the theory documented on the test above: a
    /// 38-character string that DOES stay within the QR Alphanumeric
    /// charset (uppercase + digits only, no lowercase) does encode to
    /// the spec's 25x25-module symbol. This isolates "is the module
    /// count purely a function of character-set/mode, as documented
    /// above" from "is generate_qr_matrix itself somehow broken" -
    /// it demonstrates the latter is fine.
    #[test]
    fn all_uppercase_38_chars_does_fit_25x25() {
        let url = "A".repeat(38);
        let matrix = generate_qr_matrix(&url).unwrap();
        assert_eq!(matrix.len(), 25);
        assert_eq!(matrix[0].len(), 25);
    }

    #[test]
    fn empty_url_does_not_panic_and_still_produces_a_square_code() {
        let matrix = generate_qr_matrix("").unwrap();
        assert!(!matrix.is_empty());
        let width = matrix[0].len();
        assert!(matrix.iter().all(|row| row.len() == width));
    }

    #[test]
    fn empty_url_png_is_still_a_valid_png() {
        let png = generate_qr_png("", 3).unwrap();
        assert_eq!(&png[..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn module_size_scales_the_output_dimensions() {
        let url = "https://www.gs2.be/1234567890abcdefghi";
        let small = generate_qr_png(url, 1).unwrap();
        let large = generate_qr_png(url, 8).unwrap();
        // Not a pixel-perfect size assertion (that's encoded in the PNG
        // header, not trivial to assert on bytes alone), but a larger
        // module size must produce a strictly larger file.
        assert!(large.len() > small.len());
    }

    #[test]
    fn matrix_has_at_least_one_dark_and_one_light_module() {
        // A degenerate all-dark or all-light "QR code" would be a sign
        // the encoder silently failed. Any real QR code (even for a
        // trivial input) has both finder patterns (dark) and quiet
        // areas (light) once rendered - but at the *matrix* level
        // (module data only, no PNG quiet zone), it must at minimum
        // have positioning-pattern contrast.
        let url = "https://www.gs2.be/1234567890abcdefghi";
        let matrix = generate_qr_matrix(url).unwrap();
        let dark = matrix.iter().flatten().filter(|&&m| m).count();
        let light = matrix.iter().flatten().filter(|&&m| !m).count();
        assert!(dark > 0);
        assert!(light > 0);
    }

    #[test]
    fn long_url_still_encodes_without_panicking() {
        // Defensive: the FDM's URL is always ~38 chars per spec, but the
        // generator must degrade gracefully (bump QR version, not panic
        // or silently truncate) if it is ever handed something longer.
        let long = "https://www.gs2.be/".to_string() + &"a".repeat(200);
        let matrix = generate_qr_matrix(&long).unwrap();
        assert!(matrix.len() > 25, "a longer payload needs a bigger code");
    }

    #[test]
    fn different_urls_produce_different_matrices() {
        let a = generate_qr_matrix("https://www.gs2.be/1234567890abcdefghi").unwrap();
        let b = generate_qr_matrix("https://www.gs2.be/9876543210abcdefghi").unwrap();
        assert_ne!(a, b);
    }
}
