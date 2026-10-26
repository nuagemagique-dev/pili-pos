//! The export on a synthetic archive: golden files, the files' semantics (what is read
//! and what is ignored), the journal, the profiles and the guards on the source.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

mod common;

use common::*;
use pos_accounting::{Accounts, Decimal, PeriodSpec, Profile};
use std::path::Path;

/// A month of three days: two closed by a Z, one still open.
struct Month {
    synth: Synth,
    s1: serde_json::Value,
    s2: serde_json::Value,
    s4: serde_json::Value,
}

fn month_archive() -> Month {
    let mut s = Synth::new();
    s.open_period("p1", "2026-10-01");
    // Events the export must not read.
    s.ignored("P", "p1");
    s.ignored("T", "p1");
    s.ignored("C", "p1");
    s.unsigned_sale("p1");

    let s1 = s.sale(
        "p1",
        SaleSpec::new(&[L("10", "A", 12.10)], &[P("CASH", "PAYMENT", 12.10)]),
    );
    let s2 = s.sale(
        "p1",
        SaleSpec::new(&[L("22", "B", 11.20)], &[P("CARD_DEBIT", "PAYMENT", 11.20)]).table(),
    );
    s.sale(
        "p1",
        SaleSpec::new(&[L("GV", "X", 50.0)], &[P("CARD_DEBIT", "PAYMENT", 50.0)]),
    );
    let s4 = s.sale(
        "p1",
        SaleSpec::new(
            &[L("22", "B", 28.0)],
            &[
                P("VOUCHER_STORE", "PAYMENT", 20.0),
                P("CASH", "PAYMENT", 8.0),
            ],
        )
        .table(),
    );
    s.sale(
        "p1",
        SaleSpec::new(
            &[L("30", "C", 10.60)],
            &[P("CHEQUE_MEAL", "PAYMENT", 10.60)],
        ),
    );
    s.sale(
        "p1",
        SaleSpec::new(&[L("10", "A", -12.10)], &[P("CASH", "PAYMENT", -12.10)])
            .refund_of(s1.clone()),
    );
    s.sale(
        "p1",
        SaleSpec::new(
            &[L("10", "A", 10.03)],
            &[P("CASH", "PAYMENT", 10.03), P("CASH", "ROUNDING", 0.02)],
        ),
    );
    s.sale(
        "p1",
        SaleSpec::new(&[L("10", "A", 20.0)], &[P("ONLINE", "PAYMENT", 20.0)]).online(),
    );
    s.money("p1", 100.0, "Fond de caisse");
    s.money("p1", -40.0, "=Banque");
    s.invoice("p1", &[s2.clone(), s4.clone()], "20260001", "BE0308357159");
    s.close("p1", 1);

    s.open_period("p2", "2026-10-02");
    s.sale(
        "p2",
        SaleSpec::new(&[L("10", "A", 24.20)], &[P("CASH", "PAYMENT", 24.20)]),
    );
    s.close("p2", 2);

    // A third day with no Z yet.
    s.open_period("p3", "2026-10-03");
    s.sale(
        "p3",
        SaleSpec::new(&[L("10", "A", 99.0)], &[P("CASH", "PAYMENT", 99.0)]),
    );
    Month {
        synth: s,
        s1,
        s2,
        s4,
    }
}

fn golden(name: &str, actual: &[u8]) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
    }
    let expected = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(
        String::from_utf8_lossy(actual),
        String::from_utf8_lossy(&expected),
        "golden file {name} differs"
    );
    assert_eq!(
        actual,
        expected.as_slice(),
        "golden file {name}: bytes differ"
    );
}

fn export_month() -> (Month, tempfile::TempDir, pos_accounting::ExportSummary) {
    let m = month_archive();
    let out = tempfile::tempdir().unwrap();
    let summary = run(&m.synth.path, out.path(), month("2026-10")).unwrap();
    (m, out, summary)
}

