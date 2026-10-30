//! Dialog 06 records. Bytes are 7 bits on the wire (9600 baud, 7 data
//! bits, odd parity, 1 stop bit); the serial port handles the parity, so
//! a byte above 7Fh reaching this module is a damaged one.

pub const EOT: u8 = 0x04;
pub const ENQ: u8 = 0x05;
pub const STX: u8 = 0x02;
pub const ETX: u8 = 0x03;
pub const ESC: u8 = 0x1B;
pub const ACK: u8 = 0x06;
pub const NAK: u8 = 0x15;

/// The scale status of a record 02 that means kilograms.
pub const STATUS_KG: u8 = 0x33;
/// The text of records 04 and 05 is exactly this long.
pub const TEXT_LEN: usize = 13;
/// Record 10 carries at most this many checksum and correction pairs.
pub const MAX_CHECK_VALUES: usize = 5;
/// The scale answers NAK past 50 characters.
const MAX_FRAME: usize = 50;

/// Digits of the unit price field: a scale is set up for one or the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceDigits {
    Five,
    Six,
}

impl PriceDigits {
    fn width(self) -> usize {
        match self {
            Self::Five => 5,
            Self::Six => 6,
        }
    }

    /// The highest unit price in cents the field can carry.
    pub fn max_cents(self) -> i64 {
        match self {
            Self::Five => 99_999,
            Self::Six => 999_999,
        }
    }
}

/// One checksum and its correction value (record 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckValue {
    pub checksum: u16,
    pub correction: u16,
}

/// A valid weight value (record 02), as received.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightRecord {
    pub status: u8,
    pub weight_grams: i64,
    pub unit_price_cents: i64,
    pub selling_price_cents: i64,
}

/// What the scale said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Ack,
    Nak,
    Weight(WeightRecord),
    /// Record 09: the two status digits, 20 for "still in motion".
    Status(u8),
    /// Record 11, D0 = 30h: the software check failed.
    CheckFailed,
    /// Record 11, D0 = 31h: the software check passed.
    CheckOk,
    /// Record 11, D0 = 32h: send record 10 for this random number.
    CheckRequest(String),
    /// A frame that cannot be read: bad bytes, a bad length, an unknown
    /// record. Never a weight.
    Garbled,
}

fn digits(value: i64, width: usize) -> Option<String> {
    let text = format!("{value:0width$}");
    (value >= 0 && text.len() == width).then_some(text)
}

/// `EOT ENQ`: the request for scale data.
pub fn enq() -> Vec<u8> {
    vec![EOT, ENQ]
}

/// Record 08: the status after a NAK.
pub fn status_request() -> Vec<u8> {
    vec![EOT, STX, b'0', b'8', ETX]
}

/// Record 01, 03, 04 or 05: the unit price, with a tare in grams and a
/// text of exactly [`TEXT_LEN`] ASCII characters when given. `None` when
/// a value does not fit its field.
pub fn price_record(
    unit_price_cents: i64,
    digit_count: PriceDigits,
    tare_grams: Option<i64>,
    text: Option<&str>,
) -> Option<Vec<u8>> {
    let price = digits(unit_price_cents, digit_count.width())?;
    let tare = tare_grams.map(|t| digits(t, 4)).transpose_some()?;
    if let Some(t) = text
        && (t.len() != TEXT_LEN || !t.bytes().all(|b| (0x20..0x7F).contains(&b)))
    {
        return None;
    }
    let record: &[u8; 2] = match (tare.is_some(), text.is_some()) {
        (false, false) => b"01",
        (true, false) => b"03",
        (false, true) => b"04",
        (true, true) => b"05",
    };
    let mut out = vec![EOT, STX, record[0], record[1], ESC];
    out.extend_from_slice(price.as_bytes());
    if let Some(t) = &tare {
        out.push(ESC);
        out.extend_from_slice(t.as_bytes());
    }
    if let Some(t) = text {
        out.push(ESC);
        out.extend_from_slice(t.as_bytes());
    }
    if tare.is_none() && text.is_none() {
        // Record 01 alone closes its price with a separator (spec, page 1).
        out.push(ESC);
    }
    out.push(ETX);
    Some(out)
}

trait TransposeSome<T> {
    /// `None` means nothing was given, `Some(None)` a value that did not
    /// fit: the outer `Option` is `None` only for the second.
    fn transpose_some(self) -> Option<Option<T>>;
}

impl<T> TransposeSome<T> for Option<Option<T>> {
    fn transpose_some(self) -> Option<Option<T>> {
        match self {
            None => Some(None),
            Some(Some(v)) => Some(Some(v)),
            Some(None) => None,
        }
    }
}

