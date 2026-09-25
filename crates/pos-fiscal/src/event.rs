use chrono::{Local, Utc};
use uuid::Uuid;

use crate::inputs::*;
use crate::response::FdmReference;
use crate::types::*;
use pos_core::config::PosConfig;
use pos_core::order::{Order, OrderLine};
use pos_core::vat::VatCode;

/// Software version string sent to FDM.
pub const POS_SW_VERSION: &str = concat!("moyun-pos/", env!("CARGO_PKG_VERSION"));

// ── Common Fields ──────────────────────────────────────────────

/// Build common fields shared by all FDM mutations. `ticket_medium`
/// states what the POS does with the event: `None` when nothing is
/// printed (drawer, money in/out, staff presence).
pub fn build_common_fields(
    config: &PosConfig,
    ticket_number: i64,
    employee_insz: &str,
    booking_period_id: &str,
    ticket_medium: TicketMedium,
) -> CommonFields {
    let now = Local::now();

    CommonFields {
        language: fiscal_language(&config.language),
        vat_no: config.restaurant_vat.clone(),
        est_no: config.restaurant_est_no.clone(),
        pos_id: config.pos_id.clone(),
        pos_fiscal_ticket_no: ticket_number,
        pos_date_time: now.to_rfc3339(),
        pos_sw_version: POS_SW_VERSION.to_string(),
        terminal_id: Some(config.terminal_id.clone()),
        device_id: config.device_id.clone(),
        booking_period_id: booking_period_id.to_string(),
        booking_date: Utc::now().format("%Y-%m-%d").to_string(),
        ticket_medium,
        employee_id: employee_insz.to_string(),
    }
}

/// FDM `language`: the national language of the establishment's
/// linguistic region (`pos.json::language`, restricted to FR/NL/DE).
fn fiscal_language(code: &str) -> Language {
    match crate::ticket::ReceiptLanguage::from_code(code) {
        crate::ticket::ReceiptLanguage::Fr => Language::Fr,
        crate::ticket::ReceiptLanguage::Nl => Language::Nl,
        crate::ticket::ReceiptLanguage::De => Language::De,
    }
}

/// Convert pos-core VatCode to fiscal VatLabel.
pub fn vat_code_to_label(code: VatCode) -> VatLabel {
    match code {
        VatCode::A => VatLabel::A,
        VatCode::B => VatLabel::B,
        VatCode::C => VatLabel::C,
        VatCode::D => VatLabel::D,
        VatCode::X => VatLabel::X,
    }
}

/// Build transaction lines from order lines (shared by Sale, Order, PreBill).
fn build_transaction(lines: &[OrderLine]) -> TransactionInput {
    let transaction_lines: Vec<TransactionLineInput> = lines
        .iter()
        .map(|line| {
            let unit_price = line.unit_price_cents as f64 / 100.0;
            let line_total = unit_price * line.quantity as f64;
            let vat_label = vat_code_to_label(line.vat_code);

            TransactionLineInput {
                line_type: TransactionLineType::SingleProduct,
                main_product: ProductInput {
                    gtin: None,
                    product_id: line.menu_item_id.to_string(),
                    product_name: line.name.clone(),
                    department_id: line.department_id.clone(),
                    department_name: line.department_name.clone(),
                    quantity: line.quantity as f64,
                    quantity_type: QuantityType::Piece,
                    neg_quantity_reason: if line.quantity < 0 {
                        Some(NegQuantityReason::Refund)
                    } else {
                        None
                    },
                    unit_price,
                    vats: vec![VatInput {
                        label: vat_label,
                        price: line_total,
                        price_changes: None,
                    }],
                },
                sub_products: None,
                cost_center: None,
                line_total,
            }
        })
        .collect();

    let transaction_total = transaction_lines.iter().map(|l| l.line_total).sum::<f64>();

    TransactionInput {
        transaction_lines,
        transaction_total,
    }
}

/// Build a CostCenterInput for a table.
fn build_table_cost_center(table_number: i32, reference: &str) -> CostCenterInput {
    CostCenterInput {
        id: format!("T{table_number:02}"),
        cost_center_type: CostCenterType::Table,
        reference: reference.to_string(),
        cost_center: None,
    }
}

// ── Event N — Sale ─────────────────────────────────────────────