#[test]
fn closed_periods_are_exported_and_the_open_one_is_named_and_left_out() {
    let (_m, _out, summary) = export_month();
    assert_eq!(
        summary.days,
        vec![("2026-10-01".to_string(), 1), ("2026-10-02".to_string(), 2)]
    );
    assert_eq!(summary.left_out, vec!["2026-10-03".to_string()]);
    assert!(
        summary
            .folder
            .ends_with("export-comptable-2026-10-01-2026-10-31")
    );
    let note = read(&summary.folder, "LISEZMOI.txt");
    assert!(note.contains("  - 2026-10-03"), "{note}");
    assert!(note.contains("Z n. 1"), "{note}");
    assert!(note.contains("Z n. 2"), "{note}");
    // The open day's 99.00 sale is in none of the files.
    for name in ["jours.csv", "tva.csv", "paiements.csv", "journal.csv"] {
        assert!(!read(&summary.folder, name).contains("99,00"), "{name}");
    }
}

#[test]
fn the_detail_files_match_their_golden_copies() {
    let (_m, _out, summary) = export_month();
    for name in [
        "jours.csv",
        "tva.csv",
        "paiements.csv",
        "remboursements.csv",
        "factures.csv",
        "bons_cadeaux.csv",
        "mouvements_caisse.csv",
        "rapports_z.csv",
        "journal.csv",
    ] {
        let bytes = std::fs::read(summary.folder.join(name)).unwrap();
        golden(name, &bytes);
    }
}

#[test]
fn every_file_is_utf8_with_bom_semicolons_and_crlf() {
    let (_m, _out, summary) = export_month();
    for name in [
        "jours.csv",
        "tva.csv",
        "paiements.csv",
        "remboursements.csv",
        "factures.csv",
        "bons_cadeaux.csv",
        "mouvements_caisse.csv",
        "rapports_z.csv",
        "journal.csv",
    ] {
        let bytes = std::fs::read(summary.folder.join(name)).unwrap();
        assert_eq!(&bytes[..3], [0xEF, 0xBB, 0xBF], "{name}: BOM");
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.ends_with("\r\n"), "{name}: CRLF");
        assert!(!text.replace("\r\n", "").contains('\n'), "{name}: bare LF");
        let widths: std::collections::BTreeSet<usize> =
            rows(&summary.folder, name).iter().map(Vec::len).collect();
        assert_eq!(widths.len(), 1, "{name}: ragged rows {widths:?}");
    }
}

#[test]
fn the_first_day_adds_up_by_hand() {
    let (_m, _out, summary) = export_month();
    let jours = rows(&summary.folder, "jours.csv");
    // 8 signed sales, one of them a refund; 129.83 turnover.
    assert_eq!(jours[1], ["2026-10-01", "1", "7", "1", "129,83", "12983"]);
    assert_eq!(jours[2], ["2026-10-02", "2", "1", "0", "24,20", "2420"]);
    let pay = rows(&summary.folder, "paiements.csv");
    let cash = pay
        .iter()
        .find(|r| r[0] == "2026-10-01" && r[2] == "especes")
        .unwrap();
    // 12.10 + 8.00 - 12.10 + 10.03 collected, 0.02 of rounding.
    assert_eq!(cash[3], "18,03");
    assert_eq!(cash[5], "0,02");
    assert_eq!(cash[9], "18,05");
    let gift = pay
        .iter()
        .find(|r| r[0] == "2026-10-01" && r[2] == "bon_cadeau")
        .unwrap();
    assert_eq!(gift[3], "20,00");
}

#[test]
fn the_channel_comes_from_the_signed_cost_center_and_a_refund_follows_its_ticket() {
    let (_m, _out, summary) = export_month();
    let tva = rows(&summary.folder, "tva.csv");
    let get = |channel: &str, label: &str| {
        tva.iter()
            .find(|r| r[0] == "2026-10-01" && r[2] == channel && r[3] == label)
            .map(|r| r[9].clone())
    };
    assert_eq!(get("sur_place", "B").as_deref(), Some("39,20"));
    // Takeaway A: sale 12.10, its refund -12.10 (no cost center of its own) and 10.03.
    assert_eq!(get("emporter", "A").as_deref(), Some("10,03"));
    assert_eq!(get("en_ligne", "A").as_deref(), Some("20,00"));
    assert_eq!(get("emporter", "C").as_deref(), Some("10,60"));
    assert_eq!(get("sur_place", "A"), None);
    let refunds = rows(&summary.folder, "remboursements.csv");
    assert_eq!(refunds.len(), 2);
    assert_eq!(refunds[1][4], "emporter");
    assert_eq!(refunds[1][6], "-12,10");
    assert!(refunds[1][8].starts_with('N'));
}