/// Record 10: the checksums and correction values, four hex digits each.
pub fn check_record(values: &[CheckValue]) -> Vec<u8> {
    let mut out = vec![EOT, STX, b'1', b'0', ESC];
    for v in values.iter().take(MAX_CHECK_VALUES) {
        out.extend_from_slice(format!("{:04X}{:04X}", v.checksum, v.correction).as_bytes());
    }
    out.push(ETX);
    out
}

/// The text a product sends to the scale: its name without accents, cut
/// and padded to [`TEXT_LEN`] characters. `None` when the name has no
/// ASCII form (a Chinese name): the price then goes without a text.
pub fn scale_text(name: &str) -> Option<String> {
    let mut out = String::with_capacity(TEXT_LEN);
    for c in name.trim().chars() {
        let plain = match c {
            'à' | 'â' | 'ä' => 'a',
            'À' | 'Â' | 'Ä' => 'A',
            'ç' => 'c',
            'Ç' => 'C',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'î' | 'ï' => 'i',
            'Î' | 'Ï' => 'I',
            'ô' | 'ö' => 'o',
            'Ô' | 'Ö' => 'O',
            'ù' | 'û' | 'ü' => 'u',
            'Ù' | 'Û' | 'Ü' => 'U',
            '\'' | '’' => '\'',
            c if (' '..='~').contains(&c) => c,
            _ => return None,
        };
        out.push(plain);
    }
    if out.is_empty() {
        return None;
    }
    let mut text: String = out.chars().take(TEXT_LEN).collect();
    while text.len() < TEXT_LEN {
        text.push(' ');
    }
    Some(text)
}

/// Reads the scale's bytes into messages, across any cut of the stream.
#[derive(Debug, Default)]
pub struct Decoder {
    frame: Option<Vec<u8>>,
    /// In a run of bad bytes: the run counts as one damaged frame.
    noise: bool,
}

impl Decoder {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Message> {
        let mut out = Vec::new();
        for &b in bytes {
            if b > 0x7F {
                // A damaged byte: whatever frame it was in is lost.
                self.frame = None;
                self.noise(&mut out);
                continue;
            }
            match &mut self.frame {
                Some(frame) => match b {
                    ETX => {
                        let body = std::mem::take(frame);
                        self.frame = None;
                        out.push(parse_body(&body));
                    }
                    STX => {
                        // A new frame inside a frame: the first one is lost.
                        frame.clear();
                        out.push(Message::Garbled);
                    }
                    _ if frame.len() >= MAX_FRAME => {
                        self.frame = None;
                        out.push(Message::Garbled);
                    }
                    _ => frame.push(b),
                },
                None => match b {
                    ACK => {
                        self.noise = false;
                        out.push(Message::Ack);
                    }
                    NAK => {
                        self.noise = false;
                        out.push(Message::Nak);
                    }
                    STX => {
                        self.noise = false;
                        self.frame = Some(Vec::new());
                    }
                    _ => self.noise(&mut out),
                },
            }
        }
        out
    }

    fn noise(&mut self, out: &mut Vec<Message>) {
        if !self.noise {
            self.noise = true;
            out.push(Message::Garbled);
        }
    }
}

fn number(field: &[u8]) -> Option<i64> {
    if field.is_empty() || !field.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(field).ok()?.parse().ok()
}

fn parse_body(body: &[u8]) -> Message {
    let parsed = match body.get(..2) {
        Some(b"02") => parse_weight(body),
        Some(b"09") => parse_status(body),
        Some(b"11") => parse_check(body),
        _ => None,
    };
    parsed.unwrap_or(Message::Garbled)
}

fn parse_weight(body: &[u8]) -> Option<Message> {
    let fields: Vec<&[u8]> = body.split(|&b| b == ESC).collect();
    let [_, status, weight, unit, selling] = fields.as_slice() else {
        return None;
    };
    let [status] = status else {
        return None;
    };
    if weight.len() != 5 || !matches!(unit.len(), 5 | 6) || selling.len() != 6 {
        return None;
    }
    Some(Message::Weight(WeightRecord {
        status: *status,
        weight_grams: number(weight)?,
        unit_price_cents: number(unit)?,
        selling_price_cents: number(selling)?,
    }))
}

fn parse_status(body: &[u8]) -> Option<Message> {
    let fields: Vec<&[u8]> = body.split(|&b| b == ESC).collect();
    let [_, status] = fields.as_slice() else {
        return None;
    };
    if status.len() != 2 {
        return None;
    }
    Some(Message::Status(u8::try_from(number(status)?).ok()?))
}

