//! Reading the signed requests and FDM answers of the archive.
//!
//! Everything here is a pure function of the archived JSON: the request the POS
//! signed (lines, payments, cost center) and the answer of the FDM (`vatCalc`).
//! Nothing is recomputed from the menu. The refund rule is the one of the
//! statistics export (`pos-export`): a sale that refers to an earlier ticket
//! (`fdmRef` in its request) or has a negative total.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::error::{AccountingError, Result};

/// The VAT labels of the SCE 2.0 specification, in report order.
pub const LABELS: [&str; 5] = ["A", "B", "C", "D", "X"];

/// Department id of the gift voucher system product (decision V12-D25).
pub const GIFT_VOUCHER_DEPARTMENT: &str = "GV";

/// Euro amount of the archived JSON (a multiple of 0.01) in cents.
pub fn cents(v: &Value) -> i64 {
    (v.as_f64().unwrap_or(0.0) * 100.0).round() as i64
}

/// VAT rate of the archived JSON (percent) in basis points.
pub fn rate_bp(v: &Value) -> i64 {
    (v.as_f64().unwrap_or(0.0) * 100.0).round() as i64
}

fn string(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

/// An archived event with an FDM answer.
#[derive(Debug, Clone)]
pub struct RawEvent {
    pub id: i64,
    /// `fiscal_events.event_type`: N, I, F or R here.
    pub label: String,
    pub request: Value,
    pub response: Value,
    pub fdm_datetime: Option<String>,
}

/// How the customer ordered, from the signed cost center.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Channel {
    /// Cost center type `TABLE`.
    DineIn,
    /// No cost center: a direct sale.
    Takeaway,
    /// Any other cost center type (online orders).
    Online,
}

impl Channel {
    /// Value written in the CSV files.
    pub fn code(self) -> &'static str {
        match self {
            Self::DineIn => "sur_place",
            Self::Takeaway => "emporter",
            Self::Online => "en_ligne",
        }
    }
}

/// Channel of a request from its own cost center (absent: takeaway).
pub fn channel_of(request: &Value) -> Channel {
    match request.get("costCenter").filter(|c| c.is_object()) {
        None => Channel::Takeaway,
        Some(c) if c["type"].as_str() == Some("TABLE") => Channel::DineIn,
        Some(_) => Channel::Online,
    }
}

/// One VAT row of the FDM's `vatCalc`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatRow {
    pub label: String,
    pub rate_bp: i64,
    pub base: i64,
    pub vat: i64,
    pub total: i64,
}

/// One signed payment line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayLine {
    /// `PaymentType` of the specification (`CASH`, `CARD_DEBIT`, `VOUCHER_STORE`...).
    pub payment_type: String,
    /// `PAYMENT`, `ROUNDING` or `TIP`.
    pub amount_type: String,
    pub cents: i64,
}

/// One signed product line: amounts per VAT label, after price changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaleLine {
    pub department_id: String,
    pub product_id: String,
    pub vats: Vec<(String, i64)>,
}

impl SaleLine {
    pub fn is_gift_voucher(&self) -> bool {
        self.department_id == GIFT_VOUCHER_DEPARTMENT || self.product_id == GIFT_VOUCHER_DEPARTMENT
    }

    pub fn amount(&self) -> i64 {
        self.vats.iter().map(|(_, a)| a).sum()
    }
}

/// A signed sale (label N), refunds included.
#[derive(Debug, Clone)]
pub struct Sale {
    pub id: i64,
    pub ticket_no: i64,
    /// Local time the POS gave, as signed (`posDateTime`).
    pub pos_datetime: String,
    pub fdm_datetime: String,
    pub refund: bool,
    pub channel: Channel,
    pub lines: Vec<SaleLine>,
    pub vats: Vec<VatRow>,
    pub payments: Vec<PayLine>,
    /// The ticket a refund refers to (the request's `fdmRef`).
    pub refers_to: Option<Value>,
    /// The FDM reference of this ticket (the answer's `fdmRef`).
    pub signed_ref: Value,
}

impl Sale {
    /// Σ line amounts after price changes: what the Z report counts as turnover.
    pub fn lines_total(&self) -> i64 {
        self.lines.iter().map(SaleLine::amount).sum()
    }

    /// Σ of the FDM's VAT rows (total with VAT).
    pub fn vat_total(&self) -> i64 {
        self.vats.iter().map(|v| v.total).sum()
    }

