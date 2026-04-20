/// ESC/POS binary command builder for thermal printers.
/// Reference: Epson ESC/POS Application Programming Guide.
/// Target: Epson TM-m30III (80mm, 576 dots/line at 203 DPI).
const DOTS_PER_LINE: u16 = 576; // 80mm at 203 DPI
const CHARS_PER_LINE: u8 = 48; // Font A, 12x24 dots

// ── ESC/POS Command Constants ──────────────────────────────────

const ESC: u8 = 0x1B;
const GS: u8 = 0x1D;
const LF: u8 = 0x0A;

/// Builder that accumulates ESC/POS binary commands.
#[derive(Debug, Clone)]
pub struct EscPosBuilder {
    buf: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TextStyle {
    pub bold: bool,
    pub underline: bool,
    pub double_height: bool,
    pub double_width: bool,
    pub font_b: bool,
}

impl EscPosBuilder {
    pub fn new() -> Self {
        let mut builder = Self { buf: Vec::with_capacity(4096) };
        builder.init();
        builder
    }

    /// ESC @ — Initialize printer (reset to defaults).
    pub fn init(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[ESC, b'@']);
        self
    }

    // ── Text Formatting ────────────────────────────────────────

    /// ESC a n — Set alignment (0=left, 1=center, 2=right).
    pub fn align(&mut self, alignment: Alignment) -> &mut Self {
        let n = match alignment {
            Alignment::Left => 0,
            Alignment::Center => 1,
            Alignment::Right => 2,
        };
        self.buf.extend_from_slice(&[ESC, b'a', n]);
        self
    }

    /// Apply a text style. Combines ESC E (bold), ESC - (underline),
    /// GS ! (character size), ESC M (font).
    pub fn style(&mut self, s: TextStyle) -> &mut Self {
        // ESC E n — Bold on/off
        self.buf.extend_from_slice(&[ESC, b'E', u8::from(s.bold)]);

        // ESC - n — Underline (0=off, 1=1-dot, 2=2-dot)
        self.buf.extend_from_slice(&[ESC, b'-', u8::from(s.underline)]);

        // GS ! n — Character size selection
        // Bits 0-3: width magnification, bits 4-7: height magnification
        let size = (u8::from(s.double_width) << 4) | u8::from(s.double_height);
        self.buf.extend_from_slice(&[GS, b'!', size]);

        // ESC M n — Font selection (0=Font A 12x24, 1=Font B 9x17)
        self.buf.extend_from_slice(&[ESC, b'M', u8::from(s.font_b)]);

        self
    }

    /// Reset to default text style.
    pub fn style_reset(&mut self) -> &mut Self {
        self.style(TextStyle::default())
    }

    // ── Text Output ────────────────────────────────────────────

    /// Write text (encoded as ASCII/CP437). No automatic line feed.
    pub fn text(&mut self, s: &str) -> &mut Self {
        self.buf.extend_from_slice(s.as_bytes());
        self
    }

    /// Write text followed by a line feed.
    pub fn textln(&mut self, s: &str) -> &mut Self {
        self.text(s);
        self.lf()
    }

    /// Write a line of text padded/truncated to fit the line width.
    /// Left-aligned text, right-aligned value on the same line.
    pub fn text_columns(&mut self, left: &str, right: &str) -> &mut Self {
        let width = self.chars_per_line() as usize;
        let right_len = right.len().min(width);
        let left_max = width.saturating_sub(right_len + 1);
        let left_truncated = if left.len() > left_max {
            &left[..left_max]
        } else {
            left
        };
        let padding = width - left_truncated.len() - right_len;

        self.text(left_truncated);
        for _ in 0..padding {
            self.buf.push(b' ');
        }
        self.text(right);
        self.lf()
    }

    /// Line feed.
    pub fn lf(&mut self) -> &mut Self {
        self.buf.push(LF);
        self
    }

    /// Feed n lines.
    pub fn feed(&mut self, lines: u8) -> &mut Self {
        // ESC d n — Print and feed n lines
        self.buf.extend_from_slice(&[ESC, b'd', lines]);
        self
    }

    /// Print a horizontal separator line.
    pub fn separator(&mut self, ch: char) -> &mut Self {
        let line: String = std::iter::repeat_n(ch, self.chars_per_line() as usize).collect();
        self.textln(&line)
    }

    /// Print an empty line.
    pub fn blank_line(&mut self) -> &mut Self {
        self.lf()
    }

    // ── Barcode / QR Code ──────────────────────────────────────

    /// Print QR code using GS ( k — QR Code commands (Function 180+).
    /// This uses the Epson QR code function which is cleaner than raster.
    pub fn qr_code(&mut self, data: &str, module_size: u8) -> &mut Self {
        // Function 165: Select the model (Model 2)
        // GS ( k pL pH cn fn n1 n2
        self.buf.extend_from_slice(&[GS, b'(', b'k', 4, 0, 49, 65, 50, 0]);

        // Function 167: Set module size (1-16)
        self.buf.extend_from_slice(&[GS, b'(', b'k', 3, 0, 49, 67, module_size]);

        // Function 169: Set error correction level (49=M)
        // 48=L, 49=M, 50=Q, 51=H
        self.buf.extend_from_slice(&[GS, b'(', b'k', 3, 0, 49, 69, 49]);

        // Function 180: Store QR data
        let data_bytes = data.as_bytes();
        let store_len = data_bytes.len() + 3; // +3 for cn, fn, m
        let pl = (store_len & 0xFF) as u8;
        let ph = ((store_len >> 8) & 0xFF) as u8;
        self.buf.extend_from_slice(&[GS, b'(', b'k', pl, ph, 49, 80, 48]);
        self.buf.extend_from_slice(data_bytes);

        // Function 181: Print QR code
        self.buf.extend_from_slice(&[GS, b'(', b'k', 3, 0, 49, 81, 48]);

        self
    }

