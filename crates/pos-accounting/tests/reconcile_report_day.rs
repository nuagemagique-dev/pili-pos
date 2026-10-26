//! The export on real archives built by the engine.
//!
//! 1. The service day of the SPF Reports v1.3 scenario (T1 to T8, Z of M184), played
//!    through the real commands: the export's day must equal the Z the POS signed.
//! 2. A mixed day against the admission mock FDM: split payments with meal vouchers,
//!    cash rounding, a refund, money in and out, plus everything the export must leave
//!    out (a pro forma, a training sale, a copy, an open order, the sale of a period no
//!    Z closed yet).

mod common;

use pos_engine_testkit as kit;

use common::*;
use kit::TestApp;
use serde_json::Value;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::checkout::{
    CheckoutPaymentMethod, PaymentPart, do_checkout, do_checkout_split,
};
use pos_engine::commands::copy::do_copy_ticket;
use pos_engine::commands::financial::{MoneyDirection, do_money_in_out};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::refund::{RefundItem, do_refund_checkout};
use pos_engine::commands::report::{do_generate_x_report, do_generate_z_report};
use pos_engine::commands::tables::{do_sign_pre_bill, do_sign_table_order};

// ── 1. The Reports 1.3 scenario ─────────────────────────────────────

const U1: &str = "75061189702";
const U2: &str = "84022899837";

fn read_json(dir: &str, suffix: &str) -> Value {
    let dir = kit::reports_root().join(dir);
    let path = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().ends_with(suffix))
        .unwrap_or_else(|| panic!("no *{suffix} in {}", dir.display()));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

async fn fdm_signs(app: &TestApp, t: &str) {
    kit::mock_response(
        app,
        read_json(&format!("InputReport/{t}"), "_Response.json"),
    )
    .await;
}

fn open_order(app: &TestApp, user: &str, order_type: OrderType, table: Option<i32>) -> i64 {
    app.login_as(user, UserRole::Admin);
    let conn = app.state.db.lock().unwrap();
    pos_db::queries::orders::create_order(&conn, order_type, table, user)
        .unwrap()
        .id
}

fn add(app: &TestApp, order_id: i64, menu_item_id: i64, quantity: i32, price: Option<i64>) {
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id,
            quantity,
            modifiers_json: None,
            unit_price_cents: price,
        },
    )
    .expect("add order line");
}

