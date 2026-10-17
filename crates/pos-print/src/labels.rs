//! Localized text for printed tickets, reports and payment names.
//!
//! Centralizes every fixed string that must follow the ticket
//! language (FR/NL/DE - certification manual §VI). Kept in one file
//! so `receipt.rs` and `report.rs` can never disagree on wording, and
//! so a missing translation is a single compile error (exhaustive
//! `match` on `ReceiptLanguage`).
//!
//! ASCII only, like `pos_fiscal::ticket`'s spec-mandated labels: these
//! print the same whatever code page a replacement printer starts in,
//! even though the general text path (`escpos::encode_cp858`) does
//! support accented characters for user-authored content (menu items,
//! addresses, ...).

use pos_fiscal::ticket::ReceiptLanguage;

/// Fixed receipt labels per ticket language. The receipt rows follow
/// the SPF example VAT ticket of 2026-09-24 (AM of 29 April 2024 art.
/// 20); `date`, `ticket` and `total` also serve the X/Z reports.
pub(crate) struct Labels {
    pub(crate) vat_no: &'static str,
    pub(crate) est_unit: &'static str,
    pub(crate) date: &'static str,
    pub(crate) ticket: &'static str,
    pub(crate) ticket_no: &'static str,
    pub(crate) terminal: &'static str,
    pub(crate) pos_serial: &'static str,
    pub(crate) software: &'static str,
    pub(crate) device: &'static str,
    pub(crate) user: &'static str,
    pub(crate) total: &'static str,
    pub(crate) total_row: &'static str,
    pub(crate) payment_header: &'static str,
    pub(crate) rounding: &'static str,
    /// Printed where the payments would be on a sale closed unpaid.
    pub(crate) unpaid: &'static str,
    /// VAT table columns: rate, code, taxable base, VAT, gross.
    pub(crate) vat_columns: [&'static str; 5],
    pub(crate) fdm_info: &'static str,
    pub(crate) legal_name: &'static str,
    pub(crate) refund: &'static str,
    pub(crate) pre_bill: &'static str,
    pub(crate) copy: &'static str,
    pub(crate) copy_made_at: &'static str,
    pub(crate) original_fdm: &'static str,
}

pub(crate) fn labels(lang: ReceiptLanguage) -> &'static Labels {
    match lang {
        ReceiptLanguage::Fr => &Labels {
            vat_no: "TVA",
            est_unit: "Unite d'etablissement",
            date: "Date:",
            ticket: "Ticket:",
            ticket_no: "Ticketnr.:",
            terminal: "Terminal:",
            pos_serial: "POS Serial:",
            software: "Logiciel de caisse v.:",
            device: "Input device MAC:",
            user: "Utilisateur:",
            total: "TOTAL",
            total_row: "TOTAL :",
            payment_header: "Mode de paiement :",
            rounding: "Arrondi",
            unpaid: "Non paye",
            vat_columns: ["% TVA", "CODE", "HT", "TVA", "Brut"],
            fdm_info: "INFORMATIONS FDM",
            legal_name: "Denomination legale :",
            refund: "REMBOURSEMENT",
            pre_bill: "ADDITION PROVISOIRE",
            copy: "COPIE",
            copy_made_at: "Copie etablie le",
            original_fdm: "Informations FDM d'origine :",
        },
        ReceiptLanguage::Nl => &Labels {
            vat_no: "BTW",
            est_unit: "Vestigingseenheid",
            date: "Datum:",
            ticket: "Ticket:",
            ticket_no: "Ticketnr.:",
            terminal: "Terminal:",
            pos_serial: "POS Serial:",
            software: "Kassasoftware v.:",
            device: "Input device MAC:",
            user: "Gebruiker:",
            total: "TOTAAL",
            total_row: "TOTAAL :",
            payment_header: "Betaalwijze :",
            rounding: "Afronding",
            unpaid: "Niet betaald",
            vat_columns: ["% BTW", "CODE", "Excl.", "BTW", "Bruto"],
            fdm_info: "FDM-INFORMATIE",
            legal_name: "Juridische naam :",
            refund: "TERUGBETALING",
            pre_bill: "VOORLOPIGE REKENING",
            copy: "KOPIE",
            copy_made_at: "Kopie gemaakt op",
            original_fdm: "Oorspronkelijke FDM-gegevens :",
        },
        ReceiptLanguage::De => &Labels {
            vat_no: "MwSt",
            est_unit: "Niederlassungseinheit",
            date: "Datum:",
            ticket: "Bon:",
            ticket_no: "Bon-Nr.:",
            terminal: "Terminal:",
            pos_serial: "POS Serial:",
            software: "Kassensoftware v.:",
            device: "Input device MAC:",
            user: "Benutzer:",
            total: "GESAMT",
            total_row: "GESAMT :",
            payment_header: "Zahlungsart :",
            rounding: "Rundung",
            unpaid: "Nicht bezahlt",
            vat_columns: ["% MwSt", "CODE", "Netto", "MwSt", "Brutto"],
            fdm_info: "FDM-INFORMATIONEN",
            legal_name: "Rechtlicher Name :",
            refund: "RUECKERSTATTUNG",
            pre_bill: "VORLAEUFIGE RECHNUNG",
            copy: "KOPIE",
            copy_made_at: "Kopie erstellt am",
            original_fdm: "Urspruengliche FDM-Daten :",
        },
    }
}