/// Build a SaleInput from an order + its lines + payment info.
/// For refunds, pass `original_fdm_ref` referencing the original ticket.
// The argument list mirrors the spec-mandated SaleInput composition;
// grouping them into an ad-hoc struct would just move the same fields.
#[allow(clippy::too_many_arguments)]
pub fn build_sale_input(
    config: &PosConfig,
    order: &Order,
    lines: &[OrderLine],
    ticket_number: i64,
    booking_period_id: &str,
    payment_type: PaymentType,
    payment_amount_cents: i64,
    original_fdm_ref: Option<&FdmReference>,
) -> SaleInput {
    let common = build_common_fields(
        config,
        ticket_number,
        &order.user_insz,
        booking_period_id,
        TicketMedium::Paper,
    );
    let transaction = build_transaction(lines);

    let cost_center = order
        .table_number
        .map(|t| build_table_cost_center(t, &Uuid::new_v4().to_string()));

    let payment_amount = payment_amount_cents as f64 / 100.0;
    let financials = vec![PaymentLineInput {
        id: "PAY1".to_string(),
        name: match payment_type {
            PaymentType::Cash => "Cash".to_string(),
            PaymentType::CardDebit => "Card".to_string(),
            _ => "Payment".to_string(),
        },
        payment_type,
        provider: None,
        input_method: InputMethod::Manual,
        amount: payment_amount,
        amount_type: PaymentLineType::Payment,
        foreign_currency: None,
        reference: None,
        drawer: None,
    }];

    SaleInput {
        common,
        fdm_ref: original_fdm_ref.map(|r| FdmReferenceInput {
            fdm_id: r.fdm_id.clone(),
            fdm_date_time: r.fdm_date_time.clone(),
            event_label: r.event_label,
            event_counter: r.event_counter,
            total_counter: r.total_counter,
        }),
        cost_center,
        transaction,
        financials,
    }
}

// ── Event P — Order (pro forma) ────────────────────────────────

/// Build an OrderInput (event P) for an open table / interrupted sale.
/// `cost_center_ref` is the session GUID grouping connected transactions.
pub fn build_order_input(
    config: &PosConfig,
    order: &Order,
    lines: &[OrderLine],
    ticket_number: i64,
    booking_period_id: &str,
    cost_center_ref: &str,
) -> OrderInput {
    let common = build_common_fields(
        config,
        ticket_number,
        &order.user_insz,
        booking_period_id,
        TicketMedium::Paper,
    );
    let transaction = build_transaction(lines);
    let table = order.table_number.unwrap_or(0);
    let cost_center = build_table_cost_center(table, cost_center_ref);

    OrderInput {
        common,
        cost_center,
        transaction,
    }
}

/// Build a PreBillInput (event P) for a provisional bill / account overview.
pub fn build_pre_bill_input(
    config: &PosConfig,
    order: &Order,
    lines: &[OrderLine],
    ticket_number: i64,
    booking_period_id: &str,
) -> PreBillInput {
    let common = build_common_fields(
        config,
        ticket_number,
        &order.user_insz,
        booking_period_id,
        TicketMedium::Paper,
    );
    let transaction = build_transaction(lines);
    let cost_center = order
        .table_number
        .map(|t| build_table_cost_center(t, &Uuid::new_v4().to_string()));

    PreBillInput {
        common,
        cost_center,
        transaction,
    }
}

// ── Event C — Copy ─────────────────────────────────────────────

/// Build a CopyInput for reprinting a previous event.
/// `fdm_ref` references the original event (fdmId + eventLabel + counters).
pub fn build_copy_input(
    config: &PosConfig,
    ticket_number: i64,
    employee_insz: &str,
    booking_period_id: &str,
    original_ref: &FdmReference,
) -> CopyInput {
    let common = build_common_fields(
        config,
        ticket_number,
        employee_insz,
        booking_period_id,
        TicketMedium::Paper,
    );

    CopyInput {
        common,
        fdm_ref: FdmReferenceInput {
            fdm_id: original_ref.fdm_id.clone(),
            fdm_date_time: original_ref.fdm_date_time.clone(),
            event_label: original_ref.event_label,
            event_counter: original_ref.event_counter,
            total_counter: original_ref.total_counter,
        },
    }
}

// ── Event I — Invoice ──────────────────────────────────────────

