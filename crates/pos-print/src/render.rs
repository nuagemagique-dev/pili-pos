//! ESC/POS → plain-text renderer.
//!
//! Interprets the byte stream produced by [`crate::escpos::EscPosBuilder`]
//! and renders it as the text a thermal printer would lay down, one
//! line per printed line, honouring alignment and character size.
//!
//! Purpose: the certification dossier (§VI, `TICKETS_EXEMPLES_FR.md`)
//! must contain transcriptions of every ticket type. Hand-written
//! transcriptions drift from the implementation; rendering the real
//! `format_receipt` output guarantees the dossier matches what the
//! printer produces at the intake appointment.
//!
//! Only the commands the builder emits are interpreted; anything
//! unknown is skipped defensively.

/// Paper width in characters (Font A on 80 mm — keep in sync with
/// `escpos::CHARS_PER_LINE`).
const WIDTH: usize = 48;

#[derive(Clone, Copy, PartialEq)]
enum Align {
    Left,
    Center,
    Right,
}

/// Render an ESC/POS byte stream to plain text.
pub fn render_plain_text(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut line = String::new();
    let mut align = Align::Left;
    let mut double_width = false;

    let flush = |line: &mut String, out: &mut String, align: Align, double_width: bool| {
        // A double-width glyph occupies two columns.
        let effective_width = if double_width { WIDTH / 2 } else { WIDTH };
        let content = line.trim_end();
        let padded = match align {
            Align::Left => content.to_string(),
            Align::Center => {
                let pad = effective_width.saturating_sub(content.chars().count()) / 2;
                format!("{}{}", " ".repeat(pad), content)
            }
            Align::Right => {
                let pad = effective_width.saturating_sub(content.chars().count());
                format!("{}{}", " ".repeat(pad), content)
            }
        };
        out.push_str(padded.trim_end());
        out.push('\n');
        line.clear();
    };

    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            0x0A => {
                flush(&mut line, &mut out, align, double_width);
                i += 1;
            }
            0x1B => {
                // ESC commands
                match bytes.get(i + 1) {
                    Some(b'@') => i += 2, // init
                    Some(b'a') => {
                        align = match bytes.get(i + 2) {
                            Some(1) => Align::Center,
                            Some(2) => Align::Right,
                            _ => Align::Left,
                        };
                        i += 3;
                    }
                    Some(b'E') | Some(b'-') | Some(b'M') | Some(b't') => i += 3, // style/code page
                    Some(b'd') => {
                        // feed n lines
                        let n = bytes.get(i + 2).copied().unwrap_or(0);
                        for _ in 0..n {
                            out.push('\n');
                        }
                        i += 3;
                    }
                    Some(b'p') => i += 5, // drawer kick
                    _ => i += 2,
                }
            }
            0x1D => {
                // GS commands
                match bytes.get(i + 1) {
                    Some(b'!') => {
                        let n = bytes.get(i + 2).copied().unwrap_or(0);
                        double_width = (n >> 4) & 0x0F > 0;
                        i += 3;
                    }
                    Some(b'V') => {
                        out.push_str(&format!("✂{}\n", "-".repeat(WIDTH - 1)));
                        i += 4;
                    }
                    Some(b'(') if bytes.get(i + 2) == Some(&b'k') => {
                        // QR functions: GS ( k pL pH cn fn ...
                        let pl = bytes.get(i + 3).copied().unwrap_or(0) as usize;
                        let ph = bytes.get(i + 4).copied().unwrap_or(0) as usize;
                        let len = pl | (ph << 8);
                        let func = bytes.get(i + 6).copied().unwrap_or(0);
                        if func == 80 {
                            // Function 180: store QR data — payload after cn/fn/m
                            let start = i + 8;
                            let end = (i + 5 + len).min(bytes.len());
                            let data = String::from_utf8_lossy(&bytes[start..end]);
                            out.push_str(&format!("{:^WIDTH$}\n", format!("[QR 25x25: {data}]")));
                        }
                        i += 5 + len;
                    }
                    Some(b'v') => {
                        // raster image — skip header + data
                        let bpr = bytes.get(i + 4).copied().unwrap_or(0) as usize
                            | ((bytes.get(i + 5).copied().unwrap_or(0) as usize) << 8);
                        let h = bytes.get(i + 6).copied().unwrap_or(0) as usize
                            | ((bytes.get(i + 7).copied().unwrap_or(0) as usize) << 8);
                        out.push_str(&format!("{:^WIDTH$}\n", "[IMAGE]"));
                        i += 8 + bpr * h;
                    }
                    _ => i += 2,
                }
            }
            b => {
                line.push(crate::escpos::decode_cp858(b));
                i += 1;
            }
        }
    }
    if !line.is_empty() {
        flush(&mut line, &mut out, align, double_width);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::escpos::{Alignment, EscPosBuilder};

    #[test]
    fn renders_aligned_text() {
        let mut p = EscPosBuilder::new();
        p.align(Alignment::Center);
        p.textln("HELLO");
        p.align(Alignment::Left);
        p.text_columns("TOTAL", "12.00");
        let text = render_plain_text(&p.build());
        let lines: Vec<&str> = text.lines().collect();
        // centered within 48 chars: (48-5)/2 = 21 spaces
        assert_eq!(lines[0], format!("{}HELLO", " ".repeat(21)));
        assert!(lines[1].starts_with("TOTAL"));
        assert!(lines[1].ends_with("12.00"));
        assert_eq!(lines[1].len(), 48);
    }

    #[test]
    fn renders_qr_placeholder_with_data() {
        let mut p = EscPosBuilder::new();
        p.qr_code("https://www.gs2.be/abc", 6);
        let text = render_plain_text(&p.build());
        assert!(text.contains("[QR 25x25: https://www.gs2.be/abc]"));
    }

    #[test]
    fn cut_renders_scissor_line() {
        let mut p = EscPosBuilder::new();
        p.textln("x");
        p.cut();
        let text = render_plain_text(&p.build());
        assert!(text.contains('✂'));
    }
}