/// Fixed X/Z (and user X/Z) report labels per ticket language.
/// Distinct from [`Labels`]: report metadata uses different words
/// than the customer-facing receipt (e.g. "Employe:" for the operator
/// who ran the report, vs. "Utilisateur:" for the operator who rang
/// up a sale). Words shared with the receipt (VAT number, date,
/// terminal, ticket number, total) are reused from [`labels`] instead
/// of duplicated here.
pub(crate) struct ReportLabels {
    /// "CHIFFRE D'AFFAIRES": the report's name after X or Z (AM of 29
    /// April 2024 art. 29).
    pub(crate) turnover: &'static str,
    pub(crate) report_no: &'static str,
    pub(crate) opening_period: &'static str,
    pub(crate) start: &'static str,
    pub(crate) end: &'static str,
    pub(crate) first: &'static str,
    pub(crate) last: &'static str,
    pub(crate) employee: &'static str,
    pub(crate) total_word: &'static str,
    pub(crate) vat_overview: &'static str,
    /// VAT table columns: rate, base, VAT, total.
    pub(crate) vat_columns: [&'static str; 4],
    pub(crate) departments: &'static str,
    pub(crate) corrections: &'static str,
    pub(crate) refunds: &'static str,
    pub(crate) drawer: &'static str,
    pub(crate) training: &'static str,
    pub(crate) training_tickets: &'static str,
    pub(crate) reductions: &'static str,
    pub(crate) line_reductions: &'static str,
    pub(crate) event_reductions: &'static str,
    pub(crate) invoices_title: &'static str,
    pub(crate) invoice_no: &'static str,
    pub(crate) invoices: &'static str,
    pub(crate) payments_title: &'static str,
    pub(crate) payments_total: &'static str,
    pub(crate) roundings: &'static str,
    /// Payment method corrections (count, amount moved).
    pub(crate) payment_corrections: &'static str,
    /// User report (M182 / M183).
    pub(crate) user_report_prefix: &'static str,
    pub(crate) first_event: &'static str,
    pub(crate) last_event: &'static str,
    pub(crate) punch_in: &'static str,
    pub(crate) punch_out: &'static str,
}

pub(crate) fn report_labels(lang: ReceiptLanguage) -> &'static ReportLabels {
    match lang {
        ReceiptLanguage::Fr => &ReportLabels {
            turnover: "CHIFFRE D'AFFAIRES",
            report_no: "Rapport nr.",
            opening_period: "Periode d'ouverture",
            start: "Debut",
            end: "Fin",
            first: "Premier",
            last: "Dernier",
            employee: "Employe:",
            total_word: "Total",
            vat_overview: "Apercu TVA",
            vat_columns: ["Taux", "HT", "TVA", "TTC"],
            departments: "Departements du chiffre d'affaires",
            corrections: "Corrections",
            refunds: "Remboursements",
            drawer: "Tiroir-caisse",
            training: "Formation",
            training_tickets: "Tickets de formation",
            reductions: "Reductions",
            line_reductions: "Reductions de lignes",
            event_reductions: "Reductions d'evenements",
            invoices_title: "FACTURE",
            invoice_no: "Facture nr.",
            invoices: "Factures",
            payments_title: "PAIEMENTS ENREGISTRES",
            payments_total: "Total des paiements",
            roundings: "Arrondis",
            payment_corrections: "Corrections de paiement",
            user_report_prefix: "RAPPORT UTIL.",
            first_event: "Premier event:",
            last_event: "Dernier event:",
            punch_in: "Pointage IN:",
            punch_out: "Pointage OUT:",
        },
        ReceiptLanguage::Nl => &ReportLabels {
            turnover: "OMZET",
            report_no: "Rapport nr.",
            opening_period: "Openingsperiode",
            start: "Begin",
            end: "Einde",
            first: "Eerste",
            last: "Laatste",
            employee: "Werknemer:",
            total_word: "Totaal",
            vat_overview: "BTW-overzicht",
            vat_columns: ["Tarief", "Excl.", "BTW", "Incl."],
            departments: "Omzet per afdeling",
            corrections: "Correcties",
            refunds: "Terugbetalingen",
            drawer: "Kassalade",
            training: "Opleiding",
            training_tickets: "Opleidingstickets",
            reductions: "Kortingen",
            line_reductions: "Lijnkortingen",
            event_reductions: "Ticketkortingen",
            invoices_title: "FACTUUR",
            invoice_no: "Factuur nr.",
            invoices: "Facturen",
            payments_title: "GEREGISTREERDE BETALINGEN",
            payments_total: "Totaal betalingen",
            roundings: "Afrondingen",
            payment_corrections: "Betalingscorrecties",
            user_report_prefix: "RAPPORT GEBR.",
            first_event: "Eerste event:",
            last_event: "Laatste event:",
            punch_in: "Klok IN:",
            punch_out: "Klok OUT:",
        },
        ReceiptLanguage::De => &ReportLabels {
            turnover: "UMSATZ",
            report_no: "Bericht Nr.",
            opening_period: "Oeffnungszeitraum",
            start: "Beginn",
            end: "Ende",
            first: "Erstes",
            last: "Letztes",
            employee: "Mitarbeiter:",
            total_word: "Gesamt",
            vat_overview: "MwSt-Uebersicht",
            vat_columns: ["Satz", "Netto", "MwSt", "Brutto"],
            departments: "Umsatz nach Abteilung",
            corrections: "Korrekturen",
            refunds: "Rueckerstattungen",
            drawer: "Kassenschublade",
            training: "Schulung",
            training_tickets: "Schulungsbons",
            reductions: "Rabatte",
            line_reductions: "Zeilenrabatte",
            event_reductions: "Bonrabatte",
            invoices_title: "RECHNUNG",
            invoice_no: "Rechnung Nr.",
            invoices: "Rechnungen",
            payments_title: "ERFASSTE ZAHLUNGEN",
            payments_total: "Summe Zahlungen",
            roundings: "Rundungen",
            payment_corrections: "Zahlungskorrekturen",
            user_report_prefix: "BERICHT BENUTZER",
            first_event: "Erstes Event:",
            last_event: "Letztes Event:",
            punch_in: "Zeit IN:",
            punch_out: "Zeit OUT:",
        },
    }
}