async fn play_reports_day() -> TestApp {
    let app = TestApp::new().await;
    {
        let conn = app.state.db.lock().unwrap();
        pos_engine::install::apply_seed(
            &conn,
            include_str!("../../../src-tauri/seed_spf_test.sql"),
        )
        .expect("seed");
        for (insz, name) in [(U1, "Operateur 1"), (U2, "Operateur 2")] {
            conn.execute(
                "INSERT OR IGNORE INTO users (insz, name, role) VALUES (?1, ?2, 'cashier')",
                rusqlite::params![insz, name],
            )
            .unwrap();
        }
    }
    // T1 table 1 order (P), T2 paid cash (N), T3 refunded (N).
    let table_1 = open_order(&app, U1, OrderType::DineIn, Some(1));
    add(&app, table_1, 10006, 2, None);
    add(&app, table_1, 22001, 1, None);
    fdm_signs(&app, "T1").await;
    do_sign_table_order(&app.state, table_1).await.unwrap();
    fdm_signs(&app, "T2").await;
    let sale = do_checkout(&app.state, table_1, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(sale.success, "T2: {:?}", sale.error);
    let t2 = app.fiscal_events().last().unwrap().id;
    fdm_signs(&app, "T3").await;
    let refund = do_refund_checkout(
        &app.state,
        t2,
        vec![
            RefundItem {
                line_id: None,
                menu_item_id: 10006,
                quantity: 2,
            },
            RefundItem {
                line_id: None,
                menu_item_id: 22001,
                quantity: 1,
            },
        ],
        U1.to_string(),
    )
    .await
    .unwrap();
    assert!(refund.success, "T3: {:?}", refund.error);
    // T4 takeaway by card.
    let takeaway = open_order(&app, U2, OrderType::Takeaway, None);
    add(&app, takeaway, 12009, 1, None);
    fdm_signs(&app, "T4").await;
    let sale = do_checkout(&app.state, takeaway, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(sale.success, "T4: {:?}", sale.error);
    // T5 to T8 table 10: two rounds, pre-bill, paid cash.
    let table_10 = open_order(&app, U2, OrderType::DineIn, Some(10));
    add(&app, table_10, 10006, 2, None);
    fdm_signs(&app, "T5").await;
    do_sign_table_order(&app.state, table_10).await.unwrap();
    add(&app, table_10, 22010, 2, Some(2000));
    add(&app, table_10, 15001, 1, None);
    fdm_signs(&app, "T6").await;
    do_sign_table_order(&app.state, table_10).await.unwrap();
    fdm_signs(&app, "T7").await;
    do_sign_pre_bill(&app.state, table_10).await.unwrap();
    fdm_signs(&app, "T8").await;
    let sale = do_checkout(&app.state, table_10, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(sale.success, "T8: {:?}", sale.error);

    app.login_as(U1, UserRole::Admin);
    kit::mock_response(
        &app,
        read_json("M184_signReportTurnoverZUserZ", "_Response.json"),
    )
    .await;
    let z = do_generate_z_report(&app.state, U1.to_string())
        .await
        .unwrap();
    assert!(z.success, "Z: {:?}", z.error);
    app
}

fn booking_dates(app: &TestApp) -> Vec<String> {
    let conn = app.state.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT DISTINCT booking_date FROM booking_periods ORDER BY booking_date")
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn one_day(date: &str) -> pos_accounting::PeriodSpec {
    pos_accounting::PeriodSpec::Range {
        from: date.into(),
        to: date.into(),
    }
}

fn copy_of(app: &TestApp, dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("pos.db");
    snapshot(&app.state.db.lock().unwrap(), &path);
    path
}

#[tokio::test]
async fn the_reports_scenario_day_equals_its_signed_z_to_the_cent() {
    let app = play_reports_day().await;
    let dir = tempfile::tempdir().unwrap();
    let db = copy_of(&app, dir.path());
    let date = booking_dates(&app).remove(0);
    let out = dir.path().join("out");
    let summary = run(&db, &out, one_day(&date)).unwrap();
    assert_eq!(summary.days, vec![(date.clone(), 1)]);
    let f = &summary.folder;

    // The published M184: 4 sale tickets (one is the refund), 102.04 EUR.
    let fixture = read_json("M184_signReportTurnoverZUserZ", "GQL-Variables.json");
    let z = &fixture["turnover"]["turnover"];
    assert_eq!(z["transactions"][0]["ticketCount"], 4);
    let jours = rows(f, "jours.csv");
    assert_eq!(jours[1][2..], ["3", "1", "102,04", "10204"]);

    // VAT per label equals the published Z, whatever the channel split.
    let tva = rows(f, "tva.csv");
    let sum = |label: &str, col: usize| -> i64 {
        tva.iter()
            .skip(1)
            .filter(|r| r[3] == label)
            .map(|r| r[col].parse::<i64>().unwrap())
            .sum()
    };
    assert_eq!(sum("A", 10), 6204);
    assert_eq!(sum("B", 10), 4000);
    for v in z["vats"].as_array().unwrap() {
        let label = v["label"].as_str().unwrap();
        assert_eq!(
            sum(label, 6),
            (v["taxableAmount"].as_f64().unwrap() * 100.0).round() as i64,
            "base {label}"
        );
        assert_eq!(
            sum(label, 8),
            (v["vatAmount"].as_f64().unwrap() * 100.0).round() as i64,
            "VAT {label}"
        );
    }
    // Table sales and the takeaway keep their channel; the refund follows its table.
    let channels: std::collections::BTreeSet<&str> =
        tva.iter().skip(1).map(|r| r[2].as_str()).collect();
    assert_eq!(channels, ["emporter", "sur_place"].into_iter().collect());
    let refunds = rows(f, "remboursements.csv");
    assert_eq!(refunds.len(), 2);
    assert_eq!(refunds[1][4], "sur_place");
    assert_eq!(refunds[1][6], "-52,00");

    // Payments: 99 cash and 3.04 card, as in the published Z.
    let pay = rows(f, "paiements.csv");
    let get = |m: &str| pay.iter().find(|r| r[2] == m).map(|r| r[3].clone());
    assert_eq!(get("especes").as_deref(), Some("99,00"));
    assert_eq!(get("carte").as_deref(), Some("3,04"));

    // The P events (T1, T5, T6, T7) are not sales.
    assert_eq!(rows(f, "rapports_z.csv")[1][4], "4");
    for (doc, debit, credit) in journal_balances(f) {
        assert_eq!(debit, credit, "journal document {doc}");
    }
}

// ── 2. A mixed day ──────────────────────────────────────────────────

const INSZ: &str = "12345678958";

async fn mock_app() -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1), (2, 1, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
                (1, 1, 1200, 'A', 0, 1),
                (2, 2, 2800, 'B', 1, 1),
                (3, 2, 334, 'B', 2, 1);",
        )
        .unwrap();
    app.login_as(INSZ, UserRole::Admin);
    app
}

fn order(app: &TestApp, table: Option<i32>, lines: &[(i64, i32)]) -> i64 {
    let id = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: if table.is_some() {
                OrderType::DineIn
            } else {
                OrderType::Takeaway
            },
            table_number: table,
            user_insz: INSZ.to_string(),
        },
    )
    .unwrap()
    .id;
    for &(item, qty) in lines {
        add(app, id, item, qty, None);
    }
    id
}

