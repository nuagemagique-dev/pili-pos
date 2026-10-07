//! Full Service Integration Test
//!
//! Simulates a complete restaurant service day end-to-end:
//!
//!   1. Login (create user in SQLite, validate INSZ)
//!   2. Work-in (Event S → signWorkIn)
//!   3. Open table 7 (create order in SQLite)
//!   4. Take order - round 1 (add lines to DB, Event P → signOrder)
//!   5. Add items - round 2 (more lines, second Event P → signOrder)
//!   6. Modify order (change quantity, remove item)
//!   7. Pre-bill (Event P → signPreBill, verify printout)
//!   8. Cash payment + close (Event N → signSale)
//!   9. Print receipt (ESC/POS generation, verify content)
//!  10. Second table: quick sale table 3
//!  11. Z-report end-of-day (Event R → signReportTurnoverZ)
//!  12. Work-out (Event S → signWorkOut)
//!
//! Verifies at every step:
//! - SQLite state (orders, order_lines, fiscal_events, users, z_reports)
//! - Sequential counters per event type
//! - JSON archival (request + response stored)
//! - VAT calculation correctness (B=12% food, A=21% drinks)
//! - FDM communication (mock validates GraphQL structure)
//! - Receipt binary (ESC/POS contains required labels + FDM control data)

mod common;

use std::sync::atomic::{AtomicI64, Ordering};

use pos_core::config::PosConfig;
use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
use pos_db::queries::{fiscal, orders, reports, users};
use pos_fiscal::archive::{self, ArchiveParams, get_archived_event};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::event::*;
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::inputs::*;
use pos_fiscal::qr;
use pos_fiscal::response::{FdmReference, SignResult, VatCalcItem};
use pos_fiscal::signature;
use pos_fiscal::ticket::{FiscalTicket, TicketLine, TicketParams, TicketPayment};
use pos_fiscal::types::*;
use pos_print::receipt;
use uuid::Uuid;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer};

/// A booking period for tests.
fn bk(period_id: &str) -> pos_fiscal::event::Booking {
    pos_fiscal::event::Booking {
        period_id: period_id.to_string(),
        date: "2026-07-03".to_string(),
    }
}

// ── Test Infrastructure ────────────────────────────────────────

static TOTAL_COUNTER: AtomicI64 = AtomicI64::new(100);

fn next_total() -> i64 {
    TOTAL_COUNTER.fetch_add(1, Ordering::Relaxed) + 1
}

fn config(fdm_url: &str) -> PosConfig {
    PosConfig {
        table_count: 20,
        pos_id: "CFOD0061234567".to_string(),
        terminal_id: "POS1".to_string(),
        device_id: "DEV001".to_string(),
        restaurant_name: "Tokyo Sushi Bar".to_string(),
        restaurant_address: "Rue de Flandre 1, 1000 Bruxelles".to_string(),
        restaurant_vat: "BE0499999960".to_string(),
        restaurant_est_no: "2123456791".to_string(),
        fdm_url: fdm_url.to_string(),
        printer_address: None,
        training_mode: false,
        hubrise_enabled: false,
        hubrise: None,
        language: "fr".to_string(),
        backup_dir: None,
        kitchen_printer_address: None,
        fdm_token_file: None,
        fdm_ca_file: None,
        fdm_client_cert_file: None,
        fdm_client_key_file: None,
        restaurant_legal_name: None,
        day_end_hour: 5,
        digital_ticket: None,
    }
}

fn make_sign_result(
    op: EventOperation,
    label: EventLabel,
    counter: i64,
    vat_calc: Option<Vec<VatCalcItem>>,
) -> SignResult {
    let tc = next_total();
    let sig_bytes = format!("sig-{label:?}-{counter}-{tc}");
    let digital_sig = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        sig_bytes.as_bytes(),
    );
    let short = if label == EventLabel::N {
        Some(signature::compute_short_signature(&digital_sig).unwrap())
    } else {
        None
    };

    SignResult {
        pos_id: "CFOD0061234567".to_string(),
        pos_fiscal_ticket_no: counter,
        pos_date_time: chrono::Local::now().to_rfc3339(),
        terminal_id: Some("POS1".to_string()),
        device_id: "DEV001".to_string(),
        event_operation: op,
        fdm_ref: FdmReference {
            fdm_id: "BMC0TEST0001".to_string(),
            fdm_date_time: chrono::Utc::now().to_rfc3339(),
            event_label: label,
            event_counter: counter,
            total_counter: tc,
        },
        fdm_sw_version: "BMC-FDM-1.0".to_string(),
        digital_signature: digital_sig,
        short_signature: short,
        verification_url: if label == EventLabel::N {
            Some("https://www.gs2.be/Abc123DefGhi456JklMn".to_string())
        } else {
            None
        },
        vat_calc,
        buffer_capacity_used: 2.3,
        warnings: None,
        informations: None,
        footer: vec!["www.systemedecaisseenregistreuse.be".to_string()],
    }
}

