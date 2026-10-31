//! The allergen sheet (V12-D21): per product, what it contains and what
//! it may contain as traces, from the 14 substances of Regulation (EU)
//! No 1169/2011, Annex II. One product or the whole menu.
//!
//! A non-fiscal print: it shows no price and carries the mention that
//! it is not a valid VAT ticket (the same words as the bar slip) and
//! the regulation footer. A product whose list is not declared reads
//! "information not available", never "no allergen".

use pos_fiscal::ticket::{ReceiptLanguage, invalid_disclaimer};

use crate::escpos::{Alignment, EscPosBuilder, PaperWidth, TextStyle, wrap};

/// An option of a product with its own allergen statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetOption {
    pub name: String,
    /// The option's list is declared (possibly empty).
    pub declared: bool,
    /// Allergen names, already in the sheet language, Annex order.
    pub contains: Vec<String>,
    pub traces: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetProduct {
    pub name: String,
    pub declared: bool,
    pub contains: Vec<String>,
    pub traces: Vec<String>,
    pub options: Vec<SheetOption>,
}

#[derive(Debug, Clone)]
pub struct AllergenSheet {
    /// Establishment name (header).
    pub establishment: String,
    /// Local date, already formatted ("17/10/2026").
    pub date: String,
    /// `fr`, `nl`, `en` or `de`; anything else prints in French.
    pub lang: String,
    pub products: Vec<SheetProduct>,
}

struct Words {
    title: &'static str,
    contains: &'static str,
    traces: &'static str,
    none: &'static str,
    unknown: &'static str,
    options: &'static str,
    regulation: &'static str,
}

fn words(lang: &str) -> &'static Words {
    match lang {
        "nl" => &Words {
            title: "ALLERGENEN",
            contains: "Bevat",
            traces: "Mogelijke sporen",
            none: "Geen allergenen",
            unknown: "Informatie niet beschikbaar, vraag het aan de verantwoordelijke",
            options: "Opties",
            regulation: "Volgens verordening (EU) nr. 1169/2011, bijlage II. Informatie verstrekt door de exploitant.",
        },
        "en" => &Words {
            title: "ALLERGENS",
            contains: "Contains",
            traces: "May contain traces of",
            none: "No allergen",
            unknown: "Information not available, ask the manager",
            options: "Options",
            regulation: "According to Regulation (EU) No 1169/2011, Annex II. Information provided by the operator.",
        },
        "de" => &Words {
            title: "ALLERGENE",
            contains: "Enthaelt",
            traces: "Kann Spuren enthalten von",
            none: "Keine Allergene",
            unknown: "Information nicht verfuegbar, bitte den Verantwortlichen fragen",
            options: "Optionen",
            regulation: "Gemaess Verordnung (EU) Nr. 1169/2011, Anhang II. Angaben des Betreibers.",
        },
        _ => &Words {
            title: "ALLERGÈNES",
            contains: "Contient",
            traces: "Traces possibles",
            none: "Aucun allergène",
            unknown: "Information non disponible, demandez au responsable",
            options: "Options",
            regulation: "Selon le règlement (UE) n° 1169/2011, annexe II. Informations fournies par l'exploitant.",
        },
    }
}

/// The ticket language of the "not a VAT ticket" mention. English has
/// no national-language ticket: the French mention applies, with its
/// English reading below.
fn disclaimer(lang: &str) -> Vec<&'static str> {
    match lang {
        "nl" => vec![invalid_disclaimer(ReceiptLanguage::Nl)],
        "de" => vec![invalid_disclaimer(ReceiptLanguage::De)],
        "en" => vec![
            invalid_disclaimer(ReceiptLanguage::Fr),
            "THIS IS NOT A VALID VAT RECEIPT",
        ],
        _ => vec![invalid_disclaimer(ReceiptLanguage::Fr)],
    }
}

fn bold(p: &mut EscPosBuilder, text: &str) {
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln_wrapped(text);
    p.style_reset();
}