/// Labels of the bar / kitchen order slip (`bar_ticket.rs`).
pub(crate) struct BarLabels {
    pub(crate) title: &'static str,
    pub(crate) date: &'static str,
    pub(crate) operator: &'static str,
    pub(crate) table: &'static str,
    pub(crate) direct_sale: &'static str,
    pub(crate) items_header: &'static str,
    /// Title of the table transfer notice.
    pub(crate) transfer_title: &'static str,
    /// Title of the kitchen note change slip.
    pub(crate) note_title: &'static str,
    /// Label before a line's kitchen note on the note slip.
    pub(crate) note: &'static str,
    /// Printed when a kitchen note was removed.
    pub(crate) note_removed: &'static str,
}

pub(crate) fn bar_labels(lang: ReceiptLanguage) -> &'static BarLabels {
    match lang {
        ReceiptLanguage::Fr => &BarLabels {
            title: "BON DE COMMANDE",
            date: "Date:",
            operator: "Operateur:",
            table: "Table",
            direct_sale: "Vente directe",
            items_header: "Qte   Article",
            transfer_title: "TRANSFERT DE TABLE",
            note_title: "MODIFICATION",
            note: "Note :",
            note_removed: "Note supprimee",
        },
        ReceiptLanguage::Nl => &BarLabels {
            title: "BESTELBON",
            date: "Datum:",
            operator: "Bediende:",
            table: "Tafel",
            direct_sale: "Directe verkoop",
            items_header: "Aantal Artikel",
            transfer_title: "TAFELVERPLAATSING",
            note_title: "WIJZIGING",
            note: "Opmerking:",
            note_removed: "Opmerking verwijderd",
        },
        ReceiptLanguage::De => &BarLabels {
            title: "BESTELLSCHEIN",
            date: "Datum:",
            operator: "Bediener:",
            table: "Tisch",
            direct_sale: "Direktverkauf",
            items_header: "Menge  Artikel",
            transfer_title: "TISCHWECHSEL",
            note_title: "AENDERUNG",
            note: "Hinweis:",
            note_removed: "Hinweis geloescht",
        },
    }
}

