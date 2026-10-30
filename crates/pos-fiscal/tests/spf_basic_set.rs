//! SPF BasicSet Integration Tests
//!
//! These tests replay the core use cases from the SCE 2.0 POS specification:
//! - Direct sale (Event N)
//! - Complete refund (Event N with fdmRef to original)
//! - Interrupted sale with table management (Event P → signOrder → signSale)
//! - Provisional bill / pre-bill (Event P → signPreBill)
//! - Training mode (Event T)
//! - Copy of a previous ticket (Event C → signCopy)
//! - End-of-day Z-report (Event R → signReportTurnoverZ)
//! - Employee presence (Event S → signWorkIn / signWorkOut)
//!
//! Each test spins up a mock FDM (wiremock) that validates the incoming
//! GraphQL mutation and returns a realistic SignResult.

mod common;

use std::sync::atomic::{AtomicI64, Ordering};

use pos_core::config::PosConfig;
use pos_core::order::{Order, OrderLine, OrderStatus, OrderType};
use pos_core::vat::VatCode;
use pos_fiscal::archive::{self, ArchiveParams};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::event::*;
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::inputs::*;
use pos_fiscal::response::{FdmReference, SignResult};
use pos_fiscal::signature;
use pos_fiscal::types::*;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

/// A booking period for tests.
fn bk(period_id: &str) -> pos_fiscal::event::Booking {
    pos_fiscal::event::Booking {
        period_id: period_id.to_string(),
        date: "2026-07-03".to_string(),
    }
}

// ── Shared test helpers ────────────────────────────────────────

static TOTAL_COUNTER: AtomicI64 = AtomicI64::new(0);

fn next_total() -> i64 {
    TOTAL_COUNTER.fetch_add(1, Ordering::Relaxed) + 1
}

fn test_config(fdm_url: &str) -> PosConfig {
    PosConfig {
        table_count: 20,
        tables: None,
        payment_terminal: None,
        peppol: None,
        alerts: None,
        pos_id: "CFOD0061234567".to_string(),
        terminal_id: "POS1".to_string(),
        device_id: "DEV001".to_string(),
        restaurant_name: "Tokyo Sushi Bar".to_string(),
        restaurant_address: "Rue de la Sushi 1, 1000 Bruxelles".to_string(),
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
        printer_paper_mm: Default::default(),
        scale: None,
        kitchen_printer_paper_mm: Default::default(),
        fdm_token_file: None,
        fdm_ca_file: None,
        fdm_client_cert_file: None,
        fdm_client_key_file: None,
        restaurant_legal_name: None,
        day_end_hour: 5,
        digital_ticket: None,
        export: None,
    }
}

fn test_order(table: Option<i32>) -> Order {
    Order {
        id: 1,
        table_number: table,
        order_type: if table.is_some() {
            OrderType::DineIn
        } else {
            OrderType::Takeaway
        },
        status: OrderStatus::Open,
        user_insz: "85073000148".to_string(),
        created_at: chrono::Utc::now(),
        closed_at: None,
        subtotal_cents: 0,
        total_cents: 0,
    }
}

fn food_line(id: i64, name: &str, price_cents: i64, qty: i32) -> OrderLine {
    OrderLine {
        id,
        order_id: 1,
        menu_item_id: id,
        quantity: qty,
        unit_price_cents: price_cents,
        vat_code: VatCode::B, // 12% restaurant
        name: name.to_string(),
        modifiers_json: None,
        created_at: chrono::Utc::now(),
        department_id: "1".to_string(),
        department_name: "Test".to_string(),
        weight_grams: None,
        weighed_amount_cents: None,
    }
}

fn drink_line(id: i64, name: &str, price_cents: i64, qty: i32) -> OrderLine {
    OrderLine {
        id,
        order_id: 1,
        menu_item_id: id,
        quantity: qty,
        unit_price_cents: price_cents,
        vat_code: VatCode::A, // 21% beverages
        name: name.to_string(),
        modifiers_json: None,
        created_at: chrono::Utc::now(),
        department_id: "1".to_string(),
        department_name: "Test".to_string(),
        weight_grams: None,
        weighed_amount_cents: None,
    }
}

fn total_cents(lines: &[OrderLine]) -> i64 {
    lines
        .iter()
        .map(|l| l.unit_price_cents * l.quantity as i64)
        .sum()
}