    /// Amount of the gift voucher lines (VAT X sales of vouchers).
    pub fn gift_voucher_sold(&self) -> i64 {
        self.lines
            .iter()
            .filter(|l| l.is_gift_voucher())
            .map(SaleLine::amount)
            .sum()
    }

    /// Σ of the `PAYMENT` lines (without rounding and tips).
    pub fn paid(&self) -> i64 {
        self.payments
            .iter()
            .filter(|p| p.amount_type == "PAYMENT")
            .map(|p| p.cents)
            .sum()
    }
}

fn unreadable(event: &RawEvent, detail: &str) -> AccountingError {
    AccountingError::EventUnreadable {
        event_id: event.id,
        detail: detail.to_string(),
    }
}

/// Rate in percent the FDM applies to a label (used only when a signed sale
/// carries no VAT calculation).
fn standard_rate(label: &str) -> f64 {
    match label {
        "A" => 21.0,
        "B" => 12.0,
        "C" => 6.0,
        _ => 0.0,
    }
}

/// The VAT of a ticket: the FDM's calculation, else the lines at the standard rates.
fn vat_rows(response: &Value, lines: &[SaleLine]) -> Vec<VatRow> {
    let mut rows: BTreeMap<String, VatRow> = BTreeMap::new();
    match response["vatCalc"].as_array() {
        Some(calc) => {
            for item in calc {
                let label = string(&item["label"]);
                let row = rows.entry(label.clone()).or_insert(VatRow {
                    label,
                    rate_bp: rate_bp(&item["rate"]),
                    base: 0,
                    vat: 0,
                    total: 0,
                });
                row.base += cents(&item["taxableAmount"]);
                row.vat += cents(&item["vatAmount"]);
                row.total += cents(&item["totalAmount"]);
            }
        }
        None => {
            for (label, amount) in lines.iter().flat_map(|l| l.vats.iter()) {
                let rate = standard_rate(label);
                let row = rows.entry(label.clone()).or_insert(VatRow {
                    label: label.clone(),
                    rate_bp: (rate * 100.0).round() as i64,
                    base: 0,
                    vat: 0,
                    total: 0,
                });
                row.total += amount;
            }
            for row in rows.values_mut() {
                let rate = row.rate_bp as f64 / 100.0;
                row.base = (row.total as f64 / (1.0 + rate / 100.0)).round() as i64;
                row.vat = row.total - row.base;
            }
        }
    }
    rows.into_values().collect()
}

fn sale_lines(request: &Value) -> Vec<SaleLine> {
    request["transaction"]["transactionLines"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|line| {
            let product = &line["mainProduct"];
            let vats = product["vats"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|vat| {
                    let changes: i64 = vat["priceChanges"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|c| cents(&c["amount"]))
                        .sum();
                    (string(&vat["label"]), cents(&vat["price"]) + changes)
                })
                .collect();
            SaleLine {
                department_id: string(&product["departmentId"]),
                product_id: string(&product["productId"]),
                vats,
            }
        })
        .collect()
}

fn pay_lines(request: &Value) -> Vec<PayLine> {
    request["financials"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| PayLine {
            payment_type: string(&f["type"]),
            amount_type: f["amountType"].as_str().unwrap_or("PAYMENT").to_string(),
            cents: cents(&f["amount"]),
        })
        .collect()
}

fn pos_time(request: &Value) -> String {
    string(&request["posDateTime"])
}

/// Parse a signed sale. `original` is the archived ticket a refund refers to, when
/// the caller found it: a refund whose own request has no cost center takes the
/// channel of that ticket.
pub fn parse_sale(event: &RawEvent, original: Option<&RawEvent>) -> Result<Sale> {
    if !event.request.is_object() {
        return Err(unreadable(event, "the request is not an object"));
    }
    if !event.request["transaction"].is_object() {
        return Err(unreadable(event, "a sale without a transaction"));
    }
    let lines = sale_lines(&event.request);
    let refers_to = event
        .request
        .get("fdmRef")
        .filter(|v| v.is_object())
        .cloned();
    let lines_total: i64 = lines.iter().map(SaleLine::amount).sum();
    let channel = if event
        .request
        .get("costCenter")
        .is_some_and(Value::is_object)
    {
        channel_of(&event.request)
    } else if let Some(original) = original {
        channel_of(&original.request)
    } else {
        Channel::Takeaway
    };
    Ok(Sale {
        id: event.id,
        ticket_no: event.request["posFiscalTicketNo"].as_i64().unwrap_or(0),
        pos_datetime: pos_time(&event.request),
        fdm_datetime: event
            .fdm_datetime
            .clone()
            .unwrap_or_else(|| string(&event.response["fdmRef"]["fdmDateTime"])),
        refund: refers_to.is_some() || lines_total < 0,
        channel,
        vats: vat_rows(&event.response, &lines),
        payments: pay_lines(&event.request),
        lines,
        refers_to,
        signed_ref: event.response["fdmRef"].clone(),
    })
}

