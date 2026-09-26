//! Printer, cash drawer and back-office commands, end to end.
//!
//! The printer is a TCP sink that records every job, so these tests
//! check what reaches the paper and when the drawer opens:
//!
//! - the drawer opens for cash sales, cash refunds and the signed F
//!   event only - never for a card sale, never when the FDM refused;
//! - the pre-bill prints a PRO FORMA ticket, a table order prints
//!   nothing, training tickets carry their TRAINING marks;
//! - X, user X and Z reports print;
//! - tables, Z history, the Z reminder, the event list and the
//!   database backup behave as the screens expect.

mod common;

use common::*;

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use moyun_pos::commands::copy::do_list_fiscal_events;
use moyun_pos::commands::financial::do_open_drawer_fiscal;
use moyun_pos::commands::refund::{RefundItem, do_refund_checkout};
use moyun_pos::commands::report::{
    do_backup_database, do_generate_user_x_report, do_generate_x_report, do_generate_z_report,
    do_list_z_reports, do_needs_z_report,
};
use moyun_pos::commands::tables::{do_list_tables, do_sign_pre_bill, do_sign_table_order};
use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
use pos_fiscal::types::EventLabel;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

const INSZ: &str = "12345678901";

fn seed_catalogue(app: &TestApp) {
    let conn = app.state.db.lock().unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO users (insz, name, role, active) VALUES (?1, 'Anna', 'admin', 1)",
        [INSZ],
    )
    .unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO categories (id, sort_order, active) VALUES (1, 0, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
         VALUES (1, 1, 1000, 'B', 0, 1)",
        [],
    )
    .unwrap();
}

fn open_order(app: &TestApp, table: Option<i32>, lines: i32) -> i64 {
    seed_catalogue(app);
    let conn = app.state.db.lock().unwrap();
    let order =
        pos_db::queries::orders::create_order(&conn, OrderType::DineIn, table, INSZ).unwrap();
    for _ in 0..lines {
        pos_db::queries::orders::add_order_line(
            &conn,
            NewOrderLine {
                order_id: order.id,
                menu_item_id: 1,
                quantity: 1,
                unit_price_cents: 1000,
                vat_code: VatCode::B,
                name: "Burger",
                modifiers_json: None,
                department_id: "1",
                department_name: "Cuisine",
            },
        )
        .unwrap();
    }
    order.id
}

async fn sale(app: &TestApp, method: CheckoutPaymentMethod) -> i64 {
    let order_id = open_order(app, Some(1), 1);
    mock_from_fixture(app, &load_uc(&basic_set_root(), "M110")).await;
    let result = do_checkout(&app.state, order_id, method).await.unwrap();
    assert!(result.success, "{:?}", result.error);
    result.fiscal_event_id.unwrap()
}

async fn with_printer() -> (TestApp, PrinterSink) {
    let app = TestApp::new().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    (app, printer)
}

async fn fdm_refuses(app: &TestApp) {
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .mount(&app.fdm)
        .await;
}

// ── Cash drawer ────────────────────────────────────────────────

#[tokio::test]
// covers: FAQ_Q405
async fn a_cash_sale_prints_and_opens_the_drawer() {
    let (app, printer) = with_printer().await;
    sale(&app, CheckoutPaymentMethod::Cash).await;
    assert_eq!(printer.settle(1).await.len(), 1);
    assert_eq!(printer.drawer_kicks(), 1);
    // The sale opens the drawer itself: no signDrawerOpen with it.
    assert_eq!(app.counter_for(EventLabel::F), 0);
}

#[tokio::test]
async fn a_card_sale_prints_but_keeps_the_drawer_shut() {
    let (app, printer) = with_printer().await;
    sale(&app, CheckoutPaymentMethod::Card).await;
    assert_eq!(printer.settle(1).await.len(), 1);
    assert_eq!(printer.drawer_kicks(), 0);
}

#[tokio::test]
async fn a_refused_sale_neither_prints_nor_opens_the_drawer() {
    let (app, printer) = with_printer().await;
    let order_id = open_order(&app, Some(1), 1);
    fdm_refuses(&app).await;
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(!result.success);
    assert!(printer.settle(0).await.is_empty());
    assert_eq!(printer.drawer_kicks(), 0);
}

#[tokio::test]
async fn a_cash_refund_prints_and_opens_the_drawer() {
    let (app, printer) = with_printer().await;
    let original = sale(&app, CheckoutPaymentMethod::Card).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M111")).await;
    let refund = do_refund_checkout(
        &app.state,
        original,
        vec![RefundItem {
            menu_item_id: 1,
            quantity: 1,
        }],
        INSZ.to_string(),
    )
    .await
    .unwrap();
    assert!(refund.success, "{:?}", refund.error);
    let receipts = printer.settle(2).await;
    assert_eq!(receipts.len(), 2);
    assert_eq!(printer.drawer_kicks(), 1, "only the refund paid cash");
}

