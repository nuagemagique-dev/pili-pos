//! Content of the statistics file built from the shared fixture (card X1-T3).

mod common;

use std::collections::BTreeMap;
use std::path::Path;

use common::*;
use pos_export::pseudonym::user_ref;
use pos_export::stats::{STATS_SCHEMA_VERSION, STATS_TABLES, StatsSummary, build_stats};
use pos_export::zref::{ZRef, latest_real_z, real_zs_after};
use rusqlite::Connection;

fn built() -> (tempfile::TempDir, Fixture, std::path::PathBuf, StatsSummary) {
    let dir = tempfile::tempdir().unwrap();
    let fx = fixture_db(&dir.path().join("pos.db"));
    let out = dir.path().join("stats-20261004-z000042.sqlite");
    let summary = build_fixture_stats(&fx, &out);
    (dir, fx, out, summary)
}

fn scalar(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

/// Every row of every table as text, in a stable order, except `generated_at`.
fn dump(path: &Path) -> BTreeMap<String, Vec<Vec<String>>> {
    let conn = open_ro(path);
    let mut out = BTreeMap::new();
    for table in table_names(&conn) {
        let columns = column_names(&conn, &table);
        let select = columns
            .iter()
            .map(|c| format!("quote(\"{c}\")"))
            .collect::<Vec<_>>()
            .join(", ");
        let order = (1..=columns.len())
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {select} FROM \"{table}\" ORDER BY {order}"
            ))
            .unwrap();
        let rows: Vec<Vec<String>> = stmt
            .query_map([], |r| {
                (0..columns.len())
                    .map(|i| r.get::<_, String>(i))
                    .collect::<rusqlite::Result<Vec<String>>>()
            })
            .unwrap()
            .map(Result::unwrap)
            .filter(|row| !(table == "meta" && row[0] == "'generated_at'"))
            .collect();
        out.insert(table, rows);
    }
    out
}

#[test]
fn the_real_z_reports_are_listed_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let fx = fixture_db(&dir.path().join("pos.db"));
    let conn = open_ro(&fx.db_path);
    let all = real_zs_after(&conn, None).unwrap();
    assert_eq!(
        all.iter().map(|z| z.z_report_no).collect::<Vec<_>>(),
        vec![41, 42]
    );
    assert_eq!(all[0].booking_date, "2026-10-03");
    let z = &all[1];
    assert_eq!(z.booking_date, fx.booking_date);
    assert_eq!(z.period_id, fx.period_id);
    assert_eq!(z.closing_event_id, fx.closing_event_id);
    assert!(z.closed_at.starts_with("20"), "{}", z.closed_at);
    assert_eq!(real_zs_after(&conn, Some(41)).unwrap(), vec![z.clone()]);
    assert!(real_zs_after(&conn, Some(42)).unwrap().is_empty());
    assert_eq!(latest_real_z(&conn).unwrap().as_ref(), Some(z));
}

#[test]
fn a_training_z_is_never_a_real_z() {
    let dir = tempfile::tempdir().unwrap();
    let fx = fixture_db(&dir.path().join("pos.db"));
    // The fixture has a training Z without a number. Even a numbered row that
    // points at it and closes a period would not make it real.
    {
        let conn = Connection::open(&fx.db_path).unwrap();
        let training_z = fx.other_event_ids[0];
        pos_db::queries::reports::save_z_report(&conn, "2026-10-05", 43, "{}", Some(training_z))
            .unwrap();
        pos_db::queries::booking::close_period(&conn, "bp-fixture-0005", training_z).unwrap();
    }
    let conn = open_ro(&fx.db_path);
    let numbers: Vec<i64> = real_zs_after(&conn, None)
        .unwrap()
        .iter()
        .map(|z| z.z_report_no)
        .collect();
    assert_eq!(numbers, vec![41, 42]);
    assert_eq!(latest_real_z(&conn).unwrap().unwrap().z_report_no, 42);
    assert!(real_zs_after(&conn, Some(42)).unwrap().is_empty());
}

