//! Each day against its Z report.
//!
//! The Z report is signed by the FDM and archived; the detail files are built from the
//! day's signed tickets. They must agree to the cent, or the export is refused: a gap
//! means the archive is incomplete or was changed, and an accountant must not receive
//! books that do not match the fiscal report.
//!
//! Checked per day, in this order: the number of N tickets, the turnover, the VAT of
//! every label (base, VAT, total), the payments of every `(type, amount type)`.

use crate::aggregate::Day;
use crate::csv::{Decimal, amount};
use crate::error::{AccountingError, Result};

fn fail(day: &Day, field: &str, expected: String, found: String) -> AccountingError {
    AccountingError::ReconciliationFailed {
        day: day.booking_date.clone(),
        z_report_no: day.z_report_no,
        field: field.to_string(),
        expected,
        found,
    }
}

fn money(cents: i64) -> String {
    amount(cents, Decimal::Point)
}

fn check_money(day: &Day, field: &str, expected: i64, found: i64) -> Result<()> {
    if expected == found {
        Ok(())
    } else {
        Err(fail(day, field, money(expected), money(found)))
    }
}

/// Fail with `reconciliation_failed` when the day differs from its Z report.
pub fn reconcile(day: &Day) -> Result<()> {
    let z = &day.z;
    let count = i64::try_from(day.sales.len()).unwrap_or(i64::MAX);
    if z.n_count != count {
        return Err(fail(
            day,
            "tickets",
            z.n_count.to_string(),
            count.to_string(),
        ));
    }
    check_money(day, "turnover", z.n_amount, day.turnover())?;

    let found = day.vat_by_label();
    let labels: std::collections::BTreeSet<&String> = z.vats.keys().chain(found.keys()).collect();
    for label in labels {
        let expected = z.vats.get(label);
        let actual = found.get(label).copied().unwrap_or_default();
        check_money(
            day,
            &format!("vat.{label}.base"),
            expected.map_or(0, |v| v.base),
            actual.base,
        )?;
        check_money(
            day,
            &format!("vat.{label}.vat"),
            expected.map_or(0, |v| v.vat),
            actual.vat,
        )?;
        check_money(
            day,
            &format!("vat.{label}.total"),
            expected.map_or(0, |v| v.total),
            actual.total,
        )?;
    }

    let found = day.payments_by_type();
    let keys: std::collections::BTreeSet<&(String, String)> =
        z.payments.keys().chain(found.keys()).collect();
    for key in keys {
        check_money(
            day,
            &format!("payments.{}.{}", key.0, key.1),
            z.payments.get(key).copied().unwrap_or(0),
            found.get(key).copied().unwrap_or(0),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{Channel, PayLine, Sale, SaleLine, VatRow, ZReport, ZVat};
    use serde_json::Value;
    use std::collections::BTreeMap;

    fn sale(total: i64, paid: i64) -> Sale {
        Sale {
            id: 1,
            ticket_no: 1,
            pos_datetime: String::new(),
            fdm_datetime: String::new(),
            refund: false,
            channel: Channel::Takeaway,
            lines: vec![SaleLine {
                department_id: "10".into(),
                product_id: "1".into(),
                vats: vec![("A".into(), total)],
            }],
            vats: vec![VatRow {
                label: "A".into(),
                rate_bp: 2100,
                base: total * 100 / 121,
                vat: total - total * 100 / 121,
                total,
            }],
            payments: vec![PayLine {
                payment_type: "CASH".into(),
                amount_type: "PAYMENT".into(),
                cents: paid,
            }],
            refers_to: None,
            signed_ref: Value::Null,
        }
    }

    fn day(total: i64) -> Day {
        let s = sale(1210, 1210);
        let mut vats = BTreeMap::new();
        vats.insert(
            "A".to_string(),
            ZVat {
                rate_bp: 2100,
                base: s.vats[0].base,
                vat: s.vats[0].vat,
                total: 1210,
            },
        );
        let mut payments = BTreeMap::new();
        payments.insert(("CASH".to_string(), "PAYMENT".to_string()), 1210);
        Day {
            booking_date: "2026-10-01".into(),
            z_report_no: 4,
            z: ZReport {
                ticket_no: 9,
                pos_datetime: String::new(),
                fdm_datetime: String::new(),
                n_count: 1,
                n_amount: total,
                refunds: 0,
                vats,
                payments,
            },
            sales: vec![s],
            invoices: vec![],
            moves: vec![],
        }
    }

    #[test]
    fn a_day_equal_to_its_z_passes() {
        reconcile(&day(1210)).unwrap();
    }

    #[test]
    fn a_turnover_gap_names_the_day_and_the_field() {
        let err = reconcile(&day(1211)).unwrap_err();
        assert_eq!(err.code(), "reconciliation_failed");
        assert_eq!(err.exit_code(), 2);
        let text = err.to_string();
        assert!(
            text.contains("2026-10-01") && text.contains("turnover"),
            "{text}"
        );
    }

    #[test]
    fn a_ticket_count_gap_is_found_first() {
        let mut d = day(1210);
        d.z.n_count = 2;
        let AccountingError::ReconciliationFailed { field, .. } = reconcile(&d).unwrap_err() else {
            panic!("wrong error");
        };
        assert_eq!(field, "tickets");
    }

    #[test]
    fn a_vat_gap_names_the_label() {
        let mut d = day(1210);
        d.z.vats.get_mut("A").unwrap().vat += 1;
        let AccountingError::ReconciliationFailed { field, .. } = reconcile(&d).unwrap_err() else {
            panic!("wrong error");
        };
        assert_eq!(field, "vat.A.vat");
    }

    #[test]
    fn a_vat_label_missing_from_the_tickets_is_a_gap() {
        let mut d = day(1210);
        d.z.vats.insert(
            "B".into(),
            ZVat {
                rate_bp: 1200,
                base: 100,
                vat: 12,
                total: 112,
            },
        );
        let AccountingError::ReconciliationFailed { field, .. } = reconcile(&d).unwrap_err() else {
            panic!("wrong error");
        };
        assert_eq!(field, "vat.B.base");
    }

    #[test]
    fn a_payment_gap_names_type_and_amount_type() {
        let mut d = day(1210);
        d.sales[0].payments[0].cents = 1200;
        let AccountingError::ReconciliationFailed { field, .. } = reconcile(&d).unwrap_err() else {
            panic!("wrong error");
        };
        assert_eq!(field, "payments.CASH.PAYMENT");
    }
}