#[test]
fn gift_vouchers_sold_and_used_are_listed() {
    let (_m, _out, summary) = export_month();
    let rows = rows(&summary.folder, "bons_cadeaux.csv");
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows[1][2], "vente");
    assert_eq!(rows[1][5], "50,00");
    assert_eq!(rows[2][2], "utilisation");
    assert_eq!(rows[2][5], "20,00");
}

#[test]
fn the_invoice_of_the_m150_shape_is_listed_with_the_vat_of_its_tickets() {
    let (_m, _out, summary) = export_month();
    let rows = rows(&summary.folder, "factures.csv");
    assert_eq!(rows.len(), 2);
    let header = &rows[0];
    let col = |name: &str| header.iter().position(|h| h == name).unwrap();
    let row = &rows[1];
    assert_eq!(row[col("numero_facture")], "20260001");
    assert_eq!(row[col("tva_client")], "BE0308357159");
    // s2 (B 11.20: 10.00 + 1.20) and s4 (B 28.00: 25.00 + 3.00).
    assert_eq!(row[col("base_B")], "35,00");
    assert_eq!(row[col("tva_B")], "4,20");
    assert_eq!(row[col("total")], "39,20");
    assert_eq!(row[col("tickets_introuvables")], "0");
    assert!(row[col("tickets")].starts_with('N'));
}

#[test]
fn cash_movements_are_signed_and_free_text_cannot_start_a_formula() {
    let (_m, _out, summary) = export_month();
    let rows = rows(&summary.folder, "mouvements_caisse.csv");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1][4], "entree");
    assert_eq!(rows[1][7], "100,00");
    assert_eq!(rows[2][4], "sortie");
    assert_eq!(rows[2][6], "'=Banque");
    assert_eq!(rows[2][8], "-4000");
}

#[test]
fn each_journal_entry_balances_and_the_gift_voucher_is_a_liability() {
    let (_m, _out, summary) = export_month();
    let balances = journal_balances(&summary.folder);
    assert_eq!(balances.len(), 2);
    for (doc, debit, credit) in &balances {
        assert_eq!(debit, credit, "document {doc}");
    }
    assert_eq!(balances[0], ("1".to_string(), 12985, 12985));
    let journal = read(&summary.folder, "journal.csv");
    // Gift vouchers sold: credit 489000, no sales account for the X line.
    assert!(
        journal.contains(";489000;Bons cadeaux vendus;0.00;50.00;X"),
        "{journal}"
    );
    assert!(!journal.contains(";700400;"), "{journal}");
    // Vouchers redeemed: debit 489000.
    assert!(journal.contains(";489000;Encaissements bons cadeaux utilises;20.00;0.00;"));
    // Cash rounding of two cents in favour of the till.
    assert!(journal.contains(";757000;Arrondis de caisse;0.00;0.02;"));
}

#[test]
fn an_accounts_file_changes_the_account_numbers() {
    let m = month_archive();
    let out = tempfile::tempdir().unwrap();
    let file = out.path().join("comptes.json");
    std::fs::write(
        &file,
        r#"{"journal":"CAI","sales":{"A":"701000"},"payments":{"especes":"550000"}}"#,
    )
    .unwrap();
    let mut req = request(&m.synth.path, &out.path().join("o"), month("2026-10"));
    req.accounts = Some(file);
    let summary = pos_accounting::export(&req).unwrap();
    let journal = read(&summary.folder, "journal.csv");
    assert!(journal.contains(";CAI;1;550000;"), "{journal}");
    assert!(journal.contains(";CAI;1;701000;"), "{journal}");
    assert!(journal.contains(";700100;"), "defaults stay for the rest");
    assert_eq!(
        Accounts::load(&out.path().join("comptes.json"))
            .unwrap()
            .journal,
        "CAI"
    );
}