/// Mount a mock that matches a specific mutation name in the body.
async fn mock_mutation(
    server: &MockServer,
    mutation_contains: &str,
    op: EventOperation,
    label: EventLabel,
    counter: i64,
    vat_calc: Option<Vec<VatCalcItem>>,
) -> SignResult {
    let result = make_sign_result(op, label, counter, vat_calc);
    let mutation_key = if mutation_contains.contains("signWorkIn") {
        "signWorkIn"
    } else if mutation_contains.contains("signWorkOut") {
        "signWorkOut"
    } else if mutation_contains.contains("signPreBill") {
        "signPreBill"
    } else if mutation_contains.contains("signOrder") {
        "signOrder"
    } else if mutation_contains.contains("signSale") {
        "signSale"
    } else if mutation_contains.contains("signReportTurnoverZ") {
        "signReportTurnoverZ"
    } else {
        mutation_contains
    };

    let mut data = serde_json::Map::new();
    data.insert(
        mutation_key.to_string(),
        serde_json::to_value(&result).unwrap(),
    );
    let body = serde_json::json!({ "data": data });

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains(mutation_contains))
        .respond_with(common::EchoRequest(body.clone()))
        .up_to_n_times(1)
        .named(mutation_contains)
        .mount(server)
        .await;

    result
}

fn setup_db() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    pos_db::schema::run_migrations(&conn).unwrap();
    conn
}

