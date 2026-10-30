//! The gift voucher slips (V12-D30): the voucher itself, printed after
//! the sale that sold it (or again as a duplicate), and the balance
//! slip printed after a sale that leaves a balance on a voucher.
//!
//! Non-fiscal prints: they carry the mention that they are not a valid
//! VAT ticket (the same words as the bar slip). The voucher shows its
//! value, its code (large, with a QR of the code), its dates and the
//! rules of use; the balance slip masks the code to its last 4 digits.

use pos_fiscal::ticket::{ReceiptLanguage, invalid_disclaimer};

use crate::escpos::{Alignment, EscPosBuilder, PaperWidth, TextStyle};

/// A voucher to print.
#[derive(Debug, Clone)]
pub struct VoucherSlip {
    pub establishment: String,
    /// The establishment's address lines, as configured.
    pub address: Vec<String>,
    pub face_value_cents: i64,
    /// The 12 digits, unspaced.
    pub code: String,
    /// Already formatted, `17.10.2026`.
    pub issued_on: String,
    pub expires_on: String,
    /// The sale that sold it, as the ticket prints it (`N 1234`), if known.
    pub reference: Option<String>,
    /// `fr`, `nl`, `en` or `de`; anything else prints in French.
    pub lang: String,
    pub duplicate: bool,
    pub training: bool,
}

/// The balance left on a voucher after a payment.
#[derive(Debug, Clone)]
pub struct BalanceSlip {
    pub establishment: String,
    /// The last 4 digits of the code.
    pub code_last4: String,
    pub balance_cents: i64,
    pub expires_on: String,
    pub lang: String,
    pub training: bool,
}

struct Words {
    title: &'static str,
    duplicate: &'static str,
    value: &'static str,
    code: &'static str,
    issued: &'static str,
    valid_until: &'static str,
    rules: &'static str,
    reference: &'static str,
    balance_title: &'static str,
    balance: &'static str,
}

fn words(lang: &str) -> &'static Words {
    match lang {
        "nl" => &Words {
            title: "CADEAUBON",
            duplicate: "DUPLICAAT",
            value: "Waarde",
            code: "Code",
            issued: "Uitgegeven op",
            valid_until: "Geldig tot",
            rules: "Te gebruiken in een of meerdere keren, zonder terugbetaling van wisselgeld",
            reference: "Ticket",
            balance_title: "SALDO VAN DE BON",
            balance: "Saldo",
        },
        "de" => &Words {
            title: "GESCHENKGUTSCHEIN",
            duplicate: "DUPLIKAT",
            value: "Wert",
            code: "Code",
            issued: "Ausgestellt am",
            valid_until: "Gueltig bis",
            rules: "In einem oder mehreren Malen einloesbar, kein Wechselgeld",
            reference: "Beleg",
            balance_title: "GUTSCHEINSALDO",
            balance: "Restbetrag",
        },
        _ => &Words {
            title: "BON CADEAU",
            duplicate: "DUPLICATA",
            value: "Valeur",
            code: "Code",
            issued: "Emis le",
            valid_until: "Valable jusqu'au",
            rules: "Utilisable en une ou plusieurs fois, sans rendu de monnaie",
            reference: "Ticket",
            balance_title: "SOLDE DU BON",
            balance: "Solde",
        },
    }
}

fn receipt_lang(lang: &str) -> ReceiptLanguage {
    match lang {
        "nl" => ReceiptLanguage::Nl,
        "de" => ReceiptLanguage::De,
        _ => ReceiptLanguage::Fr,
    }
}

/// `5000` is `50,00 EUR`.
pub fn euros(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    format!("{sign}{},{:02} EUR", abs / 100, abs % 100)
}