#[tokio::test]
// covers: FAQ_Q405
async fn the_drawer_opens_with_its_signed_event() {
    let (app, printer) = with_printer().await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M131")).await;
    let result = do_open_drawer_fiscal(&app.state, INSZ.to_string())
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    assert_eq!(app.counter_for(EventLabel::F), 1);
    printer.settle(0).await;
    assert_eq!(printer.drawer_kicks(), 1);
    assert!(
        printer.receipts().is_empty(),
        "a drawer event prints nothing"
    );
}

#[tokio::test]
async fn the_drawer_stays_shut_when_the_fdm_refuses_its_event() {
    let (app, printer) = with_printer().await;
    fdm_refuses(&app).await;
    let result = do_open_drawer_fiscal(&app.state, INSZ.to_string())
        .await
        .unwrap();
    assert!(!result.success);
    assert_eq!(app.counter_for(EventLabel::F), 0);
    printer.settle(0).await;
    assert_eq!(
        printer.drawer_kicks(),
        0,
        "no drawer opening without a signed F"
    );
}

// ── Tickets ────────────────────────────────────────────────────

#[tokio::test]
async fn the_sale_receipt_carries_the_fdm_control_data() {
    let (app, printer) = with_printer().await;
    sale(&app, CheckoutPaymentMethod::Card).await;
    let receipt = &printer.settle(1).await[0];
    for needle in ["Burger", "FOD01000001", M110_SHORT_SIGNATURE] {
        assert!(receipt.contains(needle), "missing {needle}");
    }
}

#[tokio::test]
async fn a_table_order_prints_nothing_and_the_pre_bill_prints_pro_forma() {
    let (app, printer) = with_printer().await;
    let order_id = open_order(&app, Some(3), 2);

    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;
    do_sign_table_order(&app.state, order_id).await.unwrap();
    assert!(
        printer.settle(0).await.is_empty(),
        "an order registration is not printed"
    );

    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M123")).await;
    do_sign_pre_bill(&app.state, order_id).await.unwrap();
    let receipts = printer.settle(1).await;
    assert_eq!(receipts.len(), 1);
    assert!(receipts[0].contains("PRO FORMA"));
    assert!(receipts[0].contains("CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE"));
    assert_eq!(printer.drawer_kicks(), 0);
}

#[tokio::test]
// covers: FAQ_Q507
async fn vi_training_a_training_sale_prints_its_training_marks() {
    let (app, printer) = with_printer().await;
    app.state.config.lock().unwrap().training_mode = true;
    let order_id = open_order(&app, Some(1), 1);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M171_")).await;
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let receipt = &printer.settle(1).await[0];
    assert!(receipt.contains("TRAINING"));
    assert!(receipt.contains("CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE"));
    // Not a VAT ticket: neither its title nor a verification QR code.
    assert!(!receipt.contains("TICKET DE CAISSE TVA\n"), "{receipt}");
    assert!(!receipt.contains("[QR"), "no verification URL for T");
    // The FDM's control data of the T event.
    assert!(receipt.contains("INFORMATIONS FDM"), "{receipt}");
    assert!(receipt.contains("Event: T;"), "{receipt}");
}

// ── Reports ────────────────────────────────────────────────────

#[tokio::test]
async fn x_and_user_x_reports_print() {
    let (app, printer) = with_printer().await;
    sale(&app, CheckoutPaymentMethod::Card).await;

    mock_from_fixture(&app, &load_uc(&reports_root(), "M180")).await;
    assert!(
        do_generate_x_report(&app.state, INSZ.into())
            .await
            .unwrap()
            .success
    );
    mock_from_fixture(&app, &load_uc(&reports_root(), "M182")).await;
    assert!(
        do_generate_user_x_report(&app.state, INSZ.into())
            .await
            .unwrap()
            .success
    );
    assert_eq!(printer.settle(3).await.len(), 3, "sale + X + user X");
}

#[tokio::test]
async fn a_z_report_prints_and_is_listed_in_the_history() {
    let (app, printer) = with_printer().await;
    let dir = tempfile::tempdir().unwrap();
    app.state.config.lock().unwrap().backup_dir = Some(dir.path().to_string_lossy().into());
    sale(&app, CheckoutPaymentMethod::Card).await;
    assert!(do_list_z_reports(&app.state).unwrap().is_empty());

    mock_from_fixture(&app, &load_uc(&reports_root(), "M184")).await;
    let z = do_generate_z_report(&app.state, INSZ.into()).await.unwrap();
    assert!(z.success, "{:?}", z.error);
    assert!(printer.settle(2).await.len() >= 2, "sale + Z");

    let history = do_list_z_reports(&app.state).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(Some(history[0].sequence_number), z.report_no);
    assert!(history[0].fiscal_event_id.is_some());

    // The Z makes a backup of the database.
    let backups: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert_eq!(backups.len(), 1);
}