/// Build a mock SignResult for a given mutation.
fn mock_sign_result(
    event_op: EventOperation,
    event_label: EventLabel,
    event_counter: i64,
    include_vat_calc: bool,
) -> SignResult {
    let tc = next_total();
    // Fake base64 signature (valid base64, not a real ECDSA sig)
    let fake_sig = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        b"fake-ecdsa-signature-for-testing-purpose",
    );
    let short_sig = if event_label == EventLabel::N {
        Some(signature::compute_short_signature(&fake_sig).unwrap())
    } else {
        None
    };

    SignResult {
        pos_id: "CFOD0061234567".to_string(),
        pos_fiscal_ticket_no: event_counter,
        pos_date_time: chrono::Local::now().to_rfc3339(),
        terminal_id: Some("POS1".to_string()),
        device_id: "DEV001".to_string(),
        event_operation: event_op,
        fdm_ref: FdmReference {
            fdm_id: "FDM00TEST01".to_string(),
            fdm_date_time: chrono::Utc::now().to_rfc3339(),
            event_label,
            event_counter,
            total_counter: tc,
        },
        fdm_sw_version: "MOCK-1.0".to_string(),
        digital_signature: fake_sig,
        short_signature: short_sig,
        verification_url: if event_label == EventLabel::N {
            Some("https://www.gs2.be/aBcDeFgHiJkLmNoPqRs".to_string())
        } else {
            None
        },
        vat_calc: if include_vat_calc {
            Some(vec![pos_fiscal::response::VatCalcItem {
                label: VatLabel::B,
                rate: 12.0,
                taxable_amount: 22.32,
                vat_amount: 2.68,
                total_amount: 25.00,
                out_of_scope: false,
            }])
        } else {
            None
        },
        buffer_capacity_used: 1.5,
        warnings: None,
        informations: None,
        footer: vec!["www.systemedecaisseenregistreuse.be".to_string()],
    }
}

/// Register a mock FDM endpoint that accepts any mutation and returns a SignResult.
async fn mock_fdm_mutation(
    server: &MockServer,
    mutation_name: &str,
    event_op: EventOperation,
    event_label: EventLabel,
    event_counter: i64,
    include_vat: bool,
) {
    let result = mock_sign_result(event_op, event_label, event_counter, include_vat);
    let mut response_data = serde_json::Map::new();
    response_data.insert(
        mutation_name.to_string(),
        serde_json::to_value(&result).unwrap(),
    );
    let body = serde_json::json!({ "data": response_data });

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(common::EchoRequest(body.clone()))
        .up_to_n_times(1)
        .named(mutation_name)
        .mount(server)
        .await;
}

fn setup_db() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    pos_db::schema::run_migrations(&conn).unwrap();
    conn
}

// ═══════════════════════════════════════════════════════════════
// Test 1: Direct Sale (Event N)
// SCE 2.0 §1.1 - Simple dine-in sale with food + drink
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_direct_sale() {
    let server = MockServer::start().await;
    let config = test_config(&format!("{}/graphql", server.uri()));
    let conn = setup_db();
    let bp_id = Uuid::new_v4().to_string();

    // Set up mock FDM
    mock_fdm_mutation(
        &server,
        "signSale",
        EventOperation::Sale,
        EventLabel::N,
        1,
        true,
    )
    .await;

    // Build order: 2x Saumon roll (12.50) + 1x Bière Asahi (3.90)
    let lines = vec![
        food_line(29, "Saumon roll x8", 1250, 2),
        drink_line(160, "Bière Asahi", 390, 1),
    ];
    let total = total_cents(&lines); // 2890

    let mut order = test_order(Some(5));
    order.total_cents = total;

    // Get sequence number
    let ticket_no = FiscalCounter::next(&conn, EventLabel::N).unwrap();
    assert_eq!(ticket_no, 1);

    // Build and send
    let sale = build_sale_input(
        &config,
        &ident(),
        &order,
        &lines,
        ticket_no,
        &bk(&bp_id),
        PaymentType::Cash,
        total,
        0,
        None,
    );

    // Verify structure
    assert_eq!(sale.common.pos_id, "CFOD0061234567");
    assert_eq!(sale.common.vat_no, "BE0499999960");
    assert_eq!(sale.common.est_no, "2123456791");
    assert_eq!(sale.common.employee_id, "85073000148");
    assert_eq!(sale.transaction.transaction_lines.len(), 2);
    assert!((sale.transaction.transaction_total - 28.90).abs() < 0.01);
    assert_eq!(sale.financials.len(), 1);
    assert!((sale.financials[0].amount - 28.90).abs() < 0.01);

    // Verify VAT codes
    let food_vat = &sale.transaction.transaction_lines[0].main_product.vats[0];
    assert_eq!(food_vat.label, VatLabel::B); // 12% restaurant
    let drink_vat = &sale.transaction.transaction_lines[1].main_product.vats[0];
    assert_eq!(drink_vat.label, VatLabel::A); // 21% beverages

    // Verify cost center (table 5)
    assert!(sale.cost_center.is_some());
    let cc = sale.cost_center.as_ref().unwrap();
    assert_eq!(cc.id, "T05");
    assert_eq!(cc.cost_center_type, CostCenterType::Table);

    // Send to FDM
    let fdm = FdmClient::new(config.fdm_url.as_str());
    let result = fdm.sign_sale(sale, false).await.unwrap();

    // Verify response
    assert_eq!(result.event_operation, EventOperation::Sale);
    assert_eq!(result.fdm_ref.event_label, EventLabel::N);
    assert_eq!(result.fdm_ref.event_counter, 1);
    assert!(result.short_signature.is_some());
    assert!(result.verification_url.is_some());
    assert!(result.vat_calc.is_some());

    // Verify shortSignature = Hex(SHA1(Base64Decode(digitalSignature)))
    let valid = signature::verify_short_signature(
        &result.digital_signature,
        result.short_signature.as_ref().unwrap(),
    )
    .unwrap();
    assert!(valid);

    // Archive
    let event_id = archive::archive_event(
        &conn,
        ArchiveParams {
            event_type: EventLabel::N,
            sequence_number: ticket_no,
            order_id: None, // test DB has no orders
            pos_id: &config.pos_id,
            terminal_id: &config.terminal_id,
            user_insz: &order.user_insz,
            request_json: &serde_json::to_string(&result).unwrap(),
            sign_result: Some(&result),
        },
    )
    .unwrap();
    assert!(event_id > 0);
}