fn labels(app: &TestApp) -> Vec<String> {
    app.fiscal_events()
        .iter()
        .map(|e| e.event_label.clone().unwrap())
        .collect()
}

#[tokio::test]
async fn a_mixed_day_reconciles_and_leaves_out_what_is_not_a_signed_sale() {
    let app = mock_app().await;

    // A pro forma (table 4 left), then paid cash: a table sale of 52.00.
    let table = order(&app, Some(4), &[(1, 2), (2, 1)]);
    do_sign_table_order(&app.state, table).await.unwrap();
    let paid = do_checkout(&app.state, table, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(paid.success, "{:?}", paid.error);

    // A takeaway by card (52.00), later partly refunded in cash (12.00).
    let card = order(&app, None, &[(1, 2), (2, 1)]);
    let sale = do_checkout(&app.state, card, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let card_event = sale.fiscal_event_id.unwrap();
    let r = do_refund_checkout(
        &app.state,
        card_event,
        vec![RefundItem {
            line_id: None,
            menu_item_id: 1,
            quantity: 1,
        }],
        INSZ.to_string(),
    )
    .await
    .unwrap();
    assert!(r.success, "{:?}", r.error);

    // A takeaway paid with 5.00 of meal vouchers and the cash rest (3.34 + 12.00 = 15.34).
    let split = order(&app, None, &[(1, 1), (3, 1)]);
    let r = do_checkout_split(
        &app.state,
        split,
        &[
            PaymentPart::of(CheckoutPaymentMethod::MealVoucher, 500),
            PaymentPart::cash(),
        ],
        None,
    )
    .await
    .unwrap();
    assert!(r.success, "{:?}", r.error);

    // Money in and out, and a copy of the card ticket.
    for (cents, direction, reason) in [
        (5000, MoneyDirection::In, "Fond"),
        (2000, MoneyDirection::Out, "Banque"),
    ] {
        let r = do_money_in_out(&app.state, INSZ.into(), cents, direction, reason.into())
            .await
            .unwrap();
        assert!(r.success, "{:?}", r.error);
    }
    let r = do_copy_ticket(&app.state, card_event, INSZ.into())
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);

    // Training: a sale that must not count.
    app.state.config.lock().unwrap().training_mode = true;
    let training = order(&app, None, &[(1, 2), (2, 1)]);
    let r = do_checkout(&app.state, training, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    app.state.config.lock().unwrap().training_mode = false;

    // An open order, never sold.
    let _open = order(&app, None, &[(2, 3)]);

    let x = do_generate_x_report(&app.state, INSZ.into()).await.unwrap();
    assert!(x.success, "{:?}", x.error);
    let z = do_generate_z_report(&app.state, INSZ.into()).await.unwrap();
    assert!(z.success, "{:?}", z.error);

    // The next day starts: a sale in a period no Z closes.
    let next = order(&app, None, &[(1, 1)]);
    let r = do_checkout(&app.state, next, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);

    let seen = labels(&app);
    for needed in ["P", "T", "C", "F", "R"] {
        assert!(
            seen.iter().any(|l| l == needed),
            "{needed} missing in {seen:?}"
        );
    }

    let dir = tempfile::tempdir().unwrap();
    let db = copy_of(&app, dir.path());
    let date = booking_dates(&app).remove(0);
    let summary = run(&db, &dir.path().join("out"), one_day(&date)).unwrap();
    let f = &summary.folder;
    assert_eq!(summary.days.len(), 1);
    assert_eq!(
        summary.left_out,
        vec![date.clone()],
        "the open period is named"
    );
    assert!(read(f, "LISEZMOI.txt").contains(&format!("  - {date}")));

    // 52.00 + 52.00 - 12.00 + 15.34 = 107.34; three sales and one refund.
    let jours = rows(f, "jours.csv");
    assert_eq!(jours[1][2..], ["3", "1", "107,34", "10734"]);

    let pay = rows(f, "paiements.csv");
    let line = |m: &str| pay.iter().find(|r| r[2] == m).cloned();
    assert_eq!(line("carte").unwrap()[3], "52,00");
    assert_eq!(line("cheques_repas").unwrap()[3], "5,00");
    // Cash: 52.00 + 10.34 - 12.00, rounded to 5 cents on the rest of the split sale.
    let cash = line("especes").unwrap();
    assert_eq!(cash[3], "50,34");
    assert_eq!(cash[5], "0,01", "rounding on its own column");

    let moves = rows(f, "mouvements_caisse.csv");
    assert_eq!(moves.len(), 3);
    assert_eq!(
        (moves[1][4].as_str(), moves[1][7].as_str()),
        ("entree", "50,00")
    );
    assert_eq!(
        (moves[2][4].as_str(), moves[2][7].as_str()),
        ("sortie", "-20,00")
    );

    // The takeaway and the table sale keep their channels.
    let tva = rows(f, "tva.csv");
    let channels: std::collections::BTreeSet<&str> =
        tva.iter().skip(1).map(|r| r[2].as_str()).collect();
    assert_eq!(channels, ["emporter", "sur_place"].into_iter().collect());

    // The training sale (52.00), the pro forma and the open order (84.00) changed nothing.
    for name in ["jours.csv", "paiements.csv", "journal.csv"] {
        assert!(!read(f, name).contains("84,00"), "{name}");
    }
    for (doc, debit, credit) in journal_balances(f) {
        assert_eq!(debit, credit, "document {doc}");
    }
    // Cash rounding reaches the journal as a balanced line.
    assert!(read(f, "journal.csv").contains("Arrondis de caisse"));
}

#[tokio::test]
async fn an_export_of_a_period_still_open_is_refused() {
    let app = mock_app().await;
    let id = order(&app, None, &[(1, 1)]);
    let r = do_checkout(&app.state, id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    let dir = tempfile::tempdir().unwrap();
    let db = copy_of(&app, dir.path());
    let date = booking_dates(&app).remove(0);
    let err = run(&db, &dir.path().join("out"), one_day(&date)).unwrap_err();
    assert_eq!(err.code(), "period_open");
}