/// `123456789012` is `1234 5678 9012`.
pub fn spaced_code(code: &str) -> String {
    code.as_bytes()
        .chunks(4)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn disclaimer(p: &mut EscPosBuilder, lang: &str) {
    p.align(Alignment::Center);
    p.textln_wrapped(invalid_disclaimer(receipt_lang(lang)));
}

fn training_mark(p: &mut EscPosBuilder, training: bool) {
    if training {
        p.align(Alignment::Center);
        p.textln("TRAINING");
    }
}

/// ESC/POS bytes of the voucher.
pub fn format_gift_voucher(slip: &VoucherSlip, paper: PaperWidth) -> Vec<u8> {
    let w = words(&slip.lang);
    let mut p = EscPosBuilder::for_paper(paper);
    p.align(Alignment::Center);
    training_mark(&mut p, slip.training);
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.textln(w.title);
    p.style_reset();
    if slip.duplicate {
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.textln(w.duplicate);
        p.style_reset();
    }
    if !slip.establishment.trim().is_empty() {
        p.textln_wrapped(&slip.establishment);
    }
    for line in slip.address.iter().filter(|l| !l.trim().is_empty()) {
        p.textln_wrapped(line);
    }
    p.separator('=');
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.textln_wrapped(&format!("{}: {}", w.value, euros(slip.face_value_cents)));
    p.style_reset();
    p.blank_line();
    p.textln(w.code);
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.textln(&spaced_code(&slip.code));
    p.style_reset();
    p.qr_code(&slip.code, 6);
    p.blank_line();
    p.textln_wrapped(&format!("{}: {}", w.issued, slip.issued_on));
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln_wrapped(&format!("{} {}", w.valid_until, slip.expires_on));
    p.style_reset();
    p.separator('-');
    p.textln_wrapped(w.rules);
    if let Some(reference) = slip.reference.as_deref().filter(|r| !r.is_empty()) {
        p.textln_wrapped(&format!("{}: {}", w.reference, reference));
    }
    p.separator('-');
    disclaimer(&mut p, &slip.lang);
    p.feed(4);
    p.cut();
    p.build()
}

/// ESC/POS bytes of the balance slip.
pub fn format_balance_slip(slip: &BalanceSlip, paper: PaperWidth) -> Vec<u8> {
    let w = words(&slip.lang);
    let mut p = EscPosBuilder::for_paper(paper);
    p.align(Alignment::Center);
    training_mark(&mut p, slip.training);
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln(w.balance_title);
    p.style_reset();
    if !slip.establishment.trim().is_empty() {
        p.textln_wrapped(&slip.establishment);
    }
    p.separator('-');
    p.textln_wrapped(&format!("{}: **** **** {}", w.code, slip.code_last4));
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.textln_wrapped(&format!("{}: {}", w.balance, euros(slip.balance_cents)));
    p.style_reset();
    p.textln_wrapped(&format!("{} {}", w.valid_until, slip.expires_on));
    p.separator('-');
    disclaimer(&mut p, &slip.lang);
    p.feed(4);
    p.cut();
    p.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{one_line, render_plain_text};

    fn voucher(lang: &str) -> VoucherSlip {
        VoucherSlip {
            establishment: "Restaurant Demo".into(),
            address: vec!["Rue Exemple 1".into(), "4000 Liege".into()],
            face_value_cents: 5000,
            code: "123456789012".into(),
            issued_on: "17.10.2026".into(),
            expires_on: "17.10.2027".into(),
            reference: Some("N 1234".into()),
            lang: lang.into(),
            duplicate: false,
            training: false,
        }
    }

    fn text(slip: &VoucherSlip) -> String {
        render_plain_text(&format_gift_voucher(slip, PaperWidth::Mm80))
    }

    #[test]
    fn the_voucher_and_the_balance_slip_fit_58_mm_paper() {
        for lang in ["fr", "nl", "de", "en"] {
            crate::render::assert_58_mm_prints_the_80_mm_words(|paper| {
                format_gift_voucher(&voucher(lang), paper)
            });
            let slip = BalanceSlip {
                establishment: "Restaurant Demo".into(),
                code_last4: "9012".into(),
                balance_cents: 3150,
                expires_on: "17.10.2027".into(),
                lang: lang.into(),
                training: true,
            };
            crate::render::assert_58_mm_prints_the_80_mm_words(|paper| {
                format_balance_slip(&slip, paper)
            });
        }
    }

    #[test]
    fn the_amount_and_the_code_are_spelled_out() {
        assert_eq!(euros(5000), "50,00 EUR");
        assert_eq!(euros(1850), "18,50 EUR");
        assert_eq!(euros(5), "0,05 EUR");
        assert_eq!(spaced_code("123456789012"), "1234 5678 9012");
    }

    #[test]
    fn the_french_voucher_golden() {
        let out = text(&voucher("fr"));
        let body = out.split("CECI").next().unwrap();
        let one = one_line(body);
        for needle in [
            "BON CADEAU",
            "Restaurant Demo",
            "Rue Exemple 1",
            "Valeur: 50,00 EUR",
            "1234 5678 9012",
            "Emis le: 17.10.2026",
            "Valable jusqu'au 17.10.2027",
            "Utilisable en une ou plusieurs fois, sans rendu de monnaie",
            "Ticket: N 1234",
        ] {
            assert!(one.contains(needle), "{needle}: {one}");
        }
        assert!(!one.contains("DUPLICATA"));
        assert!(one_line(&out).contains("CECI N'EST PAS UN TICKET DE CAISSE T.V.A."));
    }

    #[test]
    fn a_duplicate_says_so_and_training_is_marked() {
        let mut v = voucher("fr");
        v.duplicate = true;
        v.training = true;
        let out = one_line(&text(&v));
        assert!(out.contains("DUPLICATA"), "{out}");
        assert!(out.contains("TRAINING"), "{out}");
    }

    #[test]
    fn every_language_has_the_mention_and_the_dates() {
        // The establishment language is fr, nl or de (zh and en print in French).
        for lang in ["fr", "nl", "en", "de", "zh"] {
            let out = one_line(&text(&voucher(lang)));
            assert!(out.contains("17.10.2027"), "{lang}: {out}");
            assert!(out.contains("1234 5678 9012"), "{lang}: {out}");
            assert!(
                out.contains("T.V.A.") || out.contains("BTW") || out.contains("MWST"),
                "{lang}: {out}"
            );
        }
        assert!(one_line(&text(&voucher("nl"))).contains("CADEAUBON"));
        assert!(one_line(&text(&voucher("en"))).contains("BON CADEAU"));
        assert!(one_line(&text(&voucher("de"))).contains("GESCHENKGUTSCHEIN"));
    }

    #[test]
    fn the_balance_slip_masks_the_code() {
        let slip = BalanceSlip {
            establishment: "Restaurant Demo".into(),
            code_last4: "9012".into(),
            balance_cents: 3150,
            expires_on: "17.10.2027".into(),
            lang: "fr".into(),
            training: false,
        };
        let out = one_line(&render_plain_text(&format_balance_slip(
            &slip,
            PaperWidth::Mm80,
        )));
        assert!(out.contains("SOLDE DU BON"), "{out}");
        assert!(out.contains("**** **** 9012"), "{out}");
        assert!(out.contains("Solde: 31,50 EUR"), "{out}");
        assert!(!out.contains("1234 5678"), "{out}");
        assert!(out.contains("Valable jusqu'au 17.10.2027"), "{out}");
    }
}