// ═══════════════════════════════════════════════════════════════
// Test 2: Complete Refund (Event N with fdmRef)
// SCE 2.0 §1.2 - Refund references original sale
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_complete_refund() {
    let server = MockServer::start().await;
    let config = test_config(&format!("{}/graphql", server.uri()));
    let bp_id = Uuid::new_v4().to_string();

    mock_fdm_mutation(
        &server,
        "signSale",
        EventOperation::Sale,
        EventLabel::N,
        2,
        true,
    )
    .await;

    // Refund: negative quantities referencing original
    let order = test_order(Some(5));
    let lines = vec![OrderLine {
        id: 1,
        order_id: 1,
        menu_item_id: 29,
        quantity: -2, // negative = refund
        unit_price_cents: 1250,
        vat_code: VatCode::B,
        name: "Saumon roll x8".to_string(),
        modifiers_json: None,
        created_at: chrono::Utc::now(),
        department_id: "1".to_string(),
        department_name: "Test".to_string(),
        weight_grams: None,
        weighed_amount_cents: None,
    }];

    let mut sale = build_sale_input(
        &config,
        &ident(),
        &order,
        &lines,
        2,
        &bk(&bp_id),
        PaymentType::Cash,
        -2500,
        0,
        None,
    );

    // Attach reference to original sale (required for refunds)
    sale.fdm_ref = Some(FdmReferenceInput {
        fdm_id: "FDM00TEST01".to_string(),
        fdm_date_time: "2026-04-09T19:00:00+02:00".to_string(),
        event_label: EventLabel::N,
        event_counter: 1,
        total_counter: 1,
    });

    // Verify negative quantity and refund reference
    let line = &sale.transaction.transaction_lines[0];
    assert!(line.main_product.quantity < 0.0);
    assert!(line.line_total < 0.0);
    assert!(sale.fdm_ref.is_some());

    let fdm = FdmClient::new(config.fdm_url.as_str());
    let result = fdm.sign_sale(sale, false).await.unwrap();
    assert_eq!(result.fdm_ref.event_label, EventLabel::N);
}