/// Build an InvoiceInput (event I) for a B2B invoice covering one or
/// more prior sales. `fdm_refs` reference the underlying N events
/// (the M150 fixture shows an invoice consolidating two sales).
pub fn build_invoice_input(
    config: &PosConfig,
    ticket_number: i64,
    employee_insz: &str,
    booking_period_id: &str,
    invoice_no: &str,
    customer_vat_no: &str,
    fdm_refs: &[FdmReference],
) -> InvoiceInput {
    let common = build_common_fields(
        config,
        ticket_number,
        employee_insz,
        booking_period_id,
        TicketMedium::Paper,
    );

    InvoiceInput {
        common,
        invoice_no: invoice_no.to_string(),
        customer_vat_no: customer_vat_no.to_string(),
        cost_center: None,
        fdm_refs: fdm_refs
            .iter()
            .map(|r| FdmReferenceInput {
                fdm_id: r.fdm_id.clone(),
                fdm_date_time: r.fdm_date_time.clone(),
                event_label: r.event_label,
                event_counter: r.event_counter,
                total_counter: r.total_counter,
            })
            .collect(),
    }
}

// ── Event S — Social (Work In/Out) ─────────────────────────────

/// Build a WorkInOutInput for employee clock in/out.
pub fn build_work_in_out(
    config: &PosConfig,
    ticket_number: i64,
    employee_insz: &str,
    booking_period_id: &str,
) -> WorkInOutInput {
    WorkInOutInput {
        common: build_common_fields(
            config,
            ticket_number,
            employee_insz,
            booking_period_id,
            TicketMedium::None,
        ),
    }
}

// ── Event R — Reports ──────────────────────────────────────────

/// Data needed to build turnover report inputs.
pub struct TurnoverReportData {
    pub pos_devices: Vec<PosDeviceInput>,
    pub fdm_devices: Vec<FdmDeviceInput>,
    pub turnover: TurnoverInput,
}

/// Build a ReportTurnoverXInput (non-resetting mid-day report).
pub fn build_report_turnover_x(
    config: &PosConfig,
    ticket_number: i64,
    employee_insz: &str,
    booking_period_id: &str,
    data: TurnoverReportData,
) -> ReportTurnoverXInput {
    let common = build_common_fields(
        config,
        ticket_number,
        employee_insz,
        booking_period_id,
        TicketMedium::Paper,
    );

    ReportTurnoverXInput {
        common,
        pos_devices: data.pos_devices,
        fdm_devices: data.fdm_devices,
        turnover: data.turnover,
    }
}

/// Build a ReportTurnoverZInput (end-of-day report, resets counters).
pub fn build_report_turnover_z(
    config: &PosConfig,
    ticket_number: i64,
    employee_insz: &str,
    booking_period_id: &str,
    report_no: i64,
    report_booking_date: &str,
    data: TurnoverReportData,
) -> ReportTurnoverZInput {
    let common = build_common_fields(
        config,
        ticket_number,
        employee_insz,
        booking_period_id,
        TicketMedium::Paper,
    );

    ReportTurnoverZInput {
        common,
        report_no,
        report_booking_date: report_booking_date.to_string(),
        pos_devices: data.pos_devices,
        fdm_devices: data.fdm_devices,
        turnover: data.turnover,
    }
}

/// Data needed to build user report inputs.
pub struct UserReportData {
    pub pos_devices: Vec<PosDeviceInput>,
    pub fdm_devices: Vec<FdmDeviceInput>,
    pub users: Vec<UserItemInput>,
}

/// Build a ReportUserXInput (non-resetting user report).
pub fn build_report_user_x(
    config: &PosConfig,
    ticket_number: i64,
    employee_insz: &str,
    booking_period_id: &str,
    data: UserReportData,
) -> ReportUserXInput {
    let common = build_common_fields(
        config,
        ticket_number,
        employee_insz,
        booking_period_id,
        TicketMedium::Paper,
    );

    ReportUserXInput {
        common,
        pos_devices: data.pos_devices,
        fdm_devices: data.fdm_devices,
        users: data.users,
    }
}

