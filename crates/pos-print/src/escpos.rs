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
        let mut builder = Self {
            buf: Vec::with_capacity(4096),
        };
        builder.init();
        builder
    }

    /// ESC @: initialize the printer (reset to defaults), then ESC t 19:
    /// select code page CP858 (Latin-1 + €) so French accents on
    /// product names/addresses print correctly.
    pub fn init(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[ESC, b'@']);
        self.buf.extend_from_slice(&[ESC, b't', 19]);
        self
    }

    // ── Text Formatting ────────────────────────────────────────

    /// ESC a n - Set alignment (0=left, 1=center, 2=right).
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
        // ESC E n - Bold on/off
        self.buf.extend_from_slice(&[ESC, b'E', u8::from(s.bold)]);

        // ESC - n - Underline (0=off, 1=1-dot, 2=2-dot)
        self.buf
            .extend_from_slice(&[ESC, b'-', u8::from(s.underline)]);

        // GS ! n - Character size selection
        // Bits 0-3: width magnification, bits 4-7: height magnification
        let size = (u8::from(s.double_width) << 4) | u8::from(s.double_height);
        self.buf.extend_from_slice(&[GS, b'!', size]);

        // ESC M n - Font selection (0=Font A 12x24, 1=Font B 9x17)
        self.buf.extend_from_slice(&[ESC, b'M', u8::from(s.font_b)]);

        self
    }

    /// Reset to default text style.
    pub fn style_reset(&mut self) -> &mut Self {
        self.style(TextStyle::default())
    }

    // ── Text Output ────────────────────────────────────────────

    /// Write text encoded as CP858 (selected in `init`), after
    /// [`printable`] replaced what CP858 lacks. Characters with no
    /// stand-in print as `?`. No automatic line feed.
    pub fn text(&mut self, s: &str) -> &mut Self {
        for ch in printable(s).chars() {
            self.buf.push(encode_cp858(ch));
        }
        self
    }

    /// Write text followed by a line feed.
    pub fn textln(&mut self, s: &str) -> &mut Self {
        self.text(s);
        self.lf()
    }

    /// Write text as whole lines of at most [`CHARS_PER_LINE`]
    /// columns, breaking between words (see [`wrap`]), so a long name
    /// or address is never cut by the printer mid-word.
    pub fn textln_wrapped(&mut self, s: &str) -> &mut Self {
        for line in wrap(s, self.chars_per_line() as usize) {
            self.textln(&line);
        }
        self
    }

    /// Write a line of text padded/truncated to fit the line width.
    /// Left-aligned text, right-aligned value on the same line.
    ///
    /// Both `left` and `right` are clamped to `chars_per_line()`: an
    /// over-long `right` value (e.g. a runaway price string) must
    /// never push the printed row past the physical paper width -
    /// see `text_columns_never_exceeds_the_configured_width_*` tests.
    pub fn text_columns(&mut self, left: &str, right: &str) -> &mut Self {
        // Widths count printed characters (CP858 is single-byte, so
        // one char == one column), not UTF-8 bytes.
        let width = self.chars_per_line() as usize;
        let (left, right) = (printable(left), printable(right));
        let right_len = right.chars().count().min(width);
        let right_truncated: String = right.chars().take(right_len).collect();
        let left_max = width.saturating_sub(right_len + 1);
        let left_truncated: String = left.chars().take(left_max).collect();
        let padding = width - left_truncated.chars().count() - right_len;

        self.text(&left_truncated);
        for _ in 0..padding {
            self.buf.push(b' ');
        }
        self.text(&right_truncated);
        self.lf()
    }

    /// Line feed.
    pub fn lf(&mut self) -> &mut Self {
        self.buf.push(LF);
        self
    }

    /// Feed n lines.
    pub fn feed(&mut self, lines: u8) -> &mut Self {
        // ESC d n - Print and feed n lines
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

    /// Print QR code using GS ( k - QR Code commands (Function 180+).
    /// This uses the Epson QR code function which is cleaner than raster.
    pub fn qr_code(&mut self, data: &str, module_size: u8) -> &mut Self {
        // pL pH is 16-bit: past the QR capacity, print the data as text.
        if data.len() > QR_MAX_BYTES {
            return self.textln_wrapped(data);
        }
        // Function 165: Select the model (Model 2)
        // GS ( k pL pH cn fn n1 n2
        self.buf
            .extend_from_slice(&[GS, b'(', b'k', 4, 0, 49, 65, 50, 0]);

        // Function 167: Set module size (1-16)
        self.buf
            .extend_from_slice(&[GS, b'(', b'k', 3, 0, 49, 67, module_size]);

        // Function 169: Set error correction level (49=M)
        // 48=L, 49=M, 50=Q, 51=H
        self.buf
            .extend_from_slice(&[GS, b'(', b'k', 3, 0, 49, 69, 49]);

        // Function 180: Store QR data
        let data_bytes = data.as_bytes();
        let store_len = data_bytes.len() + 3; // +3 for cn, fn, m
        let pl = (store_len & 0xFF) as u8;
        let ph = ((store_len >> 8) & 0xFF) as u8;
        self.buf
            .extend_from_slice(&[GS, b'(', b'k', pl, ph, 49, 80, 48]);
        self.buf.extend_from_slice(data_bytes);

        // Function 181: Print QR code
        self.buf
            .extend_from_slice(&[GS, b'(', b'k', 3, 0, 49, 81, 48]);

        self
    }

    /// Print a raster image from a 1-bit bitmap.
    /// `width_pixels` must be a multiple of 8.
    /// `data`: row-major, MSB first, 1=black, 0=white.
    pub fn raster_image(
        &mut self,
        width_pixels: u16,
        height_pixels: u16,
        data: &[u8],
    ) -> &mut Self {
        let bytes_per_row = width_pixels.div_ceil(8);
        // GS v 0 m xL xH yL yH d1...dk
        // m=0: normal mode
        self.buf.extend_from_slice(&[
            GS,
            b'v',
            b'0',
            0,
            (bytes_per_row & 0xFF) as u8,
            ((bytes_per_row >> 8) & 0xFF) as u8,
            (height_pixels & 0xFF) as u8,
            ((height_pixels >> 8) & 0xFF) as u8,
        ]);
        self.buf.extend_from_slice(data);
        self
    }

    // ── Cut ────────────────────────────────────────────────────

    /// GS V m - Partial cut (leave small bridge).
    pub fn cut(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[GS, b'V', 66, 3]); // Partial cut, feed 3 lines
        self
    }

    /// GS V m - Full cut.
    pub fn full_cut(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[GS, b'V', 65, 3]);
        self
    }

    // ── Cash Drawer ────────────────────────────────────────────

    /// ESC p m t1 t2 - Open cash drawer.
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

    /// Whether the buffer is empty (only init commands).
    pub fn is_empty(&self) -> bool {
        self.buf.len() <= 5 // ESC @ (2) + ESC t 19 (3)
    }
}