// ═══════════════════════════════════════════════════════════════
// Test 3: Interrupted Sale with Table (Event P → N)
// SCE 2.0 §1.3 - Items added to open table, then finalized
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_interrupted_sale_table_flow() {
    let server = MockServer::start().await;
    let config = test_config(&format!("{}/graphql", server.uri()));
    let conn = setup_db();
    let bp_id = Uuid::new_v4().to_string();

    // Step 1: Customer sits at table 7, orders first round
    let order = test_order(Some(7));
    let initial_lines = vec![food_line(50, "Maki saumon x6", 470, 1)];

    mock_fdm_mutation(
        &server,
        "signOrder",
        EventOperation::Order,
        EventLabel::P,
        1,
        false,
    )
    .await;

    let p_ticket = FiscalCounter::next(&conn, EventLabel::P).unwrap();
    let order_input = build_order_input(
        &config,
        &ident(),
        &order,
        &initial_lines,
        p_ticket,
        &bk(&bp_id),
    );

    // Verify cost center is mandatory for signOrder
    assert_eq!(order_input.cost_center.id, "T07");
    assert_eq!(
        order_input.cost_center.reference,
        pos_fiscal::event::order_reference(&order)
    );

    let fdm = FdmClient::new(config.fdm_url.as_str());
    let p_result = fdm.sign_order(order_input, false).await.unwrap();
    assert_eq!(p_result.fdm_ref.event_label, EventLabel::P);
    assert_eq!(p_result.event_operation, EventOperation::Order);

    // Archive P event
    archive::archive_event(
        &conn,
        ArchiveParams {
            event_type: EventLabel::P,
            sequence_number: p_ticket,
            order_id: None, // test DB has no orders
            pos_id: &config.pos_id,
            terminal_id: &config.terminal_id,
            user_insz: &order.user_insz,
            request_json: "{}",
            sign_result: Some(&p_result),
        },
    )
    .unwrap();

    // Step 2: Customer adds more items
    // (In real flow, another signOrder would be sent)

    // Step 3: Customer pays - final sale (Event N)
    let all_lines = vec![
        food_line(50, "Maki saumon x6", 470, 1),
        drink_line(155, "Bière Tsingtao", 380, 2),
    ];
    let total = total_cents(&all_lines); // 1230

    // Need a new mock for signSale (wiremock consumes the previous ones)
    mock_fdm_mutation(
        &server,
        "signSale",
        EventOperation::Sale,
        EventLabel::N,
        1,
        true,
    )
    .await;

    let n_ticket = FiscalCounter::next(&conn, EventLabel::N).unwrap();
    let sale = build_sale_input(
        &config,
        &ident(),
        &order,
        &all_lines,
        n_ticket,
        &bk(&bp_id),
        PaymentType::CardDebit,
        total,
        0,
        None,
    );

    let n_result = fdm.sign_sale(sale, false).await.unwrap();
    assert_eq!(n_result.fdm_ref.event_label, EventLabel::N);
    assert_eq!(n_result.event_operation, EventOperation::Sale);

    // Verify sequential numbering: P=1, N=1 (independent counters)
    assert_eq!(FiscalCounter::current(&conn, EventLabel::P).unwrap(), 1);
    assert_eq!(FiscalCounter::current(&conn, EventLabel::N).unwrap(), 1);
}

// ═══════════════════════════════════════════════════════════════
// Test 4: Provisional Bill / Pre-Bill (Event P)
// SCE 2.0 §1.4 - Account overview before payment
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_pre_bill() {
    let server = MockServer::start().await;
    let config = test_config(&format!("{}/graphql", server.uri()));
    let conn = setup_db();
    let bp_id = Uuid::new_v4().to_string();

    let order = test_order(Some(3));
    let lines = vec![
        food_line(1, "Plateau sushi mix", 2480, 1),
        food_line(129, "Soupe miso", 400, 2),
        drink_line(156, "Bière Kirin", 380, 1),
    ];

    mock_fdm_mutation(
        &server,
        "signPreBill",
        EventOperation::PreBill,
        EventLabel::P,
        1,
        false,
    )
    .await;

    let ticket_no = FiscalCounter::next(&conn, EventLabel::P).unwrap();
    let pre_bill = build_pre_bill_input(&config, &ident(), &order, &lines, ticket_no, &bk(&bp_id));

    // PreBill has transaction but no financials (no payment yet)
    assert_eq!(pre_bill.transaction.transaction_lines.len(), 3);
    let expected_total = 24.80 + 4.00 * 2.0 + 3.80;
    assert!((pre_bill.transaction.transaction_total - expected_total).abs() < 0.01);

    // Cost center optional for pre-bill (table 3)
    assert!(pre_bill.cost_center.is_some());
    assert_eq!(pre_bill.cost_center.as_ref().unwrap().id, "T03");

    let fdm = FdmClient::new(config.fdm_url.as_str());
    let result = fdm.sign_pre_bill(pre_bill, false).await.unwrap();
    assert_eq!(result.fdm_ref.event_label, EventLabel::P);
    assert_eq!(result.event_operation, EventOperation::PreBill);

    // Pre-bill should NOT have shortSignature or verificationUrl
    assert!(result.short_signature.is_none());
    assert!(result.verification_url.is_none());
}

