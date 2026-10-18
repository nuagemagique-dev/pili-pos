//! Non-fiscal sales reports (V11-D20): the Reports 1.3 service day replayed through the real
//! commands must give a sales report whose total is the turnover of label N the Z printed; hours,
//! refunds, training, price changes, the range and the "nothing is signed" guarantee are checked
//! on that day and on events written straight into the archive.

use pos_engine_testkit as common;

use common::*;
use serde_json::{Value, json};

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::access::{ERR_FORBIDDEN, ERR_NOT_LOGGED_IN};
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::order::{AddOrderLineInput, do_add_order_line};
use pos_engine::commands::refund::{RefundItem, do_refund_checkout};
use pos_engine::commands::report::do_generate_z_report;
use pos_engine::commands::sales_report::{do_print_sales_report, do_sales_report};
use pos_engine::commands::tables::{do_sign_pre_bill, do_sign_table_order};

const U1: &str = "75061189702";
const U2: &str = "84022899837";

fn read_json(dir: &str, suffix: &str) -> Value {
    let dir = reports_root().join(dir);
    let path = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().ends_with(suffix))
        .unwrap_or_else(|| panic!("no *{suffix} in {}", dir.display()));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

async fn fdm_signs(app: &TestApp, t: &str) {
    mock_response(
        app,
        read_json(&format!("InputReport/{t}"), "_Response.json"),
    )
    .await;
}

