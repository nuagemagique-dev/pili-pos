use chrono::Utc;

use crate::inputs::*;
use crate::payment::{Settlement, payment_line_identity};
use crate::response::FdmReference;
use crate::types::*;
use pos_core::config::PosConfig;
use pos_core::order::{Order, OrderLine};
use pos_core::vat::VatCode;

/// Software version string sent to FDM.
pub const POS_SW_VERSION: &str = concat!("pili-pos/", env!("CARGO_PKG_VERSION"));

// ── Common Fields ──────────────────────────────────────────────

/// The booking period an event belongs to (POS spec v2.2 §2.2.2): its
/// id, and the booking date the period opened on (`YYYY-MM-DD`). Every
/// event of a period carries the same date, even after midnight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Booking {
    pub period_id: String,
    pub date: String,
}

/// Belgian local time, whatever the system time zone.
pub fn now_in_belgium() -> chrono::DateTime<chrono_tz::Tz> {
    Utc::now().with_timezone(&chrono_tz::Europe::Brussels)
}

/// A date-time as the FDM takes it (`posDateTime`, POS description
/// v2.2): RFC 3339 in Brussels time, with the offset of that instant
/// (+01:00 in winter, +02:00 in summer) and milliseconds, like the
/// spec's example `2022-11-03T15:01:25.123+01:00` and the official
/// fixtures. A finer fraction (chrono's default: micro- or
/// nanoseconds) is refused by a certified FDM.
pub fn belgian_timestamp(at: chrono::DateTime<Utc>) -> String {
    at.with_timezone(&chrono_tz::Europe::Brussels)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, false)
}

/// Now, as the FDM takes it (see [`belgian_timestamp`]).
pub fn fdm_now() -> String {
    belgian_timestamp(Utc::now())
}