// ═══════════════════════════════════════════════════════════════
// Test 5: Training Mode (Event T)
// SCE 2.0 §1.5 - All events carry label T, not counted fiscally
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_training_mode() {
    let server = MockServer::start().await;
    let mut config = test_config(&format!("{}/graphql", server.uri()));
    config.training_mode = true;
    let conn = setup_db();
    let bp_id = Uuid::new_v4().to_string();

    let order = test_order(Some(1));
    let lines = vec![food_line(29, "Saumon", 200, 3)];
    let total = total_cents(&lines);

    // Training uses T label for counters
    mock_fdm_mutation(
        &server,
        "signSale",
        EventOperation::Sale,
        EventLabel::T,
        1,
        false,
    )
    .await;

    let ticket_no = FiscalCounter::next(&conn, EventLabel::T).unwrap();
    assert_eq!(ticket_no, 1);

    let sale = build_sale_input(
        &config,
        &ident(),
        &order,
        &lines,
        ticket_no,
        &bk(&bp_id),
        PaymentType::Cash,
        total,
        0,
        None,
    );

    // isTraining = true is passed to FDM
    let fdm = FdmClient::new(config.fdm_url.as_str());
    let result = fdm.sign_sale(sale, true).await.unwrap();
    assert_eq!(result.fdm_ref.event_label, EventLabel::T);

    // Training events use their own counter, don't affect N counter
    assert_eq!(FiscalCounter::current(&conn, EventLabel::T).unwrap(), 1);
    assert_eq!(FiscalCounter::current(&conn, EventLabel::N).unwrap(), 0);

    // Archive with T label
    archive::archive_event(
        &conn,
        ArchiveParams {
            event_type: EventLabel::T,
            sequence_number: ticket_no,
            order_id: None, // test DB has no orders
            pos_id: &config.pos_id,
            terminal_id: &config.terminal_id,
            user_insz: &order.user_insz,
            request_json: "{}",
            sign_result: Some(&result),
        },
    )
    .unwrap();

    // Verify training event archived with T label
    let archived = archive::get_archived_event(&conn, EventLabel::T, 1).unwrap();
    assert!(archived.is_some());
}

// ═══════════════════════════════════════════════════════════════
// Test 6: Copy Ticket (Event C)
// SCE 2.0 §1.8 - Reprint references original via fdmRef
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_copy_ticket() {
    let server = MockServer::start().await;
    let config = test_config(&format!("{}/graphql", server.uri()));
    let conn = setup_db();
    let bp_id = Uuid::new_v4().to_string();

    // The original sale we're copying
    let original_ref = FdmReference {
        fdm_id: "FDM00TEST01".to_string(),
        fdm_date_time: "2026-04-09T19:30:00+02:00".to_string(),
        event_label: EventLabel::N,
        event_counter: 42,
        total_counter: 100,
    };

    mock_fdm_mutation(
        &server,
        "signCopy",
        EventOperation::Copy,
        EventLabel::C,
        1,
        false,
    )
    .await;

    let ticket_no = FiscalCounter::next(&conn, EventLabel::C).unwrap();
    let copy = build_copy_input(
        &config,
        &ident(),
        ticket_no,
        "85073000148",
        &bk(&bp_id),
        &original_ref,
    );

    // Verify the fdmRef points to original
    assert_eq!(copy.fdm_ref.fdm_id, "FDM00TEST01");
    assert_eq!(copy.fdm_ref.event_label, EventLabel::N);
    assert_eq!(copy.fdm_ref.event_counter, 42);
    assert_eq!(copy.fdm_ref.total_counter, 100);

    let fdm = FdmClient::new(config.fdm_url.as_str());
    let result = fdm.sign_copy(copy, false).await.unwrap();
    assert_eq!(result.event_operation, EventOperation::Copy);
    assert_eq!(result.fdm_ref.event_label, EventLabel::C);

    // Copy has its own counter
    assert_eq!(FiscalCounter::current(&conn, EventLabel::C).unwrap(), 1);
}