#[tokio::test]
async fn a_training_z_is_not_listed_in_the_history() {
    let (app, _printer) = with_printer().await;
    app.state.config.lock().unwrap().training_mode = true;
    let fixture = load_uc(&reports_root(), "M184");
    mock_response(&app, in_training(read_response_envelope(&fixture))).await;
    let z = do_generate_z_report(&app.state, INSZ.into()).await.unwrap();
    assert!(z.success, "{:?}", z.error);
    assert!(z.report_no.is_none());
    assert!(do_list_z_reports(&app.state).unwrap().is_empty());
}

#[tokio::test]
async fn the_z_reminder_needs_an_open_period_from_an_earlier_day() {
    let app = TestApp::new().await;
    assert!(!do_needs_z_report(&app.state).unwrap(), "nothing recorded");

    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO booking_periods (id, booking_date) VALUES ('old-period', '2020-01-01')",
            [],
        )
        .unwrap();
    assert!(
        do_needs_z_report(&app.state).unwrap(),
        "yesterday's day not closed"
    );
}

// ── Back office ────────────────────────────────────────────────

#[tokio::test]
async fn tables_show_only_orders_with_lines() {
    let app = TestApp::new().await;
    let busy = open_order(&app, Some(2), 3);
    open_order(&app, Some(4), 0); // tapped and left: shown free
    let paid = open_order(&app, Some(5), 1);
    app.state
        .db
        .lock()
        .unwrap()
        .execute("UPDATE orders SET status = 'closed' WHERE id = ?1", [paid])
        .unwrap();

    let tables = do_list_tables(&app.state, 6).unwrap();
    assert_eq!(tables.len(), 6);
    assert_eq!(
        tables.iter().map(|t| t.table_number).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5, 6]
    );
    let t2 = &tables[1];
    assert_eq!(t2.status, "occupied");
    assert_eq!(t2.order_id, Some(busy));
    assert_eq!(t2.line_count, 3);
    assert_eq!(t2.total_cents, 3000);
    for t in [&tables[0], &tables[2], &tables[3], &tables[4], &tables[5]] {
        assert_eq!(t.status, "free", "table {}", t.table_number);
        assert!(t.order_id.is_none());
        assert_eq!(t.total_cents, 0);
    }
}

#[tokio::test]
async fn no_tables_when_none_are_configured() {
    let app = TestApp::new().await;
    assert!(do_list_tables(&app.state, 0).unwrap().is_empty());
}

#[tokio::test]
async fn the_event_list_is_newest_first_and_limited() {
    let (app, _printer) = with_printer().await;
    sale(&app, CheckoutPaymentMethod::Card).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M131")).await;
    do_open_drawer_fiscal(&app.state, INSZ.to_string())
        .await
        .unwrap();

    let all = do_list_fiscal_events(&app.state, None).unwrap();
    let all = all.as_array().unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0]["eventType"], "F", "newest first");
    assert_eq!(all[1]["eventType"], "N");
    assert_eq!(all[1]["shortSignature"], M110_SHORT_SIGNATURE);

    let one = do_list_fiscal_events(&app.state, Some(1)).unwrap();
    assert_eq!(one.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn a_backup_is_a_readable_copy_of_the_archive() {
    let app = TestApp::new().await;
    let dir = tempfile::tempdir().unwrap();
    app.state.config.lock().unwrap().backup_dir = Some(dir.path().to_string_lossy().into());
    sale(&app, CheckoutPaymentMethod::Card).await;

    let path = do_backup_database(&app.state).unwrap();
    assert!(path.starts_with(dir.path().to_str().unwrap()));
    let copy = rusqlite::Connection::open(&path).unwrap();
    let (events, sig): (i64, String) = copy
        .query_row(
            "SELECT COUNT(*), MAX(short_signature) FROM fiscal_events",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(events, 1);
    assert_eq!(sig, M110_SHORT_SIGNATURE);
    // The copy keeps the append-only protection of the archive.
    assert!(copy.execute("DELETE FROM fiscal_events", []).is_err());
}

#[tokio::test]
async fn a_backup_into_an_unwritable_place_fails_cleanly() {
    let app = TestApp::new().await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("not-a-dir");
    std::fs::write(&file, "x").unwrap();
    app.state.config.lock().unwrap().backup_dir = Some(file.to_string_lossy().into());
    assert!(do_backup_database(&app.state).is_err());
}