/// A signed invoice (label I): the M150 shape. The amounts are those of the sales it
/// lists (`fdmRefs`), which are already counted in the sales of their own day.
#[derive(Debug, Clone)]
pub struct Invoice {
    pub id: i64,
    pub ticket_no: i64,
    pub pos_datetime: String,
    pub invoice_no: String,
    pub customer_vat_no: String,
    pub customer_name: String,
    /// The `fdmRefs` of the request.
    pub refs: Vec<Value>,
}

pub fn parse_invoice(event: &RawEvent) -> Result<Invoice> {
    let request = &event.request;
    if !request.is_object() {
        return Err(unreadable(event, "the request is not an object"));
    }
    let name = request["customerName"]
        .as_str()
        .or_else(|| request["customer"]["name"].as_str())
        .or_else(|| request["costCenter"]["name"].as_str())
        .unwrap_or_default();
    Ok(Invoice {
        id: event.id,
        ticket_no: request["posFiscalTicketNo"].as_i64().unwrap_or(0),
        pos_datetime: pos_time(request),
        invoice_no: request["invoiceNo"]
            .as_str()
            .map(str::to_string)
            .or_else(|| request["invoiceNo"].as_i64().map(|n| n.to_string()))
            .unwrap_or_default(),
        customer_vat_no: string(&request["customerVatNo"]),
        customer_name: name.to_string(),
        refs: request["fdmRefs"].as_array().cloned().unwrap_or_default(),
    })
}

/// One cash movement of a financial event (label F).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoneyMove {
    pub id: i64,
    pub ticket_no: i64,
    pub pos_datetime: String,
    /// Signed: positive money in, negative money out.
    pub cents: i64,
    pub payment_type: String,
    pub label: String,
}

/// The money in and out lines of a financial event (drawer openings and drawer
/// declarations are not movements).
pub fn parse_money(event: &RawEvent) -> Vec<MoneyMove> {
    event.request["financials"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| f["amountType"].as_str() == Some("MONEY_IN_OUT"))
        .map(|f| MoneyMove {
            id: event.id,
            ticket_no: event.request["posFiscalTicketNo"].as_i64().unwrap_or(0),
            pos_datetime: pos_time(&event.request),
            cents: cents(&f["amount"]),
            payment_type: string(&f["type"]),
            label: string(&f["name"]),
        })
        .collect()
}

/// VAT row of a Z report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZVat {
    pub rate_bp: i64,
    pub base: i64,
    pub vat: i64,
    pub total: i64,
}

/// The signed turnover Z report that closed a booking period.
#[derive(Debug, Clone)]
pub struct ZReport {
    pub ticket_no: i64,
    pub pos_datetime: String,
    pub fdm_datetime: String,
    /// Tickets of label N (refunds included).
    pub n_count: i64,
    pub n_amount: i64,
    /// Refund tickets, `negQuantities` of reason REFUND.
    pub refunds: i64,
    pub vats: BTreeMap<String, ZVat>,
    /// `(PaymentType, amountType)` to total.
    pub payments: BTreeMap<(String, String), i64>,
}