async fn scenario_app() -> TestApp {
    let app = TestApp::new().await;
    let conn = app.state.db.lock().unwrap();
    pos_engine::install::apply_seed(&conn, include_str!("../../../src-tauri/seed_spf_test.sql"))
        .expect("seed");
    for (insz, name) in [(U1, "Operateur 1"), (U2, "Operateur 2")] {
        conn.execute(
            "INSERT OR IGNORE INTO users (insz, name, role) VALUES (?1, ?2, 'cashier')",
            rusqlite::params![insz, name],
        )
        .unwrap();
    }
    drop(conn);
    app
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

/// T1 to T8 of the Reports 1.3 scenario, as `spf_e2e_report_fixtures.rs` plays them.
async fn play_service_day(app: &TestApp) {
    let table_1 = open_order(app, U1, OrderType::DineIn, Some(1));
    add(app, table_1, 10006, 2, None);
    add(app, table_1, 22001, 1, None);
    fdm_signs(app, "T1").await;
    do_sign_table_order(&app.state, table_1).await.unwrap();

    fdm_signs(app, "T2").await;
    assert!(
        do_checkout(&app.state, table_1, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );
    let t2 = app.fiscal_events().last().unwrap().id;

    fdm_signs(app, "T3").await;
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
    assert!(refund.success, "{:?}", refund.error);

    let takeaway = open_order(app, U2, OrderType::Takeaway, None);
    add(app, takeaway, 12009, 1, None);
    fdm_signs(app, "T4").await;
    assert!(
        do_checkout(&app.state, takeaway, CheckoutPaymentMethod::Card)
            .await
            .unwrap()
            .success
    );

    let table_10 = open_order(app, U2, OrderType::DineIn, Some(10));
    add(app, table_10, 10006, 2, None);
    fdm_signs(app, "T5").await;
    do_sign_table_order(&app.state, table_10).await.unwrap();
    add(app, table_10, 22010, 2, Some(2000));
    add(app, table_10, 15001, 1, None);
    fdm_signs(app, "T6").await;
    do_sign_table_order(&app.state, table_10).await.unwrap();
    fdm_signs(app, "T7").await;
    do_sign_pre_bill(&app.state, table_10).await.unwrap();
    fdm_signs(app, "T8").await;
    assert!(
        do_checkout(&app.state, table_10, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );
    app.login_as(U1, UserRole::Admin);
}

fn today(app: &TestApp) -> String {
    let hour = app.state.config.lock().unwrap().day_end_hour;
    pos_fiscal::event::now_in_belgium()
        .checked_sub_signed(chrono::Duration::hours(i64::from(hour)))
        .unwrap()
        .date_naive()
        .to_string()
}

fn counters(app: &TestApp) -> Vec<(String, i64)> {
    let conn = app.state.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT event_label, current_value FROM fiscal_counters ORDER BY event_label")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

async fn fdm_requests(app: &TestApp) -> usize {
    app.fdm.received_requests().await.unwrap_or_default().len()
}

#[tokio::test]
async fn the_service_day_total_is_the_turnover_n_of_the_z() {
    let app = scenario_app().await;
    play_service_day(&app).await;
    let day = today(&app);

    // Before the Z: the period is open and marked so.
    let open = do_sales_report(&app.state, day.clone(), day.clone()).unwrap();
    assert!(open.open_period);
    assert_eq!(open.business_date, day);
    assert_eq!(open.days.len(), 1);
    assert!(open.days[0].open);
    assert_eq!(open.days[0].z_number, None);
    assert!(!open.days[0].matches_z);

    mock_response(
        &app,
        read_json("M184_signReportTurnoverZUserZ", "_Response.json"),
    )
    .await;
    let z = do_generate_z_report(&app.state, U1.to_string())
        .await
        .unwrap();
    assert!(z.success, "{:?}", z.error);

    let report = do_sales_report(&app.state, day.clone(), day).unwrap();
    assert!(!report.open_period);
    assert_eq!(report.days.len(), 1);
    let d = &report.days[0];
    assert_eq!(d.z_number, Some(1));
    assert!(d.matches_z, "{d:?}");
    assert_eq!(d.z_total_cents, Some(d.total_cents));
    assert_eq!(d.total_cents, report.total_cents);

    // The Z of the scenario: N amount of the published report.
    let fixture = read_json("M184_signReportTurnoverZUserZ", "GQL-Variables.json");
    let n = fixture["turnover"]["turnover"]["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["eventLabel"] == "N")
        .unwrap();
    let published = (n["amount"].as_f64().unwrap() * 100.0).round() as i64;
    assert_eq!(report.total_cents, published);

    // T2, T3 (refund), T4 and T8 are the sales.
    assert_eq!(report.ticket_count, 4);
    assert!(report.refunds_cents < 0, "the refund is negative");
    assert_eq!(
        report
            .by_product
            .iter()
            .map(|p| p.amount_cents)
            .sum::<i64>()
            + report.price_changes_cents,
        report.total_cents
    );
    assert_eq!(
        report.by_hour.iter().map(|h| h.amount_cents).sum::<i64>(),
        report.total_cents
    );
    assert_eq!(report.by_hour.iter().map(|h| h.tickets).sum::<i64>(), 4);
    // The refunded Dry Martini and Burger (T2/T3) cancel out; the dish of the day is sold twice.
    let dish = report
        .by_product
        .iter()
        .find(|p| p.product_id == "22010")
        .unwrap();
    assert_eq!(dish.quantity_milli, 2000);
    assert_eq!(dish.amount_cents, 4000);
    let burger = report
        .by_product
        .iter()
        .find(|p| p.product_id == "22001")
        .unwrap();
    assert_eq!(burger.quantity_milli, 0);
    assert_eq!(burger.amount_cents, 0);
}

#[tokio::test]
async fn reading_and_printing_the_report_signs_nothing() {
    let app = scenario_app().await;
    play_service_day(&app).await;
    let day = today(&app);
    let (events, counts, sent) = (
        app.fiscal_events().len(),
        counters(&app),
        fdm_requests(&app).await,
    );
    let report = do_sales_report(&app.state, day.clone(), day.clone()).unwrap();
    assert!(report.total_cents != 0);
    // No printer configured: the print says so and still signs nothing.
    assert_eq!(
        do_print_sales_report(&app.state, day.clone(), day)
            .await
            .unwrap_err(),
        "printer_not_configured"
    );
    assert_eq!(app.fiscal_events().len(), events);
    assert_eq!(counters(&app), counts);
    assert_eq!(fdm_requests(&app).await, sent);
}

#[tokio::test]
async fn a_training_sale_is_not_in_the_report() {
    let app = scenario_app().await;
    play_service_day(&app).await;
    let day = today(&app);
    let before = do_sales_report(&app.state, day.clone(), day.clone()).unwrap();

    app.state.config.lock().unwrap().training_mode = true;
    let order = open_order(&app, U2, OrderType::Takeaway, None);
    add(&app, order, 12009, 1, None);
    mock_response(
        &app,
        in_training(read_json("InputReport/T4", "_Response.json")),
    )
    .await;
    let sale = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    assert_eq!(app.fiscal_events().last().unwrap().event_type, "T");
    app.login_as(U1, UserRole::Admin);

    let after = do_sales_report(&app.state, day.clone(), day).unwrap();
    assert_eq!(after.total_cents, before.total_cents);
    assert_eq!(after.ticket_count, before.ticket_count);
    assert_eq!(after.by_product, before.by_product);
}

// ── Events written straight into the archive ───────────────────────

struct Line {
    id: &'static str,
    name: &'static str,
    quantity: f64,
    price: f64,
    /// (scope, amount) price changes on the line's VAT row.
    changes: Vec<(&'static str, f64)>,
}

fn line(id: &'static str, name: &'static str, quantity: f64, price: f64) -> Line {
    Line {
        id,
        name,
        quantity,
        price,
        changes: vec![],
    }
}

fn insert_event(
    app: &TestApp,
    label: &str,
    period_id: &str,
    pos_date_time: &str,
    lines: &[Line],
    rounding: f64,
) -> i64 {
    let lines_json: Vec<Value> = lines
        .iter()
        .map(|l| {
            let changes: Vec<Value> = l
                .changes
                .iter()
                .map(|(scope, amount)| json!({"id": "d", "name": "Remise", "scope": scope, "type": "PUBLIC", "amount": amount}))
                .collect();
            json!({
                "lineType": "SINGLE_PRODUCT",
                "mainProduct": {
                    "productId": l.id, "productName": l.name,
                    "departmentId": "D", "departmentName": "Plats",
                    "quantity": l.quantity, "unitPrice": l.price,
                    "vats": [{"label": "B", "price": l.price * l.quantity.abs().max(1.0) * l.quantity.signum(), "priceChanges": changes}],
                },
            })
        })
        .collect();
    let request = json!({
        "bookingPeriodId": period_id,
        "posDateTime": pos_date_time,
        "transaction": {"transactionLines": lines_json, "transactionTotal": 0},
        "financials": [{"amountType": "ROUNDING", "amount": rounding}],
    });
    let conn = app.state.db.lock().unwrap();
    let seq: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(id), 0) + 1 FROM fiscal_events",
            [],
            |r| r.get(0),
        )
        .unwrap();
    conn.execute(
        "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
         VALUES (?1, ?2, 'P', 'T', ?3, ?4)",
        rusqlite::params![label, seq, U1, request.to_string()],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn new_period(app: &TestApp, id: &str, date: &str) {
    let conn = app.state.db.lock().unwrap();
    pos_db::queries::booking::current_period(&conn, id, date).unwrap();
}

fn close_with_z(app: &TestApp, id: &str, number: i64, n_amount: f64) {
    let conn = app.state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
         VALUES ('R', ?1, 'P', 'T', ?2, '{}')",
        rusqlite::params![1000 + number, U1],
    )
    .unwrap();
    let event_id = conn.last_insert_rowid();
    let z = json!({"turnover": {"transactions": [{"eventLabel": "N", "ticketCount": 1, "amount": n_amount}]}});
    pos_db::queries::reports::save_z_report(&conn, "x", number, &z.to_string(), Some(event_id))
        .unwrap();
    pos_db::queries::booking::close_period(&conn, id, event_id).unwrap();
}

#[tokio::test]
async fn hours_are_brussels_local_and_listed_from_the_day_end_hour() {
    let app = TestApp::new().await;
    app.state.config.lock().unwrap().day_end_hour = 4;
    new_period(&app, "p1", "2026-07-10");
    // 22:30 UTC in July is 00:30 in Brussels (+02:00), still the business day of the 10th.
    for (when, price) in [
        ("2026-07-10T23:50:00+02:00", 10.0),
        ("2026-07-10T22:30:00+00:00", 20.0),
        ("2026-07-11T03:30:00+02:00", 30.0),
        ("2026-07-10T18:05:00+02:00", 40.0),
        ("2026-07-10T18:55:00+02:00", 5.0),
    ] {
        insert_event(&app, "N", "p1", when, &[line("1", "Maki", 1.0, price)], 0.0);
    }
    let report = do_sales_report(&app.state, "2026-07-10".into(), "2026-07-10".into()).unwrap();
    let hours: Vec<(u32, i64, i64)> = report
        .by_hour
        .iter()
        .map(|h| (h.hour, h.tickets, h.amount_cents))
        .collect();
    // 45,00 is the biggest hour; 10,00 / 20,00 / 30,00 are 22 / 44 / 67 percent of it.
    let bars: Vec<u32> = report.by_hour.iter().map(|h| h.bar_percent).collect();
    assert_eq!(bars, vec![100, 22, 44, 67]);
    assert_eq!(
        hours,
        vec![(18, 2, 4500), (23, 1, 1000), (0, 1, 2000), (3, 1, 3000),]
    );
    assert_eq!(report.total_cents, 10_500);

    // From midnight: the plain clock order.
    app.state.config.lock().unwrap().day_end_hour = 0;
    let report = do_sales_report(&app.state, "2026-07-10".into(), "2026-07-10".into()).unwrap();
    let order: Vec<u32> = report.by_hour.iter().map(|h| h.hour).collect();
    assert_eq!(order, vec![0, 3, 18, 23]);
}

#[tokio::test]
async fn refunds_are_negative_and_stay_inside_the_total() {
    let app = TestApp::new().await;
    new_period(&app, "p1", "2026-07-10");
    insert_event(
        &app,
        "N",
        "p1",
        "2026-07-10T12:00:00+02:00",
        &[line("1", "Maki", 3.0, 6.5), line("2", "Soupe", 1.0, 4.0)],
        0.0,
    );
    insert_event(
        &app,
        "N",
        "p1",
        "2026-07-10T12:30:00+02:00",
        &[line("1", "Maki", -1.0, 6.5)],
        0.0,
    );
    let report = do_sales_report(&app.state, "2026-07-10".into(), "2026-07-10".into()).unwrap();
    assert_eq!(report.refunds_cents, -650);
    assert_eq!(report.total_cents, 1950 + 400 - 650);
    assert_eq!(report.ticket_count, 2);
    let maki = report
        .by_product
        .iter()
        .find(|p| p.product_id == "1")
        .unwrap();
    assert_eq!((maki.quantity_milli, maki.amount_cents), (2000, 1300));
    // Biggest amount first.
    assert_eq!(report.by_product[0].product_id, "1");
}

#[tokio::test]
async fn price_changes_and_rounding_are_apart_and_the_total_is_the_sum_of_the_tickets() {
    let app = TestApp::new().await;
    new_period(&app, "p1", "2026-07-10");
    let mut discounted = line("1", "Maki", 1.0, 10.0);
    discounted.changes = vec![("LINE", -1.0), ("EVENT", -0.5)];
    insert_event(
        &app,
        "N",
        "p1",
        "2026-07-10T12:00:00+02:00",
        &[discounted],
        -0.02,
    );
    let report = do_sales_report(&app.state, "2026-07-10".into(), "2026-07-10".into()).unwrap();
    // 10.00 - 1.00 on the line, -0.50 for the whole ticket; the rounding is not in the total.
    assert_eq!(report.by_product[0].amount_cents, 900);
    assert_eq!(report.price_changes_cents, -50);
    assert_eq!(report.rounding_cents, -2);
    assert_eq!(report.total_cents, 850);
}

#[tokio::test]
async fn a_gap_with_the_z_is_shown_and_two_zs_on_one_date_are_two_rows() {
    let app = TestApp::new().await;
    new_period(&app, "p1", "2026-07-10");
    insert_event(
        &app,
        "N",
        "p1",
        "2026-07-10T12:00:00+02:00",
        &[line("1", "Maki", 1.0, 10.0)],
        0.0,
    );
    close_with_z(&app, "p1", 7, 12.0);
    new_period(&app, "p2", "2026-07-10");
    insert_event(
        &app,
        "N",
        "p2",
        "2026-07-10T20:00:00+02:00",
        &[line("1", "Maki", 1.0, 5.0)],
        0.0,
    );
    close_with_z(&app, "p2", 8, 5.0);
    new_period(&app, "p3", "2026-07-11");
    insert_event(
        &app,
        "N",
        "p3",
        "2026-07-11T20:00:00+02:00",
        &[line("1", "Maki", 1.0, 2.0)],
        0.0,
    );

    let report = do_sales_report(&app.state, "2026-07-10".into(), "2026-07-11".into()).unwrap();
    assert_eq!(report.days.len(), 3);
    assert_eq!(
        (
            report.days[0].z_number,
            report.days[0].total_cents,
            report.days[0].z_total_cents,
            report.days[0].matches_z
        ),
        (Some(7), 1000, Some(1200), false)
    );
    assert_eq!(report.days[0].gap_cents, Some(-200));
    assert_eq!(report.days[1].gap_cents, Some(0));
    assert_eq!(report.days[2].gap_cents, None);
    assert!(report.days[1].matches_z);
    assert_eq!(report.days[1].z_number, Some(8));
    assert!(report.days[2].open && report.open_period);
    assert_eq!(report.total_cents, 1700);

    // Only the requested dates.
    let one = do_sales_report(&app.state, "2026-07-11".into(), "2026-07-11".into()).unwrap();
    assert_eq!(one.days.len(), 1);
    assert_eq!(one.total_cents, 200);
}

#[tokio::test]
async fn training_events_and_other_labels_are_left_out() {
    let app = TestApp::new().await;
    new_period(&app, "p1", "2026-07-10");
    let l = [line("1", "Maki", 1.0, 10.0)];
    insert_event(&app, "N", "p1", "2026-07-10T12:00:00+02:00", &l, 0.0);
    for label in ["T", "P", "C"] {
        insert_event(&app, label, "p1", "2026-07-10T12:10:00+02:00", &l, 0.0);
    }
    let report = do_sales_report(&app.state, "2026-07-10".into(), "2026-07-10".into()).unwrap();
    assert_eq!((report.ticket_count, report.total_cents), (1, 1000));
}

#[tokio::test]
async fn an_empty_range_is_an_empty_report() {
    let app = TestApp::new().await;
    let report = do_sales_report(&app.state, "2026-07-10".into(), "2026-07-12".into()).unwrap();
    assert!(report.days.is_empty() && report.by_product.is_empty() && report.by_hour.is_empty());
    assert_eq!((report.total_cents, report.ticket_count), (0, 0));
    assert!(!report.open_period);
}

#[tokio::test]
async fn the_range_is_limited_to_92_days() {
    let app = TestApp::new().await;
    assert!(do_sales_report(&app.state, "2026-07-01".into(), "2026-09-30".into()).is_ok());
    for (from, to) in [
        ("2026-07-01", "2026-10-01"),
        ("2026-07-02", "2026-07-01"),
        ("2026-07-01", "not a date"),
        ("", ""),
    ] {
        assert_eq!(
            do_sales_report(&app.state, from.into(), to.into()).unwrap_err(),
            "sales_report_range_invalid",
            "{from} {to}"
        );
    }
    assert_eq!(
        do_print_sales_report(&app.state, "2026-07-01".into(), "2026-12-01".into())
            .await
            .unwrap_err(),
        "sales_report_range_invalid"
    );
}

#[tokio::test]
async fn only_a_manager_may_read_or_print_it() {
    let app = TestApp::new().await;
    app.logout();
    let err = do_sales_report(&app.state, "2026-07-01".into(), "2026-07-01".into()).unwrap_err();
    assert_eq!(err, ERR_NOT_LOGGED_IN);
    for role in [UserRole::Cashier, UserRole::Waiter] {
        app.login_as(U2, role);
        assert_eq!(
            do_sales_report(&app.state, "2026-07-01".into(), "2026-07-01".into()).unwrap_err(),
            ERR_FORBIDDEN
        );
        assert_eq!(
            do_print_sales_report(&app.state, "2026-07-01".into(), "2026-07-01".into())
                .await
                .unwrap_err(),
            ERR_FORBIDDEN
        );
    }
}

#[tokio::test]
async fn an_unreachable_printer_is_an_error_not_a_silent_success() {
    let app = TestApp::new().await;
    app.state.config.lock().unwrap().printer_address = Some("127.0.0.1:1".into());
    let err = do_print_sales_report(&app.state, "2026-07-01".into(), "2026-07-01".into())
        .await
        .unwrap_err();
    assert!(err.starts_with("print_failed|detail="), "{err}");
}
