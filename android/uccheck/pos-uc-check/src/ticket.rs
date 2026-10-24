//! Ticket text normalisation for the `tickets` check.
//!
//! Two honest runs print the same tickets except for what comes from the
//! clock, from generated identifiers, from the FDM's signature arithmetic
//! or from the installation's identity. Those values are masked; every
//! other character, the spacing included, is compared. A masked value
//! keeps its length (digits and hex digits become `#`, separators stay),
//! so a column that moved still shows. An amount, a quantity, a label or
//! a counter is never masked: a difference there is a real difference
//! (P7D8). The same rules are documented in `docs/android/UC.md`.

use regex::{Captures, Regex};

/// What a rule masks and why.
pub struct Rule {
    pub name: &'static str,
    pub reason: &'static str,
}

/// The rules, in the order they are applied. Documentation of
/// [`normalise_ticket`]: the code below applies exactly these.
pub const RULES: &[Rule] = &[
    Rule {
        name: "iso date and time",
        reason: "2026-10-09T23:36:36Z, 2026-10-10 01:36: the clock of the run",
    },
    Rule {
        name: "day/month/year and time",
        reason: "10/10/26 01:36:36 on the report header: the clock of the run",
    },
    Rule {
        name: "iso date",
        reason: "2026-10-09, the booking date: the day of the run",
    },
    Rule {
        name: "day.month.year",
        reason: "10.10.2026 on a ticket header: the day of the run",
    },
    Rule {
        name: "time",
        reason: "01:36 or 01:36:36: the clock of the run",
    },
    Rule {
        name: "uuid",
        reason: "BookingPeriodId: generated when a booking period opens",
    },
    Rule {
        name: "signature",
        reason: "32 or more hex digits: the short signature, derived by the FDM from the clock",
    },
    Rule {
        name: "qr payload",
        reason: "[QR: ...]: the verification URL the FDM signed, which carries the signature",
    },
    Rule {
        name: "software version line",
        reason: "Logiciel de caisse v.: pili-pos/x.y.z against pili-pos-android/x.y.z",
    },
    Rule {
        name: "vat table row order",
        reason: "the rows of the VAT table of a sale ticket follow the order of the FDM's vatCalc, and pos-fdm-mock builds it from a hash map: its order changes from run to run (a real FDM answers in a fixed order)",
    },
    Rule {
        name: "device and terminal lines",
        reason: "Input device MAC, Terminal, POS Serial, POSID, TerminalId: the installation's identity, set by each side's provisioning",
    },
];

struct Compiled {
    iso_datetime: Regex,
    dmy_datetime: Regex,
    iso_date: Regex,
    dmy_date: Regex,
    time: Regex,
    uuid: Regex,
    signature: Regex,
    qr: Regex,
    version_line: Regex,
    identity_label: Regex,
    identity_spaced: Regex,
    vat_row: Regex,
}

fn compiled() -> Compiled {
    let re = |p: &str| Regex::new(p).expect("ticket normalisation pattern");
    Compiled {
        iso_datetime: re(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}(:\d{2})?(\.\d+)?(Z|[+-]\d{2}:\d{2})?"),
        dmy_datetime: re(r"\d{2}[./]\d{2}[./]\d{2,4} \d{2}:\d{2}(:\d{2})?"),
        iso_date: re(r"\d{4}-\d{2}-\d{2}"),
        dmy_date: re(r"\d{2}[./]\d{2}[./]\d{2,4}"),
        time: re(r"\b\d{2}:\d{2}(:\d{2})?\b"),
        uuid: re(
            r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b",
        ),
        signature: re(r"\b[0-9a-fA-F]{32,}\b"),
        qr: re(r"\[QR: [^\]]*\]"),
        version_line: re(r"(?m)^(Logiciel de caisse v\.:).*$"),
        identity_label: re(r"(?m)^(Input device MAC:|Terminal:|POS Serial:).*$"),
        identity_spaced: re(r"(?m)^(POSID|TerminalId)[ \t]+\S+[ \t]*$"),
        vat_row: re(r"^\s*\d+,\d{2}%\s+[A-Z]\s+\S+\s+\S+\s+\S+\s*$"),
    }
}

/// The match with every digit and hex digit replaced by `#` (separators
/// and letters of a date, such as the `T` and `Z` of an ISO stamp, stay).
fn mask(caps: &Captures) -> String {
    caps[0]
        .chars()
        .map(|c| if c.is_ascii_hexdigit() { '#' } else { c })
        .collect()
}

/// A date or a time: only the digits are masked.
fn mask_digits(caps: &Captures) -> String {
    caps[0]
        .chars()
        .map(|c| if c.is_ascii_digit() { '#' } else { c })
        .collect()
}

/// The ticket with the volatile values masked.
pub fn normalise_ticket(ticket: &str) -> String {
    let c = compiled();
    let text = ticket.replace("\r\n", "\n");
    let text = c.iso_datetime.replace_all(&text, mask_digits).into_owned();
    let text = c.dmy_datetime.replace_all(&text, mask_digits).into_owned();
    let text = c.iso_date.replace_all(&text, mask_digits).into_owned();
    let text = c.dmy_date.replace_all(&text, mask_digits).into_owned();
    let text = c.time.replace_all(&text, mask_digits).into_owned();
    let text = c.uuid.replace_all(&text, mask).into_owned();
    let text = c.signature.replace_all(&text, mask).into_owned();
    let text = c.qr.replace_all(&text, "[QR: <qr>]").into_owned();
    let text = c
        .version_line
        .replace_all(&text, "$1 <version>")
        .into_owned();
    let text = c.identity_label.replace_all(&text, "$1 <id>").into_owned();
    let text = c.identity_spaced.replace_all(&text, "$1 <id>").into_owned();
    sort_vat_rows(&text, &c.vat_row)
}