#[test]
fn counts_and_totals_match_the_fixture_and_the_z() {
    let (_dir, fx, out, summary) = built();
    assert_eq!(summary.tickets, 6);
    assert_eq!(summary.tickets, fx.tickets.len() as i64);
    assert_eq!(summary.lines, 9);
    assert_eq!(summary.lines, fx.lines);
    assert_eq!(summary.bytes, std::fs::metadata(&out).unwrap().len() as i64);

    let conn = open_ro(&out);
    assert_eq!(scalar(&conn, "SELECT COUNT(*) FROM tickets"), 6);
    assert_eq!(scalar(&conn, "SELECT COUNT(*) FROM ticket_lines"), 9);

    // The sum of the ticket totals is the Z total, and the Z says so.
    let sum = scalar(&conn, "SELECT SUM(total_cents) FROM tickets");
    assert_eq!(sum, fx.z_total_cents);
    assert_eq!(
        scalar(&conn, "SELECT total_cents FROM z_report"),
        fx.z_total_cents
    );
    assert_eq!(scalar(&conn, "SELECT sales_count FROM z_report"), 6);
    assert_eq!(scalar(&conn, "SELECT refunds_count FROM z_report"), 1);
    assert_eq!(
        scalar(&conn, "SELECT COUNT(*) FROM tickets WHERE kind = 'refund'"),
        1
    );

    // A ticket is the sum of its lines, a line its price plus its discount.
    assert_eq!(
        scalar(
            &conn,
            "SELECT COUNT(*) FROM tickets t WHERE t.total_cents !=
                (SELECT SUM(amount_cents) FROM ticket_lines l WHERE l.ticket_id = t.ticket_id)"
        ),
        0
    );
    assert_eq!(
        scalar(
            &conn,
            "SELECT COUNT(*) FROM ticket_lines
             WHERE amount_cents != CAST(ROUND(quantity * unit_price_cents) AS INTEGER) + discount_cents"
        ),
        0
    );
    // Discounts: the rows of ticket_discounts add up to the discounts of the lines.
    assert_eq!(
        scalar(&conn, "SELECT SUM(amount_cents) FROM ticket_discounts"),
        scalar(&conn, "SELECT SUM(discount_cents) FROM ticket_lines")
    );
    assert_eq!(
        scalar(&conn, "SELECT SUM(amount_cents) FROM ticket_discounts"),
        -130 - 270
    );
    let ticket_wide: Vec<(i64, i64)> = conn
        .prepare("SELECT ticket_id, amount_cents FROM ticket_discounts WHERE line_no IS NULL")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(ticket_wide, vec![(fx.tickets[2].ticket_id, -270)]);
}

