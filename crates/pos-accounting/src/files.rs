//! The detail files, one function each: tables of integer cents built from the days.
//!
//! Every amount column is followed by a `_centimes` column holding the same amount as
//! an integer number of cents, so a spreadsheet or a script never has to parse a
//! decimal mark.

use std::collections::BTreeMap;

use crate::aggregate::{Amounts, Day, Kind, Method};
use crate::csv::{Decimal, Table, amount, text};
use crate::parse::{LABELS, Sale};

/// Names of the files of an export, in the order of the manifest listing.
pub const FILE_NAMES: [&str; 8] = [
    "jours.csv",
    "tva.csv",
    "paiements.csv",
    "remboursements.csv",
    "factures.csv",
    "bons_cadeaux.csv",
    "mouvements_caisse.csv",
    "rapports_z.csv",
];

fn push_amount(row: &mut Vec<String>, cents: i64, dec: Decimal) {
    row.push(amount(cents, dec));
    row.push(cents.to_string());
}

/// `2026-10-04T12:30:00.000+02:00` as `2026-10-04 12:30:00`.
fn local_time(pos_datetime: &str) -> String {
    pos_datetime
        .chars()
        .take(19)
        .collect::<String>()
        .replace('T', " ")
}

fn rate_text(rate_bp: i64, dec: Decimal) -> String {
    if rate_bp % 100 == 0 {
        (rate_bp / 100).to_string()
    } else {
        amount(rate_bp, dec)
    }
}

/// Rate of a label on a day, from the Z report, else from a ticket.
fn rate_of(day: &Day, label: &str) -> i64 {
    day.z.vats.get(label).map_or_else(
        || {
            day.sales
                .iter()
                .flat_map(|s| s.vats.iter())
                .find(|v| v.label == label)
                .map_or(0, |v| v.rate_bp)
        },
        |v| v.rate_bp,
    )
}

/// One row per booking period: tickets, refunds, turnover.
pub fn jours(days: &[Day], dec: Decimal) -> Table {
    let mut t = Table::new(&[
        "date",
        "rapport_z",
        "tickets",
        "remboursements",
        "total",
        "total_centimes",
    ]);
    for d in days {
        let mut row = vec![
            d.booking_date.clone(),
            d.z_report_no.to_string(),
            d.tickets().to_string(),
            d.refunds().to_string(),
        ];
        push_amount(&mut row, d.turnover(), dec);
        t.push(row);
    }
    t
}

/// VAT per period, channel and label, as the FDM calculated it.
pub fn tva(days: &[Day], dec: Decimal) -> Table {
    let mut t = Table::new(&[
        "date",
        "rapport_z",
        "canal",
        "code_tva",
        "taux",
        "base",
        "base_centimes",
        "tva",
        "tva_centimes",
        "total",
        "total_centimes",
    ]);
    for d in days {
        for ((channel, label), a) in d.vat_by_channel() {
            let mut row = vec![
                d.booking_date.clone(),
                d.z_report_no.to_string(),
                channel.code().to_string(),
                label.clone(),
                rate_text(rate_of(d, &label), dec),
            ];
            let Amounts { base, vat, total } = a;
            push_amount(&mut row, base, dec);
            push_amount(&mut row, vat, dec);
            push_amount(&mut row, total, dec);
            t.push(row);
        }
    }
    t
}

/// Payments per period and method, with rounding and tips apart.
pub fn paiements(days: &[Day], dec: Decimal) -> Table {
    let mut t = Table::new(&[
        "date",
        "rapport_z",
        "moyen",
        "paiements",
        "paiements_centimes",
        "arrondis",
        "arrondis_centimes",
        "pourboires",
        "pourboires_centimes",
        "total",
        "total_centimes",
    ]);
    for d in days {
        let by = d.payments_by_method();
        let methods: std::collections::BTreeSet<Method> = by.keys().map(|(m, _)| *m).collect();
        for m in methods {
            let get = |k: Kind| by.get(&(m, k)).copied().unwrap_or(0);
            let (p, r, tip) = (get(Kind::Payment), get(Kind::Rounding), get(Kind::Tip));
            let mut row = vec![
                d.booking_date.clone(),
                d.z_report_no.to_string(),
                m.code().to_string(),
            ];
            push_amount(&mut row, p, dec);
            push_amount(&mut row, r, dec);
            push_amount(&mut row, tip, dec);
            push_amount(&mut row, p + r + tip, dec);
            t.push(row);
        }
    }
    t
}

fn methods_of(sale: &Sale) -> String {
    let set: std::collections::BTreeSet<&str> = sale
        .payments
        .iter()
        .filter(|p| p.amount_type == "PAYMENT")
        .map(|p| Method::from_type(&p.payment_type).code())
        .collect();
    set.into_iter().collect::<Vec<_>>().join("+")
}

/// Each refund ticket.
pub fn remboursements(days: &[Day], dec: Decimal) -> Table {
    let mut t = Table::new(&[
        "date",
        "rapport_z",
        "ticket",
        "date_heure",
        "canal",
        "moyen",
        "total",
        "total_centimes",
        "ticket_origine",
    ]);
    for d in days {
        for s in d.sales.iter().filter(|s| s.refund) {
            let origin = s.refers_to.as_ref().map_or_else(String::new, |r| {
                format!(
                    "{}{}",
                    r["eventLabel"].as_str().unwrap_or("N"),
                    r["eventCounter"].as_i64().unwrap_or(0)
                )
            });
            let mut row = vec![
                d.booking_date.clone(),
                d.z_report_no.to_string(),
                s.ticket_no.to_string(),
                local_time(&s.pos_datetime),
                s.channel.code().to_string(),
                methods_of(s),
            ];
            push_amount(&mut row, s.lines_total(), dec);
            row.push(origin);
            t.push(row);
        }
    }
    t
}

