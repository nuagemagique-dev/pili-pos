//! A tiny CSV writer: UTF-8 with a BOM, `;` separator, CRLF line ends.
//!
//! Fields holding `;`, a quote or a line break are quoted. Text that comes from the
//! till (a customer name, a cash movement label) is neutralised against spreadsheet
//! formulas by [`text`]. Amounts are integer cents; [`amount`] formats them.

/// How amounts are written in the text columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decimal {
    /// `12,50` (default, Belgian spreadsheets).
    Comma,
    /// `12.50`.
    Point,
}

impl Decimal {
    fn mark(self) -> char {
        match self {
            Self::Comma => ',',
            Self::Point => '.',
        }
    }
}

/// `1250` as `12,50` or `12.50`; negative amounts keep their sign (`-0,05`).
pub fn amount(cents: i64, decimal: Decimal) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    format!("{sign}{}{}{:02}", abs / 100, decimal.mark(), abs % 100)
}

/// Neutralise a free text against spreadsheet formulas: a leading `=`, `+`, `-`,
/// `@`, tab or carriage return gets a leading apostrophe.
pub fn text(value: &str) -> String {
    let value = value.trim();
    match value.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{value}"),
        _ => value.to_string(),
    }
}

fn field(value: &str) -> String {
    if value.contains([';', '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// One CSV document: a header and rows, rendered by [`Table::render`].
#[derive(Debug, Clone)]
pub struct Table {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(header: &[&str]) -> Self {
        Self {
            header: header.iter().map(|h| (*h).to_string()).collect(),
            rows: Vec::new(),
        }
    }

    /// Add a row; it must have as many fields as the header.
    pub fn push(&mut self, row: Vec<String>) {
        debug_assert_eq!(row.len(), self.header.len(), "row width");
        self.rows.push(row);
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// BOM, header and rows, `;` separated, CRLF terminated.
    pub fn render(&self) -> Vec<u8> {
        let mut out = String::from('\u{feff}');
        for row in std::iter::once(&self.header).chain(self.rows.iter()) {
            let line: Vec<String> = row.iter().map(|f| field(f)).collect();
            out.push_str(&line.join(";"));
            out.push_str("\r\n");
        }
        out.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_use_the_chosen_decimal_mark() {
        assert_eq!(amount(1250, Decimal::Comma), "12,50");
        assert_eq!(amount(1250, Decimal::Point), "12.50");
        assert_eq!(amount(-5, Decimal::Comma), "-0,05");
        assert_eq!(amount(0, Decimal::Point), "0.00");
        assert_eq!(amount(100_000_007, Decimal::Point), "1000000.07");
        assert_eq!(amount(i64::MIN, Decimal::Point), "-92233720368547758.08");
    }

    #[test]
    fn a_table_is_bom_semicolon_and_crlf() {
        let mut t = Table::new(&["a", "b"]);
        t.push(vec!["1".into(), "x;y".into()]);
        t.push(vec!["say \"hi\"".into(), "line\nbreak".into()]);
        let bytes = t.render();
        assert_eq!(&bytes[..3], [0xEF, 0xBB, 0xBF]);
        let s = String::from_utf8(bytes[3..].to_vec()).unwrap();
        assert_eq!(
            s,
            "a;b\r\n1;\"x;y\"\r\n\"say \"\"hi\"\"\";\"line\nbreak\"\r\n"
        );
    }

    #[test]
    fn free_text_cannot_start_a_formula() {
        assert_eq!(text("=1+1"), "'=1+1");
        assert_eq!(text("  @cmd"), "'@cmd");
        assert_eq!(text("-5"), "'-5");
        assert_eq!(text("Fond de caisse"), "Fond de caisse");
    }
}