#[test]
fn vat_per_rate_equals_the_z() {
    let (_dir, fx, out, _) = built();
    let conn = open_ro(&out);
    let from_tickets: Vec<(String, i64, i64, i64, i64)> = conn
        .prepare(
            "SELECT vat_label, vat_rate_bp, SUM(base_cents), SUM(vat_cents), SUM(total_cents)
             FROM ticket_vat GROUP BY vat_label, vat_rate_bp ORDER BY vat_label",
        )
        .unwrap()
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let from_z: Vec<(String, i64, i64, i64, i64)> = conn
        .prepare(
            "SELECT vat_label, vat_rate_bp, base_cents, vat_cents, total_cents FROM z_vat ORDER BY vat_label",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(from_tickets, from_z);
    let expected: Vec<(String, i64, i64, i64, i64)> = fx
        .z_vat
        .iter()
        .map(|&(l, r, b, v, t)| (l.to_string(), r, b, v, t))
        .collect();
    assert_eq!(from_z, expected);
    // Each ticket's own VAT adds up to its total.
    assert_eq!(
        scalar(
            &conn,
            "SELECT COUNT(*) FROM tickets t WHERE t.total_cents !=
                (SELECT SUM(total_cents) FROM ticket_vat v WHERE v.ticket_id = t.ticket_id)"
        ),
        0
    );
    assert_eq!(
        scalar(
            &conn,
            "SELECT COUNT(*) FROM ticket_vat WHERE base_cents + vat_cents != total_cents"
        ),
        0
    );
    // Every line has its rate.
    assert_eq!(
        scalar(
            &conn,
            "SELECT COUNT(*) FROM ticket_lines WHERE vat_rate_bp IS NULL"
        ),
        0
    );
    assert_eq!(
        scalar(
            &conn,
            "SELECT vat_rate_bp FROM ticket_lines WHERE product_name = 'Biere Asahi' LIMIT 1"
        ),
        2100
    );
}

#[test]
fn payments_match_the_tickets_and_the_z() {
    let (_dir, fx, out, _) = built();
    let conn = open_ro(&out);
    let per_ticket: Vec<(i64, i64)> = conn
        .prepare(
            "SELECT t.ticket_id, COALESCE((SELECT SUM(amount_cents) FROM ticket_payments p
                 WHERE p.ticket_id = t.ticket_id AND p.kind = 'payment'), 0)
             FROM tickets t ORDER BY t.ticket_id",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for (ticket, (id, paid)) in fx.tickets.iter().zip(per_ticket) {
        assert_eq!(ticket.ticket_id, id);
        let expected = if ticket.unpaid { 0 } else { ticket.total_cents };
        assert_eq!(paid, expected, "ticket {id}");
    }
    // Meal voucher plus cash plus the cash rounding line of the takeaway sale.
    let takeaway: Vec<(String, String, i64)> = conn
        .prepare("SELECT method, kind, amount_cents FROM ticket_payments WHERE ticket_id = ?1 ORDER BY line_no")
        .unwrap()
        .query_map([fx.tickets[1].ticket_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        takeaway,
        vec![
            ("meal_voucher".to_string(), "payment".to_string(), 1000),
            ("cash".to_string(), "payment".to_string(), 1534),
            ("cash".to_string(), "rounding".to_string(), 1),
        ]
    );
    let ticket_sums: Vec<(String, String, i64)> = conn
        .prepare("SELECT method, kind, SUM(amount_cents) FROM ticket_payments GROUP BY method, kind ORDER BY method, kind")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let z_rows: Vec<(String, String, i64)> = conn
        .prepare("SELECT method, kind, amount_cents FROM z_payments ORDER BY method, kind")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(ticket_sums, z_rows);
    let expected: Vec<(String, String, i64)> = fx
        .z_payments
        .iter()
        .map(|&(m, k, a)| (m.to_string(), k.to_string(), a))
        .collect();
    assert_eq!(z_rows, expected);
}