impl Default for EscPosBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The characters of CP858 above 0x7F, box-drawing aside: every
/// Latin-1 letter (French, Dutch, German and other European menus),
/// € and the usual symbols. Kept as one table so encoding (printer)
/// and decoding (renderer) can never disagree.
const CP858_HIGH: &[(char, u8)] = &[
    ('Ç', 0x80),
    ('ü', 0x81),
    ('é', 0x82),
    ('â', 0x83),
    ('ä', 0x84),
    ('à', 0x85),
    ('ç', 0x87),
    ('ê', 0x88),
    ('ë', 0x89),
    ('è', 0x8A),
    ('ï', 0x8B),
    ('î', 0x8C),
    ('ì', 0x8D),
    ('Ä', 0x8E),
    ('É', 0x90),
    ('ô', 0x93),
    ('ö', 0x94),
    ('ò', 0x95),
    ('û', 0x96),
    ('ù', 0x97),
    ('ÿ', 0x98),
    ('Ö', 0x99),
    ('Ü', 0x9A),
    ('ß', 0xE1),
    ('´', 0xEF),
    ('À', 0xB7),
    ('Â', 0xB6),
    ('È', 0xD4),
    ('Ê', 0xD2),
    ('Ë', 0xD3),
    ('Î', 0xD7),
    ('Ï', 0xD8),
    ('Ô', 0xE2),
    ('Û', 0xEA),
    ('Ù', 0xEB),
    ('€', 0xD5),
    ('°', 0xF8),
    ('²', 0xFD),
    ('·', 0xFA),
    ('å', 0x86),
    ('Å', 0x8F),
    ('æ', 0x91),
    ('Æ', 0x92),
    ('ø', 0x9B),
    ('£', 0x9C),
    ('Ø', 0x9D),
    ('×', 0x9E),
    ('ƒ', 0x9F),
    ('á', 0xA0),
    ('í', 0xA1),
    ('ó', 0xA2),
    ('ú', 0xA3),
    ('ñ', 0xA4),
    ('Ñ', 0xA5),
    ('ª', 0xA6),
    ('º', 0xA7),
    ('¿', 0xA8),
    ('®', 0xA9),
    ('¬', 0xAA),
    ('½', 0xAB),
    ('¼', 0xAC),
    ('¡', 0xAD),
    ('«', 0xAE),
    ('»', 0xAF),
    ('Á', 0xB5),
    ('©', 0xB8),
    ('¢', 0xBD),
    ('¥', 0xBE),
    ('ã', 0xC6),
    ('Ã', 0xC7),
    ('¤', 0xCF),
    ('ð', 0xD0),
    ('Ð', 0xD1),
    ('Í', 0xD6),
    ('¦', 0xDD),
    ('Ì', 0xDE),
    ('Ó', 0xE0),
    ('Ò', 0xE3),
    ('õ', 0xE4),
    ('Õ', 0xE5),
    ('µ', 0xE6),
    ('þ', 0xE7),
    ('Þ', 0xE8),
    ('Ú', 0xE9),
    ('ý', 0xEC),
    ('Ý', 0xED),
    ('¯', 0xEE),
    ('±', 0xF1),
    ('¾', 0xF3),
    ('¶', 0xF4),
    ('§', 0xF5),
    ('÷', 0xF6),
    ('¸', 0xF7),
    ('¨', 0xF9),
    ('¹', 0xFB),
    ('³', 0xFC),
];