/// Localized name of an FDM event-label letter (N/P/T/C/I/F/S/R), for
/// the report's per-type transaction breakdown. Unknown letters print
/// unchanged: defensive, the report must never fail to print over an
/// unrecognized label.
pub(crate) fn event_label_name(label: &str, lang: ReceiptLanguage) -> &str {
    use ReceiptLanguage::{De, Fr, Nl};
    match (label, lang) {
        ("N", Fr) => "Ventes",
        ("N", Nl) => "Verkoop",
        ("N", De) => "Umsaetze",
        // "Pro forma" is the literal mention used by the spec in all
        // three language versions (see `ticket::ticket_type_label`).
        ("P", _) => "Pro forma",
        ("T", Fr) => "Formation",
        ("T", Nl) => "Opleiding",
        ("T", De) => "Schulung",
        ("C", Fr) => "Copies",
        ("C", Nl) => "Kopieen",
        ("C", De) => "Kopien",
        ("I", Fr) => "Factures",
        ("I", Nl) => "Facturen",
        ("I", De) => "Rechnungen",
        ("F", Fr) => "Financier",
        ("F", Nl) => "Financieel",
        ("F", De) => "Finanzbewegungen",
        ("S", Fr) => "Social",
        ("S", Nl) => "Sociaal",
        ("S", De) => "Sozial",
        ("R", Fr) => "Rapports",
        ("R", Nl) => "Rapporten",
        ("R", De) => "Berichte",
        _ => label,
    }
}

/// A payment method kind the POS actually uses today, recognized from
/// a printed name string so a receipt or report row can be given a
/// localized label. Distinct from `pos_fiscal::types::PaymentType`
/// (the full FDM wire enum): this stays a private implementation
/// detail of [`payment_label`].
#[derive(Clone, Copy, PartialEq, Eq)]
enum PaymentKind {
    Cash,
    Card,
    MealVoucher,
    /// Paid in an ordering platform's app (online orders).
    App,
    /// Paid online on the restaurant's webshop (online orders).
    Online,
    Other,
}

/// Recognize a payment kind from a raw method/name string: the FDM
/// `financials.name` sent on the wire ("Cash"/"Card"/"Meal voucher"/
/// "Payment"), or its `type` enum wire value ("CASH"/"CARD_DEBIT"/
/// "CHEQUE_MEAL"/...) if that ever
/// ends up in the string instead. Matching is case-insensitive and by
/// substring, so any casing or card sub-type variant is recognized.
fn payment_kind_from_name(name: &str) -> Option<PaymentKind> {
    let lower = name.to_ascii_lowercase();
    if lower.contains("meal") || lower.contains("repas") {
        Some(PaymentKind::MealVoucher)
    } else if lower.contains("cash") {
        Some(PaymentKind::Cash)
    } else if lower.contains("card") {
        Some(PaymentKind::Card)
    } else if lower == "app" {
        Some(PaymentKind::App)
    } else if lower == "online" {
        Some(PaymentKind::Online)
    } else if lower == "other" || lower == "payment" {
        Some(PaymentKind::Other)
    } else {
        None
    }
}

fn payment_kind_label(kind: PaymentKind, lang: ReceiptLanguage) -> &'static str {
    use PaymentKind::{App, Card, Cash, MealVoucher, Online, Other};
    use ReceiptLanguage::{De, Fr, Nl};
    match (kind, lang) {
        (Cash, Fr) => "Especes",
        (Cash, Nl) => "Contant",
        (Cash, De) => "Bar",
        (Card, Fr) => "Carte",
        (Card, Nl) => "Kaart",
        (Card, De) => "Karte",
        (MealVoucher, Fr) => "Cheques-repas",
        (MealVoucher, Nl) => "Maaltijdcheques",
        (MealVoucher, De) => "Essensgutscheine",
        (App, Fr) => "Application",
        (App, Nl) => "App",
        (App, De) => "App",
        (Online, Fr) => "Paiement en ligne",
        (Online, Nl) => "Online betaling",
        (Online, De) => "Online-Zahlung",
        (Other, Fr) => "Autre",
        (Other, Nl) => "Overige",
        (Other, De) => "Sonstige",
    }
}

/// Localized name of a payment method, for printing on a receipt or
/// report. `name` is the raw method string already carried by the
/// row - the FDM `financials.name` sent on the wire ("Cash"/"Card"/
/// "Payment", see `pos_fiscal::event::build_sale_input`), or, for a
/// report built from archived JSON, whatever name was archived at the
/// time. Unrecognized values print unchanged.
///
/// This never changes what is sent to the FDM: `financials.name`
/// stays exactly as built today. It only chooses what gets printed.
pub(crate) fn payment_label(name: &str, lang: ReceiptLanguage) -> String {
    match payment_kind_from_name(name) {
        Some(kind) => payment_kind_label(kind, lang).to_string(),
        None => name.to_string(),
    }
}