// ═══════════════════════════════════════════════════════════════
// Test 7: Z-Report (Event R)
// SCE 2.0 §Reports - End-of-day with reportNo + turnover
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_z_report() {
    let server = MockServer::start().await;
    let config = test_config(&format!("{}/graphql", server.uri()));
    let conn = setup_db();
    let bp_id = Uuid::new_v4().to_string();
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    mock_fdm_mutation(
        &server,
        "signReportTurnoverZ",
        EventOperation::ReportTurnoverZ,
        EventLabel::R,
        1,
        false,
    )
    .await;

    let ticket_no = FiscalCounter::next(&conn, EventLabel::R).unwrap();
    let report_no = 1_i64;

    // Build turnover data (minimal for test)
    let turnover_data = TurnoverReportData {
        pos_devices: vec![PosDeviceInput {
            pos_id: config.pos_id.clone(),
            terminal_id: Some(config.terminal_id.clone()),
            first_pos_date_time: "2026-04-09T11:00:00+02:00".to_string(),
            last_pos_date_time: "2026-04-09T22:30:00+02:00".to_string(),
        }],
        fdm_devices: vec![FdmDeviceInput {
            fdm_id: "FDM00TEST01".to_string(),
            first_fdm_date_time: "2026-04-09T11:00:05+02:00".to_string(),
            first_total_counter: 1,
            last_fdm_date_time: "2026-04-09T22:30:10+02:00".to_string(),
            last_total_counter: 50,
        }],
        turnover: TurnoverInput {
            transactions: vec![
                EventTotalInput {
                    event_label: EventLabel::N,
                    ticket_count: 45,
                    amount: 3250.00,
                },
                EventTotalInput {
                    event_label: EventLabel::P,
                    ticket_count: 12,
                    amount: 0.0,
                },
            ],
            departments: vec![DepartmentTotalInput {
                department_id: "FOOD".to_string(),
                department_name: "Restaurant".to_string(),
                amount: 3250.00,
            }],
            vats: vec![
                VatTotalInput {
                    label: VatLabel::B,
                    rate: 12.0,
                    taxable_amount: 2589.29,
                    vat_amount: 310.71,
                    total_amount: 2900.00,
                },
                VatTotalInput {
                    label: VatLabel::A,
                    rate: 21.0,
                    taxable_amount: 289.26,
                    vat_amount: 60.74,
                    total_amount: 350.00,
                },
            ],
            payments: vec![PaymentTotalInput {
                id: "cash".to_string(),
                name: "Cash".to_string(),
                payment_type: PaymentType::Cash,
                amount: vec![PaymentTotalAmountInput {
                    amount_type: PaymentLineType::Payment,
                    normal_amount: 3250.00,
                    negative_corrections: 0.0,
                    positive_corrections: 0.0,
                    corrections_count: 0,
                    total_amount: 3250.00,
                    normal_foreign: None,
                    corrections_foreign: None,
                }],
            }],
            drawers_open_count: 45,
            neg_quantities: vec![],
            price_changes: vec![],
            invoices: None,
        },
    };

    let report = build_report_turnover_z(
        &config,
        &ident(),
        ticket_no,
        "85073000148",
        &bk(&bp_id),
        report_no,
        &today,
        turnover_data,
    );

    // Verify Z-report structure
    assert_eq!(report.report_no, 1);
    assert_eq!(report.report_booking_date, today);
    assert_eq!(report.turnover.transactions.len(), 2);
    assert_eq!(report.turnover.vats.len(), 2);
    assert_eq!(report.turnover.drawers_open_count, 45);
    assert_eq!(report.pos_devices.len(), 1);
    assert_eq!(report.fdm_devices.len(), 1);

    // Verify it serializes correctly (camelCase)
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("reportNo"));
    assert!(json.contains("reportBookingDate"));
    assert!(json.contains("drawersOpenCount"));
    assert!(json.contains("ticketCount"));

    let fdm = FdmClient::new(config.fdm_url.as_str());
    let result = fdm.sign_report_turnover_z(report, false).await.unwrap();
    assert_eq!(result.event_operation, EventOperation::ReportTurnoverZ);
    assert_eq!(result.fdm_ref.event_label, EventLabel::R);

    // Store Z-report in DB
    pos_db::queries::reports::save_z_report(&conn, &today, report_no, &json, None).unwrap();
    let next = pos_db::queries::reports::next_z_report_number(&conn).unwrap();
    assert_eq!(next, 2);
}