/// Split `s` (after [`printable`]) into lines of at most `width`
/// columns, breaking at spaces; a word longer than a line is split.
/// Always returns at least one line.
pub fn wrap(s: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in printable(s).split_whitespace() {
        let mut word: Vec<char> = word.chars().collect();
        loop {
            let used = line.chars().count();
            let sep = usize::from(used > 0);
            if used + sep + word.len() <= width {
                if sep == 1 {
                    line.push(' ');
                }
                line.extend(word.iter());
                break;
            }
            if used > 0 {
                lines.push(std::mem::take(&mut line));
                continue;
            }
            // A word wider than the paper: cut it.
            let rest = word.split_off(width);
            lines.push(word.iter().collect());
            word = rest;
        }
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

/// Byte capacity of a QR Code Model 2 (version 40, level L), the most
/// the printer's store function can hold.
pub const QR_MAX_BYTES: usize = 7089;

/// Replace what CP858 cannot print with its usual stand-in, so a
/// product typed on a phone or a Mac never prints `?`: `œ` → `oe`
/// (bœuf), typographic quotes and apostrophes → `'` and `"`, dashes →
/// `-`, `…` → `...`, special spaces → a space. Combining accents
/// (decomposed text) are dropped, keeping the base letter. Anything
/// else is left for [`encode_cp858`] (which prints `?`).
///
/// Control characters are dropped (a tab becomes a space): in external
/// text an ESC or GS byte would be a printer command.
pub fn printable(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'œ' => out.push_str("oe"),
            'Œ' => out.push_str("OE"),
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{2032}' | '\u{02BC}' => out.push('\''),
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{2033}' => out.push('"'),
            '\u{2010}'..='\u{2015}' | '\u{2212}' => out.push('-'),
            '\u{2026}' => out.push_str("..."),
            '\u{2022}' => out.push('·'),
            '\u{2122}' => out.push_str("TM"),
            '\u{00A0}' | '\u{2000}'..='\u{200A}' | '\u{202F}' | '\u{205F}' => out.push(' '),
            '\u{200B}'..='\u{200D}' | '\u{FEFF}' | '\u{00AD}' => {}
            '\u{0300}'..='\u{036F}' => {}
            '\t' => out.push(' '),
            '\u{0000}'..='\u{001F}' | '\u{007F}' => {}
            _ => out.push(ch),
        }
    }
    out
}