/// `text` wrapped to the paper, every line after `indent`.
fn indented(p: &mut EscPosBuilder, indent: &str, text: &str) {
    let width = usize::from(p.chars_per_line()).saturating_sub(indent.chars().count());
    for line in wrap(text, width) {
        p.textln(&format!("{indent}{line}"));
    }
}

fn statement(
    p: &mut EscPosBuilder,
    w: &Words,
    declared: Option<(&[String], &[String])>,
    indent: &str,
) {
    // `None`: no declaration, which is not the same as "no allergen".
    let Some((contains, traces)) = declared else {
        indented(p, indent, w.unknown);
        return;
    };
    if contains.is_empty() && traces.is_empty() {
        indented(p, indent, w.none);
        return;
    }
    if !contains.is_empty() {
        indented(
            p,
            indent,
            &format!("{}: {}", w.contains, contains.join(", ")),
        );
    }
    if !traces.is_empty() {
        indented(p, indent, &format!("{}: {}", w.traces, traces.join(", ")));
    }
}

/// ESC/POS bytes of the sheet, ready for a `TcpPrinter`.
pub fn format_allergen_sheet(sheet: &AllergenSheet, paper: PaperWidth) -> Vec<u8> {
    let w = words(&sheet.lang);
    let mut p = EscPosBuilder::for_paper(paper);

    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.textln(w.title);
    p.style_reset();
    if !sheet.establishment.trim().is_empty() {
        p.textln_wrapped(&sheet.establishment);
    }
    p.textln(&sheet.date);
    p.separator('=');

    p.align(Alignment::Left);
    for product in &sheet.products {
        bold(&mut p, &product.name);
        statement(
            &mut p,
            w,
            product
                .declared
                .then_some((&product.contains[..], &product.traces[..])),
            "  ",
        );
        if !product.options.is_empty() {
            p.textln(&format!("  {}:", w.options));
            for option in &product.options {
                indented(&mut p, "  - ", &option.name);
                statement(
                    &mut p,
                    w,
                    option
                        .declared
                        .then_some((&option.contains[..], &option.traces[..])),
                    "      ",
                );
            }
        }
        p.separator('-');
    }

    p.align(Alignment::Center);
    p.textln_wrapped(w.regulation);
    for line in disclaimer(&sheet.lang) {
        p.textln_wrapped(line);
    }
    p.feed(4);
    p.cut();
    p.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::render_plain_text;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    fn sheet(lang: &str) -> AllergenSheet {
        AllergenSheet {
            establishment: "Restaurant Demo".into(),
            date: "17/10/2026".into(),
            lang: lang.into(),
            products: vec![
                SheetProduct {
                    name: "Maki saumon".into(),
                    declared: true,
                    contains: names(&["Poissons", "Sésame"]),
                    traces: names(&["Mollusques"]),
                    options: vec![
                        SheetOption {
                            name: "Sauce soja".into(),
                            declared: true,
                            contains: names(&["Soja", "Céréales contenant du gluten"]),
                            traces: vec![],
                        },
                        SheetOption {
                            name: "Wasabi".into(),
                            declared: false,
                            contains: vec![],
                            traces: vec![],
                        },
                    ],
                },
                SheetProduct {
                    name: "Riz nature".into(),
                    declared: true,
                    contains: vec![],
                    traces: vec![],
                    options: vec![],
                },
                SheetProduct {
                    name: "Dessert du jour".into(),
                    declared: false,
                    contains: vec![],
                    traces: vec![],
                    options: vec![],
                },
            ],
        }
    }

    fn text(s: &AllergenSheet) -> String {
        render_plain_text(&format_allergen_sheet(s, PaperWidth::Mm80))
    }

    #[test]
    fn the_french_sheet_golden() {
        let out = text(&sheet("fr"));
        let body = out.split("VALABLE").next().unwrap();
        let expected = "\
\x20                  ALLERGÈNES
                Restaurant Demo
                   17/10/2026
================================================
Maki saumon
  Contient: Poissons, Sésame
  Traces possibles: Mollusques
  Options:
  - Sauce soja
      Contient: Soja, Céréales contenant du
      gluten
  - Wasabi
      Information non disponible, demandez au
      responsable
------------------------------------------------
Riz nature
  Aucun allergène
------------------------------------------------
Dessert du jour
  Information non disponible, demandez au
  responsable
------------------------------------------------
Selon le règlement (UE) n° 1169/2011, annexe II.
    Informations fournies par l'exploitant.
   CECI N'EST PAS UN TICKET DE CAISSE T.V.A.
                    ";
        assert_eq!(body, expected);
    }

    #[test]
    fn a_product_without_information_is_never_no_allergen() {
        let out = crate::render::one_line(&text(&sheet("fr")));
        let at = out.find("Dessert du jour").unwrap();
        let tail = &out[at..];
        assert!(tail.contains("Information non disponible"), "{tail}");
        let none_at = out.find("Riz nature").unwrap();
        assert!(out[none_at..at].contains("Aucun all"), "{out}");
    }

    #[test]
    fn the_sheet_has_both_footers_and_no_price() {
        for lang in ["fr", "nl", "en", "de"] {
            let out = text(&sheet(lang));
            let one = crate::render::one_line(&out);
            assert!(!out.contains('€') && !out.contains("EUR"), "{lang}: {out}");
            assert!(
                !out.contains(",00") && !out.contains(".00"),
                "{lang}: {out}"
            );
            assert!(one.contains("1169/2011"), "{lang}: {one}");
            assert!(
                one.contains("annex")
                    || one.contains("Annex")
                    || one.contains("annexe")
                    || one.contains("bijlage")
                    || one.contains("Anhang"),
                "{lang}: {one}"
            );
        }
        let fr = crate::render::one_line(&text(&sheet("fr")));
        assert!(
            fr.contains("CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE"),
            "{fr}"
        );
        assert!(
            fr.contains("Informations fournies par l'exploitant"),
            "{fr}"
        );
        let nl = crate::render::one_line(&text(&sheet("nl")));
        assert!(nl.contains("DIT IS GEEN GELDIG BTW-KASTICKET"), "{nl}");
        let de = crate::render::one_line(&text(&sheet("de")));
        assert!(
            de.contains("DIES IST KEIN GUELTIGER MWST.-KASSENZETTEL"),
            "{de}"
        );
        let en = crate::render::one_line(&text(&sheet("en")));
        assert!(en.contains("THIS IS NOT A VALID VAT RECEIPT"), "{en}");
        assert!(en.contains("May contain traces of"), "{en}");
    }

    #[test]
    fn the_title_follows_the_language_and_unknown_languages_print_french() {
        assert!(text(&sheet("nl")).contains("ALLERGENEN"));
        assert!(text(&sheet("en")).contains("ALLERGENS"));
        assert!(text(&sheet("de")).contains("ALLERGENE"));
        assert!(text(&sheet("zh")).contains("ALLERG"));
        assert!(text(&sheet("zh")).contains("Contient"));
    }

    #[test]
    fn every_language_fits_58_mm_paper() {
        for lang in ["fr", "nl", "en", "de"] {
            crate::render::assert_58_mm_prints_the_80_mm_words(|paper| {
                format_allergen_sheet(&sheet(lang), paper)
            });
        }
    }

    #[test]
    fn lines_fit_the_paper() {
        let mut s = sheet("fr");
        s.products[0].name =
            "Un produit au nom vraiment tres long qui depasse la largeur du papier".into();
        for line in text(&s).lines() {
            assert!(line.chars().count() <= 48, "too wide: {line:?}");
        }
    }

    #[test]
    fn the_sheet_ends_with_a_cut() {
        let bytes = format_allergen_sheet(&sheet("fr"), PaperWidth::Mm80);
        assert!(bytes.windows(3).any(|w| w == [0x1D, 0x56, 0x42]
            || w == [0x1D, 0x56, 0x41]
            || w[0] == 0x1D && w[1] == 0x56));
    }
}