#[test]
fn the_winbooks_and_yuki_profiles_write_their_layouts() {
    let m = month_archive();
    for (profile, header, date) in [
        (Profile::Winbooks, "DBKCODE;DBKTYPE;DOCNUMBER", "2026/10/01"),
        (Profile::Yuki, "Date;Journal;Reference", "01-10-2026"),
    ] {
        let out = tempfile::tempdir().unwrap();
        let mut req = request(&m.synth.path, out.path(), month("2026-10"));
        req.profile = profile;
        let summary = pos_accounting::export(&req).unwrap();
        let journal = read(&summary.folder, "journal.csv");
        assert!(journal.starts_with(header), "{journal}");
        assert!(journal.contains(date), "{journal}");
    }
}

#[test]
fn decimal_point_changes_the_text_columns_but_not_the_cents() {
    let m = month_archive();
    let out = tempfile::tempdir().unwrap();
    let mut req = request(&m.synth.path, out.path(), month("2026-10"));
    req.decimal = Decimal::Point;
    let summary = pos_accounting::export(&req).unwrap();
    let jours = rows(&summary.folder, "jours.csv");
    assert_eq!(jours[1], ["2026-10-01", "1", "7", "1", "129.83", "12983"]);
}

#[test]
fn a_date_range_picks_the_days_inside_it() {
    let m = month_archive();
    let out = tempfile::tempdir().unwrap();
    let summary = run(
        &m.synth.path,
        out.path(),
        PeriodSpec::Range {
            from: "2026-10-02".into(),
            to: "2026-10-02".into(),
        },
    )
    .unwrap();
    assert_eq!(summary.days, vec![("2026-10-02".to_string(), 2)]);
    assert_eq!(summary.left_out.len(), 0);
    assert!(
        summary
            .folder
            .ends_with("export-comptable-2026-10-02-2026-10-02")
    );
}

#[test]
fn a_range_of_open_periods_only_is_refused_and_an_empty_range_too() {
    let m = month_archive();
    let out = tempfile::tempdir().unwrap();
    let range = |from: &str, to: &str| PeriodSpec::Range {
        from: from.into(),
        to: to.into(),
    };
    let err = run(&m.synth.path, out.path(), range("2026-10-03", "2026-10-03")).unwrap_err();
    assert_eq!(err.code(), "period_open");
    let err = run(&m.synth.path, out.path(), range("2026-11-01", "2026-11-30")).unwrap_err();
    assert_eq!(err.code(), "empty_range");
    assert_eq!(
        std::fs::read_dir(out.path()).unwrap().count(),
        0,
        "nothing written"
    );
}

#[test]
fn the_manifest_lists_every_other_file_with_its_hash() {
    let (_m, _out, summary) = export_month();
    let manifest = read(&summary.folder, "MANIFEST.sha256");
    let mut listed = Vec::new();
    for line in manifest.lines() {
        let (hash, name) = line.split_once("  ").unwrap();
        assert_eq!(sha256_of(&summary.folder.join(name)), hash, "{name}");
        listed.push(name.to_string());
    }
    let mut sorted = listed.clone();
    sorted.sort();
    assert_eq!(listed, sorted, "sorted by name");
    assert_eq!(listed.len(), 10);
    assert!(!listed.contains(&"MANIFEST.sha256".to_string()));
    assert_eq!(summary.files.last().unwrap(), "MANIFEST.sha256");
}

#[test]
fn an_existing_folder_is_never_overwritten() {
    let m = month_archive();
    let out = tempfile::tempdir().unwrap();
    let first = run(&m.synth.path, out.path(), month("2026-10")).unwrap();
    let before = read(&first.folder, "jours.csv");
    let err = run(&m.synth.path, out.path(), month("2026-10")).unwrap_err();
    assert_eq!(err.code(), "out_exists");
    assert_eq!(read(&first.folder, "jours.csv"), before);
    assert!(
        !out.path()
            .join("export-comptable-2026-10-01-2026-10-31.partial")
            .exists()
    );
}