#[cfg(test)]
mod online_payment_label_tests {
    use super::*;

    #[test]
    fn app_and_online_payments_have_printed_names() {
        assert_eq!(payment_label("App", ReceiptLanguage::Fr), "Application");
        assert_eq!(
            payment_label("Online", ReceiptLanguage::Fr),
            "Paiement en ligne"
        );
        assert_eq!(
            payment_label("Online", ReceiptLanguage::Nl),
            "Online betaling"
        );
        assert_eq!(
            payment_label("Online", ReceiptLanguage::De),
            "Online-Zahlung"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── payment_label ────────────────────────────────────────────

    #[test]
    fn recognizes_the_fdm_wire_names_case_insensitively() {
        assert_eq!(payment_label("Cash", ReceiptLanguage::Fr), "Especes");
        assert_eq!(payment_label("CASH", ReceiptLanguage::Nl), "Contant");
        assert_eq!(payment_label("cash", ReceiptLanguage::De), "Bar");
        assert_eq!(payment_label("Card", ReceiptLanguage::Fr), "Carte");
        assert_eq!(payment_label("CARD", ReceiptLanguage::Nl), "Kaart");
        assert_eq!(payment_label("card", ReceiptLanguage::De), "Karte");
    }

    #[test]
    fn recognizes_a_card_sub_type_wire_value() {
        // The PaymentType wire enum has sub-types (CARD_DEBIT, ...):
        // any string containing "card" is still recognized.
        assert_eq!(payment_label("CARD_DEBIT", ReceiptLanguage::Fr), "Carte");
        assert_eq!(payment_label("CardCredit", ReceiptLanguage::Nl), "Kaart");
    }

    #[test]
    fn recognizes_the_generic_other_fallback() {
        assert_eq!(payment_label("Payment", ReceiptLanguage::Fr), "Autre");
        assert_eq!(payment_label("other", ReceiptLanguage::Nl), "Overige");
        assert_eq!(payment_label("OTHER", ReceiptLanguage::De), "Sonstige");
    }

    #[test]
    fn falls_back_to_the_raw_name_when_unrecognized() {
        assert_eq!(payment_label("Voucher", ReceiptLanguage::Nl), "Voucher");
        assert_eq!(payment_label("-", ReceiptLanguage::De), "-");
        assert_eq!(payment_label("Espèces", ReceiptLanguage::Fr), "Espèces");
    }

    #[test]
    fn recognizes_meal_vouchers() {
        assert_eq!(
            payment_label("Meal voucher", ReceiptLanguage::Fr),
            "Cheques-repas"
        );
        assert_eq!(
            payment_label("CHEQUE_MEAL", ReceiptLanguage::Nl),
            "Maaltijdcheques"
        );
        assert_eq!(
            payment_label("Meal voucher", ReceiptLanguage::De),
            "Essensgutscheine"
        );
    }

    #[test]
    fn every_language_has_a_distinct_cash_and_card_label() {
        for lang in [
            ReceiptLanguage::Fr,
            ReceiptLanguage::Nl,
            ReceiptLanguage::De,
        ] {
            let cash = payment_label("Cash", lang);
            let card = payment_label("Card", lang);
            let meal = payment_label("Meal voucher", lang);
            assert_ne!(cash, card, "{lang:?}");
            assert_ne!(meal, card, "{lang:?}");
            assert_ne!(meal, cash, "{lang:?}");
            for label in [&cash, &card, &meal] {
                assert!(label.is_ascii(), "{lang:?}: {label:?}");
            }
        }
    }

    // ── event_label_name ─────────────────────────────────────────

    #[test]
    fn every_event_label_is_translated_in_every_language() {
        for label in ["N", "P", "T", "C", "I", "F", "S", "R"] {
            for lang in [
                ReceiptLanguage::Fr,
                ReceiptLanguage::Nl,
                ReceiptLanguage::De,
            ] {
                let name = event_label_name(label, lang);
                assert!(name.is_ascii(), "{label}/{lang:?}: {name:?}");
                assert_ne!(name, label, "{label}/{lang:?} did not translate");
            }
        }
    }

    #[test]
    fn unknown_event_label_prints_unchanged() {
        assert_eq!(event_label_name("Z", ReceiptLanguage::Fr), "Z");
    }
}