/// Build a ReportUserZInput (end-of-day user report).
pub fn build_report_user_z(
    config: &PosConfig,
    ticket_number: i64,
    employee_insz: &str,
    booking_period_id: &str,
    report_no: i64,
    report_booking_date: &str,
    data: UserReportData,
) -> ReportUserZInput {
    let common = build_common_fields(
        config,
        ticket_number,
        employee_insz,
        booking_period_id,
        TicketMedium::Paper,
    );

    ReportUserZInput {
        common,
        report_no,
        report_booking_date: report_booking_date.to_string(),
        pos_devices: data.pos_devices,
        fdm_devices: data.fdm_devices,
        users: data.users,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pos_core::order::{OrderStatus, OrderType};

    fn test_config() -> PosConfig {
        PosConfig {
            pos_id: "CFOD0061234567".to_string(),
            terminal_id: "POS1".to_string(),
            device_id: "DEV001".to_string(),
            restaurant_name: "Tokyo Sushi Bar".to_string(),
            restaurant_address: "Rue Test 1, 1000 Bruxelles".to_string(),
            restaurant_vat: "BE0499999960".to_string(),
            restaurant_est_no: "2123456749".to_string(),
            ..Default::default()
        }
    }

    fn test_order() -> Order {
        Order {
            id: 1,
            table_number: Some(5),
            order_type: OrderType::DineIn,
            status: OrderStatus::Open,
            user_insz: "00000000029".to_string(),
            created_at: chrono::Utc::now(),
            closed_at: None,
            subtotal_cents: 2500,
            total_cents: 2500,
        }
    }

    fn test_lines() -> Vec<OrderLine> {
        vec![OrderLine {
            id: 1,
            order_id: 1,
            menu_item_id: 42,
            quantity: 2,
            unit_price_cents: 1250,
            vat_code: VatCode::B,
            name: "Saumon roll x8".to_string(),
            modifiers_json: None,
            created_at: chrono::Utc::now(),
            department_id: "1".to_string(),
            department_name: "Test".to_string(),
        }]
    }

    #[test]
    fn build_sale_input_structure() {
        let config = test_config();
        let order = test_order();
        let lines = test_lines();
        let bp_id = Uuid::new_v4().to_string();

        let sale = build_sale_input(
            &config,
            &order,
            &lines,
            1,
            &bp_id,
            PaymentType::Cash,
            2500,
            None,
        );

        assert_eq!(sale.common.pos_id, "CFOD0061234567");
        assert_eq!(sale.common.employee_id, "00000000029");
        assert_eq!(sale.common.est_no, "2123456749");
        assert_eq!(sale.transaction.transaction_lines.len(), 1);

        let line = &sale.transaction.transaction_lines[0];
        assert_eq!(line.main_product.product_name, "Saumon roll x8");
        assert!((line.line_total - 25.0).abs() < 0.01);
        assert!((sale.transaction.transaction_total - 25.0).abs() < 0.01);
    }

    #[test]
    fn sale_input_serializes_to_json() {
        let config = test_config();
        let order = test_order();
        let lines = test_lines();
        let bp_id = Uuid::new_v4().to_string();

        let sale = build_sale_input(
            &config,
            &order,
            &lines,
            1,
            &bp_id,
            PaymentType::Cash,
            2500,
            None,
        );
        let json = serde_json::to_string(&sale).unwrap();

        assert!(json.contains("posId"));
        assert!(json.contains("posFiscalTicketNo"));
        assert!(json.contains("transactionLines"));
        assert!(json.contains("productName"));
        assert!(json.contains("estNo"));
    }

    #[test]
    fn build_order_input_has_cost_center() {
        let config = test_config();
        let order = test_order();
        let lines = test_lines();
        let bp_id = Uuid::new_v4().to_string();
        let cc_ref = Uuid::new_v4().to_string();

        let order_input = build_order_input(&config, &order, &lines, 1, &bp_id, &cc_ref);

        assert_eq!(order_input.cost_center.id, "T05");
        assert_eq!(
            order_input.cost_center.cost_center_type,
            CostCenterType::Table
        );
        assert_eq!(order_input.cost_center.reference, cc_ref);
        assert_eq!(order_input.transaction.transaction_lines.len(), 1);
    }

    #[test]
    fn build_copy_input_references_original() {
        let config = test_config();
        let bp_id = Uuid::new_v4().to_string();
        let original = FdmReference {
            fdm_id: "FDM001".to_string(),
            fdm_date_time: "2026-04-09T19:00:00+02:00".to_string(),
            event_label: EventLabel::N,
            event_counter: 42,
            total_counter: 100,
        };

        let copy = build_copy_input(&config, 2, "00000000029", &bp_id, &original);

        assert_eq!(copy.fdm_ref.fdm_id, "FDM001");
        assert_eq!(copy.fdm_ref.event_counter, 42);
        assert_eq!(copy.fdm_ref.total_counter, 100);
    }

    #[test]
    fn negative_quantity_sets_refund_reason() {
        let config = test_config();
        let order = test_order();
        let lines = vec![OrderLine {
            id: 1,
            order_id: 1,
            menu_item_id: 42,
            quantity: -2,
            unit_price_cents: 1250,
            vat_code: VatCode::B,
            name: "Saumon roll x8".to_string(),
            modifiers_json: None,
            created_at: chrono::Utc::now(),
            department_id: "1".to_string(),
            department_name: "Test".to_string(),
        }];
        let bp_id = Uuid::new_v4().to_string();

        let sale = build_sale_input(
            &config,
            &order,
            &lines,
            1,
            &bp_id,
            PaymentType::Cash,
            -2500,
            None,
        );

        let product = &sale.transaction.transaction_lines[0].main_product;
        assert_eq!(product.neg_quantity_reason, Some(NegQuantityReason::Refund));
    }

    #[test]
    fn positive_quantity_has_no_reason() {
        let config = test_config();
        let order = test_order();
        let lines = test_lines();
        let bp_id = Uuid::new_v4().to_string();

        let sale = build_sale_input(
            &config,
            &order,
            &lines,
            1,
            &bp_id,
            PaymentType::Cash,
            2500,
            None,
        );

        let product = &sale.transaction.transaction_lines[0].main_product;
        assert_eq!(product.neg_quantity_reason, None);
    }

    #[test]
    fn refund_sale_includes_fdm_ref() {
        let config = test_config();
        let order = test_order();
        let lines = vec![OrderLine {
            id: 1,
            order_id: 1,
            menu_item_id: 42,
            quantity: -1,
            unit_price_cents: 1250,
            vat_code: VatCode::B,
            name: "Saumon roll x8".to_string(),
            modifiers_json: None,
            created_at: chrono::Utc::now(),
            department_id: "1".to_string(),
            department_name: "Test".to_string(),
        }];
        let bp_id = Uuid::new_v4().to_string();
        let original_ref = FdmReference {
            fdm_id: "FDM001".to_string(),
            fdm_date_time: "2026-04-12T19:00:00+02:00".to_string(),
            event_label: EventLabel::N,
            event_counter: 42,
            total_counter: 100,
        };

        let sale = build_sale_input(
            &config,
            &order,
            &lines,
            2,
            &bp_id,
            PaymentType::Cash,
            -1250,
            Some(&original_ref),
        );

        let fdm_ref = sale.fdm_ref.clone().expect("refund should have fdm_ref");
        assert_eq!(fdm_ref.fdm_id, "FDM001");
        assert_eq!(fdm_ref.event_label, EventLabel::N);
        assert_eq!(fdm_ref.event_counter, 42);
        assert_eq!(fdm_ref.total_counter, 100);

        // Verify JSON serialization includes fdmRef
        let json = serde_json::to_string(&sale).unwrap();
        assert!(json.contains("fdmRef"));
        assert!(json.contains("FDM001"));
    }

    #[test]
    fn normal_sale_has_no_fdm_ref() {
        let config = test_config();
        let order = test_order();
        let lines = test_lines();
        let bp_id = Uuid::new_v4().to_string();

        let sale = build_sale_input(
            &config,
            &order,
            &lines,
            1,
            &bp_id,
            PaymentType::Cash,
            2500,
            None,
        );
        assert!(sale.fdm_ref.is_none());

        // Verify JSON serialization does NOT include fdmRef
        let json = serde_json::to_string(&sale).unwrap();
        assert!(!json.contains("fdmRef"));
    }

    #[test]
    fn build_pre_bill_has_transaction() {
        let config = test_config();
        let order = test_order();
        let lines = test_lines();
        let bp_id = Uuid::new_v4().to_string();

        let pre_bill = build_pre_bill_input(&config, &order, &lines, 1, &bp_id);
        assert!((pre_bill.transaction.transaction_total - 25.0).abs() < 0.01);
        assert!(pre_bill.cost_center.is_some());
    }
}