#[test]
fn the_source_is_read_only_and_leaves_no_trace_beside_it() {
    let m = month_archive();
    // A snapshot in a folder of its own, as an export delivers it.
    let copy_dir = tempfile::tempdir().unwrap();
    let copy = copy_dir.path().join("pos.db");
    snapshot(&rusqlite::Connection::open(&m.synth.path).unwrap(), &copy);
    let before_hash = sha256_of(&copy);
    let before_mtime = std::fs::metadata(&copy).unwrap().modified().unwrap();
    let out = tempfile::tempdir().unwrap();
    run(&copy, out.path(), month("2026-10")).unwrap();
    assert_eq!(sha256_of(&copy), before_hash);
    assert_eq!(
        std::fs::metadata(&copy).unwrap().modified().unwrap(),
        before_mtime
    );
    let names: Vec<String> = std::fs::read_dir(copy_dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["pos.db"], "no journal or WAL file created");
}

#[test]
fn a_copy_of_a_live_database_with_a_wal_file_is_refused() {
    let m = month_archive();
    let dir = tempfile::tempdir().unwrap();
    let copy = dir.path().join("pos.db");
    std::fs::copy(&m.synth.path, &copy).unwrap();
    std::fs::write(dir.path().join("pos.db-wal"), b"pending frames").unwrap();
    let err = run(&copy, &dir.path().join("o"), month("2026-10")).unwrap_err();
    assert_eq!(err.code(), "db_unreadable");
    assert!(err.to_string().contains("WAL"), "{err}");
}

#[test]
fn the_source_can_be_a_read_only_file() {
    let m = month_archive();
    let mut perms = std::fs::metadata(&m.synth.path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&m.synth.path, perms).unwrap();
    let out = tempfile::tempdir().unwrap();
    run(&m.synth.path, out.path(), month("2026-10")).unwrap();
}

#[test]
fn a_source_without_migration_4_or_not_a_database_is_refused() {
    let m = month_archive();
    let out = tempfile::tempdir().unwrap();

    let old = out.path().join("old.db");
    {
        let conn = rusqlite::Connection::open(&m.synth.path).unwrap();
        snapshot(&conn, &old);
    }
    rusqlite::Connection::open(&old)
        .unwrap()
        .execute("DELETE FROM _migrations_applied WHERE version = 4", [])
        .unwrap();
    let err = run(&old, &out.path().join("a"), month("2026-10")).unwrap_err();
    assert_eq!(err.code(), "not_a_pili_db");

    let text = out.path().join("notes.txt");
    std::fs::write(
        &text,
        "hello, this is not a database at all, just a long enough text",
    )
    .unwrap();
    let err = run(&text, &out.path().join("b"), month("2026-10")).unwrap_err();
    assert_eq!(err.code(), "not_a_pili_db");

    let err = run(
        &out.path().join("missing.db"),
        &out.path().join("c"),
        month("2026-10"),
    )
    .unwrap_err();
    assert_eq!(err.code(), "db_unreadable");

    let other = out.path().join("other.db");
    rusqlite::Connection::open(&other)
        .unwrap()
        .execute_batch("CREATE TABLE t (a INTEGER);")
        .unwrap();
    let err = run(&other, &out.path().join("d"), month("2026-10")).unwrap_err();
    assert_eq!(err.code(), "not_a_pili_db");
}

#[test]
fn an_export_folder_is_checked_against_its_manifest() {
    let m = month_archive();
    let root = tempfile::tempdir().unwrap();
    let spf = root.path().join("export");
    std::fs::create_dir(&spf).unwrap();
    std::fs::copy(&m.synth.path, spf.join("pos.db")).unwrap();
    std::fs::write(spf.join("notes.txt"), "side file").unwrap();
    let manifest = format!(
        "{}  notes.txt\n{}  pos.db\n",
        sha256_of(&spf.join("notes.txt")),
        sha256_of(&spf.join("pos.db"))
    );
    std::fs::write(spf.join("MANIFEST.sha256"), manifest).unwrap();

    let out = root.path().join("out");
    let summary = run(&spf, &out, month("2026-10")).unwrap();
    assert!(read(&summary.folder, "LISEZMOI.txt").contains("MANIFEST.sha256 : oui"));
    assert!(read(&summary.folder, "LISEZMOI.txt").contains(&sha256_of(&spf.join("pos.db"))));

    // A file changed after the manifest was written.
    std::fs::write(spf.join("notes.txt"), "changed").unwrap();
    let err = run(&spf, &root.path().join("out2"), month("2026-10")).unwrap_err();
    assert_eq!(err.code(), "manifest_mismatch");

    // A listed file missing.
    std::fs::remove_file(spf.join("notes.txt")).unwrap();
    let err = run(&spf, &root.path().join("out3"), month("2026-10")).unwrap_err();
    assert_eq!(err.code(), "manifest_mismatch");

    // A folder with no manifest is read without the check.
    std::fs::remove_file(spf.join("MANIFEST.sha256")).unwrap();
    let summary = run(&spf, &root.path().join("out4"), month("2026-10")).unwrap();
    assert!(read(&summary.folder, "LISEZMOI.txt").contains("MANIFEST.sha256 : non applicable"));
}