/// Build common fields shared by all FDM mutations. `ticket_medium`
/// states what the POS does with the event: `None` when nothing is
/// printed (drawer, money in/out, staff presence).
pub fn build_common_fields(
    config: &PosConfig,
    ticket_number: i64,
    employee_insz: &str,
    booking: &Booking,
    ticket_medium: TicketMedium,
) -> CommonFields {
    CommonFields {
        language: fiscal_language(&config.language),
        vat_no: config.restaurant_vat.clone(),
        est_no: config.restaurant_est_no.clone(),
        pos_id: config.pos_id.clone(),
        pos_fiscal_ticket_no: ticket_number,
        pos_date_time: fdm_now(),
        pos_sw_version: POS_SW_VERSION.to_string(),
        terminal_id: Some(config.terminal_id.clone()),
        device_id: config.device_id.clone(),
        booking_period_id: booking.period_id.clone(),
        booking_date: booking.date.clone(),
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

/// Cents to euros, exactly: the nearest `f64` to a two-decimal amount,
/// which serialises with at most two decimals.
fn eur(cents: i64) -> f64 {
    cents as f64 / 100.0
}

/// Build transaction lines (shared by Sale, Order, PreBill). Negative
/// lines carry `neg_reason`: REFUND on refunds, CORRECTION when an
/// entry is corrected before the sale is closed.
fn build_transaction(lines: &[OrderLine], neg_reason: NegQuantityReason) -> TransactionInput {
    let transaction_lines: Vec<TransactionLineInput> = lines
        .iter()
        .map(|line| {
            // Amounts are computed in cents and converted once: a
            // product of floats (1.10 x 3 = 3.3000000000000003) is not
            // the multiple of 0.01 the FDM requires (POS spec v2.2).
            let unit_price = eur(line.unit_price_cents);
            let line_total = eur(line.unit_price_cents * i64::from(line.quantity));
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
                    neg_quantity_reason: (line.quantity < 0).then_some(neg_reason),
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

    let transaction_total = eur(lines
        .iter()
        .map(|l| l.unit_price_cents * i64::from(l.quantity))
        .sum());

    TransactionInput {
        transaction_lines,
        transaction_total,
    }
}

/// The `reference` that ties together every event of one order (its P
/// orders, pre-bills and the final N): stable for the order's lifetime.
pub fn order_reference(order: &Order) -> String {
    format!("O{}", order.id)
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

// ── Event N - Sale ─────────────────────────────────────────────

/// Build a SaleInput from an order + its entries + payment info.
/// `lines` is the order's journal (additions and corrections in
/// chronological order, POS description v2.2 §2). For refunds, pass
/// `original_fdm_ref` referencing the original ticket.
///
/// `payment_amount_cents` is the amount of the sale itself; a cash
/// rounding (`rounding_cents`, 0 for none) goes on its own `ROUNDING`
/// line of the same payment method, never into the payment amount
/// (producers FAQ Q403, Advanced Set UC201).
// The argument list mirrors the spec-mandated SaleInput composition;
// grouping them into an ad-hoc struct would just move the same fields.
#[allow(clippy::too_many_arguments)]
pub fn build_sale_input(
    config: &PosConfig,
    order: &Order,
    lines: &[OrderLine],
    ticket_number: i64,
    booking: &Booking,
    payment_type: PaymentType,
    payment_amount_cents: i64,
    rounding_cents: i64,
    original_fdm_ref: Option<&FdmReference>,
) -> SaleInput {
    let common = build_common_fields(
        config,
        ticket_number,
        &order.user_insz,
        booking,
        TicketMedium::Paper,
    );
    let neg_reason = if original_fdm_ref.is_some() {
        NegQuantityReason::Refund
    } else {
        NegQuantityReason::Correction
    };
    let transaction = build_transaction(lines, neg_reason);

    let cost_center = order
        .table_number
        .map(|t| build_table_cost_center(t, &order_reference(order)));

    let (id, name) = payment_line_identity(payment_type);
    let financials = financial_lines(
        &[(id, name, payment_type, payment_amount_cents)],
        rounding_cents,
    );

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

/// [`build_sale_input`] for a sale paid in several parts (split
/// payment, meal vouchers): one PAYMENT line per settled part, in the
/// settlement's order (cash last), and the cash rounding on its own
/// ROUNDING line right after the cash payment.
pub fn build_sale_input_split(
    config: &PosConfig,
    order: &Order,
    lines: &[OrderLine],
    ticket_number: i64,
    booking: &Booking,
    settlement: &Settlement,
) -> SaleInput {
    let mut sale = build_sale_input(
        config,
        order,
        lines,
        ticket_number,
        booking,
        PaymentType::Cash,
        0,
        0,
        None,
    );
    let parts: Vec<_> = settlement
        .payments
        .iter()
        .map(|p| {
            let payment_type = p.method.payment_type();
            let (id, name) = payment_line_identity(payment_type);
            (id, name, payment_type, p.amount_cents)
        })
        .collect();
    sale.financials = financial_lines(&parts, settlement.rounding_cents);
    sale
}

/// The `financials` of a sale: a PAYMENT line per `(id, name, type,
/// amount)`, and `rounding_cents` (if any) on a ROUNDING line of the
/// cash payment, right after it (FAQ Q403). Only cash is rounded.
fn financial_lines(
    payments: &[(&str, &str, PaymentType, i64)],
    rounding_cents: i64,
) -> Vec<PaymentLineInput> {
    let line =
        |id: &str, name: &str, payment_type, amount_cents: i64, amount_type| PaymentLineInput {
            id: id.to_string(),
            name: name.to_string(),
            payment_type,
            provider: None,
            input_method: InputMethod::Manual,
            amount: eur(amount_cents),
            amount_type,
            foreign_currency: None,
            reference: None,
            drawer: None,
        };
    let mut out = Vec::new();
    for &(id, name, payment_type, amount_cents) in payments {
        out.push(line(
            id,
            name,
            payment_type,
            amount_cents,
            PaymentLineType::Payment,
        ));
        if rounding_cents != 0 && payment_type == PaymentType::Cash {
            out.push(line(
                id,
                name,
                payment_type,
                rounding_cents,
                PaymentLineType::Rounding,
            ));
        }
    }
    out
}

// ── Event P - Order (pro forma) ────────────────────────────────

/// Build an OrderInput (event P) for an open table / interrupted sale.
/// `lines` are the order's entries not yet transmitted, corrections
/// included. Orders are not printed in v1.0.0 (no kitchen ticket).
pub fn build_order_input(
    config: &PosConfig,
    order: &Order,
    lines: &[OrderLine],
    ticket_number: i64,
    booking: &Booking,
) -> OrderInput {
    let common = build_common_fields(
        config,
        ticket_number,
        &order.user_insz,
        booking,
        TicketMedium::None,
    );
    let transaction = build_transaction(lines, NegQuantityReason::Correction);
    let table = order.table_number.unwrap_or(0);
    let cost_center = build_table_cost_center(table, &order_reference(order));

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
    booking: &Booking,
) -> PreBillInput {
    let common = build_common_fields(
        config,
        ticket_number,
        &order.user_insz,
        booking,
        TicketMedium::Paper,
    );
    let transaction = build_transaction(lines, NegQuantityReason::Correction);
    let cost_center = order
        .table_number
        .map(|t| build_table_cost_center(t, &order_reference(order)));

    PreBillInput {
        common,
        cost_center,
        transaction,
    }
}

// ── Event C - Copy ─────────────────────────────────────────────

/// Build a CopyInput for reprinting a previous event.
/// `fdm_ref` references the original event (fdmId + eventLabel + counters).
pub fn build_copy_input(
    config: &PosConfig,
    ticket_number: i64,
    employee_insz: &str,
    booking: &Booking,
    original_ref: &FdmReference,
) -> CopyInput {
    let common = build_common_fields(
        config,
        ticket_number,
        employee_insz,
        booking,
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

// ── Event I - Invoice ──────────────────────────────────────────

/// Build an InvoiceInput (event I) for a B2B invoice covering one or
/// more prior sales. `fdm_refs` reference the underlying N events
/// (the M150 fixture shows an invoice consolidating two sales).
pub fn build_invoice_input(
    config: &PosConfig,
    ticket_number: i64,
    employee_insz: &str,
    booking: &Booking,
    invoice_no: &str,
    customer_vat_no: &str,
    fdm_refs: &[FdmReference],
) -> InvoiceInput {
    let common = build_common_fields(
        config,
        ticket_number,
        employee_insz,
        booking,
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

// ── Event S - Social (Work In/Out) ─────────────────────────────

/// Build a WorkInOutInput for employee clock in/out.
pub fn build_work_in_out(
    config: &PosConfig,
    ticket_number: i64,
    employee_insz: &str,
    booking: &Booking,
) -> WorkInOutInput {
    WorkInOutInput {
        common: build_common_fields(
            config,
            ticket_number,
            employee_insz,
            booking,
            TicketMedium::None,
        ),
    }
}

// ── Event R - Reports ──────────────────────────────────────────

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
    booking: &Booking,
    data: TurnoverReportData,
) -> ReportTurnoverXInput {
    let common = build_common_fields(
        config,
        ticket_number,
        employee_insz,
        booking,
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
    booking: &Booking,
    report_no: i64,
    report_booking_date: &str,
    data: TurnoverReportData,
) -> ReportTurnoverZInput {
    let common = build_common_fields(
        config,
        ticket_number,
        employee_insz,
        booking,
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
    booking: &Booking,
    data: UserReportData,
) -> ReportUserXInput {
    let common = build_common_fields(
        config,
        ticket_number,
        employee_insz,
        booking,
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
    booking: &Booking,
    report_no: i64,
    report_booking_date: &str,
    data: UserReportData,
) -> ReportUserZInput {
    let common = build_common_fields(
        config,
        ticket_number,
        employee_insz,
        booking,
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

    /// A booking period for tests.
    fn bk(period_id: &str) -> Booking {
        Booking {
            period_id: period_id.to_string(),
            date: "2026-07-03".to_string(),
        }
    }

    #[test]
    fn events_carry_the_booking_date_of_their_period() {
        // A period opened on the 3rd keeps that date after midnight.
        let booking = Booking {
            period_id: "1b4e28ba-2fa1-11d2-883f-0016d3cca427".into(),
            date: "2026-07-03".into(),
        };
        let common = build_common_fields(
            &test_config(),
            1,
            "00000000029",
            &booking,
            TicketMedium::None,
        );
        assert_eq!(common.booking_date, "2026-07-03");
        assert_eq!(common.booking_period_id, booking.period_id);
    }

    #[test]
    fn pos_date_time_is_belgian_time_whatever_the_system_zone() {
        let common = build_common_fields(
            &test_config(),
            1,
            "00000000029",
            &bk("bp"),
            TicketMedium::None,
        );
        let at = chrono::DateTime::parse_from_rfc3339(&common.pos_date_time).unwrap();
        let offset = at.offset().local_minus_utc();
        assert!(
            offset == 3600 || offset == 7200,
            "Europe/Brussels is UTC+1 or UTC+2, got {}",
            common.pos_date_time
        );
        // Same instant and same offset as Brussels at that instant.
        let belgian = at.with_timezone(&chrono_tz::Europe::Brussels);
        assert_eq!(
            belgian.to_rfc3339_opts(chrono::SecondsFormat::Millis, false),
            common.pos_date_time
        );
    }

    #[test]
    fn belgian_timestamps_carry_milliseconds_and_the_offset_of_the_season() {
        use chrono::TimeZone;
        let at = |y, m, d, h, min, s, nanos| {
            Utc.with_ymd_and_hms(y, m, d, h, min, s).unwrap() + chrono::Duration::nanoseconds(nanos)
        };
        // Summer (CEST, +02:00), with a nanosecond clock.
        assert_eq!(
            belgian_timestamp(at(2026, 9, 27, 16, 31, 5, 123_456_789)),
            "2026-09-27T18:31:05.123+02:00"
        );
        // Winter (CET, +01:00), a whole second still shows .000.
        assert_eq!(
            belgian_timestamp(at(2026, 11, 3, 14, 1, 25, 0)),
            "2026-11-03T15:01:25.000+01:00"
        );
        // Both sides of the switch to winter time (2026-10-25 01:00 UTC).
        assert_eq!(
            belgian_timestamp(at(2026, 10, 25, 0, 59, 59, 0)),
            "2026-10-25T02:59:59.000+02:00"
        );
        assert_eq!(
            belgian_timestamp(at(2026, 10, 25, 1, 0, 0, 0)),
            "2026-10-25T02:00:00.000+01:00"
        );
    }
    use super::*;
    use pos_core::order::{OrderStatus, OrderType};
    use uuid::Uuid;

    fn test_config() -> PosConfig {
        PosConfig {
            pos_id: "CFOD0061234567".to_string(),
            terminal_id: "POS1".to_string(),
            device_id: "DEV001".to_string(),
            restaurant_name: "Restaurant Exemple".to_string(),
            restaurant_address: "Rue Test 1, 1000 Bruxelles".to_string(),
            restaurant_vat: "BE0499999960".to_string(),
            restaurant_est_no: "2123456791".to_string(),
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
            &bk(&bp_id),
            PaymentType::Cash,
            2500,
            0,
            None,
        );

        assert_eq!(sale.common.pos_id, "CFOD0061234567");
        assert_eq!(sale.common.employee_id, "00000000029");
        assert_eq!(sale.common.est_no, "2123456791");
        assert_eq!(sale.transaction.transaction_lines.len(), 1);

        let line = &sale.transaction.transaction_lines[0];
        assert_eq!(line.main_product.product_name, "Saumon roll x8");
        assert!((line.line_total - 25.0).abs() < 0.01);
        assert!((sale.transaction.transaction_total - 25.0).abs() < 0.01);
    }

    #[test]
    fn a_split_sale_has_a_payment_line_per_part_and_the_rounding_after_cash() {
        use crate::payment::{PaymentMethod, PaymentPart, settle};
        let (config, order, lines) = (test_config(), test_order(), test_lines());
        let bp_id = Uuid::new_v4().to_string();
        // 25,00: 3,13 in meal vouchers, 3,97 by card, the rest (17,90)
        // cash. The sale total here is the settlement's, 25,02, so the
        // cash rest 17,92 is rounded down by 0,02.
        let settlement = settle(
            2502,
            &[
                PaymentPart::of(PaymentMethod::MealVoucher, 313),
                PaymentPart::of(PaymentMethod::Card, 397),
                PaymentPart::cash(),
            ],
        )
        .unwrap();
        let sale = build_sale_input_split(&config, &order, &lines, 1, &bk(&bp_id), &settlement);
        let financials = serde_json::to_value(&sale.financials).unwrap();
        let summary: Vec<(String, String, String, f64)> = financials
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    f["id"].as_str().unwrap().to_string(),
                    f["type"].as_str().unwrap().to_string(),
                    f["amountType"].as_str().unwrap().to_string(),
                    f["amount"].as_f64().unwrap(),
                )
            })
            .collect();
        let row =
            |id: &str, t: &str, a: &str, x: f64| (id.to_string(), t.to_string(), a.to_string(), x);
        assert_eq!(
            summary,
            [
                row("3", "CHEQUE_MEAL", "PAYMENT", 3.13),
                row("2", "CARD_DEBIT", "PAYMENT", 3.97),
                row("1", "CASH", "PAYMENT", 17.92),
                row("1", "CASH", "ROUNDING", -0.02),
            ]
        );
    }

    #[test]
    fn an_unpaid_settlement_sends_empty_financials() {
        let (config, order, lines) = (test_config(), test_order(), test_lines());
        let bp_id = Uuid::new_v4().to_string();
        let unpaid = crate::payment::Settlement {
            payments: vec![],
            rounding_cents: 0,
        };
        let sale = build_sale_input_split(&config, &order, &lines, 1, &bk(&bp_id), &unpaid);
        let json = serde_json::to_value(&sale).unwrap();
        assert_eq!(json["financials"], serde_json::json!([]));
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
            &bk(&bp_id),
            PaymentType::Cash,
            2500,
            0,
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

        let order_input = build_order_input(&config, &order, &lines, 1, &bk(&bp_id));

        assert_eq!(order_input.cost_center.id, "T05");
        assert_eq!(
            order_input.cost_center.cost_center_type,
            CostCenterType::Table
        );
        assert_eq!(order_input.cost_center.reference, order_reference(&order));
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

        let copy = build_copy_input(&config, 2, "00000000029", &bk(&bp_id), &original);

        assert_eq!(copy.fdm_ref.fdm_id, "FDM001");
        assert_eq!(copy.fdm_ref.event_counter, 42);
        assert_eq!(copy.fdm_ref.total_counter, 100);
    }

    #[test]
    // covers: FAQ_Q301
    fn negative_quantity_reason_depends_on_the_sale() {
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
        let original = FdmReference {
            fdm_id: "CBX01000042".into(),
            fdm_date_time: "2026-07-03T12:34:57+02:00".into(),
            event_label: EventLabel::N,
            event_counter: 15,
            total_counter: 59,
        };
        let reason = |fdm_ref: Option<&FdmReference>| {
            build_sale_input(
                &config,
                &order,
                &lines,
                1,
                &bk(&bp_id),
                PaymentType::Cash,
                -2500,
                0,
                fdm_ref,
            )
            .transaction
            .transaction_lines[0]
                .main_product
                .neg_quantity_reason
        };

        // Refund of a signed ticket vs correction before the sale closes.
        assert_eq!(reason(Some(&original)), Some(NegQuantityReason::Refund));
        assert_eq!(reason(None), Some(NegQuantityReason::Correction));
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
            &bk(&bp_id),
            PaymentType::Cash,
            2500,
            0,
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
            &bk(&bp_id),
            PaymentType::Cash,
            -1250,
            0,
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
            &bk(&bp_id),
            PaymentType::Cash,
            2500,
            0,
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

        let pre_bill = build_pre_bill_input(&config, &order, &lines, 1, &bk(&bp_id));
        assert!((pre_bill.transaction.transaction_total - 25.0).abs() < 0.01);
        assert!(pre_bill.cost_center.is_some());
    }

    // ── ticketMedium per event type (POS description v2.2 §1) ──────
    //
    // Sale/pre-bill are printed (Paper); Order and Work In/Out are
    // never printed on their own (None).

    #[test]
    fn sale_has_paper_ticket_medium() {
        let config = test_config();
        let order = test_order();
        let lines = test_lines();
        let sale = build_sale_input(
            &config,
            &order,
            &lines,
            1,
            &bk("bp"),
            PaymentType::Cash,
            2500,
            0,
            None,
        );
        assert_eq!(sale.common.ticket_medium, TicketMedium::Paper);
    }

    #[test]
    fn order_has_no_ticket_medium() {
        let config = test_config();
        let order = test_order();
        let lines = test_lines();
        let order_input = build_order_input(&config, &order, &lines, 1, &bk("bp"));
        assert_eq!(order_input.common.ticket_medium, TicketMedium::None);
    }

    #[test]
    fn pre_bill_has_paper_ticket_medium() {
        let config = test_config();
        let order = test_order();
        let lines = test_lines();
        let pre_bill = build_pre_bill_input(&config, &order, &lines, 1, &bk("bp"));
        assert_eq!(pre_bill.common.ticket_medium, TicketMedium::Paper);
    }

    #[test]
    fn work_in_out_has_no_ticket_medium() {
        let config = test_config();
        let w = build_work_in_out(&config, 1, "00000000029", &bk("bp"));
        assert_eq!(w.common.ticket_medium, TicketMedium::None);
        assert_eq!(w.common.employee_id, "00000000029");
    }

    // ── build_work_in_out (S event) ─────────────────────────────

    #[test]
    fn work_in_out_carries_identity_and_serializes() {
        let config = test_config();
        let bp_id = Uuid::new_v4().to_string();
        let w = build_work_in_out(&config, 7, "94060242576", &bk(&bp_id));
        assert_eq!(w.common.pos_id, "CFOD0061234567");
        assert_eq!(w.common.pos_fiscal_ticket_no, 7);
        assert_eq!(w.common.employee_id, "94060242576");
        assert_eq!(w.common.booking_period_id, bp_id);

        let json = serde_json::to_string(&w).unwrap();
        assert!(json.contains("\"posFiscalTicketNo\":7"));
        assert!(json.contains("\"employeeId\":\"94060242576\""));
        // WorkInOutInput has no fields beyond CommonFields.
        assert!(!json.contains("transaction"));
    }

    // ── build_invoice_input (I event) ───────────────────────────

    #[test]
    fn invoice_input_field_by_field() {
        let config = test_config();
        let bp_id = Uuid::new_v4().to_string();
        let refs = vec![
            FdmReference {
                fdm_id: "CBX01000042".to_string(),
                fdm_date_time: "2026-04-12T19:00:00+02:00".to_string(),
                event_label: EventLabel::N,
                event_counter: 1,
                total_counter: 10,
            },
            FdmReference {
                fdm_id: "CBX01000042".to_string(),
                fdm_date_time: "2026-04-12T19:05:00+02:00".to_string(),
                event_label: EventLabel::N,
                event_counter: 2,
                total_counter: 11,
            },
        ];

        let invoice = build_invoice_input(
            &config,
            3,
            "00000000029",
            &bk(&bp_id),
            "INV-2026-001",
            "BE0123456789",
            &refs,
        );

        assert_eq!(invoice.invoice_no, "INV-2026-001");
        assert_eq!(invoice.customer_vat_no, "BE0123456789");
        assert!(invoice.cost_center.is_none());
        assert_eq!(invoice.fdm_refs.len(), 2);
        assert_eq!(invoice.fdm_refs[0].event_counter, 1);
        assert_eq!(invoice.fdm_refs[1].event_counter, 2);
        assert_eq!(invoice.common.ticket_medium, TicketMedium::Paper);

        let json = serde_json::to_string(&invoice).unwrap();
        assert!(json.contains("\"invoiceNo\":\"INV-2026-001\""));
        assert!(json.contains("\"customerVatNo\":\"BE0123456789\""));
        assert!(json.contains("\"fdmRefs\""));
    }

    #[test]
    fn invoice_input_with_no_underlying_sales_has_empty_refs() {
        let config = test_config();
        let invoice =
            build_invoice_input(&config, 1, "00000000029", &bk("bp"), "INV-1", "BE1", &[]);
        assert!(invoice.fdm_refs.is_empty());
    }

    // ── Report builders (R event: X/Z turnover + user) ──────────

    fn empty_turnover_data() -> TurnoverReportData {
        TurnoverReportData {
            pos_devices: vec![],
            fdm_devices: vec![],
            turnover: TurnoverInput {
                transactions: vec![],
                departments: vec![],
                vats: vec![],
                payments: vec![],
                drawers_open_count: 0,
                neg_quantities: vec![],
                price_changes: vec![],
                invoices: None,
            },
        }
    }

    #[test]
    fn report_turnover_x_is_not_resetting_and_has_no_report_no() {
        let config = test_config();
        let x = build_report_turnover_x(
            &config,
            100,
            "00000000029",
            &bk("bp"),
            empty_turnover_data(),
        );
        assert_eq!(x.common.pos_fiscal_ticket_no, 100);
        assert_eq!(x.common.ticket_medium, TicketMedium::Paper);
        // ReportTurnoverXInput has no report_no field at all - X never
        // resets counters (unlike Z).
        let json = serde_json::to_string(&x).unwrap();
        assert!(!json.contains("reportNo"));
    }

    #[test]
    fn report_turnover_z_carries_report_no_and_booking_date() {
        let config = test_config();
        let z = build_report_turnover_z(
            &config,
            101,
            "00000000029",
            &bk("bp"),
            42,
            "2026-04-12",
            empty_turnover_data(),
        );
        assert_eq!(z.report_no, 42);
        assert_eq!(z.report_booking_date, "2026-04-12");
        let json = serde_json::to_string(&z).unwrap();
        assert!(json.contains("\"reportNo\":42"));
        assert!(json.contains("\"reportBookingDate\":\"2026-04-12\""));
    }

    fn empty_user_data() -> UserReportData {
        UserReportData {
            pos_devices: vec![],
            fdm_devices: vec![],
            users: vec![],
        }
    }

    #[test]
    fn report_user_x_has_no_report_no() {
        let config = test_config();
        let x = build_report_user_x(&config, 102, "00000000029", &bk("bp"), empty_user_data());
        let json = serde_json::to_string(&x).unwrap();
        assert!(!json.contains("reportNo"));
        assert_eq!(x.common.ticket_medium, TicketMedium::Paper);
    }

    #[test]
    fn report_user_z_carries_report_no() {
        let config = test_config();
        let z = build_report_user_z(
            &config,
            103,
            "00000000029",
            &bk("bp"),
            42,
            "2026-04-12",
            empty_user_data(),
        );
        assert_eq!(z.report_no, 42);
        assert_eq!(z.report_booking_date, "2026-04-12");
    }

    #[test]
    fn turnover_report_carries_department_and_vat_totals_through() {
        let config = test_config();
        let mut data = empty_turnover_data();
        data.turnover.departments.push(DepartmentTotalInput {
            department_id: "12".to_string(),
            department_name: "Sushi".to_string(),
            amount: 125.50,
        });
        data.turnover.vats.push(VatTotalInput {
            label: VatLabel::B,
            rate: 12.0,
            taxable_amount: 112.05,
            vat_amount: 13.45,
            total_amount: 125.50,
        });
        data.turnover.drawers_open_count = 3;

        let x = build_report_turnover_x(&config, 1, "00000000029", &bk("bp"), data);
        assert_eq!(x.turnover.departments.len(), 1);
        assert_eq!(x.turnover.departments[0].department_id, "12");
        assert_eq!(x.turnover.vats[0].label, VatLabel::B);
        assert_eq!(x.turnover.drawers_open_count, 3);
    }

    // ── build_transaction edge cases ────────────────────────────

    #[test]
    fn empty_order_has_zero_total_and_no_lines() {
        let config = test_config();
        let order = test_order();
        let bp_id = Uuid::new_v4().to_string();

        let sale = build_sale_input(
            &config,
            &order,
            &[], // no lines at all
            1,
            &bk(&bp_id),
            PaymentType::Cash,
            0,
            0,
            None,
        );
        assert!(sale.transaction.transaction_lines.is_empty());
        assert_eq!(sale.transaction.transaction_total, 0.0);
    }

    #[test]
    fn department_id_and_name_are_carried_through_to_the_product() {
        let config = test_config();
        let order = test_order();
        let lines = vec![OrderLine {
            id: 1,
            order_id: 1,
            menu_item_id: 42,
            quantity: 1,
            unit_price_cents: 1000,
            vat_code: VatCode::C,
            name: "Takeaway roll".to_string(),
            modifiers_json: None,
            created_at: chrono::Utc::now(),
            department_id: "12".to_string(),
            department_name: "Sushi".to_string(),
        }];
        let sale = build_sale_input(
            &config,
            &order,
            &lines,
            1,
            &bk("bp"),
            PaymentType::Cash,
            1000,
            0,
            None,
        );
        let product = &sale.transaction.transaction_lines[0].main_product;
        assert_eq!(product.department_id, "12");
        assert_eq!(product.department_name, "Sushi");
        assert_eq!(product.vats[0].label, VatLabel::C);
    }

    #[test]
    fn gtin_is_absent_from_serialized_product() {
        // GTIN is Option<String> and never populated by build_transaction
        // (no barcode scanning in the MVP) - the JSON key must be
        // entirely absent, not `null`, per skip_serializing_if.
        let config = test_config();
        let order = test_order();
        let lines = test_lines();
        let sale = build_sale_input(
            &config,
            &order,
            &lines,
            1,
            &bk("bp"),
            PaymentType::Cash,
            2500,
            0,
            None,
        );
        let json = serde_json::to_string(&sale).unwrap();
        assert!(!json.contains("gtin"));
    }

    #[test]
    fn many_lines_transaction_total_sums_exactly() {
        let config = test_config();
        let order = test_order();
        let lines: Vec<OrderLine> = (0..12)
            .map(|i| OrderLine {
                id: i,
                order_id: 1,
                menu_item_id: i,
                quantity: (i % 3) as i32 + 1,
                unit_price_cents: 100 * (i + 1),
                vat_code: VatCode::B,
                name: format!("Item {i}"),
                modifiers_json: None,
                created_at: chrono::Utc::now(),
                department_id: "1".to_string(),
                department_name: "Food".to_string(),
            })
            .collect();
        let expected_cents: i64 = lines
            .iter()
            .map(|l| l.unit_price_cents * l.quantity as i64)
            .sum();

        let sale = build_sale_input(
            &config,
            &order,
            &lines,
            1,
            &bk("bp"),
            PaymentType::Cash,
            expected_cents,
            0,
            None,
        );
        assert_eq!(sale.transaction.transaction_total, eur(expected_cents));
        assert_eq!(sale.transaction.transaction_lines.len(), 12);
    }

    /// Every amount a message carries, as serialised: never more than
    /// two decimals (POS spec v2.2: a multiple of 0.01), whatever the
    /// prices and quantities.
    #[test]
    fn amounts_are_serialised_as_exact_cents() {
        fn check(v: &serde_json::Value, path: &str, bad: &mut Vec<String>) {
            match v {
                serde_json::Value::Object(m) => {
                    for (k, v) in m {
                        check(v, &format!("{path}.{k}"), bad);
                    }
                }
                serde_json::Value::Array(a) => {
                    for (i, v) in a.iter().enumerate() {
                        check(v, &format!("{path}[{i}]"), bad);
                    }
                }
                serde_json::Value::Number(n) => {
                    let s = n.to_string();
                    if s.split_once('.').is_some_and(|(_, d)| d.len() > 2) || s.contains('e') {
                        bad.push(format!("{path} = {s}"));
                    }
                }
                _ => {}
            }
        }
        let config = test_config();
        let order = test_order();
        let mut bad = Vec::new();
        // Prices with every cent ending, quantities up to 25, refunds.
        for price in (1..=2_000)
            .step_by(7)
            .chain([110, 115, 330, 995, 1_999, 123_456])
        {
            let lines: Vec<OrderLine> = (1..=25)
                .step_by(4)
                .flat_map(|q| [q, -q])
                .enumerate()
                .map(|(i, quantity)| OrderLine {
                    id: i as i64,
                    order_id: 1,
                    menu_item_id: i as i64,
                    quantity,
                    unit_price_cents: price + i as i64,
                    vat_code: VatCode::B,
                    name: "Item".to_string(),
                    modifiers_json: None,
                    created_at: chrono::Utc::now(),
                    department_id: "1".to_string(),
                    department_name: "Food".to_string(),
                })
                .collect();
            let total: i64 = lines
                .iter()
                .map(|l| l.unit_price_cents * i64::from(l.quantity))
                .sum();
            let sale = build_sale_input(
                &config,
                &order,
                &lines,
                1,
                &bk("bp"),
                PaymentType::Cash,
                total,
                0,
                None,
            );
            assert_eq!(sale.transaction.transaction_total, eur(total));
            for l in &sale.transaction.transaction_lines {
                assert_eq!(
                    l.line_total, l.main_product.vats[0].price,
                    "the line's VAT price is its total"
                );
            }
            check(&serde_json::to_value(&sale).unwrap(), "sale", &mut bad);
            let pre_bill = build_pre_bill_input(&config, &order, &lines, 1, &bk("bp"));
            check(
                &serde_json::to_value(&pre_bill).unwrap(),
                "preBill",
                &mut bad,
            );
        }
        assert!(
            bad.is_empty(),
            "{} amounts with float noise, e.g. {:?}",
            bad.len(),
            &bad[..bad.len().min(5)]
        );
    }

    #[test]
    fn a_price_times_a_quantity_is_exact() {
        let config = test_config();
        let lines = vec![OrderLine {
            id: 1,
            order_id: 1,
            menu_item_id: 1,
            quantity: 3,
            unit_price_cents: 110,
            vat_code: VatCode::A,
            name: "Eau".to_string(),
            modifiers_json: None,
            created_at: chrono::Utc::now(),
            department_id: "1".to_string(),
            department_name: "Boissons".to_string(),
        }];
        let sale = build_sale_input(
            &config,
            &test_order(),
            &lines,
            1,
            &bk("bp"),
            PaymentType::Cash,
            330,
            0,
            None,
        );
        let json = serde_json::to_string(&sale).unwrap();
        assert!(json.contains("\"lineTotal\":3.3"), "{json}");
        assert!(!json.contains("3.3000000000000003"), "{json}");
    }

    #[test]
    fn vat_code_to_label_maps_every_code() {
        for (code, label) in [
            (VatCode::A, VatLabel::A),
            (VatCode::B, VatLabel::B),
            (VatCode::C, VatLabel::C),
            (VatCode::D, VatLabel::D),
            (VatCode::X, VatLabel::X),
        ] {
            assert_eq!(vat_code_to_label(code), label);
        }
    }

    #[test]
    fn order_reference_is_stable_for_the_orders_lifetime() {
        let order = test_order();
        assert_eq!(order_reference(&order), format!("O{}", order.id));
        // Calling twice must yield the identical reference - it ties
        // together every P/pre-bill/N event of one order's lifetime.
        assert_eq!(order_reference(&order), order_reference(&order));
    }
}