pub fn parse_z(event: &RawEvent) -> Result<ZReport> {
    let turnover = &event.request["turnover"];
    if !turnover.is_object() {
        return Err(unreadable(event, "a Z report without a turnover"));
    }
    let n = turnover["transactions"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|t| t["eventLabel"].as_str() == Some("N"));
    let mut vats = BTreeMap::new();
    for v in turnover["vats"].as_array().into_iter().flatten() {
        let entry = vats.entry(string(&v["label"])).or_insert(ZVat {
            rate_bp: rate_bp(&v["rate"]),
            base: 0,
            vat: 0,
            total: 0,
        });
        entry.base += cents(&v["taxableAmount"]);
        entry.vat += cents(&v["vatAmount"]);
        entry.total += cents(&v["totalAmount"]);
    }
    let mut payments = BTreeMap::new();
    for p in turnover["payments"].as_array().into_iter().flatten() {
        for a in p["amount"].as_array().into_iter().flatten() {
            *payments
                .entry((string(&p["type"]), string(&a["type"])))
                .or_insert(0) += cents(&a["totalAmount"]);
        }
    }
    Ok(ZReport {
        ticket_no: event.request["posFiscalTicketNo"].as_i64().unwrap_or(0),
        pos_datetime: pos_time(&event.request),
        fdm_datetime: event
            .fdm_datetime
            .clone()
            .unwrap_or_else(|| string(&event.response["fdmRef"]["fdmDateTime"])),
        n_count: n.map_or(0, |t| t["ticketCount"].as_i64().unwrap_or(0)),
        n_amount: n.map_or(0, |t| cents(&t["amount"])),
        refunds: turnover["negQuantities"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|n| n["negQuantityReason"].as_str() == Some("REFUND"))
            .map(|n| n["ticketCount"].as_i64().unwrap_or(0))
            .sum(),
        vats,
        payments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(label: &str, request: Value, response: Value) -> RawEvent {
        RawEvent {
            id: 7,
            label: label.into(),
            request,
            response,
            fdm_datetime: Some("2026-10-01T10:00:00.000Z".into()),
        }
    }

    fn line(dept: &str, label: &str, price: f64) -> Value {
        json!({"mainProduct": {"productId": "1", "departmentId": dept, "vats": [
            {"label": label, "price": price, "priceChanges": []}]}})
    }

    fn sale_request(lines: Vec<Value>, financials: Value) -> Value {
        json!({
            "posFiscalTicketNo": 12, "posDateTime": "2026-10-01T12:00:00+02:00",
            "transaction": {"transactionLines": lines, "transactionTotal": 0},
            "financials": financials,
        })
    }

    #[test]
    fn channel_comes_from_the_signed_cost_center() {
        assert_eq!(channel_of(&json!({})), Channel::Takeaway);
        assert_eq!(channel_of(&json!({"costCenter": null})), Channel::Takeaway);
        assert_eq!(
            channel_of(&json!({"costCenter": {"type": "TABLE", "id": "T4"}})),
            Channel::DineIn
        );
        assert_eq!(
            channel_of(&json!({"costCenter": {"type": "CUSTOMER", "id": "9"}})),
            Channel::Online
        );
    }

    #[test]
    fn a_sale_reads_lines_vat_and_payments() {
        let request = sale_request(
            vec![line("10", "A", 12.0), line("22", "B", 28.0)],
            json!([
                {"type": "CASH", "amountType": "PAYMENT", "amount": 40.0},
                {"type": "CASH", "amountType": "ROUNDING", "amount": 0.0}
            ]),
        );
        let response = json!({"fdmRef": {"eventCounter": 3}, "vatCalc": [
            {"label": "A", "rate": 21, "taxableAmount": 9.92, "vatAmount": 2.08, "totalAmount": 12.0},
            {"label": "B", "rate": 12, "taxableAmount": 25.0, "vatAmount": 3.0, "totalAmount": 28.0}]});
        let sale = parse_sale(&event("N", request, response), None).unwrap();
        assert_eq!(sale.lines_total(), 4000);
        assert_eq!(sale.vat_total(), 4000);
        assert_eq!(sale.paid(), 4000);
        assert!(!sale.refund);
        assert_eq!(sale.channel, Channel::Takeaway);
        assert_eq!(sale.vats[0].rate_bp, 2100);
        assert_eq!(sale.vats[1].vat, 300);
    }

    #[test]
    fn a_refund_is_a_sale_that_refers_to_a_ticket_or_is_negative() {
        let mut request = sale_request(
            vec![line("10", "A", -12.0)],
            json!([{"type": "CASH", "amountType": "PAYMENT", "amount": -12.0}]),
        );
        let negative = parse_sale(&event("N", request.clone(), json!({})), None).unwrap();
        assert!(negative.refund);
        request["transaction"]["transactionLines"] = json!([line("10", "A", 12.0)]);
        request["fdmRef"] = json!({"eventCounter": 1});
        assert!(
            parse_sale(&event("N", request, json!({})), None)
                .unwrap()
                .refund
        );
    }

    #[test]
    fn a_refund_without_cost_center_takes_the_channel_of_its_ticket() {
        let original = event(
            "N",
            json!({"costCenter": {"type": "TABLE", "id": "T2"}}),
            json!({}),
        );
        let request = sale_request(vec![line("10", "A", -12.0)], json!([]));
        let sale = parse_sale(&event("N", request, json!({})), Some(&original)).unwrap();
        assert_eq!(sale.channel, Channel::DineIn);
    }

    #[test]
    fn without_vat_calc_the_lines_are_taxed_at_the_standard_rate() {
        let request = sale_request(vec![line("10", "A", 12.1)], json!([]));
        let sale = parse_sale(&event("N", request, json!({})), None).unwrap();
        assert_eq!(sale.vats.len(), 1);
        assert_eq!(sale.vats[0].base, 1000);
        assert_eq!(sale.vats[0].vat, 210);
    }

    #[test]
    fn gift_voucher_lines_are_found_by_department() {
        let request = sale_request(
            vec![line("GV", "X", 50.0), line("10", "A", 12.0)],
            json!([]),
        );
        let sale = parse_sale(&event("N", request, json!({})), None).unwrap();
        assert_eq!(sale.gift_voucher_sold(), 5000);
    }

    #[test]
    fn a_sale_without_a_transaction_is_unreadable() {
        let e = event("N", json!({"posFiscalTicketNo": 1}), json!({}));
        let err = parse_sale(&e, None).unwrap_err();
        assert_eq!(err.code(), "event_unreadable");
    }

    #[test]
    fn money_movements_skip_drawer_declarations() {
        let e = event(
            "F",
            json!({"posFiscalTicketNo": 5, "posDateTime": "2026-10-01T08:00:00+02:00",
                "financials": [
                  {"type": "CASH", "name": "Fond", "amountType": "MONEY_IN_OUT", "amount": 100.0},
                  {"type": "CASH", "name": "Decl", "amountType": "DRAWER_DECLARATION", "amount": 90.0},
                  {"type": "CASH", "name": "Banque", "amountType": "MONEY_IN_OUT", "amount": -40.5}]}),
            json!({}),
        );
        let moves = parse_money(&e);
        assert_eq!(moves.len(), 2);
        assert_eq!(moves[0].cents, 10000);
        assert_eq!(moves[1].cents, -4050);
        assert_eq!(moves[1].label, "Banque");
    }

    #[test]
    fn an_invoice_reads_the_m150_shape() {
        let e = event(
            "I",
            json!({"posFiscalTicketNo": 1012, "posDateTime": "2024-07-29T16:22:39.000+02:00",
                "customerVatNo": "BE0308357159", "invoiceNo": "20240005",
                "costCenter": {"id": "100", "type": "CUSTOMER", "reference": "FIN"},
                "fdmRefs": [{"eventLabel": "N", "eventCounter": 14}, {"eventLabel": "N", "eventCounter": 16}]}),
            json!({}),
        );
        let i = parse_invoice(&e).unwrap();
        assert_eq!(i.invoice_no, "20240005");
        assert_eq!(i.customer_vat_no, "BE0308357159");
        assert_eq!(i.refs.len(), 2);
    }

    #[test]
    fn a_z_report_reads_counts_vat_and_payments_by_type() {
        let e = event(
            "R",
            json!({"posFiscalTicketNo": 209, "turnover": {
                "transactions": [{"eventLabel": "N", "ticketCount": 4, "amount": 102.04},
                                 {"eventLabel": "P", "ticketCount": 3, "amount": 198}],
                "vats": [{"label": "A", "rate": 21, "taxableAmount": 51.27, "vatAmount": 10.77, "totalAmount": 62.04}],
                "payments": [{"type": "CASH", "amount": [{"type": "PAYMENT", "totalAmount": 99}]},
                             {"type": "CASH", "amount": [{"type": "PAYMENT", "totalAmount": 1}]}],
                "negQuantities": [{"negQuantityReason": "REFUND", "ticketCount": 1}]}}),
            json!({}),
        );
        let z = parse_z(&e).unwrap();
        assert_eq!((z.n_count, z.n_amount, z.refunds), (4, 10204, 1));
        assert_eq!(z.vats["A"].total, 6204);
        assert_eq!(
            z.payments[&("CASH".to_string(), "PAYMENT".to_string())],
            10000
        );
    }
}