#[test]
fn a_manifest_must_cover_pos_db_and_may_use_the_binary_form() {
    let m = month_archive();
    let root = tempfile::tempdir().unwrap();
    let spf = root.path().join("export");
    std::fs::create_dir(&spf).unwrap();
    std::fs::copy(&m.synth.path, spf.join("pos.db")).unwrap();
    std::fs::write(spf.join("notes.txt"), "side file").unwrap();

    // A manifest that lists nothing proves nothing about pos.db.
    std::fs::write(spf.join("MANIFEST.sha256"), "").unwrap();
    let err = run(&spf, &root.path().join("o1"), month("2026-10")).unwrap_err();
    assert_eq!(err.code(), "manifest_mismatch");

    // Neither does a manifest that lists other files only.
    let other = format!("{}  notes.txt\n", sha256_of(&spf.join("notes.txt")));
    std::fs::write(spf.join("MANIFEST.sha256"), other).unwrap();
    let err = run(&spf, &root.path().join("o2"), month("2026-10")).unwrap_err();
    assert_eq!(err.code(), "manifest_mismatch");

    // `sha256sum --binary` writes `hash *name`.
    let binary = format!("{} *pos.db\n", sha256_of(&spf.join("pos.db")));
    std::fs::write(spf.join("MANIFEST.sha256"), binary).unwrap();
    let summary = run(&spf, &root.path().join("o3"), month("2026-10")).unwrap();
    assert!(read(&summary.folder, "LISEZMOI.txt").contains("MANIFEST.sha256 : oui"));
}

#[test]
fn a_tampered_amount_fails_the_reconciliation_and_writes_nothing() {
    let m = month_archive();
    let out = tempfile::tempdir().unwrap();
    let tampered = out.path().join("tampered.db");
    {
        let conn = rusqlite::Connection::open(&m.synth.path).unwrap();
        snapshot(&conn, &tampered);
    }
    let conn = rusqlite::Connection::open(&tampered).unwrap();
    conn.execute_batch("DROP TRIGGER fiscal_events_no_rewrite;")
        .unwrap();
    // The cash payment of the very first sale is raised by one cent.
    let (id, json): (i64, String) = conn
        .query_row(
            "SELECT id, event_data_json FROM fiscal_events
             WHERE event_type = 'N' AND event_data_json LIKE '%\"amount\":12.1%' ORDER BY id LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value["financials"][0]["amount"] = serde_json::json!(12.11);
    conn.execute(
        "UPDATE fiscal_events SET event_data_json = ?1 WHERE id = ?2",
        rusqlite::params![value.to_string(), id],
    )
    .unwrap();
    drop(conn);

    let target = out.path().join("o");
    let err = run(&tampered, &target, month("2026-10")).unwrap_err();
    assert_eq!(err.code(), "reconciliation_failed");
    assert_eq!(err.exit_code(), 2);
    let text = err.to_string();
    assert!(
        text.contains("2026-10-01") && text.contains("payments.CASH.PAYMENT"),
        "{text}"
    );
    assert!(!target.exists() || std::fs::read_dir(&target).unwrap().count() == 0);
}

#[test]
fn a_tampered_line_price_fails_on_the_turnover() {
    let m = month_archive();
    let out = tempfile::tempdir().unwrap();
    let tampered = out.path().join("tampered.db");
    {
        let conn = rusqlite::Connection::open(&m.synth.path).unwrap();
        snapshot(&conn, &tampered);
    }
    let conn = rusqlite::Connection::open(&tampered).unwrap();
    conn.execute_batch("DROP TRIGGER fiscal_events_no_rewrite;")
        .unwrap();
    conn.execute(
        "UPDATE fiscal_events SET event_data_json = replace(event_data_json, '\"price\":24.2', '\"price\":24.3')
         WHERE event_type = 'N'",
        [],
    )
    .unwrap();
    drop(conn);
    let err = run(&tampered, &out.path().join("o"), month("2026-10")).unwrap_err();
    let text = err.to_string();
    assert_eq!(err.code(), "reconciliation_failed", "{text}");
    assert!(
        text.contains("turnover") && text.contains("2026-10-02"),
        "{text}"
    );
}

#[test]
fn a_missing_event_changes_the_ticket_count() {
    let m = month_archive();
    let out = tempfile::tempdir().unwrap();
    let cut = out.path().join("cut.db");
    {
        let conn = rusqlite::Connection::open(&m.synth.path).unwrap();
        snapshot(&conn, &cut);
    }
    let conn = rusqlite::Connection::open(&cut).unwrap();
    conn.execute_batch("DROP TRIGGER fiscal_events_no_delete;")
        .unwrap();
    conn.execute(
        "DELETE FROM fiscal_events WHERE id = (SELECT max(id) FROM fiscal_events
            WHERE event_type = 'N' AND fdm_response_json IS NOT NULL
              AND json_extract(event_data_json, '$.bookingPeriodId') = 'p1')",
        [],
    )
    .unwrap();
    drop(conn);
    let err = run(&cut, &out.path().join("o"), month("2026-10")).unwrap_err();
    assert_eq!(err.code(), "reconciliation_failed");
    assert!(err.to_string().contains("tickets"), "{err}");
}