fn parse_check(body: &[u8]) -> Option<Message> {
    let fields: Vec<&[u8]> = body.split(|&b| b == ESC).collect();
    let [_, data] = fields.as_slice() else {
        return None;
    };
    match data.split_first()? {
        (b'0', []) => Some(Message::CheckFailed),
        (b'1', []) => Some(Message::CheckOk),
        (b'2', z) if !z.is_empty() && z.iter().all(u8::is_ascii_hexdigit) => {
            Some(Message::CheckRequest(String::from_utf8(z.to_vec()).ok()?))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(bytes: &[u8]) -> Vec<Message> {
        Decoder::default().push(bytes)
    }

    #[test]
    fn the_price_records_follow_the_spec_layouts() {
        let price = |tare, text| price_record(4500, PriceDigits::Five, tare, text).unwrap();
        assert_eq!(price(None, None), b"\x04\x0201\x1b04500\x1b\x03");
        assert_eq!(price(Some(30), None), b"\x04\x0203\x1b04500\x1b0030\x03");
        assert_eq!(
            price(None, Some("Bol malatang ")),
            b"\x04\x0204\x1b04500\x1bBol malatang \x03"
        );
        assert_eq!(
            price(Some(30), Some("Bol malatang ")),
            b"\x04\x0205\x1b04500\x1b0030\x1bBol malatang \x03"
        );
    }

    #[test]
    fn a_value_that_does_not_fit_its_field_builds_no_record() {
        assert!(price_record(100_000, PriceDigits::Five, None, None).is_none());
        assert!(price_record(-1, PriceDigits::Six, None, None).is_none());
        assert!(price_record(100, PriceDigits::Six, Some(10_000), None).is_none());
        assert!(price_record(100, PriceDigits::Six, None, Some("short")).is_none());
        assert!(price_record(999_999, PriceDigits::Six, None, None).is_some());
    }

    #[test]
    fn the_spec_example_of_a_dialog_reads_back() {
        // Page 6 of the spec.
        assert_eq!(
            decode(b"\x02\x30\x32\x1b3\x1b02000\x1b123456\x1b246912\x03"),
            vec![Message::Weight(WeightRecord {
                status: STATUS_KG,
                weight_grams: 2000,
                unit_price_cents: 123_456,
                selling_price_cents: 246_912,
            })]
        );
        assert_eq!(
            decode(b"\x0211\x1b2EF\x03"),
            vec![Message::CheckRequest("EF".into())]
        );
        assert_eq!(decode(b"\x0211\x1b1\x03"), vec![Message::CheckOk]);
        assert_eq!(decode(b"\x0211\x1b0\x03"), vec![Message::CheckFailed]);
        assert_eq!(decode(b"\x0209\x1b20\x03"), vec![Message::Status(20)]);
        assert_eq!(
            check_record(&[CheckValue {
                checksum: 0x4711,
                correction: 0xF336
            }]),
            b"\x04\x0210\x1b4711F336\x03"
        );
    }

    #[test]
    fn a_frame_survives_any_cut_of_the_stream() {
        let bytes = b"\x06\x0209\x1b20\x03\x15";
        for cut in 0..bytes.len() {
            let mut decoder = Decoder::default();
            let mut got = decoder.push(&bytes[..cut]);
            got.extend(decoder.push(&bytes[cut..]));
            assert_eq!(got, vec![Message::Ack, Message::Status(20), Message::Nak]);
        }
    }

    #[test]
    fn damaged_frames_are_never_a_weight() {
        for bytes in [
            &b"\x0202\x1b3\x1b00278\x1b04500\x03"[..], // no selling price
            b"\x0202\x1b3\x1b00278\x1b04500\x1b00125\x03", // short selling price
            b"\x0202\x1b3\x1b0A278\x1b04500\x1b001251\x03", // not a digit
            b"\x0202\x1b3\x1b0027\xf8\x1b04500\x1b001251\x03", // damaged byte
            b"\x0202\x1b33\x1b00278\x1b04500\x1b001251\x03", // two status bytes
            b"\x0277\x03",                             // unknown record
            b"\x0211\x1b2\x03",                        // request without Z
        ] {
            let got = decode(bytes);
            assert!(!got.is_empty(), "{bytes:?}");
            assert!(
                got.iter().all(|m| !matches!(m, Message::Weight(_))),
                "{bytes:?}: {got:?}"
            );
        }
    }

    #[test]
    fn a_run_of_bad_bytes_is_one_damaged_frame() {
        assert_eq!(
            decode(b"\xff\xfe\x00\x06"),
            vec![Message::Garbled, Message::Ack]
        );
    }

    #[test]
    fn an_endless_frame_is_dropped() {
        let mut bytes = vec![STX];
        bytes.extend(std::iter::repeat_n(b'1', 200));
        assert!(decode(&bytes).contains(&Message::Garbled));
    }

    #[test]
    fn the_scale_text_is_plain_ascii_of_the_right_length() {
        assert_eq!(scale_text("Bol malatang").unwrap(), "Bol malatang ");
        assert_eq!(scale_text("Poké saumon épicé").unwrap(), "Poke saumon e");
        assert_eq!(scale_text("麻辣烫"), None);
        assert_eq!(scale_text("   "), None);
    }
}
