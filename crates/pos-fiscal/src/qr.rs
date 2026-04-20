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
/// The SCE 2.0 spec requires:
/// - 25x25 modules (Type 2)
/// - Medium ECC
/// - 38 alphanumeric characters
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
}