/// Each run of consecutive VAT table rows (`21,00%  A  19,83  4,17  24,00`)
/// sorted, whatever order the FDM answered in.
fn sort_vat_rows(text: &str, vat_row: &Regex) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut run: Vec<&str> = Vec::new();
    for line in text.split('\n') {
        if vat_row.is_match(line) {
            run.push(line);
            continue;
        }
        run.sort_unstable();
        out.append(&mut run);
        out.push(line);
    }
    run.sort_unstable();
    out.append(&mut run);
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SALE: &str = "            TICKET DE CAISSE T.V.A.
Restaurant Test
10.10.2026                                 01:36
Utilisateur: Gerant Test
Terminal: POS-2-ANDROID
Ticketnr.: 000001
POS Serial: CPOS0031234567
Input device MAC: pos-json-device
Logiciel de caisse v.: pili-pos/1.1.0
2 Perrier                                5,00  C
TOTAL :                                    €5,00
fdmDateTime: 2026-10-09T23:36:36Z;
Event: N; eventCounter: 1;
totalCounter: 2; shortSignature:
cb06a62957e835291f22118a8afad7651413e291;
 [QR: https://www.gs2.be/M0000100002AbCdEfGhI]
";

    #[test]
    fn two_runs_of_one_sale_normalise_equal() {
        let other = SALE
            .replace(
                "10.10.2026                                 01:36",
                "11.10.2026                                 17:59",
            )
            .replace("2026-10-09T23:36:36Z", "2026-10-11T15:59:01Z")
            .replace(
                "cb06a62957e835291f22118a8afad7651413e291",
                "0123456789abcdef0123456789abcdef01234567",
            )
            .replace("pili-pos/1.1.0", "pili-pos-android/1.0.0-dev")
            .replace("pos-json-device", "9f8e7d6c5b4a3210")
            .replace("M0000100002AbCdEfGhI", "M0000100002ZzZzZzZzZ");
        assert_ne!(SALE, other);
        assert_eq!(normalise_ticket(SALE), normalise_ticket(&other));
    }

    #[test]
    fn a_report_header_and_a_booking_period_are_masked() {
        let report = "RAPPORT X                      10/10/26 01:36:36
Date:                                 2026-10-09
BookingPeriodId:
0ee25b8f-b3d1-4f48-8f9b-ae2c5664b93b
Debut                           2026-10-10 01:36
Premier                       1 2026-10-09 23:36
POSID                             CPOS0031234567
TerminalId                         POS-2-ANDROID
";
        let normal = normalise_ticket(report);
        assert!(
            normal.contains("RAPPORT X                      ##/##/## ##:##:##"),
            "{normal}"
        );
        assert!(!normal.contains("0ee25b8f"), "{normal}");
        assert!(
            normal.contains("########-####-####-####-############"),
            "{normal}"
        );
        assert!(
            normal.contains("Premier                       1 ####-##-## ##:##"),
            "{normal}"
        );
        assert!(
            normal.contains("POSID <id>") && normal.contains("TerminalId <id>"),
            "{normal}"
        );
    }

    #[test]
    fn an_amount_a_quantity_a_counter_and_the_spacing_stay_compared() {
        let base = normalise_ticket(SALE);
        for changed in [
            SALE.replace("5,00  C", "5,50  C"),
            SALE.replace("2 Perrier", "3 Perrier"),
            SALE.replace("eventCounter: 1", "eventCounter: 2"),
            SALE.replace("totalCounter: 2", "totalCounter: 3"),
            SALE.replace("TOTAL :   ", "TOTAL : "),
            SALE.replace("Ticketnr.: 000001", "Ticketnr.: 000002"),
        ] {
            assert_ne!(base, normalise_ticket(&changed), "{changed}");
        }
    }

    #[test]
    fn a_masked_value_keeps_its_length_so_a_moved_column_shows() {
        let a = "10.10.2026   01:36";
        let b = "10.10.2026  01:36";
        assert_ne!(normalise_ticket(a), normalise_ticket(b));
        assert_eq!(normalise_ticket(a).len(), a.len());
    }

    #[test]
    fn the_rows_of_the_vat_table_are_compared_in_any_order() {
        let a = "% TVA   CODE          HT       TVA      Brut
12,00%  B          25,00      3,00     28,00
21,00%  A          19,83      4,17     24,00
-----
";
        let b = "% TVA   CODE          HT       TVA      Brut
21,00%  A          19,83      4,17     24,00
12,00%  B          25,00      3,00     28,00
-----
";
        assert_eq!(normalise_ticket(a), normalise_ticket(b));
        // A changed amount in a row still shows.
        let c = b.replace("19,83", "19,84");
        assert_ne!(normalise_ticket(a), normalise_ticket(&c));
        // Rows are not moved out of their table: a line before the header stays before it.
        let d = "TOTAL\n21,00%  A          19,83      4,17     24,00\nTAIL";
        assert_eq!(normalise_ticket(d), d);
    }

    #[test]
    fn every_rule_is_documented_with_a_reason() {
        for rule in RULES {
            assert!(!rule.name.is_empty() && rule.reason.len() > 10);
        }
        assert_eq!(RULES.len(), 11);
    }
}