    /// Print a raster image from a 1-bit bitmap.
    /// `width_pixels` must be a multiple of 8.
    /// `data`: row-major, MSB first, 1=black, 0=white.
    pub fn raster_image(&mut self, width_pixels: u16, height_pixels: u16, data: &[u8]) -> &mut Self {
        let bytes_per_row = width_pixels.div_ceil(8);
        // GS v 0 m xL xH yL yH d1...dk
        // m=0: normal mode
        self.buf.extend_from_slice(&[
            GS, b'v', b'0', 0,
            (bytes_per_row & 0xFF) as u8,
            ((bytes_per_row >> 8) & 0xFF) as u8,
            (height_pixels & 0xFF) as u8,
            ((height_pixels >> 8) & 0xFF) as u8,
        ]);
        self.buf.extend_from_slice(data);
        self
    }

    // ── Cut ────────────────────────────────────────────────────

    /// GS V m — Partial cut (leave small bridge).
    pub fn cut(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[GS, b'V', 66, 3]); // Partial cut, feed 3 lines
        self
    }

    /// GS V m — Full cut.
    pub fn full_cut(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[GS, b'V', 65, 3]);
        self
    }

    // ── Cash Drawer ────────────────────────────────────────────

    /// ESC p m t1 t2 — Open cash drawer.
    /// Pin 2 (m=0), pulse on-time t1, off-time t2 (in 2ms units).
    /// Default: 0x19 (50ms on), 0xFA (500ms off).
    pub fn open_drawer(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[ESC, b'p', 0x00, 0x19, 0xFA]);
        self
    }

    // ── Build ──────────────────────────────────────────────────

    /// Returns the number of characters per line for current font.
    pub fn chars_per_line(&self) -> u8 {
        CHARS_PER_LINE
    }

    /// Returns the dot width of the printable area.
    pub fn dots_per_line(&self) -> u16 {
        DOTS_PER_LINE
    }

    /// Consume the builder and return the raw byte buffer.
    pub fn build(self) -> Vec<u8> {
        self.buf
    }

    /// Get a reference to the current byte buffer.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Current size of the command buffer in bytes.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Whether the buffer is empty (only init command).
    pub fn is_empty(&self) -> bool {
        self.buf.len() <= 2 // ESC @ is 2 bytes
    }
}

impl Default for EscPosBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_emits_esc_at() {
        let builder = EscPosBuilder::new();
        let bytes = builder.build();
        assert_eq!(&bytes[..2], &[0x1B, b'@']);
    }

    #[test]
    fn text_and_lf() {
        let mut b = EscPosBuilder::new();
        b.textln("Hello");
        let bytes = b.build();
        // After ESC @, should have "Hello\n"
        assert!(bytes.ends_with(b"Hello\n"));
    }

    #[test]
    fn alignment_center() {
        let mut b = EscPosBuilder::new();
        b.align(Alignment::Center);
        let bytes = b.build();
        assert_eq!(&bytes[2..5], &[0x1B, b'a', 1]);
    }

    #[test]
    fn bold_on() {
        let mut b = EscPosBuilder::new();
        b.style(TextStyle { bold: true, ..Default::default() });
        let bytes = b.build();
        // ESC E 1 should be in the output
        assert!(bytes.windows(3).any(|w| w == [0x1B, b'E', 1]));
    }

    #[test]
    fn text_columns_padding() {
        let mut b = EscPosBuilder::new();
        b.text_columns("Saumon", "12.50");
        let bytes = b.build();
        let line_start = 2; // skip ESC @
        let line = &bytes[line_start..bytes.len() - 1]; // exclude final LF
        assert_eq!(line.len(), 48);
    }

    #[test]
    fn separator_48_chars() {
        let mut b = EscPosBuilder::new();
        b.separator('-');
        let bytes = b.build();
        let dashes: Vec<&u8> = bytes.iter().filter(|&&b| b == b'-').collect();
        assert_eq!(dashes.len(), 48);
    }

    #[test]
    fn open_drawer_bytes() {
        let mut b = EscPosBuilder::new();
        b.open_drawer();
        let bytes = b.build();
        assert!(bytes.windows(5).any(|w| w == [0x1B, b'p', 0x00, 0x19, 0xFA]));
    }

    #[test]
    fn cut_command() {
        let mut b = EscPosBuilder::new();
        b.cut();
        let bytes = b.build();
        assert!(bytes.windows(4).any(|w| w == [0x1D, b'V', 66, 3]));
    }

    #[test]
    fn qr_code_stores_data() {
        let mut b = EscPosBuilder::new();
        b.qr_code("https://www.gs2.be/abcdef1234567890ab", 6);
        let bytes = b.build();
        // Should contain the URL bytes somewhere in the buffer
        assert!(bytes.windows(5).any(|w| w == b"https"));
    }
}