/// Each invoice (label I). Informative: the listed tickets are already counted in the
/// sales of their own day.
pub fn factures(days: &[Day], dec: Decimal) -> Table {
    let mut header: Vec<String> = [
        "date",
        "rapport_z",
        "numero_facture",
        "ticket",
        "date_heure",
        "tva_client",
        "client",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    for label in LABELS {
        for part in ["base", "tva"] {
            header.push(format!("{part}_{label}"));
            header.push(format!("{part}_{label}_centimes"));
        }
    }
    for tail in ["total", "total_centimes", "tickets", "tickets_introuvables"] {
        header.push(tail.to_string());
    }
    let header_refs: Vec<&str> = header.iter().map(String::as_str).collect();
    let mut t = Table::new(&header_refs);
    for d in days {
        for i in &d.invoices {
            let mut row = vec![
                d.booking_date.clone(),
                d.z_report_no.to_string(),
                text(&i.invoice.invoice_no),
                i.invoice.ticket_no.to_string(),
                local_time(&i.invoice.pos_datetime),
                text(&i.invoice.customer_vat_no),
                text(&i.invoice.customer_name),
            ];
            let mut total = 0;
            for label in LABELS {
                let a = i.vats.get(label).copied().unwrap_or_default();
                push_amount(&mut row, a.base, dec);
                push_amount(&mut row, a.vat, dec);
                total += a.total;
            }
            push_amount(&mut row, total, dec);
            row.push(i.tickets.join(" "));
            row.push(i.unresolved.to_string());
            t.push(row);
        }
    }
    t
}

/// Gift vouchers: sales (VAT X lines of department `GV`) and redemptions
/// (`VOUCHER_STORE` payments), refunds as negative rows.
pub fn bons_cadeaux(days: &[Day], dec: Decimal) -> Table {
    let mut t = Table::new(&[
        "date",
        "rapport_z",
        "type",
        "ticket",
        "date_heure",
        "montant",
        "montant_centimes",
    ]);
    for d in days {
        for s in &d.sales {
            let sold = s.gift_voucher_sold();
            let used: i64 = s
                .payments
                .iter()
                .filter(|p| p.payment_type == "VOUCHER_STORE" && p.amount_type == "PAYMENT")
                .map(|p| p.cents)
                .sum();
            for (kind_pos, kind_neg, cents) in [
                ("vente", "remboursement_vente", sold),
                ("utilisation", "remboursement_utilisation", used),
            ] {
                if cents == 0 {
                    continue;
                }
                let mut row = vec![
                    d.booking_date.clone(),
                    d.z_report_no.to_string(),
                    (if cents > 0 { kind_pos } else { kind_neg }).to_string(),
                    s.ticket_no.to_string(),
                    local_time(&s.pos_datetime),
                ];
                push_amount(&mut row, cents, dec);
                t.push(row);
            }
        }
    }
    t
}

/// Cash put in and taken out (financial events), signed.
pub fn mouvements_caisse(days: &[Day], dec: Decimal) -> Table {
    let mut t = Table::new(&[
        "date",
        "rapport_z",
        "ticket",
        "date_heure",
        "sens",
        "moyen",
        "libelle",
        "montant",
        "montant_centimes",
    ]);
    for d in days {
        for m in &d.moves {
            let mut row = vec![
                d.booking_date.clone(),
                d.z_report_no.to_string(),
                m.ticket_no.to_string(),
                local_time(&m.pos_datetime),
                (if m.cents >= 0 { "entree" } else { "sortie" }).to_string(),
                Method::from_type(&m.payment_type).code().to_string(),
                text(&m.label),
            ];
            push_amount(&mut row, m.cents, dec);
            t.push(row);
        }
    }
    t
}

/// The Z reports as signed.
pub fn rapports_z(days: &[Day], dec: Decimal) -> Table {
    let mut header: Vec<String> = [
        "date",
        "rapport_z",
        "ticket_pos",
        "date_heure_fdm",
        "tickets",
        "remboursements",
        "total",
        "total_centimes",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    for label in LABELS {
        for part in ["total", "tva"] {
            header.push(format!("{part}_{label}"));
            header.push(format!("{part}_{label}_centimes"));
        }
    }
    let header_refs: Vec<&str> = header.iter().map(String::as_str).collect();
    let mut t = Table::new(&header_refs);
    for d in days {
        let z = &d.z;
        let mut row = vec![
            d.booking_date.clone(),
            d.z_report_no.to_string(),
            z.ticket_no.to_string(),
            z.fdm_datetime.clone(),
            z.n_count.to_string(),
            z.refunds.to_string(),
        ];
        push_amount(&mut row, z.n_amount, dec);
        let by: BTreeMap<&String, _> = z.vats.iter().collect();
        for label in LABELS {
            let v = by.get(&label.to_string());
            push_amount(&mut row, v.map_or(0, |v| v.total), dec);
            push_amount(&mut row, v.map_or(0, |v| v.vat), dec);
        }
        t.push(row);
    }
    t
}