/// Encode one char to CP858; unknown characters become `?`.
pub fn encode_cp858(ch: char) -> u8 {
    if ch.is_ascii() {
        return ch as u8;
    }
    CP858_HIGH
        .iter()
        .find(|(c, _)| *c == ch)
        .map(|(_, b)| *b)
        .unwrap_or(b'?')
}

/// Decode one CP858 byte back to a char (renderer side).
pub fn decode_cp858(byte: u8) -> char {
    if byte < 0x80 {
        return byte as char;
    }
    CP858_HIGH
        .iter()
        .find(|(_, b)| *b == byte)
        .map(|(c, _)| *c)
        .unwrap_or('?')
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
        // init = ESC @ (2) + ESC t 19 (3)
        assert_eq!(&bytes[5..8], &[0x1B, b'a', 1]);
    }

    #[test]
    fn bold_on() {
        let mut b = EscPosBuilder::new();
        b.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        let bytes = b.build();
        // ESC E 1 should be in the output
        assert!(bytes.windows(3).any(|w| w == [0x1B, b'E', 1]));
    }

    #[test]
    fn text_columns_padding() {
        let mut b = EscPosBuilder::new();
        b.text_columns("Saumon", "12.50");
        let bytes = b.build();
        let line_start = 5; // skip ESC @ + ESC t 19
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
        assert!(
            bytes
                .windows(5)
                .any(|w| w == [0x1B, b'p', 0x00, 0x19, 0xFA])
        );
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

    #[test]
    fn control_characters_never_reach_the_printer() {
        let hostile = "Sans wasabi\x1bp\x00\x01\x01 merci\x1d(k\x1bi\x07\x7f";
        let mut b = EscPosBuilder::new();
        b.textln(hostile);
        b.textln_wrapped(hostile);
        b.text_columns(hostile, hostile);
        let bytes = b.build();
        let body = &bytes[5..]; // after ESC @ + ESC t 19
        assert!(
            !body
                .iter()
                .any(|&c| c == 0x1B || c == 0x1D || c == 0x07 || c == 0x7F || c == 0x00),
            "{body:?}"
        );
        assert!(body.windows(11).any(|w| w == b"Sans wasabi"));
        assert_eq!(printable("a\tb\r\nc"), "a bc");
    }

    #[test]
    fn an_oversize_qr_payload_prints_as_text_not_as_a_wrapped_store() {
        let data = "A".repeat(QR_MAX_BYTES + 1);
        let mut b = EscPosBuilder::new();
        b.qr_code(&data, 6);
        let bytes = b.build();
        assert!(!bytes.windows(3).any(|w| w == [GS, b'(', b'k']));
        assert!(bytes.windows(10).any(|w| w == b"AAAAAAAAAA"));

        let mut b = EscPosBuilder::new();
        b.qr_code(&"A".repeat(QR_MAX_BYTES), 6);
        let bytes = b.build();
        let store_len = QR_MAX_BYTES + 3;
        let store = [
            GS,
            b'(',
            b'k',
            (store_len & 0xFF) as u8,
            (store_len >> 8) as u8,
            49,
            80,
            48,
        ];
        assert!(bytes.windows(8).any(|w| w == store));
    }

    // ── text_columns width invariant (receipt must never overflow
    //    the 48-column paper) ─────────────────────────────────────

    fn printed_line(bytes: &[u8]) -> &[u8] {
        // Skip init (ESC @ + ESC t 19 = 5 bytes), drop trailing LF.
        &bytes[5..bytes.len() - 1]
    }

    #[test]
    fn text_columns_never_exceeds_the_configured_width_even_with_an_overlong_right_value() {
        // Regression test: `right` used to be written in full even
        // when longer than the line width, so a runaway price/label
        // string could blow the printed row past 48 columns.
        let mut b = EscPosBuilder::new();
        let long_right = "X".repeat(80);
        b.text_columns("Left", &long_right);
        let bytes = b.build();
        assert_eq!(printed_line(&bytes).len(), 48);
    }

    #[test]
    fn text_columns_truncates_an_overlong_left_value_to_fit() {
        let mut b = EscPosBuilder::new();
        let long_left = "Y".repeat(80);
        b.text_columns(&long_left, "9.99");
        let bytes = b.build();
        assert_eq!(printed_line(&bytes).len(), 48);
        assert!(printed_line(&bytes).ends_with(b"9.99"));
    }

    #[test]
    fn text_columns_with_right_exactly_at_width_leaves_no_room_for_left() {
        let mut b = EscPosBuilder::new();
        let right = "Z".repeat(48);
        b.text_columns("Should be dropped", &right);
        let bytes = b.build();
        let line = printed_line(&bytes);
        assert_eq!(line.len(), 48);
        assert_eq!(line, right.as_bytes());
    }

    #[test]
    fn text_columns_both_short_pads_exactly_to_width() {
        let mut b = EscPosBuilder::new();
        b.text_columns("Saumon", "12.50");
        let bytes = b.build();
        let line = printed_line(&bytes);
        assert_eq!(line.len(), 48);
        assert!(line.starts_with(b"Saumon"));
        assert!(line.ends_with(b"12.50"));
    }

    // ── CP858 encoding round-trip ────────────────────────────────

    #[test]
    fn ascii_round_trips_through_cp858() {
        for byte in 0u8..0x80 {
            let ch = byte as char;
            assert_eq!(encode_cp858(ch), byte, "ascii {byte:#x}");
            assert_eq!(decode_cp858(byte), ch, "ascii {byte:#x}");
        }
    }

    #[test]
    fn every_high_cp858_char_round_trips_exactly() {
        for &(ch, byte) in CP858_HIGH {
            assert_eq!(encode_cp858(ch), byte, "encode {ch:?}");
            assert_eq!(decode_cp858(byte), ch, "decode {byte:#x}");
        }
    }

    #[test]
    fn cjk_characters_are_not_representable_and_degrade_to_question_mark() {
        // Real hardware/protocol constraint, not a bug: CP858 has no
        // CJK glyphs, so unsupported characters must degrade safely
        // to '?' instead of panicking or silently dropping the char.
        for ch in ['寿', '司', '中', '文'] {
            assert_eq!(encode_cp858(ch), b'?', "{ch:?} should encode to '?'");
        }
    }

    #[test]
    fn unmapped_high_byte_decodes_to_question_mark() {
        // 0xB0 is a box-drawing glyph, never printed by the receipts.
        assert_eq!(decode_cp858(0xB0), '?');
    }

    #[test]
    fn the_high_table_has_no_duplicate_char_or_byte() {
        let mut chars: Vec<char> = CP858_HIGH.iter().map(|(c, _)| *c).collect();
        let mut bytes: Vec<u8> = CP858_HIGH.iter().map(|(_, b)| *b).collect();
        chars.sort_unstable();
        bytes.sort_unstable();
        let (nc, nb) = (chars.len(), bytes.len());
        chars.dedup();
        bytes.dedup();
        assert_eq!(chars.len(), nc, "duplicate char");
        assert_eq!(bytes.len(), nb, "duplicate byte");
        assert!(bytes.iter().all(|b| *b >= 0x80));
    }

    #[test]
    fn every_latin1_letter_is_printable() {
        // Every letter of U+00C0–U+00FF (Latin-1) except × and ÷.
        for code in 0xC0u32..=0xFF {
            let ch = char::from_u32(code).unwrap();
            if !ch.is_alphabetic() {
                continue;
            }
            assert_ne!(encode_cp858(ch), b'?', "{ch:?} must print");
        }
    }

    #[test]
    fn menu_words_print_without_question_marks() {
        for word in [
            "Bœuf bourguignon",
            "Œuf cocotte",
            "Crème brûlée",
            "Plat du jour d\u{2019}été",
            "\u{201C}Maison\u{201D} \u{2013} 2 pers.",
            "Café\u{00A0}crème…",
            "Paëlla, jalapeño, smørrebrød",
            "Grüße, Käse, Ölfondue, Weißwurst",
            "Crêpe Façon Mère",
        ] {
            let mut b = EscPosBuilder::new();
            b.text(word);
            let printed = &b.build()[5..];
            assert!(!printed.contains(&b'?'), "{word:?} printed {printed:?}");
        }
    }

    #[test]
    fn stand_ins_are_the_usual_ones() {
        assert_eq!(printable("Bœuf"), "Boeuf");
        assert_eq!(printable("ŒUF"), "OEUF");
        assert_eq!(printable("l\u{2019}été"), "l'été");
        assert_eq!(printable("\u{201E}x\u{201C}"), "\"x\"");
        assert_eq!(printable("a\u{2013}b\u{2014}c"), "a-b-c");
        assert_eq!(printable("Suite\u{2026}"), "Suite...");
        assert_eq!(printable("10\u{202F}%"), "10 %");
        assert_eq!(
            printable("e\u{0301}"),
            "e",
            "decomposed accent keeps the letter"
        );
        assert_eq!(printable("a\u{200B}b"), "ab");
        assert_eq!(printable("寿司"), "寿司", "left for encode_cp858");
    }

    #[test]
    fn wrap_breaks_between_words() {
        assert_eq!(wrap("aa bb cc", 5), vec!["aa bb", "cc"]);
        assert_eq!(wrap("aa  bb", 10), vec!["aa bb"]);
        assert_eq!(wrap("", 10), vec![""]);
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
        assert_eq!(wrap("x abcdefgh", 4), vec!["x", "abcd", "efgh"]);
        assert_eq!(wrap("Bœuf œuf", 4), vec!["Boeu", "f", "oeuf"]);
        for (s, w) in [
            ("Rue de l'Exemple 123, 4000 Liège, Belgique", 12),
            ("a b c d e f", 1),
        ] {
            let lines = wrap(s, w);
            assert!(lines.iter().all(|l| l.chars().count() <= w), "{lines:?}");
            let joined: String = lines.concat().split_whitespace().collect();
            let original: String = printable(s).split_whitespace().collect();
            assert_eq!(joined, original, "nothing lost");
        }
    }

    #[test]
    fn columns_count_the_stand_ins() {
        // "Bœuf" prints as "Boeuf": 5 columns, not 4.
        let mut b = EscPosBuilder::new();
        b.text_columns("Bœuf", "12.00");
        let line = &b.build()[5..];
        assert_eq!(line.len(), 49, "48 columns + LF");
        assert!(line.starts_with(b"Boeuf "));
        assert!(line.ends_with(b"12.00\n"));
    }

    // ── Misc builder surface ─────────────────────────────────────

    #[test]
    fn full_cut_command() {
        let mut b = EscPosBuilder::new();
        b.full_cut();
        let bytes = b.build();
        assert!(bytes.windows(4).any(|w| w == [0x1D, b'V', 65, 3]));
    }

    #[test]
    fn is_empty_true_only_right_after_init() {
        let b = EscPosBuilder::new();
        assert!(b.is_empty());
        let mut b2 = EscPosBuilder::new();
        b2.textln("x");
        assert!(!b2.is_empty());
    }

    #[test]
    fn len_grows_as_bytes_are_appended() {
        let mut b = EscPosBuilder::new();
        let before = b.len();
        b.textln("hello");
        assert!(b.len() > before);
        assert_eq!(b.len(), b.as_bytes().len());
    }

    #[test]
    fn dots_and_chars_per_line_match_the_80mm_target() {
        let b = EscPosBuilder::new();
        assert_eq!(b.chars_per_line(), 48);
        assert_eq!(b.dots_per_line(), 576);
    }

    #[test]
    fn raster_image_header_encodes_dimensions_little_endian() {
        let mut b = EscPosBuilder::new();
        // 16 px wide (2 bytes/row), 1 px tall, all-zero data.
        b.raster_image(16, 1, &[0u8, 0u8]);
        let bytes = b.build();
        assert!(
            bytes
                .windows(8)
                .any(|w| w == [GS, b'v', b'0', 0, 2, 0, 1, 0])
        );
    }

    #[test]
    fn style_reset_clears_bold_underline_and_size() {
        let mut b = EscPosBuilder::new();
        b.style(TextStyle {
            bold: true,
            underline: true,
            double_height: true,
            double_width: true,
            font_b: true,
        });
        b.style_reset();
        let bytes = b.build();
        // style_reset() must have appended the all-off command bytes.
        assert!(bytes.windows(3).any(|w| w == [ESC, b'E', 0]));
        assert!(bytes.windows(3).any(|w| w == [ESC, b'-', 0]));
        assert!(bytes.windows(3).any(|w| w == [GS, b'!', 0]));
        assert!(bytes.windows(3).any(|w| w == [ESC, b'M', 0]));
    }
}