#[test]
fn channels_tables_kinds_and_users_follow_the_orders() {
    let (_dir, fx, out, _) = built();
    let conn = open_ro(&out);
    let key = test_key();
    type Row = (i64, String, String, Option<i64>, i64, i64, String);
    let rows: Vec<Row> = conn
        .prepare(
            "SELECT ticket_id, channel, kind, table_number, total_cents, unpaid, user_ref
             FROM tickets ORDER BY ticket_id",
        )
        .unwrap()
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(rows.len(), fx.tickets.len());
    for (row, expected) in rows.iter().zip(&fx.tickets) {
        assert_eq!(row.0, expected.ticket_id);
        assert_eq!(row.1, expected.channel, "channel of {}", row.0);
        assert_eq!(row.2, expected.kind, "kind of {}", row.0);
        assert_eq!(row.3, expected.table_number, "table of {}", row.0);
        assert_eq!(row.4, expected.total_cents, "total of {}", row.0);
        assert_eq!(row.5 == 1, expected.unpaid, "unpaid of {}", row.0);
        assert_eq!(
            row.6,
            user_ref(&key, expected.employee),
            "user of {}",
            row.0
        );
    }
    let channels: Vec<&str> = rows.iter().map(|r| r.1.as_str()).collect();
    assert_eq!(
        channels,
        vec!["table", "takeaway", "delivery", "table", "table", "online"]
    );

    // The refund is negative everywhere, as signed.
    let (quantity, unit, amount): (f64, i64, i64) = conn
        .query_row(
            "SELECT quantity, unit_price_cents, amount_cents FROM ticket_lines WHERE ticket_id = ?1",
            [fx.tickets[3].ticket_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!((quantity, unit, amount), (-1.0, 450, -450));
    assert_eq!(
        scalar(
            &conn,
            "SELECT COUNT(*) FROM ticket_payments p JOIN tickets t USING (ticket_id)
             WHERE t.kind = 'refund' AND p.amount_cents >= 0"
        ),
        0
    );
    assert_eq!(
        scalar(
            &conn,
            "SELECT COUNT(*) FROM ticket_vat v JOIN tickets t USING (ticket_id)
             WHERE t.kind = 'refund' AND v.total_cents >= 0"
        ),
        0
    );

    // Users: exactly those of the file, with their role.
    let users: BTreeMap<String, String> = conn
        .prepare("SELECT user_ref, role FROM users")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let expected: BTreeMap<String, String> = [
        ("system".to_string(), "system".to_string()),
        (user_ref(&key, ADMIN_INSZ), "admin".to_string()),
        (user_ref(&key, CASHIER_INSZ), "cashier".to_string()),
        (user_ref(&key, WAITER_INSZ), "waiter".to_string()),
    ]
    .into_iter()
    .collect();
    assert_eq!(users, expected);
    assert_eq!(
        scalar(
            &conn,
            "SELECT COUNT(*) FROM tickets WHERE user_ref NOT IN (SELECT user_ref FROM users)"
        ),
        0
    );
    let z_user: String = conn
        .query_row("SELECT user_ref FROM z_report", [], |r| r.get(0))
        .unwrap();
    assert_eq!(z_user, user_ref(&key, ADMIN_INSZ));
}

#[test]
fn product_codes_names_and_departments_come_from_the_event_and_the_menu() {
    let (_dir, _fx, out, _) = built();
    let conn = open_ro(&out);
    let rows: Vec<(String, Option<String>, Option<String>)> = conn
        .prepare("SELECT DISTINCT product_name, product_code, department FROM ticket_lines ORDER BY product_name")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let find = |name: &str| {
        rows.iter()
            .find(|r| r.0 == name)
            .unwrap_or_else(|| panic!("{name}"))
    };
    assert_eq!(find("Nigiri saumon").1.as_deref(), Some("S1"));
    assert_eq!(find("Nigiri saumon").2.as_deref(), Some("Cuisine"));
    assert_eq!(find("Biere Asahi").1.as_deref(), Some("D4"));
    assert_eq!(find("Biere Asahi").2.as_deref(), Some("Boissons"));
    // A product without a menu code has none.
    assert_eq!(find("Soupe miso").1, None);
}

#[test]
fn only_the_sales_of_the_period_are_exported() {
    let (_dir, fx, out, _) = built();
    let conn = open_ro(&out);
    let ids: Vec<i64> = conn
        .prepare("SELECT ticket_id FROM tickets ORDER BY ticket_id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        ids,
        fx.tickets.iter().map(|t| t.ticket_id).collect::<Vec<_>>()
    );
    assert_eq!(
        scalar(
            &conn,
            "SELECT COUNT(*) FROM tickets WHERE event_label != 'N'"
        ),
        0
    );
    for id in fx.excluded_event_ids.iter().chain(&fx.other_event_ids) {
        assert!(!ids.contains(id), "event {id} must not be a ticket");
    }
    // The planted events exist in the source: the exclusion is real.
    let source = open_ro(&fx.db_path);
    for label in ["S", "P", "C", "T", "F", "I"] {
        let n: i64 = source
            .query_row(
                "SELECT COUNT(*) FROM fiscal_events WHERE event_type = ?1",
                [label],
                |r| r.get(0),
            )
            .unwrap();
        assert!(n >= 1, "the fixture has a {label} event");
    }
    // The training sale (table 3) and the earlier day's sale (6,00 EUR) are absent.
    assert_eq!(
        scalar(&conn, "SELECT COUNT(*) FROM tickets WHERE table_number = 3"),
        0
    );
    assert_eq!(
        scalar(
            &conn,
            "SELECT COUNT(*) FROM ticket_lines WHERE product_name = 'Yakitori'"
        ),
        0
    );
    // The Z is the Z of the period.
    assert_eq!(scalar(&conn, "SELECT z_report_no FROM z_report"), 42);
    assert_eq!(scalar(&conn, "SELECT COUNT(*) FROM z_report"), 1);
}

#[test]
fn meta_describes_the_export() {
    let (_dir, fx, out, _) = built();
    let conn = open_ro(&out);
    let meta: BTreeMap<String, String> = conn
        .prepare("SELECT key, value FROM meta")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let get = |k: &str| meta.get(k).unwrap_or_else(|| panic!("meta {k}")).as_str();
    assert_eq!(get("format"), "pili-stats");
    assert_eq!(get("schema_version"), STATS_SCHEMA_VERSION.to_string());
    assert_eq!(STATS_SCHEMA_VERSION, 1);
    assert_eq!(get("export_id"), "20261004-z000042");
    assert_eq!(get("pos_id"), POS_ID);
    assert_eq!(get("terminal_id"), TERMINAL_ID);
    assert_eq!(get("pos_sw_version"), SW_VERSION);
    assert_eq!(get("z_report_no"), "42");
    assert_eq!(get("booking_date"), fx.booking_date);
    assert_eq!(get("period_opened_at"), "2026-10-04T07:30:00.000Z");
    assert_eq!(get("period_closed_at"), fixture_z(&fx).closed_at);
    assert_eq!(get("currency"), "EUR");
    assert_eq!(get("generated_at"), "2026-10-05T01:00:00Z");
    assert_eq!(meta.len(), 12);
    assert_eq!(scalar(&conn, "PRAGMA user_version"), 1);
    // The tables are the documented ones.
    assert_eq!(table_names(&conn).len(), STATS_TABLES.len());
}

#[test]
fn two_builds_of_the_same_input_have_the_same_content() {
    let dir = tempfile::tempdir().unwrap();
    let fx = fixture_db(&dir.path().join("pos.db"));
    let a = dir.path().join("a.sqlite");
    let b = dir.path().join("b.sqlite");
    build_fixture_stats(&fx, &a);
    let conn = open_ro(&fx.db_path);
    let mut meta = test_meta();
    meta.generated_at = "2026-10-06T09:30:00Z".into();
    build_stats(&conn, &fixture_z(&fx), &meta, &test_key(), &b).unwrap();
    assert_eq!(dump(&a), dump(&b));
    // Only generated_at differs.
    let generated = |p: &Path| -> String {
        open_ro(p)
            .query_row(
                "SELECT value FROM meta WHERE key = 'generated_at'",
                [],
                |r| r.get(0),
            )
            .unwrap()
    };
    assert_ne!(generated(&a), generated(&b));
}

#[test]
fn another_key_changes_the_pseudonyms_and_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let fx = fixture_db(&dir.path().join("pos.db"));
    let a = dir.path().join("a.sqlite");
    let b = dir.path().join("b.sqlite");
    build_fixture_stats(&fx, &a);
    build_stats(
        &open_ro(&fx.db_path),
        &fixture_z(&fx),
        &test_meta(),
        &[0x43; 32],
        &b,
    )
    .unwrap();
    let refs = |p: &Path| -> Vec<String> {
        open_ro(p)
            .prepare("SELECT user_ref FROM users WHERE user_ref != 'system' ORDER BY user_ref")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    assert_eq!(refs(&a).len(), 3);
    assert!(refs(&a).iter().all(|r| !refs(&b).contains(r)));
    // The totals do not depend on the key.
    let total = |p: &Path| scalar(&open_ro(p), "SELECT SUM(total_cents) FROM tickets");
    assert_eq!(total(&a), total(&b));
}

#[test]
fn a_rebuild_replaces_the_file_and_leaves_no_partial() {
    let dir = tempfile::tempdir().unwrap();
    let fx = fixture_db(&dir.path().join("pos.db"));
    let out = dir.path().join("stats.sqlite");
    std::fs::write(&out, b"older content").unwrap();
    std::fs::write(dir.path().join("stats.sqlite.partial"), b"leftover").unwrap();
    let summary = build_fixture_stats(&fx, &out);
    assert_eq!(summary.tickets, 6);
    assert_eq!(table_names(&open_ro(&out)).len(), STATS_TABLES.len());
    assert!(!dir.path().join("stats.sqlite.partial").exists());
}

#[test]
fn a_failed_build_leaves_nothing_behind() {
    let dir = tempfile::tempdir().unwrap();
    let fx = fixture_db(&dir.path().join("pos.db"));
    let out = dir.path().join("stats.sqlite");
    let conn = open_ro(&fx.db_path);
    let mut z: ZRef = fixture_z(&fx);
    z.period_id = "no-such-period".into();
    let err = build_stats(&conn, &z, &test_meta(), &test_key(), &out).unwrap_err();
    assert!(err.contains("not found"), "{err}");
    assert!(!out.exists());
    assert!(!dir.path().join("stats.sqlite.partial").exists());

    // The closing event must be a Z report.
    let mut z = fixture_z(&fx);
    z.closing_event_id = fx.tickets[0].ticket_id;
    assert!(build_stats(&conn, &z, &test_meta(), &test_key(), &out).is_err());
    assert!(!out.exists());
    assert!(!dir.path().join("stats.sqlite.partial").exists());
}

/// Rewrites one archived field of a ticket on a copy of the fixture (the append-only
/// trigger is dropped on the copy only) and returns the build error.
fn build_error_after(sql: &str) -> (String, i64) {
    let dir = tempfile::tempdir().unwrap();
    let fx = fixture_db(&dir.path().join("pos.db"));
    let id = fx.tickets[0].ticket_id;
    {
        let conn = Connection::open(&fx.db_path).unwrap();
        let triggers: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'trigger' AND tbl_name = 'fiscal_events'")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        for t in triggers {
            conn.execute_batch(&format!("DROP TRIGGER \"{t}\";"))
                .unwrap();
        }
        conn.execute(sql, [id]).unwrap();
    }
    let conn = open_ro(&fx.db_path);
    let out = dir.path().join("stats.sqlite");
    let err = build_stats(&conn, &fixture_z(&fx), &test_meta(), &test_key(), &out).unwrap_err();
    assert!(!out.exists());
    (err, id)
}

#[test]
fn a_missing_ticket_number_is_an_error_naming_the_event_not_a_zero() {
    let (err, id) = build_error_after(
        "UPDATE fiscal_events SET event_data_json = json_remove(event_data_json, '$.posFiscalTicketNo') WHERE id = ?1",
    );
    assert!(err.contains("posFiscalTicketNo"), "{err}");
    assert!(err.contains(&format!("event {id}")), "{err}");
}

#[test]
fn a_missing_amount_is_an_error_naming_the_event_not_a_zero() {
    let (err, id) = build_error_after(
        "UPDATE fiscal_events SET event_data_json = json_remove(event_data_json, '$.financials[0].amount') WHERE id = ?1",
    );
    assert!(err.contains("amount"), "{err}");
    assert!(err.contains(&format!("event {id}")), "{err}");
}

/// Parity with `pos-accounting`: the same archived events (a fixture file owned by
/// that crate) must give the same VAT rows, payments and refund flag here. Each
/// case replaces the archived request and answer of one ticket on a copy of the
/// fixture database.
#[test]
fn the_parity_cases_match_the_accounting_parser() {
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../pos-accounting/tests/golden/parity_cases.json"
    ))
    .unwrap();
    assert!(cases.len() >= 4);
    for case in &cases {
        let name = case["name"].as_str().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let fx = fixture_db(&dir.path().join("pos.db"));
        let id = fx.tickets[0].ticket_id;
        {
            let conn = Connection::open(&fx.db_path).unwrap();
            let triggers: Vec<String> = conn
                .prepare("SELECT name FROM sqlite_master WHERE type = 'trigger' AND tbl_name = 'fiscal_events'")
                .unwrap()
                .query_map([], |r| r.get(0))
                .unwrap()
                .map(Result::unwrap)
                .collect();
            for t in triggers {
                conn.execute_batch(&format!("DROP TRIGGER \"{t}\";"))
                    .unwrap();
            }
            conn.execute(
                "UPDATE fiscal_events SET
                    event_data_json = json_set(?2, '$.bookingPeriodId', json_extract(event_data_json, '$.bookingPeriodId')),
                    fdm_response_json = ?3 WHERE id = ?1",
                rusqlite::params![id, case["request"].to_string(), case["response"].to_string()],
            )
            .unwrap();
        }
        let out = dir.path().join("stats.sqlite");
        let conn = open_ro(&fx.db_path);
        build_stats(&conn, &fixture_z(&fx), &test_meta(), &test_key(), &out)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let stats = open_ro(&out);
        let expected = &case["expected"];

        let kind: String = stats
            .query_row("SELECT kind FROM tickets WHERE ticket_id = ?1", [id], |r| {
                r.get(0)
            })
            .unwrap();
        let refund = expected["refund"].as_bool().unwrap();
        assert_eq!(kind == "refund", refund, "{name}");

        let paid: i64 = stats
            .query_row(
                "SELECT COALESCE(SUM(amount_cents), 0) FROM ticket_payments WHERE ticket_id = ?1 AND kind = 'payment'",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(paid, expected["paid_cents"].as_i64().unwrap(), "{name}");

        let mut stmt = stats
            .prepare("SELECT vat_label, vat_rate_bp, base_cents, vat_cents, total_cents FROM ticket_vat WHERE ticket_id = ?1 ORDER BY vat_label")
            .unwrap();
        let rows: Vec<(String, i64, i64, i64, i64)> = stmt
            .query_map([id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let want = expected["vats"].as_array().unwrap();
        assert_eq!(rows.len(), want.len(), "{name}");
        for (row, w) in rows.iter().zip(want) {
            assert_eq!(row.0, w["label"].as_str().unwrap(), "{name}");
            assert_eq!(row.1, w["rate_bp"].as_i64().unwrap(), "{name}");
            assert_eq!(row.2, w["base"].as_i64().unwrap(), "{name}");
            assert_eq!(row.3, w["vat"].as_i64().unwrap(), "{name}");
            assert_eq!(row.4, w["total"].as_i64().unwrap(), "{name}");
        }
    }
}

#[test]
fn the_source_is_left_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let fx = fixture_db(&dir.path().join("pos.db"));
    let before = std::fs::read(&fx.db_path).unwrap();
    build_fixture_stats(&fx, &dir.path().join("stats.sqlite"));
    assert_eq!(std::fs::read(&fx.db_path).unwrap(), before);
}

#[test]
fn the_size_of_the_test_day() {
    let (_dir, fx, out, summary) = built();
    println!(
        "stats file of the fixture day: {} bytes, {} tickets, {} lines, source {} bytes",
        summary.bytes,
        summary.tickets,
        summary.lines,
        std::fs::metadata(&fx.db_path).unwrap().len()
    );
    assert!(summary.bytes > 0 && out.exists());
}