// ═══════════════════════════════════════════════════════════════
//  FULL SERVICE DAY - Single monolithic test
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_full_service_day_e2e() {
    let server = MockServer::start().await;
    let cfg = config(&format!("{}/graphql", server.uri()));
    let db = setup_db();
    let bp_id = Uuid::new_v4().to_string();
    let fdm = FdmClient::new(&cfg.fdm_url);

    // Seed test menu data (categories + items used in this test)
    db.execute_batch(
        "
        INSERT INTO categories (id, sort_order) VALUES (1, 1), (4, 4), (16, 16);
        INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order) VALUES
            (3, 1, 1790, 'B', 3),    -- M1 (plateau)
            (10, 1, 2480, 'B', 10),   -- Plateau sushi mix
            (50, 4, 470, 'B', 6),     -- Maki saumon x6
            (129, 1, 400, 'B', 1),    -- Soupe miso
            (160, 16, 390, 'A', 9);   -- Biere Asahi
    ",
    )
    .unwrap();

    // ────────────────────────────────────────────────────────────
    // STEP 1: Login - Create user in SQLite, validate INSZ
    // ────────────────────────────────────────────────────────────

    let user = users::create_user(&db, "93051800189", "Bruno", UserRole::Admin).unwrap();
    assert_eq!(user.insz, "93051800189");
    assert_eq!(user.role, UserRole::Admin);

    // Verify user is retrievable
    let fetched = users::get_user(&db, "93051800189").unwrap().unwrap();
    assert_eq!(fetched.name, "Bruno");

    // Validate INSZ format
    assert!(pos_core::user::validate_insz("93051800189"));
    assert!(!pos_core::user::validate_insz("00000000000")); // invalid checksum

    // ────────────────────────────────────────────────────────────
    // STEP 2: Work-In (Event S → signWorkIn)
    // ────────────────────────────────────────────────────────────

    let _s1_result = mock_mutation(
        &server,
        "signWorkIn",
        EventOperation::WorkIn,
        EventLabel::S,
        1,
        None,
    )
    .await;

    let s1_ticket = FiscalCounter::next(&db, EventLabel::S).unwrap();
    assert_eq!(s1_ticket, 1, "First S event should be #1");

    let work_in = build_work_in_out(&cfg, &ident(), s1_ticket, &user.insz, &bk(&bp_id));
    let wi_result = fdm.sign_work_in(work_in, false).await.unwrap();
    assert_eq!(wi_result.event_operation, EventOperation::WorkIn);
    assert_eq!(wi_result.fdm_ref.event_label, EventLabel::S);

    let wi_event_id = archive::archive_event(
        &db,
        ArchiveParams {
            event_type: EventLabel::S,
            sequence_number: s1_ticket,
            order_id: None,
            pos_id: &cfg.pos_id,
            terminal_id: &cfg.terminal_id,
            user_insz: &user.insz,
            request_json: "{}",
            sign_result: Some(&wi_result),
        },
    )
    .unwrap();
    assert!(wi_event_id > 0);

    // ────────────────────────────────────────────────────────────
    // STEP 3: Open Table 7 (create order in SQLite)
    // ────────────────────────────────────────────────────────────

    let order = orders::create_order(&db, OrderType::DineIn, Some(7), &user.insz).unwrap();
    assert_eq!(order.table_number, Some(7));
    assert_eq!(order.status, pos_core::order::OrderStatus::Open);
    assert_eq!(order.total_cents, 0);

    // Verify it appears in open orders
    let open = orders::list_open_orders(&db).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].id, order.id);

    // ────────────────────────────────────────────────────────────
    // STEP 4: Take Order - Round 1 (add lines + Event P signOrder)
    // ────────────────────────────────────────────────────────────

    let line1 = orders::add_order_line(
        &db,
        NewOrderLine {
            order_id: order.id,
            menu_item_id: 10,
            quantity: 1,
            unit_price_cents: 2480,
            vat_code: VatCode::B,
            name: "Plateau sushi mix",
            modifiers_json: None,
            department_id: "1",
            department_name: "Test",
        },
    )
    .unwrap();
    assert_eq!(line1.unit_price_cents, 2480);

    let line2 = orders::add_order_line(
        &db,
        NewOrderLine {
            order_id: order.id,
            menu_item_id: 129,
            quantity: 2,
            unit_price_cents: 400,
            vat_code: VatCode::B,
            name: "Soupe miso",
            modifiers_json: None,
            department_id: "1",
            department_name: "Test",
        },
    )
    .unwrap();

    let line3 = orders::add_order_line(
        &db,
        NewOrderLine {
            order_id: order.id,
            menu_item_id: 160,
            quantity: 2,
            unit_price_cents: 390,
            vat_code: VatCode::A, // Beverages = 21%
            name: "Bière Asahi",
            modifiers_json: None,
            department_id: "1",
            department_name: "Test",
        },
    )
    .unwrap();

    // Verify order totals auto-recalculated
    let order_r1 = orders::get_order(&db, order.id).unwrap();
    // 2480 + 2*400 + 2*390 = 2480 + 800 + 780 = 4060
    assert_eq!(order_r1.total_cents, 4060);

    // Send signOrder (P event) to FDM
    let _p1_result = mock_mutation(
        &server,
        "signOrder",
        EventOperation::Order,
        EventLabel::P,
        1,
        None,
    )
    .await;

    let p1_ticket = FiscalCounter::next(&db, EventLabel::P).unwrap();
    assert_eq!(p1_ticket, 1);

    let lines_r1 = orders::get_order_lines(&db, order.id).unwrap();
    assert_eq!(lines_r1.len(), 3);

    let order_input =
        build_order_input(&cfg, &ident(), &order_r1, &lines_r1, p1_ticket, &bk(&bp_id));
    assert_eq!(order_input.cost_center.id, "T07");
    assert_eq!(
        order_input.cost_center.cost_center_type,
        CostCenterType::Table
    );

    let p1_result = fdm.sign_order(order_input, false).await.unwrap();
    assert_eq!(p1_result.fdm_ref.event_label, EventLabel::P);

    archive::archive_event(
        &db,
        ArchiveParams {
            event_type: EventLabel::P,
            sequence_number: p1_ticket,
            order_id: Some(order.id),
            pos_id: &cfg.pos_id,
            terminal_id: &cfg.terminal_id,
            user_insz: &user.insz,
            request_json: &serde_json::to_string(&p1_result).unwrap(),
            sign_result: Some(&p1_result),
        },
    )
    .unwrap();

    // ────────────────────────────────────────────────────────────
    // STEP 5: Add Items - Round 2 (more lines + second signOrder)
    // ────────────────────────────────────────────────────────────

    let _line4 = orders::add_order_line(
        &db,
        NewOrderLine {
            order_id: order.id,
            menu_item_id: 50,
            quantity: 2,
            unit_price_cents: 470,
            vat_code: VatCode::B,
            name: "Maki saumon x6",
            modifiers_json: None,
            department_id: "1",
            department_name: "Test",
        },
    )
    .unwrap();

    // Order total: 4060 + 2*470 = 5000
    let order_r2 = orders::get_order(&db, order.id).unwrap();
    assert_eq!(order_r2.total_cents, 5000);

    let _p2_result = mock_mutation(
        &server,
        "signOrder",
        EventOperation::Order,
        EventLabel::P,
        2,
        None,
    )
    .await;

    let p2_ticket = FiscalCounter::next(&db, EventLabel::P).unwrap();
    assert_eq!(p2_ticket, 2, "Second P event should be #2");

    let lines_r2 = orders::get_order_lines(&db, order.id).unwrap();
    assert_eq!(lines_r2.len(), 4);

    let order_input2 =
        build_order_input(&cfg, &ident(), &order_r2, &lines_r2, p2_ticket, &bk(&bp_id));
    let p2_fdm = fdm.sign_order(order_input2, false).await.unwrap();

    archive::archive_event(
        &db,
        ArchiveParams {
            event_type: EventLabel::P,
            sequence_number: p2_ticket,
            order_id: Some(order.id),
            pos_id: &cfg.pos_id,
            terminal_id: &cfg.terminal_id,
            user_insz: &user.insz,
            request_json: "{}",
            sign_result: Some(&p2_fdm),
        },
    )
    .unwrap();

    // ────────────────────────────────────────────────────────────
    // STEP 6: Modify Order (change qty, remove item)
    // ────────────────────────────────────────────────────────────

    // Increase soupe miso from 2 to 3
    orders::update_order_line_quantity(&db, line2.id, 3).unwrap();

    // Remove one Bière Asahi (line3: qty was 2, set to 1)
    orders::update_order_line_quantity(&db, line3.id, 1).unwrap();

    // Verify totals: 2480 + 3*400 + 1*390 + 2*470 = 2480+1200+390+940 = 5010
    let order_mod = orders::get_order(&db, order.id).unwrap();
    assert_eq!(order_mod.total_cents, 5010);

    let lines_mod = orders::get_order_lines(&db, order.id).unwrap();
    assert_eq!(lines_mod.len(), 4);
    let miso = lines_mod.iter().find(|l| l.name == "Soupe miso").unwrap();
    assert_eq!(miso.quantity, 3);
    let asahi = lines_mod.iter().find(|l| l.name == "Bière Asahi").unwrap();
    assert_eq!(asahi.quantity, 1);

    // ────────────────────────────────────────────────────────────
    // STEP 7: Pre-Bill (Event P → signPreBill + receipt print)
    // ────────────────────────────────────────────────────────────

    let _pb_result = mock_mutation(
        &server,
        "signPreBill",
        EventOperation::PreBill,
        EventLabel::P,
        3,
        None,
    )
    .await;

    let pb_ticket = FiscalCounter::next(&db, EventLabel::P).unwrap();
    assert_eq!(pb_ticket, 3, "Third P event");

    let pre_bill = build_pre_bill_input(
        &cfg,
        &ident(),
        &order_mod,
        &lines_mod,
        pb_ticket,
        &bk(&bp_id),
    );

    // Verify pre-bill structure: 4 lines, no financials
    assert_eq!(pre_bill.transaction.transaction_lines.len(), 4);
    let expected_total = 24.80 + 3.0 * 4.00 + 1.0 * 3.90 + 2.0 * 4.70;
    assert!(
        (pre_bill.transaction.transaction_total - expected_total).abs() < 0.01,
        "Pre-bill total should be {expected_total}, got {}",
        pre_bill.transaction.transaction_total
    );

    let pb_fdm = fdm.sign_pre_bill(pre_bill, false).await.unwrap();
    assert_eq!(pb_fdm.event_operation, EventOperation::PreBill);
    assert!(
        pb_fdm.short_signature.is_none(),
        "Pre-bill must NOT have shortSignature"
    );
    assert!(
        pb_fdm.verification_url.is_none(),
        "Pre-bill must NOT have verificationUrl"
    );

    archive::archive_event(
        &db,
        ArchiveParams {
            event_type: EventLabel::P,
            sequence_number: pb_ticket,
            order_id: Some(order.id),
            pos_id: &cfg.pos_id,
            terminal_id: &cfg.terminal_id,
            user_insz: &user.insz,
            request_json: "{}",
            sign_result: Some(&pb_fdm),
        },
    )
    .unwrap();

    // Generate pre-bill receipt
    let pb_ticket_data = FiscalTicket::from_sign_result(TicketParams {
        identity: &ident(),
        result: &pb_fdm,
        restaurant_name: &cfg.restaurant_name,
        restaurant_address: &cfg.restaurant_address,
        restaurant_vat: &cfg.restaurant_vat,
        restaurant_est_no: &cfg.restaurant_est_no,
        restaurant_legal_name: cfg.legal_name(),
        lines: lines_mod
            .iter()
            .map(|l| TicketLine {
                quantity: l.quantity as f64,
                description: l.name.clone(),
                unit_price: l.unit_price_cents as f64 / 100.0,
                total: (l.unit_price_cents * l.quantity as i64) as f64 / 100.0,
                vat_code: l.vat_code.as_str().to_string(),
            })
            .collect(),
        payments: vec![],
        total: order_mod.total_cents as f64 / 100.0,
        is_training: false,
        rounding: None,
        lang: pos_fiscal::ticket::ReceiptLanguage::default(),
        operator: "Test",
    });

    assert_eq!(pb_ticket_data.ticket_type_label, "PRO FORMA");
    assert!(
        pb_ticket_data.needs_invalid_disclaimer(),
        "Pre-bill must include the invalid-ticket mention"
    );

    let pb_escpos = receipt::format_receipt(&pb_ticket_data);
    assert!(
        pb_escpos.windows(9).any(|w| w == b"PRO FORMA"),
        "Pre-bill ESC/POS must contain 'PRO FORMA'"
    );
    assert!(
        pos_print::render::one_line(&pos_print::render::render_plain_text(&pb_escpos))
            .contains("CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE"),
        "Pre-bill must have disclaimer"
    );

    // ────────────────────────────────────────────────────────────
    // STEP 8: Cash Payment + Close (Event N → signSale)
    // ────────────────────────────────────────────────────────────

    // FDM returns VAT calculations for event N
    let food_total_cents: i64 = lines_mod
        .iter()
        .filter(|l| l.vat_code == VatCode::B)
        .map(|l| l.unit_price_cents * l.quantity as i64)
        .sum(); // 2480 + 1200 + 940 = 4620
    let drink_total_cents: i64 = lines_mod
        .iter()
        .filter(|l| l.vat_code == VatCode::A)
        .map(|l| l.unit_price_cents * l.quantity as i64)
        .sum(); // 390

    let food_total = food_total_cents as f64 / 100.0; // 46.20
    let drink_total = drink_total_cents as f64 / 100.0; // 3.90

    // VAT B: 12% → taxable = 46.20 / 1.12 = 41.25, vat = 4.95
    let food_taxable = (food_total / 1.12 * 100.0).round() / 100.0;
    let food_vat = ((food_total - food_taxable) * 100.0).round() / 100.0;

    // VAT A: 21% → taxable = 3.90 / 1.21 = 3.22, vat = 0.68
    let drink_taxable = (drink_total / 1.21 * 100.0).round() / 100.0;
    let drink_vat = ((drink_total - drink_taxable) * 100.0).round() / 100.0;

    let sale_vat_calc = vec![
        VatCalcItem {
            label: VatLabel::B,
            rate: 12.0,
            taxable_amount: food_taxable,
            vat_amount: food_vat,
            total_amount: food_total,
            out_of_scope: false,
        },
        VatCalcItem {
            label: VatLabel::A,
            rate: 21.0,
            taxable_amount: drink_taxable,
            vat_amount: drink_vat,
            total_amount: drink_total,
            out_of_scope: false,
        },
    ];

    let _sale_mock = mock_mutation(
        &server,
        "signSale",
        EventOperation::Sale,
        EventLabel::N,
        1,
        Some(sale_vat_calc.clone()),
    )
    .await;

    let n1_ticket = FiscalCounter::next(&db, EventLabel::N).unwrap();
    assert_eq!(n1_ticket, 1, "First N event");

    let sale = build_sale_input(
        &cfg,
        &ident(),
        &order_mod,
        &lines_mod,
        n1_ticket,
        &bk(&bp_id),
        PaymentType::Cash,
        order_mod.total_cents,
        0,
        None,
    );

    // Verify sale structure
    assert_eq!(sale.transaction.transaction_lines.len(), 4);
    assert_eq!(sale.financials.len(), 1);
    assert_eq!(sale.financials[0].payment_type, PaymentType::Cash);
    assert!((sale.financials[0].amount - 50.10).abs() < 0.01);

    // Verify VAT labels on lines
    let food_lines: Vec<_> = sale
        .transaction
        .transaction_lines
        .iter()
        .filter(|l| l.main_product.vats[0].label == VatLabel::B)
        .collect();
    let drink_lines: Vec<_> = sale
        .transaction
        .transaction_lines
        .iter()
        .filter(|l| l.main_product.vats[0].label == VatLabel::A)
        .collect();
    assert_eq!(food_lines.len(), 3, "3 food lines with VAT B");
    assert_eq!(drink_lines.len(), 1, "1 drink line with VAT A");

    let sale_json = serde_json::to_string(&sale).unwrap();
    assert!(sale_json.contains("\"posId\":\"CFOD0061234567\""));
    assert!(sale_json.contains("\"employeeId\":\"93051800189\""));

    let n1_result = fdm.sign_sale(sale, false).await.unwrap();
    assert_eq!(n1_result.event_operation, EventOperation::Sale);
    assert_eq!(n1_result.fdm_ref.event_label, EventLabel::N);
    assert!(n1_result.short_signature.is_some());
    assert!(n1_result.verification_url.is_some());

    // Verify shortSignature integrity
    assert!(
        signature::verify_short_signature(
            &n1_result.digital_signature,
            n1_result.short_signature.as_ref().unwrap(),
        )
        .unwrap()
    );

    // Verify VAT calc from FDM
    let vat_calc = n1_result.vat_calc.as_ref().unwrap();
    assert_eq!(vat_calc.len(), 2);
    let vat_b = vat_calc.iter().find(|v| v.label == VatLabel::B).unwrap();
    assert!((vat_b.total_amount - food_total).abs() < 0.01);
    assert!((vat_b.taxable_amount - food_taxable).abs() < 0.01);
    let vat_a = vat_calc.iter().find(|v| v.label == VatLabel::A).unwrap();
    assert!((vat_a.total_amount - drink_total).abs() < 0.01);

    // Archive sale
    let _n1_event_id = archive::archive_event(
        &db,
        ArchiveParams {
            event_type: EventLabel::N,
            sequence_number: n1_ticket,
            order_id: Some(order.id),
            pos_id: &cfg.pos_id,
            terminal_id: &cfg.terminal_id,
            user_insz: &user.insz,
            request_json: &sale_json,
            sign_result: Some(&n1_result),
        },
    )
    .unwrap();

    // Verify archived event in DB
    let archived = get_archived_event(&db, EventLabel::N, 1).unwrap().unwrap();
    assert_eq!(archived.pos_id, "CFOD0061234567");
    assert_eq!(archived.fdm_id.as_deref(), Some("BMC0TEST0001"));
    assert!(archived.short_signature.is_some());
    assert!(archived.qr_data.is_some());
    assert!(archived.fdm_response_json.is_some());
    // Verify request JSON was stored
    assert!(archived.event_data_json.contains("posId"));

    // Close order in DB
    db.execute(
        "UPDATE orders SET status = 'closed', closed_at = datetime('now') WHERE id = ?1",
        rusqlite::params![order.id],
    )
    .unwrap();

    // Record payment
    db.execute(
        "INSERT INTO payments (order_id, method, amount_cents) VALUES (?1, 'cash', ?2)",
        rusqlite::params![order.id, order_mod.total_cents],
    )
    .unwrap();

    let closed_order = orders::get_order(&db, order.id).unwrap();
    assert_eq!(closed_order.status, pos_core::order::OrderStatus::Closed);
    assert!(closed_order.closed_at.is_some());

    // ────────────────────────────────────────────────────────────
    // STEP 9: Print Receipt (ESC/POS generation + verification)
    // ────────────────────────────────────────────────────────────

    let ticket = FiscalTicket::from_sign_result(TicketParams {
        identity: &ident(),
        result: &n1_result,
        restaurant_name: &cfg.restaurant_name,
        restaurant_address: &cfg.restaurant_address,
        restaurant_vat: &cfg.restaurant_vat,
        restaurant_est_no: &cfg.restaurant_est_no,
        restaurant_legal_name: cfg.legal_name(),
        lines: lines_mod
            .iter()
            .map(|l| TicketLine {
                quantity: l.quantity as f64,
                description: l.name.clone(),
                unit_price: l.unit_price_cents as f64 / 100.0,
                total: (l.unit_price_cents * l.quantity as i64) as f64 / 100.0,
                vat_code: l.vat_code.as_str().to_string(),
            })
            .collect(),
        payments: vec![TicketPayment {
            method: "Cash".to_string(),
            amount: 50.10,
        }],
        total: 50.10,
        is_training: false,
        rounding: None,
        lang: pos_fiscal::ticket::ReceiptLanguage::default(),
        operator: "Test",
    });

    assert_eq!(ticket.ticket_type_label, "TICKET DE CAISSE T.V.A.");
    assert!(!ticket.needs_invalid_disclaimer());
    assert!(!ticket.is_refund);
    assert!(ticket.fdm_ref.is_some());

    let escpos = receipt::format_receipt(&ticket);

    // Verify ESC/POS content
    assert!(!escpos.is_empty());
    assert!(
        escpos.windows(23).any(|w| w == b"TICKET DE CAISSE T.V.A."),
        "Must contain 'TICKET DE CAISSE T.V.A.'"
    );
    assert!(
        escpos.windows(15).any(|w| w == b"Tokyo Sushi Bar"),
        "Must contain restaurant name"
    );
    assert!(
        escpos.windows(12).any(|w| w == b"BE0499999960"),
        "Must contain VAT number"
    );
    assert!(
        escpos.windows(10).any(|w| w == b"Soupe miso"),
        "Must contain line item"
    );
    assert!(
        escpos.windows(14).any(|w| w == b"Maki saumon x6"),
        "Must contain line item"
    );
    assert!(
        escpos.windows(12).any(|w| w == b"BMC0TEST0001"),
        "Must contain FDM ID"
    );
    // The raw FDM financials.name ("Cash") is localized to the
    // ticket's language ("Especes" in FR - see pos_print::labels).
    assert!(
        escpos.windows(7).any(|w| w == b"Especes"),
        "Must contain the localized payment method"
    );

    // Verify QR code command is present (GS ( k)
    assert!(
        escpos
            .windows(3)
            .any(|w| w[0] == 0x1D && w[1] == b'(' && w[2] == b'k'),
        "Must contain QR code GS(k command"
    );

    // Verify cut command at the end (GS V)
    assert!(
        escpos.windows(2).any(|w| w[0] == 0x1D && w[1] == b'V'),
        "Must end with paper cut"
    );

    // Generate QR code separately for verification
    let qr_url = ticket.verification_url.as_ref().unwrap();
    let qr_matrix = qr::generate_qr_matrix(qr_url).unwrap();
    assert!(!qr_matrix.is_empty(), "QR matrix should be non-empty");

    // ────────────────────────────────────────────────────────────
    // STEP 10: Second Table - Quick sale on table 3
    // ────────────────────────────────────────────────────────────

    let order2 = orders::create_order(&db, OrderType::DineIn, Some(3), &user.insz).unwrap();
    orders::add_order_line(
        &db,
        NewOrderLine {
            order_id: order2.id,
            menu_item_id: 3,
            quantity: 1,
            unit_price_cents: 1790,
            vat_code: VatCode::B,
            name: "M1",
            modifiers_json: None,
            department_id: "1",
            department_name: "Test",
        },
    )
    .unwrap();

    let order2_db = orders::get_order(&db, order2.id).unwrap();
    assert_eq!(order2_db.total_cents, 1790);

    let _sale2_mock = mock_mutation(
        &server,
        "signSale",
        EventOperation::Sale,
        EventLabel::N,
        2,
        Some(vec![VatCalcItem {
            label: VatLabel::B,
            rate: 12.0,
            taxable_amount: 15.98,
            vat_amount: 1.92,
            total_amount: 17.90,
            out_of_scope: false,
        }]),
    )
    .await;

    let n2_ticket = FiscalCounter::next(&db, EventLabel::N).unwrap();
    assert_eq!(n2_ticket, 2, "Second N event");

    let lines2 = orders::get_order_lines(&db, order2.id).unwrap();
    let sale2 = build_sale_input(
        &cfg,
        &ident(),
        &order2_db,
        &lines2,
        n2_ticket,
        &bk(&bp_id),
        PaymentType::CardDebit,
        1790,
        0,
        None,
    );
    assert_eq!(sale2.financials[0].payment_type, PaymentType::CardDebit);

    let n2_result = fdm.sign_sale(sale2, false).await.unwrap();

    archive::archive_event(
        &db,
        ArchiveParams {
            event_type: EventLabel::N,
            sequence_number: n2_ticket,
            order_id: Some(order2.id),
            pos_id: &cfg.pos_id,
            terminal_id: &cfg.terminal_id,
            user_insz: &user.insz,
            request_json: "{}",
            sign_result: Some(&n2_result),
        },
    )
    .unwrap();

    db.execute(
        "UPDATE orders SET status = 'closed', closed_at = datetime('now') WHERE id = ?1",
        rusqlite::params![order2.id],
    )
    .unwrap();
    db.execute(
        "INSERT INTO payments (order_id, method, amount_cents) VALUES (?1, 'card', ?2)",
        rusqlite::params![order2.id, 1790],
    )
    .unwrap();

    // No more open orders
    let open_after = orders::list_open_orders(&db).unwrap();
    assert!(open_after.is_empty(), "All orders should be closed");

    // ────────────────────────────────────────────────────────────
    // STEP 11: Z-Report End-of-Day (Event R → signReportTurnoverZ)
    // ────────────────────────────────────────────────────────────

    let _z_mock = mock_mutation(
        &server,
        "signReportTurnoverZ",
        EventOperation::ReportTurnoverZ,
        EventLabel::R,
        1,
        None,
    )
    .await;

    let r1_ticket = FiscalCounter::next(&db, EventLabel::R).unwrap();
    assert_eq!(r1_ticket, 1, "First R event");

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let report_no = reports::next_z_report_number(&db).unwrap();
    assert_eq!(report_no, 1, "First Z-report of the day");

    // Build turnover summary from actual DB data
    let total_food = food_total_cents + 1790; // table 7 food + table 3 M1
    let total_drink = drink_total_cents; // only table 7 drinks
    let grand_total = total_food + total_drink; // 6410 + 390 = 6800

    let z_data = TurnoverReportData {
        pos_devices: vec![PosDeviceInput {
            pos_id: cfg.pos_id.clone(),
            terminal_id: Some(cfg.terminal_id.clone()),
            first_pos_date_time: chrono::Local::now().to_rfc3339(),
            last_pos_date_time: chrono::Local::now().to_rfc3339(),
        }],
        fdm_devices: vec![FdmDeviceInput {
            fdm_id: "BMC0TEST0001".to_string(),
            first_fdm_date_time: chrono::Utc::now().to_rfc3339(),
            first_total_counter: 101,
            last_fdm_date_time: chrono::Utc::now().to_rfc3339(),
            last_total_counter: next_total(),
        }],
        turnover: TurnoverInput {
            transactions: vec![
                EventTotalInput {
                    event_label: EventLabel::N,
                    ticket_count: 2,
                    amount: grand_total as f64 / 100.0,
                },
                EventTotalInput {
                    event_label: EventLabel::P,
                    ticket_count: 3,
                    amount: 0.0,
                },
                EventTotalInput {
                    event_label: EventLabel::S,
                    ticket_count: 1,
                    amount: 0.0,
                },
            ],
            departments: vec![DepartmentTotalInput {
                department_id: "FOOD".to_string(),
                department_name: "Restaurant".to_string(),
                amount: grand_total as f64 / 100.0,
            }],
            vats: vec![
                VatTotalInput {
                    label: VatLabel::B,
                    rate: 12.0,
                    taxable_amount: total_food as f64 / 100.0 / 1.12,
                    vat_amount: total_food as f64 / 100.0 - total_food as f64 / 100.0 / 1.12,
                    total_amount: total_food as f64 / 100.0,
                },
                VatTotalInput {
                    label: VatLabel::A,
                    rate: 21.0,
                    taxable_amount: total_drink as f64 / 100.0 / 1.21,
                    vat_amount: total_drink as f64 / 100.0 - total_drink as f64 / 100.0 / 1.21,
                    total_amount: total_drink as f64 / 100.0,
                },
            ],
            payments: vec![
                PaymentTotalInput {
                    id: "cash".to_string(),
                    name: "Cash".to_string(),
                    payment_type: PaymentType::Cash,
                    amount: vec![PaymentTotalAmountInput {
                        amount_type: PaymentLineType::Payment,
                        normal_amount: 50.10,
                        negative_corrections: 0.0,
                        positive_corrections: 0.0,
                        corrections_count: 0,
                        total_amount: 50.10,
                        normal_foreign: None,
                        corrections_foreign: None,
                    }],
                },
                PaymentTotalInput {
                    id: "card".to_string(),
                    name: "Card".to_string(),
                    payment_type: PaymentType::CardDebit,
                    amount: vec![PaymentTotalAmountInput {
                        amount_type: PaymentLineType::Payment,
                        normal_amount: 17.90,
                        negative_corrections: 0.0,
                        positive_corrections: 0.0,
                        corrections_count: 0,
                        total_amount: 17.90,
                        normal_foreign: None,
                        corrections_foreign: None,
                    }],
                },
            ],
            drawers_open_count: 1, // 1 cash sale opened the drawer
            neg_quantities: vec![],
            price_changes: vec![],
            invoices: None,
        },
    };

    let z_report = build_report_turnover_z(
        &cfg,
        &ident(),
        r1_ticket,
        &user.insz,
        &bk(&bp_id),
        report_no,
        &today,
        z_data,
    );

    // Verify Z-report structure
    assert_eq!(z_report.report_no, 1);
    assert_eq!(z_report.turnover.transactions.len(), 3);
    assert_eq!(z_report.turnover.payments.len(), 2);
    assert_eq!(z_report.turnover.vats.len(), 2);

    let z_json = serde_json::to_string(&z_report).unwrap();
    assert!(z_json.contains("\"reportNo\":1"));
    assert!(z_json.contains("\"drawersOpenCount\":1"));

    let z_result = fdm.sign_report_turnover_z(z_report, false).await.unwrap();
    assert_eq!(z_result.event_operation, EventOperation::ReportTurnoverZ);

    let z_event_id = archive::archive_event(
        &db,
        ArchiveParams {
            event_type: EventLabel::R,
            sequence_number: r1_ticket,
            order_id: None,
            pos_id: &cfg.pos_id,
            terminal_id: &cfg.terminal_id,
            user_insz: &user.insz,
            request_json: &z_json,
            sign_result: Some(&z_result),
        },
    )
    .unwrap();

    reports::save_z_report(&db, &today, report_no, &z_json, Some(z_event_id)).unwrap();

    // Verify Z-report stored
    let next_z = reports::next_z_report_number(&db).unwrap();
    assert_eq!(next_z, 2, "Next Z-report should be #2");

    // ────────────────────────────────────────────────────────────
    // STEP 12: Work-Out (Event S → signWorkOut)
    // ────────────────────────────────────────────────────────────

    let _wo_mock = mock_mutation(
        &server,
        "signWorkOut",
        EventOperation::WorkOut,
        EventLabel::S,
        2,
        None,
    )
    .await;

    let s2_ticket = FiscalCounter::next(&db, EventLabel::S).unwrap();
    assert_eq!(s2_ticket, 2, "Second S event (work-out)");

    let work_out = build_work_in_out(&cfg, &ident(), s2_ticket, &user.insz, &bk(&bp_id));
    let wo_result = fdm.sign_work_out(work_out, false).await.unwrap();
    assert_eq!(wo_result.event_operation, EventOperation::WorkOut);

    archive::archive_event(
        &db,
        ArchiveParams {
            event_type: EventLabel::S,
            sequence_number: s2_ticket,
            order_id: None,
            pos_id: &cfg.pos_id,
            terminal_id: &cfg.terminal_id,
            user_insz: &user.insz,
            request_json: "{}",
            sign_result: Some(&wo_result),
        },
    )
    .unwrap();

    // ════════════════════════════════════════════════════════════
    // FINAL VERIFICATION - complete state audit
    // ════════════════════════════════════════════════════════════

    // Counter state
    assert_eq!(
        FiscalCounter::current(&db, EventLabel::S).unwrap(),
        2,
        "S: work-in + work-out"
    );
    assert_eq!(
        FiscalCounter::current(&db, EventLabel::P).unwrap(),
        3,
        "P: 2 orders + 1 pre-bill"
    );
    assert_eq!(
        FiscalCounter::current(&db, EventLabel::N).unwrap(),
        2,
        "N: 2 sales"
    );
    assert_eq!(
        FiscalCounter::current(&db, EventLabel::R).unwrap(),
        1,
        "R: 1 Z-report"
    );
    assert_eq!(
        FiscalCounter::current(&db, EventLabel::T).unwrap(),
        0,
        "T: never used"
    );
    assert_eq!(
        FiscalCounter::current(&db, EventLabel::C).unwrap(),
        0,
        "C: no copies"
    );

    // Fiscal events in DB
    let all_events = fiscal::list_recent_events(&db, 100).unwrap();
    assert_eq!(
        all_events.len(),
        8,
        "Total fiscal events: S+P+P+P+N+N+R+S = 8"
    );

    // Count by type
    let n_events: Vec<_> = all_events.iter().filter(|e| e.event_type == "N").collect();
    let p_events: Vec<_> = all_events.iter().filter(|e| e.event_type == "P").collect();
    let s_events: Vec<_> = all_events.iter().filter(|e| e.event_type == "S").collect();
    let r_events: Vec<_> = all_events.iter().filter(|e| e.event_type == "R").collect();
    assert_eq!(n_events.len(), 2);
    assert_eq!(p_events.len(), 3);
    assert_eq!(s_events.len(), 2);
    assert_eq!(r_events.len(), 1);

    // All N events have FDM control data
    for n in &n_events {
        assert!(n.fdm_id.is_some(), "N event must have fdmId");
        assert!(
            n.short_signature.is_some(),
            "N event must have shortSignature"
        );
        assert!(
            n.qr_data.is_some(),
            "N event must have QR data (verificationUrl)"
        );
    }

    // Orders table
    let all_orders: i64 = db
        .query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0))
        .unwrap();
    assert_eq!(all_orders, 2, "2 orders created");
    let closed_orders: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM orders WHERE status = 'closed'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(closed_orders, 2, "Both orders closed");

    // Payments table
    let payment_count: i64 = db
        .query_row("SELECT COUNT(*) FROM payments", [], |r| r.get(0))
        .unwrap();
    assert_eq!(payment_count, 2, "2 payments recorded");
    let total_paid: i64 = db
        .query_row("SELECT SUM(amount_cents) FROM payments", [], |r| r.get(0))
        .unwrap();
    assert_eq!(total_paid, 5010 + 1790, "Total payments = 68.00 EUR");

    // Z-reports table
    let z_count: i64 = db
        .query_row("SELECT COUNT(*) FROM z_reports", [], |r| r.get(0))
        .unwrap();
    assert_eq!(z_count, 1, "1 Z-report stored");

    // Users table
    let user_count: i64 = db
        .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
        .unwrap();
    assert_eq!(user_count, 1, "1 user in system");
}

/// The software identity of the Linux shell at version 1.0.0.
fn ident() -> pos_fiscal::identity::SoftwareIdentity {
    pos_fiscal::identity::SoftwareIdentity::new("pili-pos/1.0.0").unwrap()
}