#[test]
fn the_files_hold_no_personal_data() {
    let (_m, _out, summary) = export_month();
    for name in summary.files.iter().filter(|n| *n != "MANIFEST.sha256") {
        let text = read(&summary.folder, name);
        assert!(!text.contains("75061189702"), "{name}: employee number");
        assert!(!text.contains("00000000029"), "{name}: robot user");
        assert!(!text.contains('\u{2014}'), "{name}: em-dash");
    }
}

#[test]
fn the_unused_helpers_stay_referenced() {
    let m = month_archive();
    // The refs the scenario returns are signed references a refund or invoice can name.
    for r in [&m.s1, &m.s2, &m.s4] {
        assert_eq!(r["fdmId"], FDM_ID);
    }
}

#[test]
fn the_published_m150_invoice_is_listed_even_when_its_tickets_are_elsewhere() {
    let dir = pos_engine_testkit::workspace_root()
        .join("docs/20260821_UC_BasicSet_POS_FDM_1.8/M150_signInvoice");
    let path = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().ends_with("Variables.json"))
        .expect("M150 variables");
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();

    let mut s = Synth::new();
    s.open_period("p1", "2026-10-01");
    s.sale(
        "p1",
        SaleSpec::new(&[L("10", "A", 12.10)], &[P("CASH", "PAYMENT", 12.10)]),
    );
    s.invoice_raw("p1", fixture["data"].clone());
    s.close("p1", 1);
    let out = tempfile::tempdir().unwrap();
    let summary = run(&s.path, out.path(), month("2026-10")).unwrap();
    let fact = rows(&summary.folder, "factures.csv");
    assert_eq!(fact.len(), 2);
    let header = &fact[0];
    let col = |name: &str| header.iter().position(|h| h == name).unwrap();
    assert_eq!(fact[1][col("numero_facture")], "20240005");
    assert_eq!(fact[1][col("tva_client")], "BE0308357159");
    // The two tickets it names are not in this archive: listed, flagged, no amounts.
    assert_eq!(fact[1][col("tickets")], "N14 N16");
    assert_eq!(fact[1][col("tickets_introuvables")], "2");
    assert_eq!(fact[1][col("total")], "0,00");
    // An invoice is not a sale: the day still has its one ticket.
    assert_eq!(rows(&summary.folder, "jours.csv")[1][2], "1");
}