// ═══════════════════════════════════════════════════════════════
// Test 8: Employee Presence (Event S)
// SCE 2.0 §1.9 - Work in / work out
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_employee_work_in_out() {
    let server = MockServer::start().await;
    let config = test_config(&format!("{}/graphql", server.uri()));
    let conn = setup_db();
    let bp_id = Uuid::new_v4().to_string();

    // Work In
    mock_fdm_mutation(
        &server,
        "signWorkIn",
        EventOperation::WorkIn,
        EventLabel::S,
        1,
        false,
    )
    .await;

    let s_ticket = FiscalCounter::next(&conn, EventLabel::S).unwrap();
    let work_in = build_work_in_out(&config, &ident(), s_ticket, "85073000148", &bk(&bp_id));
    assert_eq!(work_in.common.employee_id, "85073000148");

    let fdm = FdmClient::new(config.fdm_url.as_str());
    let result = fdm.sign_work_in(work_in, false).await.unwrap();
    assert_eq!(result.event_operation, EventOperation::WorkIn);
    assert_eq!(result.fdm_ref.event_label, EventLabel::S);

    // Work Out
    mock_fdm_mutation(
        &server,
        "signWorkOut",
        EventOperation::WorkOut,
        EventLabel::S,
        2,
        false,
    )
    .await;

    let s_ticket2 = FiscalCounter::next(&conn, EventLabel::S).unwrap();
    assert_eq!(s_ticket2, 2);
    let work_out = build_work_in_out(&config, &ident(), s_ticket2, "85073000148", &bk(&bp_id));

    let result2 = fdm.sign_work_out(work_out, false).await.unwrap();
    assert_eq!(result2.event_operation, EventOperation::WorkOut);

    // S counter incremented twice
    assert_eq!(FiscalCounter::current(&conn, EventLabel::S).unwrap(), 2);
}

// ═══════════════════════════════════════════════════════════════
// Test 9: Sequential Numbering Across All Event Types
// SCE 2.0 §2 - Each event type has independent counter
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_counter_independence() {
    let conn = setup_db();

    // Simulate a full service day
    assert_eq!(FiscalCounter::next(&conn, EventLabel::S).unwrap(), 1); // Work in
    assert_eq!(FiscalCounter::next(&conn, EventLabel::P).unwrap(), 1); // First order
    assert_eq!(FiscalCounter::next(&conn, EventLabel::P).unwrap(), 2); // Second order
    assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 1); // First sale
    assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 2); // Second sale
    assert_eq!(FiscalCounter::next(&conn, EventLabel::C).unwrap(), 1); // Copy of sale 1
    assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 3); // Third sale
    assert_eq!(FiscalCounter::next(&conn, EventLabel::R).unwrap(), 1); // Z-report
    assert_eq!(FiscalCounter::next(&conn, EventLabel::S).unwrap(), 2); // Work out

    // Verify final state
    assert_eq!(FiscalCounter::current(&conn, EventLabel::S).unwrap(), 2);
    assert_eq!(FiscalCounter::current(&conn, EventLabel::P).unwrap(), 2);
    assert_eq!(FiscalCounter::current(&conn, EventLabel::N).unwrap(), 3);
    assert_eq!(FiscalCounter::current(&conn, EventLabel::C).unwrap(), 1);
    assert_eq!(FiscalCounter::current(&conn, EventLabel::R).unwrap(), 1);
    assert_eq!(FiscalCounter::current(&conn, EventLabel::T).unwrap(), 0); // Never used
}

// ═══════════════════════════════════════════════════════════════
// Test 10: Full Service Day Scenario
// Combines: work-in → orders → sales → copy → Z-report → work-out
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_full_service_day() {
    let server = MockServer::start().await;
    let config = test_config(&format!("{}/graphql", server.uri()));
    let conn = setup_db();
    let bp_id = Uuid::new_v4().to_string();

    // 1. Employee clocks in
    mock_fdm_mutation(
        &server,
        "signWorkIn",
        EventOperation::WorkIn,
        EventLabel::S,
        1,
        false,
    )
    .await;
    let fdm = FdmClient::new(config.fdm_url.as_str());
    let s1 = FiscalCounter::next(&conn, EventLabel::S).unwrap();
    let wi = build_work_in_out(&config, &ident(), s1, "85073000148", &bk(&bp_id));
    fdm.sign_work_in(wi, false).await.unwrap();

    // 2. Table 1 orders (P event)
    mock_fdm_mutation(
        &server,
        "signOrder",
        EventOperation::Order,
        EventLabel::P,
        1,
        false,
    )
    .await;
    let p1 = FiscalCounter::next(&conn, EventLabel::P).unwrap();
    let order1 = test_order(Some(1));
    let lines1 = vec![food_line(3, "M1", 1790, 1)];
    let oi = build_order_input(&config, &ident(), &order1, &lines1, p1, &bk(&bp_id));
    fdm.sign_order(oi, false).await.unwrap();

    // 3. Table 1 pays (N event)
    mock_fdm_mutation(
        &server,
        "signSale",
        EventOperation::Sale,
        EventLabel::N,
        1,
        true,
    )
    .await;
    let n1 = FiscalCounter::next(&conn, EventLabel::N).unwrap();
    let sale1 = build_sale_input(
        &config,
        &ident(),
        &order1,
        &lines1,
        n1,
        &bk(&bp_id),
        PaymentType::Cash,
        1790,
        0,
        None,
    );
    let sale1_result = fdm.sign_sale(sale1, false).await.unwrap();

    // 4. Copy of table 1 receipt
    mock_fdm_mutation(
        &server,
        "signCopy",
        EventOperation::Copy,
        EventLabel::C,
        1,
        false,
    )
    .await;
    let c1 = FiscalCounter::next(&conn, EventLabel::C).unwrap();
    let copy = build_copy_input(
        &config,
        &ident(),
        c1,
        "85073000148",
        &bk(&bp_id),
        &sale1_result.fdm_ref,
    );
    fdm.sign_copy(copy, false).await.unwrap();

    // 5. Z-report
    mock_fdm_mutation(
        &server,
        "signReportTurnoverZ",
        EventOperation::ReportTurnoverZ,
        EventLabel::R,
        1,
        false,
    )
    .await;
    let r1 = FiscalCounter::next(&conn, EventLabel::R).unwrap();
    let report_data = TurnoverReportData {
        pos_devices: vec![PosDeviceInput {
            pos_id: config.pos_id.clone(),
            terminal_id: Some(config.terminal_id.clone()),
            first_pos_date_time: chrono::Local::now().to_rfc3339(),
            last_pos_date_time: chrono::Local::now().to_rfc3339(),
        }],
        fdm_devices: vec![FdmDeviceInput {
            fdm_id: "FDM00TEST01".to_string(),
            first_fdm_date_time: chrono::Utc::now().to_rfc3339(),
            first_total_counter: 1,
            last_fdm_date_time: chrono::Utc::now().to_rfc3339(),
            last_total_counter: 5,
        }],
        turnover: TurnoverInput {
            transactions: vec![EventTotalInput {
                event_label: EventLabel::N,
                ticket_count: 1,
                amount: 17.90,
            }],
            departments: vec![DepartmentTotalInput {
                department_id: "FOOD".to_string(),
                department_name: "Restaurant".to_string(),
                amount: 17.90,
            }],
            vats: vec![VatTotalInput {
                label: VatLabel::B,
                rate: 12.0,
                taxable_amount: 15.98,
                vat_amount: 1.92,
                total_amount: 17.90,
            }],
            payments: vec![PaymentTotalInput {
                id: "cash".to_string(),
                name: "Cash".to_string(),
                payment_type: PaymentType::Cash,
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
            }],
            drawers_open_count: 1,
            neg_quantities: vec![],
            price_changes: vec![],
            invoices: None,
        },
    };
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let z = build_report_turnover_z(
        &config,
        &ident(),
        r1,
        "85073000148",
        &bk(&bp_id),
        1,
        &today,
        report_data,
    );
    fdm.sign_report_turnover_z(z, false).await.unwrap();

    // 6. Employee clocks out
    mock_fdm_mutation(
        &server,
        "signWorkOut",
        EventOperation::WorkOut,
        EventLabel::S,
        2,
        false,
    )
    .await;
    let s2 = FiscalCounter::next(&conn, EventLabel::S).unwrap();
    let wo = build_work_in_out(&config, &ident(), s2, "85073000148", &bk(&bp_id));
    fdm.sign_work_out(wo, false).await.unwrap();

    // Verify all counters
    assert_eq!(FiscalCounter::current(&conn, EventLabel::S).unwrap(), 2);
    assert_eq!(FiscalCounter::current(&conn, EventLabel::P).unwrap(), 1);
    assert_eq!(FiscalCounter::current(&conn, EventLabel::N).unwrap(), 1);
    assert_eq!(FiscalCounter::current(&conn, EventLabel::C).unwrap(), 1);
    assert_eq!(FiscalCounter::current(&conn, EventLabel::R).unwrap(), 1);
}

/// The software identity of the Linux shell at version 1.0.0.
fn ident() -> pos_fiscal::identity::SoftwareIdentity {
    pos_fiscal::identity::SoftwareIdentity::new("pili-pos/1.0.0").unwrap()
}
