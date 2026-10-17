//! Totals of the turnover and user reports (POS spec v2.2 §2.2.2,
//! objects `turnover`, `users`, `posDevice`, `fdmDevice`), computed from
//! the events the FDM signed: the archived request and response of each
//! event. The archive is the fiscal truth, so the reports add up exactly
//! what the FDM received - including training labels, corrections and
//! refunds - and never read the mutable order tables.
//!
//! Amounts are summed in eurocents; the JSON amounts (multiples of 0.01)
//! are converted once, with rounding.
//!
//! Rules, per the spec:
//! - `transactions`: every eventLabel used with a `transaction` object,
//!   with its number of events and Σ(VatInput.price) +
//!   Σ(VatInput.priceChanges.amount).
//! - `departments`: the same sum of label N lines, per department.
//! - `vats`: Σ of the FDM's `vatCalc` of label N events, per VAT label.
//! - `payments`: the financials of label N events, per payment method,
//!   plus the lines of the payment corrections (label F, operation
//!   PAYMENT_CORRECTION): a negative line goes to `negativeCorrections`,
//!   a positive one to `positiveCorrections`, each line counts once in
//!   `correctionsCount`, and `totalAmount` is normal + both corrections.
//! - `negQuantities`: negative lines, per reason, counted once, on the
//!   event that first transmitted them (producers FAQ Q303, AM of 29
//!   April 2024 art. 29 v): on the order registration (P, ORDER) for
//!   table service, on the sale (N) for a direct sale or a refund, and
//!   never on a pre-bill. A table sale resends its registered entries:
//!   those are left out ([`ReportEvent::registered_negatives`]).
//! - `drawersOpenCount`: F events whose operation is DRAWER_OPEN.
//! - `users[].totalAmount`: Σ transactionTotal of the user's N events;
//!   the activity range covers every event but the reports themselves.
//!
//! - `priceChanges`: the PUBLIC price changes of label N lines, per
//!   change id and VAT label, negative and positive apart.
//!
//! - `invoices`: one entry per I event, amount from
//!   [`ReportEvent::invoice_amount`].
//!
//! Foreign currencies are not supported in v1.0.0, so their totals are
//! always empty.

use std::collections::BTreeMap;

use chrono::{DateTime, FixedOffset};
use serde_json::Value;

use crate::archive::ArchivedEvent;
use crate::inputs::{
    DepartmentTotalInput, EventTotalInput, FdmDeviceInput, InOutItemInput, InvoiceTotalInput,
    NegQuantityTotalInput, PaymentTotalAmountInput, PaymentTotalInput, PosDeviceInput,
    PriceChangeTotalInput, PriceChangeVatTotalInput, TurnoverInput, UserItemInput, VatTotalInput,
};
use crate::types::{
    EventLabel, InOut, NegQuantityReason, PaymentLineType, PaymentType, PriceChangeType, VatLabel,
};

/// One archived event, as signed by the FDM.
#[derive(Debug, Clone)]
pub struct ReportEvent {
    /// The label the event was archived (and signed) under.
    pub label: EventLabel,
    pub request: Value,
    pub response: Value,
    /// For a sale: the negative lines its order registrations (P,
    /// ORDER) already transmitted, possibly in an earlier booking
    /// period. They were counted there, so the sale does not count them
    /// again.
    pub registered_negatives: Vec<Value>,
    /// For an invoice: Σ transactionTotal of the sales in its `fdmRefs`.
    pub invoice_amount: Option<f64>,
}

impl ReportEvent {
    /// `None` for an archive row whose JSON can't be read; callers
    /// decide whether that is fatal (it never happens for rows written
    /// by `commit` in `commands/signing.rs`).
    pub fn from_archive(event: &ArchivedEvent) -> Option<Self> {
        let request = serde_json::from_str(&event.event_data_json).ok()?;
        let response = match event.fdm_response_json.as_deref() {
            Some(json) => serde_json::from_str(json).ok()?,
            None => Value::Null,
        };
        Some(Self {
            label: event.event_type,
            request,
            response,
            registered_negatives: Vec::new(),
            invoice_amount: None,
        })
    }

    /// The transaction lines of the event's request.
    pub fn lines(&self) -> &[Value] {
        self.request["transaction"]["transactionLines"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn has_transaction(&self) -> bool {
        self.request
            .get("transaction")
            .is_some_and(Value::is_object)
            || self.is_cost_center_change()
    }

    /// A table transfer (`signCostCenterChange`): its lines sit in
    /// `transfer.from[].transaction` and `transfer.to[].transaction`.
    pub fn is_cost_center_change(&self) -> bool {
        self.label == EventLabel::P && self.request.get("transfer").is_some_and(Value::is_object)
    }

    /// Every line of the transfer's items, both sides.
    fn transfer_lines(&self) -> Vec<&Value> {
        ["from", "to"]
            .into_iter()
            .filter_map(|side| self.request["transfer"][side].as_array())
            .flatten()
            .filter_map(|item| item["transaction"]["transactionLines"].as_array())
            .flatten()
            .collect()
    }

    fn employee_id(&self) -> Option<&str> {
        self.request["employeeId"].as_str()
    }

    fn pos_date_time(&self) -> Option<DateTime<FixedOffset>> {
        self.request["posDateTime"]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
    }

    fn operation(&self) -> Option<&str> {
        self.response["eventOperation"].as_str()
    }

    /// An order registration (`signOrder`), not a pre-bill.
    pub fn is_order_registration(&self) -> bool {
        self.label == EventLabel::P && self.operation() == Some("ORDER")
    }

    /// The transaction lines with a negative quantity.
    pub fn negative_lines(&self) -> Vec<Value> {
        self.lines()
            .iter()
            .chain(self.transfer_lines())
            .filter(|l| l["mainProduct"]["quantity"].as_f64().unwrap_or(0.0) < 0.0)
            .cloned()
            .collect()
    }
}

/// A JSON amount (multiple of 0.01) in eurocents.
fn cents(v: &Value) -> i64 {
    (v.as_f64().unwrap_or(0.0) * 100.0).round() as i64
}

fn eur(cents: i64) -> f64 {
    cents as f64 / 100.0
}

/// Σ(VatInput.price) + Σ(VatInput.priceChanges.amount) of one
/// transaction line: its amount after price changes.
fn line_amount(line: &Value) -> i64 {
    line["mainProduct"]["vats"]
        .as_array()
        .map(|vats| {
            vats.iter()
                .map(|v| {
                    let changes: i64 = v["priceChanges"]
                        .as_array()
                        .map(|c| c.iter().map(|c| cents(&c["amount"])).sum())
                        .unwrap_or(0);
                    cents(&v["price"]) + changes
                })
                .sum()
        })
        .unwrap_or(0)
}

fn transaction_amount(event: &ReportEvent) -> i64 {
    // A transfer takes lines off one table and puts them on another:
    // from and to cancel out (0 for a full move).
    event
        .lines()
        .iter()
        .chain(event.transfer_lines())
        .map(line_amount)
        .sum()
}

fn label_name(label: EventLabel) -> String {
    serde_json::to_value(label)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Order of the labels in `transactions`: sales first, as in the spec's
/// examples, then the others alphabetically.
fn label_rank(label: EventLabel) -> (u8, String) {
    let name = label_name(label);
    let rank = match name.as_str() {
        "N" => 0,
        "P" => 1,
        "T" => 2,
        _ => 3,
    };
    (rank, name)
}

fn is_sale(event: &ReportEvent) -> bool {
    event.label == EventLabel::N
}

impl ReportEvent {
    /// A payment correction (`signPaymentCorrection`) signed as a real
    /// F event. A training one (label T) never reaches the turnover.
    pub fn is_payment_correction(&self) -> bool {
        self.label == EventLabel::F && self.operation() == Some("PAYMENT_CORRECTION")
    }
}

/// A sale or a payment correction: the events whose `financials` the
/// payment totals add up.
fn counts_in_payments(event: &ReportEvent) -> bool {
    is_sale(event) || event.is_payment_correction()
}

// ── Turnover report ────────────────────────────────────────────────

pub fn turnover(events: &[ReportEvent]) -> TurnoverInput {
    TurnoverInput {
        transactions: transactions(events),
        departments: departments(events),
        vats: vats(events),
        payments: payments(events.iter().filter(|e| counts_in_payments(e))),
        drawers_open_count: events
            .iter()
            .filter(|e| e.label == EventLabel::F && e.operation() == Some("DRAWER_OPEN"))
            .count() as i64,
        neg_quantities: neg_quantities(events),
        price_changes: price_changes(events.iter().filter(|e| is_sale(e))),
        invoices: Some(invoices(events)),
    }
}

/// One entry per invoice of the period, in signing order.
fn invoices(events: &[ReportEvent]) -> Vec<InvoiceTotalInput> {
    events
        .iter()
        .filter(|e| e.label == EventLabel::I)
        .map(|e| InvoiceTotalInput {
            invoice_no: e.request["invoiceNo"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            amount: e.invoice_amount.unwrap_or(0.0),
        })
        .collect()
}

/// The public price changes of the sales, per change and VAT label.
fn price_changes<'a>(sales: impl Iterator<Item = &'a ReportEvent>) -> Vec<PriceChangeTotalInput> {
    // (id, name) -> label -> (negative, positive)
    let mut totals: BTreeMap<(String, String), BTreeMap<String, (i64, i64)>> = BTreeMap::new();
    for line in sales.flat_map(|e| e.lines()) {
        for vat in line["mainProduct"]["vats"].as_array().into_iter().flatten() {
            let label = vat["label"].as_str().unwrap_or_default().to_string();
            for change in vat["priceChanges"].as_array().into_iter().flatten() {
                if change["type"].as_str() != Some("PUBLIC") {
                    continue;
                }
                let key = (
                    change["id"].as_str().unwrap_or_default().to_string(),
                    change["name"].as_str().unwrap_or_default().to_string(),
                );
                let amount = cents(&change["amount"]);
                let entry = totals
                    .entry(key)
                    .or_default()
                    .entry(label.clone())
                    .or_default();
                if amount < 0 {
                    entry.0 += amount;
                } else {
                    entry.1 += amount;
                }
            }
        }
    }
    totals
        .into_iter()
        .map(|((id, name), labels)| PriceChangeTotalInput {
            id,
            name,
            price_change_type: PriceChangeType::Public,
            amount: labels
                .into_iter()
                .filter_map(|(label, (negative, positive))| {
                    let label: VatLabel = serde_json::from_value(Value::String(label)).ok()?;
                    Some(PriceChangeVatTotalInput {
                        label,
                        negative: eur(negative),
                        positive: eur(positive),
                    })
                })
                .collect(),
        })
        .collect()
}

fn transactions(events: &[ReportEvent]) -> Vec<EventTotalInput> {
    let mut totals: BTreeMap<(u8, String), (EventLabel, i64, i64)> = BTreeMap::new();
    for event in events.iter().filter(|e| e.has_transaction()) {
        let entry = totals
            .entry(label_rank(event.label))
            .or_insert((event.label, 0, 0));
        entry.1 += 1;
        entry.2 += transaction_amount(event);
    }
    totals
        .into_values()
        .map(|(event_label, ticket_count, amount)| EventTotalInput {
            event_label,
            ticket_count,
            amount: eur(amount),
        })
        .collect()
}

fn departments(events: &[ReportEvent]) -> Vec<DepartmentTotalInput> {
    let mut totals: BTreeMap<(String, String), i64> = BTreeMap::new();
    for line in events.iter().filter(|e| is_sale(e)).flat_map(|e| e.lines()) {
        let product = &line["mainProduct"];
        let key = (
            product["departmentId"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            product["departmentName"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        );
        *totals.entry(key).or_default() += line_amount(line);
    }
    totals
        .into_iter()
        .map(
            |((department_id, department_name), amount)| DepartmentTotalInput {
                department_id,
                department_name,
                amount: eur(amount),
            },
        )
        .collect()
}

/// VAT rates of the Belgian labels, used only when a signed sale has no
/// `vatCalc` (the FDM always returns one for label N).
fn standard_rate(label: VatLabel) -> f64 {
    match label {
        VatLabel::A => 21.0,
        VatLabel::B => 12.0,
        VatLabel::C => 6.0,
        _ => 0.0,
    }
}

#[derive(Default)]
struct VatSum {
    rate: f64,
    taxable: i64,
    vat: i64,
    total: i64,
}

fn vats(events: &[ReportEvent]) -> Vec<VatTotalInput> {
    let mut totals: BTreeMap<String, (VatLabel, VatSum)> = BTreeMap::new();
    let mut add = |label: VatLabel, rate: f64, taxable: i64, vat: i64, total: i64| {
        let entry = totals
            .entry(format!("{label:?}"))
            .or_insert((label, VatSum::default()));
        entry.1.rate = rate;
        entry.1.taxable += taxable;
        entry.1.vat += vat;
        entry.1.total += total;
    };
    for event in events.iter().filter(|e| is_sale(e)) {
        match event.response["vatCalc"].as_array() {
            Some(calc) => {
                for item in calc {
                    let Ok(label) = serde_json::from_value::<VatLabel>(item["label"].clone())
                    else {
                        continue;
                    };
                    add(
                        label,
                        item["rate"].as_f64().unwrap_or(0.0),
                        cents(&item["taxableAmount"]),
                        cents(&item["vatAmount"]),
                        cents(&item["totalAmount"]),
                    );
                }
            }
            None => {
                let mut per_label: BTreeMap<String, (VatLabel, i64)> = BTreeMap::new();
                for vat in event
                    .lines()
                    .iter()
                    .filter_map(|l| l["mainProduct"]["vats"].as_array())
                    .flatten()
                {
                    let Ok(label) = serde_json::from_value::<VatLabel>(vat["label"].clone()) else {
                        continue;
                    };
                    per_label
                        .entry(format!("{label:?}"))
                        .or_insert((label, 0))
                        .1 += cents(&vat["price"]);
                }
                for (label, total) in per_label.into_values() {
                    let rate = standard_rate(label);
                    let taxable = (total as f64 / (1.0 + rate / 100.0)).round() as i64;
                    add(label, rate, taxable, total - taxable, total);
                }
            }
        }
    }
    totals
        .into_values()
        .map(|(label, sum)| VatTotalInput {
            label,
            rate: sum.rate,
            taxable_amount: eur(sum.taxable),
            vat_amount: eur(sum.vat),
            total_amount: eur(sum.total),
        })
        .collect()
}

/// Payments of the given sales and payment corrections, per method
/// (id, name, type) and per amount type. A sale line is a normal
/// amount; a correction line is a negative or positive correction (the
/// line it removes and the line it adds), counted once each.
fn payments<'a>(events: impl Iterator<Item = &'a ReportEvent>) -> Vec<PaymentTotalInput> {
    /// Cents per bucket of one amount type.
    #[derive(Default)]
    struct Sums {
        normal: i64,
        negative: i64,
        positive: i64,
        count: i64,
    }
    // (id, name, type) -> (type, amount type -> (amount type, sums))
    type Key = (String, String, String);
    type Amounts = BTreeMap<String, (PaymentLineType, Sums)>;
    let mut totals: BTreeMap<Key, (PaymentType, Amounts)> = BTreeMap::new();
    for event in events {
        let correction = event.is_payment_correction();
        let Some(financials) = event.request["financials"].as_array() else {
            continue;
        };
        for financial in financials {
            let Ok(payment_type) = serde_json::from_value::<PaymentType>(financial["type"].clone())
            else {
                continue;
            };
            let amount_type =
                serde_json::from_value::<PaymentLineType>(financial["amountType"].clone())
                    .unwrap_or(PaymentLineType::Payment);
            let key = (
                financial["id"].as_str().unwrap_or_default().to_string(),
                financial["name"].as_str().unwrap_or_default().to_string(),
                format!("{payment_type:?}"),
            );
            let (_, amounts) = totals
                .entry(key)
                .or_insert_with(|| (payment_type, BTreeMap::new()));
            let sums = &mut amounts
                .entry(format!("{amount_type:?}"))
                .or_insert((amount_type, Sums::default()))
                .1;
            let amount = cents(&financial["amount"]);
            if !correction {
                sums.normal += amount;
            } else if amount < 0 {
                sums.negative += amount;
                sums.count += 1;
            } else {
                sums.positive += amount;
                sums.count += 1;
            }
        }
    }
    totals
        .into_iter()
        .map(
            |((id, name, _), (payment_type, amounts))| PaymentTotalInput {
                id,
                name,
                payment_type,
                amount: amounts
                    .into_values()
                    .map(|(amount_type, sums)| PaymentTotalAmountInput {
                        amount_type,
                        normal_amount: eur(sums.normal),
                        negative_corrections: eur(sums.negative),
                        positive_corrections: eur(sums.positive),
                        corrections_count: sums.count,
                        total_amount: eur(sums.normal + sums.negative + sums.positive),
                        normal_foreign: None,
                        corrections_foreign: None,
                    })
                    .collect(),
            },
        )
        .collect()
}

fn neg_quantities(events: &[ReportEvent]) -> Vec<NegQuantityTotalInput> {
    // reason → (reason, lines, tickets, amount)
    let mut totals: BTreeMap<String, (NegQuantityReason, i64, i64, i64)> = BTreeMap::new();
    for event in events {
        let lines = if event.is_order_registration() || event.is_cost_center_change() {
            event.negative_lines()
        } else if is_sale(event) {
            let mut registered = event.registered_negatives.clone();
            event
                .negative_lines()
                .into_iter()
                .filter(|line| match registered.iter().position(|r| r == line) {
                    Some(i) => {
                        registered.remove(i);
                        false
                    }
                    None => true,
                })
                .collect()
        } else {
            continue;
        };
        let mut seen_in_ticket: Vec<String> = Vec::new();
        for line in &lines {
            let product = &line["mainProduct"];
            let reason =
                serde_json::from_value::<NegQuantityReason>(product["negQuantityReason"].clone())
                    .unwrap_or(NegQuantityReason::Correction);
            let key = format!("{reason:?}");
            let entry = totals.entry(key.clone()).or_insert((reason, 0, 0, 0));
            entry.1 += 1;
            entry.3 += line_amount(line);
            if !seen_in_ticket.contains(&key) {
                entry.2 += 1;
                seen_in_ticket.push(key);
            }
        }
    }
    totals
        .into_values()
        .map(
            |(neg_quantity_reason, neg_quantity_count, ticket_count, amount)| {
                NegQuantityTotalInput {
                    neg_quantity_reason,
                    neg_quantity_count,
                    ticket_count,
                    amount: eur(amount),
                }
            },
        )
        .collect()
}

// ── User report ────────────────────────────────────────────────────

/// First and last event of a group, by POS time; keeps the original
/// strings (the spec asks for the timestamps as sent).
#[derive(Default)]
struct Range {
    first: Option<(DateTime<FixedOffset>, String)>,
    last: Option<(DateTime<FixedOffset>, String)>,
}

impl Range {
    fn add(&mut self, event: &ReportEvent) {
        let (Some(at), Some(raw)) = (event.pos_date_time(), event.request["posDateTime"].as_str())
        else {
            return;
        };
        if self.first.as_ref().is_none_or(|(t, _)| at < *t) {
            self.first = Some((at, raw.to_string()));
        }
        if self.last.as_ref().is_none_or(|(t, _)| at >= *t) {
            self.last = Some((at, raw.to_string()));
        }
    }

    fn first(&self) -> String {
        self.first
            .as_ref()
            .map(|(_, s)| s.clone())
            .unwrap_or_default()
    }

    fn last(&self) -> String {
        self.last
            .as_ref()
            .map(|(_, s)| s.clone())
            .unwrap_or_default()
    }
}

pub fn users(events: &[ReportEvent]) -> Vec<UserItemInput> {
    let mut by_user: BTreeMap<String, Vec<&ReportEvent>> = BTreeMap::new();
    for event in events.iter().filter(|e| e.label != EventLabel::R) {
        if let Some(id) = event.employee_id() {
            by_user.entry(id.to_string()).or_default().push(event);
        }
    }
    by_user
        .into_iter()
        .map(|(employee_id, events)| {
            let mut range = Range::default();
            events.iter().for_each(|e| range.add(e));
            let sales = || events.iter().copied().filter(|e| is_sale(e));
            UserItemInput {
                employee_id,
                total_amount: eur(sales()
                    .map(|e| cents(&e.request["transaction"]["transactionTotal"]))
                    .sum()),
                first_pos_date_time: range.first(),
                last_pos_date_time: range.last(),
                social_events: events
                    .iter()
                    .filter(|e| e.label == EventLabel::S)
                    .filter_map(|e| {
                        let in_out = match e.operation()? {
                            "WORK_IN" => InOut::In,
                            "WORK_OUT" => InOut::Out,
                            _ => return None,
                        };
                        Some(InOutItemInput {
                            pos_date_time: e.request["posDateTime"].as_str()?.to_string(),
                            in_out,
                        })
                    })
                    .collect(),
                payments: payments(events.iter().copied().filter(|e| counts_in_payments(e))),
            }
        })
        .collect()
}

// ── Devices ────────────────────────────────────────────────────────

/// Every POS terminal whose events are in the report, ordered by POS
/// then by first activity.
pub fn pos_devices(events: &[ReportEvent]) -> Vec<PosDeviceInput> {
    let mut ranges: BTreeMap<(String, String), Range> = BTreeMap::new();
    for event in events {
        let key = (
            event.request["posId"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            event.request["terminalId"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        );
        ranges.entry(key).or_default().add(event);
    }
    let mut devices: Vec<_> = ranges
        .into_iter()
        .filter(|(_, r)| r.first.is_some())
        .collect();
    devices.sort_by(|((pos_a, _), a), ((pos_b, _), b)| {
        pos_a.cmp(pos_b).then(
            a.first
                .as_ref()
                .map(|f| f.0)
                .cmp(&b.first.as_ref().map(|f| f.0)),
        )
    });
    devices
        .into_iter()
        .map(|((pos_id, terminal_id), range)| PosDeviceInput {
            pos_id,
            terminal_id: Some(terminal_id),
            first_pos_date_time: range.first(),
            last_pos_date_time: range.last(),
        })
        .collect()
}

/// Every FDM that signed events of the report, with its first and last
/// signed event (by total counter).
pub fn fdm_devices(events: &[ReportEvent]) -> Vec<FdmDeviceInput> {
    let mut devices: BTreeMap<String, FdmDeviceInput> = BTreeMap::new();
    for fdm_ref in events.iter().map(|e| &e.response["fdmRef"]) {
        let (Some(fdm_id), Some(counter), Some(at)) = (
            fdm_ref["fdmId"].as_str(),
            fdm_ref["totalCounter"].as_i64(),
            fdm_ref["fdmDateTime"].as_str(),
        ) else {
            continue;
        };
        let device = devices
            .entry(fdm_id.to_string())
            .or_insert_with(|| FdmDeviceInput {
                fdm_id: fdm_id.to_string(),
                first_fdm_date_time: at.to_string(),
                first_total_counter: counter,
                last_fdm_date_time: at.to_string(),
                last_total_counter: counter,
            });
        if counter < device.first_total_counter {
            device.first_total_counter = counter;
            device.first_fdm_date_time = at.to_string();
        }
        if counter > device.last_total_counter {
            device.last_total_counter = counter;
            device.last_fdm_date_time = at.to_string();
        }
    }
    devices.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(label: EventLabel, request: Value, response: Value) -> ReportEvent {
        ReportEvent {
            label,
            request,
            response,
            registered_negatives: Vec::new(),
            invoice_amount: None,
        }
    }

    fn line(product: &str, dept: (&str, &str), qty: i64, vats: &[(&str, f64)]) -> Value {
        json!({
            "lineType": "SINGLE_PRODUCT",
            "mainProduct": {
                "productId": product,
                "productName": product,
                "departmentId": dept.0,
                "departmentName": dept.1,
                "quantity": qty,
                "vats": vats.iter().map(|(l, p)| json!({"label": l, "price": p})).collect::<Vec<_>>(),
            },
        })
    }

    fn sale(user: &str, at: &str, lines: Vec<Value>, total: f64, financials: Value) -> Value {
        json!({
            "posId": "CPOS0031234567",
            "terminalId": "POS1",
            "employeeId": user,
            "posDateTime": at,
            "transaction": { "transactionLines": lines, "transactionTotal": total },
            "financials": financials,
        })
    }

    fn cash(amount: f64) -> Value {
        json!([{ "id": "1", "name": "Espèces", "type": "CASH", "amount": amount, "amountType": "PAYMENT" }])
    }

    fn signed(op: &str, counter: i64, vat_calc: Value) -> Value {
        json!({
            "eventOperation": op,
            "fdmRef": {
                "fdmId": "FDM01",
                "fdmDateTime": format!("2026-07-03T10:{counter:02}:00Z"),
                "eventLabel": "N",
                "eventCounter": counter,
                "totalCounter": counter,
            },
            "vatCalc": vat_calc,
        })
    }

    const U1: &str = "75061189702";
    const U2: &str = "84022899837";

    #[test]
    fn empty_archive_gives_empty_totals() {
        let t = turnover(&[]);
        assert!(t.transactions.is_empty());
        assert!(t.departments.is_empty());
        assert!(t.vats.is_empty());
        assert!(t.payments.is_empty());
        assert!(t.neg_quantities.is_empty());
        assert_eq!(t.drawers_open_count, 0);
        assert!(users(&[]).is_empty());
        assert!(pos_devices(&[]).is_empty());
        assert!(fdm_devices(&[]).is_empty());
    }

    #[test]
    fn amounts_are_summed_in_cents_without_float_drift() {
        // 3.04 * 100 = 303.99999999999994 in f64: truncation would lose a cent.
        let events: Vec<_> = (0..10)
            .map(|i| {
                event(
                    EventLabel::N,
                    sale(
                        U1,
                        "2026-07-03T12:00:00+02:00",
                        vec![line("12009", ("12", "Soft"), 1, &[("A", 3.04)])],
                        3.04,
                        cash(3.04),
                    ),
                    signed("SALE", i + 1, Value::Null),
                )
            })
            .collect();
        let t = turnover(&events);
        assert_eq!(t.transactions[0].amount, 30.4);
        assert_eq!(t.departments[0].amount, 30.4);
        assert_eq!(t.payments[0].amount[0].normal_amount, 30.4);
        assert_eq!(users(&events)[0].total_amount, 30.4);
    }

    /// A table transfer of 2 x 24,00 EUR from T01 to T05.
    fn transfer_event() -> ReportEvent {
        let mut from_line = line("1", ("10", "Aperitifs"), -2, &[("A", -24.0)]);
        from_line["mainProduct"]["negQuantityReason"] = json!("COST_CENTER_CHANGE");
        let to_line = line("1", ("10", "Aperitifs"), 2, &[("A", 24.0)]);
        event(
            EventLabel::P,
            json!({
                "employeeId": U1,
                "posDateTime": "2026-07-03T11:00:00+02:00",
                "transfer": {
                    "from": [{ "costCenter": {"id": "T01"}, "transaction": { "transactionLines": [from_line], "transactionTotal": -24.0 } }],
                    "to": [{ "costCenter": {"id": "T05"}, "transaction": { "transactionLines": [to_line], "transactionTotal": 24.0 } }],
                },
            }),
            json!({"eventOperation": "COST_CENTER_CHANGE"}),
        )
    }

    #[test]
    fn a_table_transfer_counts_once_under_p_with_an_amount_of_zero() {
        let t = turnover(&[transfer_event()]);
        let summary: Vec<_> = t
            .transactions
            .iter()
            .map(|x| (label_name(x.event_label), x.ticket_count, x.amount))
            .collect();
        assert_eq!(summary, vec![("P".to_string(), 1, 0.0)]);
        // Not a sale: no department, VAT or payment.
        assert!(t.departments.is_empty());
        assert!(t.payments.is_empty());
    }

    #[test]
    fn a_table_transfer_counts_its_negative_lines_under_cost_center_change() {
        let t = turnover(&[transfer_event()]);
        assert_eq!(t.neg_quantities.len(), 1);
        let n = &t.neg_quantities[0];
        assert_eq!(n.neg_quantity_reason, NegQuantityReason::CostCenterChange);
        assert_eq!((n.neg_quantity_count, n.ticket_count), (1, 1));
        assert_eq!(n.amount, -24.0);
    }

    #[test]
    fn transactions_count_every_label_with_a_transaction() {
        let order = json!({
            "employeeId": U1,
            "posDateTime": "2026-07-03T11:00:00+02:00",
            "transaction": { "transactionLines": [line("1", ("10", "Aperitifs"), 2, &[("A", 24.0)])], "transactionTotal": 24.0 },
        });
        let events = vec![
            event(EventLabel::P, order.clone(), Value::Null),
            event(EventLabel::P, order, Value::Null),
            event(
                EventLabel::N,
                sale(
                    U1,
                    "2026-07-03T12:00:00+02:00",
                    vec![line("1", ("10", "Aperitifs"), 2, &[("A", 24.0)])],
                    24.0,
                    cash(24.0),
                ),
                Value::Null,
            ),
            // Events without a transaction are not turnover.
            event(
                EventLabel::F,
                json!({"employeeId": U1, "financials": cash(50.0)}),
                json!({"eventOperation": "MONEY_IN_OUT"}),
            ),
            event(EventLabel::S, json!({"employeeId": U1}), Value::Null),
            event(EventLabel::R, json!({"employeeId": U1}), Value::Null),
        ];
        let t = turnover(&events);
        let summary: Vec<_> = t
            .transactions
            .iter()
            .map(|x| (label_name(x.event_label), x.ticket_count, x.amount))
            .collect();
        assert_eq!(
            summary,
            vec![("N".to_string(), 1, 24.0), ("P".to_string(), 2, 48.0)]
        );
    }

    #[test]
    fn training_events_are_reported_under_t_and_nowhere_else() {
        let training = event(
            EventLabel::T,
            sale(
                U1,
                "2026-07-03T12:00:00+02:00",
                vec![line("1", ("10", "Aperitifs"), 1, &[("A", 12.0)])],
                12.0,
                cash(12.0),
            ),
            signed(
                "SALE",
                1,
                json!([{"label": "A", "rate": 21, "taxableAmount": 9.92, "vatAmount": 2.08, "totalAmount": 12.0}]),
            ),
        );
        let t = turnover(std::slice::from_ref(&training));
        assert_eq!(t.transactions.len(), 1);
        assert_eq!(t.transactions[0].event_label, EventLabel::T);
        assert!(t.departments.is_empty());
        assert!(t.vats.is_empty());
        assert!(t.payments.is_empty());
        assert!(t.neg_quantities.is_empty());
        assert_eq!(users(&[training])[0].total_amount, 0.0);
    }

    #[test]
    fn departments_and_vats_come_from_sales_only() {
        let lines = vec![
            line("1", ("10", "Aperitifs"), 2, &[("A", 24.0)]),
            line("2", ("22", "Main Dishes"), 1, &[("B", 28.0)]),
        ];
        let vat_calc = json!([
            {"label": "A", "rate": 21, "taxableAmount": 19.83, "vatAmount": 4.17, "totalAmount": 24.0},
            {"label": "B", "rate": 12, "taxableAmount": 25.0, "vatAmount": 3.0, "totalAmount": 28.0},
        ]);
        let events = vec![
            event(
                EventLabel::P,
                sale(
                    U1,
                    "2026-07-03T11:00:00+02:00",
                    lines.clone(),
                    52.0,
                    Value::Null,
                ),
                Value::Null,
            ),
            event(
                EventLabel::N,
                sale(U1, "2026-07-03T12:00:00+02:00", lines, 52.0, cash(52.0)),
                signed("SALE", 1, vat_calc),
            ),
        ];
        let t = turnover(&events);
        let depts: Vec<_> = t
            .departments
            .iter()
            .map(|d| {
                (
                    d.department_id.as_str(),
                    d.department_name.as_str(),
                    d.amount,
                )
            })
            .collect();
        assert_eq!(
            depts,
            vec![("10", "Aperitifs", 24.0), ("22", "Main Dishes", 28.0)]
        );
        assert_eq!(t.vats.len(), 2);
        assert_eq!(t.vats[0].label, VatLabel::A);
        assert_eq!(
            (
                t.vats[0].rate,
                t.vats[0].taxable_amount,
                t.vats[0].vat_amount,
                t.vats[0].total_amount
            ),
            (21.0, 19.83, 4.17, 24.0)
        );
        assert_eq!(t.vats[1].label, VatLabel::B);
        assert_eq!(t.vats[1].total_amount, 28.0);
    }

    #[test]
    fn vat_totals_add_the_fdm_calculation_of_each_ticket() {
        // Per-ticket rounding: 3 × 1.00 € at 21 % is 3 × (0.83 + 0.17),
        // not a recomputation of 3.00 € (2.48 + 0.52).
        let calc = json!([{"label": "A", "rate": 21, "taxableAmount": 0.83, "vatAmount": 0.17, "totalAmount": 1.0}]);
        let events: Vec<_> = (1..=3)
            .map(|i| {
                event(
                    EventLabel::N,
                    sale(
                        U1,
                        "2026-07-03T12:00:00+02:00",
                        vec![line("1", ("12", "Soft"), 1, &[("A", 1.0)])],
                        1.0,
                        cash(1.0),
                    ),
                    signed("SALE", i, calc.clone()),
                )
            })
            .collect();
        let vat = &turnover(&events).vats[0];
        assert_eq!(
            (vat.taxable_amount, vat.vat_amount, vat.total_amount),
            (2.49, 0.51, 3.0)
        );
    }

    #[test]
    fn vat_totals_fall_back_to_the_standard_rates_without_a_calculation() {
        let events = vec![event(
            EventLabel::N,
            sale(
                U1,
                "2026-07-03T12:00:00+02:00",
                vec![
                    line("1", ("12", "Soft"), 1, &[("A", 62.04)]),
                    line("2", ("22", "Main"), 1, &[("B", 40.0)]),
                    line("3", ("30", "Takeaway"), 1, &[("C", 10.6)]),
                ],
                112.64,
                cash(112.64),
            ),
            Value::Null,
        )];
        let v = turnover(&events).vats;
        let got: Vec<_> = v
            .iter()
            .map(|x| {
                (
                    x.label,
                    x.rate,
                    x.taxable_amount,
                    x.vat_amount,
                    x.total_amount,
                )
            })
            .collect();
        assert_eq!(
            got,
            vec![
                (VatLabel::A, 21.0, 51.27, 10.77, 62.04),
                (VatLabel::B, 12.0, 35.71, 4.29, 40.0),
                (VatLabel::C, 6.0, 10.0, 0.6, 10.6),
            ]
        );
    }

    #[test]
    fn a_manual_and_an_automatic_meal_voucher_make_one_row() {
        let manual = json!([{ "id": "3", "name": "Meal voucher", "type": "CHEQUE_MEAL",
            "inputMethod": "MANUAL", "amount": 4.0, "amountType": "PAYMENT" }]);
        let automatic = json!([{ "id": "3", "name": "Meal voucher", "type": "CHEQUE_MEAL",
            "inputMethod": "AUTOMATIC", "provider": "Edenred", "reference": "T1",
            "amount": 9.5, "amountType": "PAYMENT" }]);
        let l = |p: f64| vec![line("1", ("10", "A"), 1, &[("A", p)])];
        let events = vec![
            event(
                EventLabel::N,
                sale(U1, "2026-07-03T12:00:00+02:00", l(4.0), 4.0, manual),
                Value::Null,
            ),
            event(
                EventLabel::N,
                sale(U1, "2026-07-03T12:10:00+02:00", l(9.5), 9.5, automatic),
                Value::Null,
            ),
        ];
        let rows = |p: &[PaymentTotalInput]| {
            p.iter()
                .map(|x| {
                    (
                        x.id.clone(),
                        x.name.clone(),
                        x.payment_type,
                        x.amount[0].normal_amount,
                    )
                })
                .collect::<Vec<_>>()
        };
        let expected = vec![(
            "3".to_string(),
            "Meal voucher".to_string(),
            PaymentType::ChequeMeal,
            13.5,
        )];
        assert_eq!(rows(&turnover(&events).payments), expected);
        let u = users(&events);
        assert_eq!(u.len(), 1);
        assert_eq!(rows(&u[0].payments), expected);
    }

    #[test]
    fn payments_are_grouped_per_method_and_refunds_reduce_them() {
        let card = json!([{ "id": "2", "name": "Carte", "type": "CARD_DEBIT", "amount": 3.04, "amountType": "PAYMENT" }]);
        let l = |p: f64| vec![line("1", ("10", "A"), 1, &[("A", p)])];
        let events = vec![
            event(
                EventLabel::N,
                sale(U1, "2026-07-03T12:00:00+02:00", l(52.0), 52.0, cash(52.0)),
                Value::Null,
            ),
            event(
                EventLabel::N,
                sale(
                    U1,
                    "2026-07-03T12:10:00+02:00",
                    l(-52.0),
                    -52.0,
                    cash(-52.0),
                ),
                Value::Null,
            ),
            event(
                EventLabel::N,
                sale(U2, "2026-07-03T12:20:00+02:00", l(3.04), 3.04, card),
                Value::Null,
            ),
            event(
                EventLabel::N,
                sale(U2, "2026-07-03T12:30:00+02:00", l(99.0), 99.0, cash(99.0)),
                Value::Null,
            ),
        ];
        let p = turnover(&events).payments;
        let got: Vec<_> = p
            .iter()
            .map(|x| {
                (
                    x.id.as_str(),
                    x.payment_type,
                    x.amount[0].normal_amount,
                    x.amount[0].total_amount,
                )
            })
            .collect();
        assert_eq!(
            got,
            vec![
                ("1", PaymentType::Cash, 99.0, 99.0),
                ("2", PaymentType::CardDebit, 3.04, 3.04),
            ]
        );
        assert!(p.iter().all(|x| x.amount[0].corrections_count == 0));
    }

    /// A payment correction F event: `lines` are (id, name, type,
    /// amount) of its financials.
    fn correction(user: &str, label: EventLabel, lines: &[(&str, &str, &str, f64)]) -> ReportEvent {
        let financials: Vec<Value> = lines
            .iter()
            .map(|(id, name, kind, amount)| {
                json!({ "id": id, "name": name, "type": kind, "inputMethod": "MANUAL",
                        "amount": amount, "amountType": "PAYMENT" })
            })
            .collect();
        event(
            label,
            json!({ "employeeId": user, "posDateTime": "2026-07-03T13:00:00+02:00",
                    "financials": financials }),
            signed("PAYMENT_CORRECTION", 8, Value::Null),
        )
    }

    #[test]
    fn a_payment_correction_moves_the_amount_between_methods() {
        let l = vec![line("1", ("10", "A"), 1, &[("A", 52.0)])];
        let events = vec![
            event(
                EventLabel::N,
                sale(U1, "2026-07-03T12:00:00+02:00", l, 52.0, cash(52.0)),
                signed("SALE", 1, Value::Null),
            ),
            correction(
                U2,
                EventLabel::F,
                &[
                    ("1", "Espèces", "CASH", -52.0),
                    ("2", "Card", "CARD_DEBIT", 52.0),
                ],
            ),
        ];
        let p = turnover(&events).payments;
        let got: Vec<_> = p
            .iter()
            .map(|x| {
                let a = &x.amount[0];
                (
                    x.id.as_str(),
                    a.normal_amount,
                    a.negative_corrections,
                    a.positive_corrections,
                    a.corrections_count,
                    a.total_amount,
                )
            })
            .collect();
        // Cash: 52 sold, 52 taken away. Card exists only through the
        // correction: normal 0, positive 52.
        assert_eq!(
            got,
            vec![
                ("1", 52.0, -52.0, 0.0, 1, 0.0),
                ("2", 0.0, 0.0, 52.0, 1, 52.0),
            ]
        );
    }

    #[test]
    fn corrections_add_up_per_method_with_a_count_per_line() {
        let events = vec![
            correction(
                U1,
                EventLabel::F,
                &[
                    ("1", "Cash", "CASH", -10.0),
                    ("2", "Card", "CARD_DEBIT", 10.0),
                ],
            ),
            correction(
                U1,
                EventLabel::F,
                &[
                    ("2", "Card", "CARD_DEBIT", -4.5),
                    ("1", "Cash", "CASH", 4.5),
                ],
            ),
        ];
        let p = turnover(&events).payments;
        let cash = &p[0].amount[0];
        assert_eq!(
            (
                cash.negative_corrections,
                cash.positive_corrections,
                cash.corrections_count,
                cash.total_amount
            ),
            (-10.0, 4.5, 2, -5.5)
        );
        let card = &p[1].amount[0];
        assert_eq!(
            (
                card.negative_corrections,
                card.positive_corrections,
                card.corrections_count,
                card.total_amount
            ),
            (-4.5, 10.0, 2, 5.5)
        );
    }

    #[test]
    fn a_training_correction_and_other_f_events_stay_out_of_the_payments() {
        let money = event(
            EventLabel::F,
            json!({ "employeeId": U1, "financials": [
                { "id": "MOV1", "name": "x", "type": "CASH", "amount": 20.0, "amountType": "MONEY_IN_OUT" }
            ] }),
            signed("MONEY_IN_OUT", 3, Value::Null),
        );
        let training = correction(
            U1,
            EventLabel::T,
            &[
                ("1", "Cash", "CASH", -5.0),
                ("2", "Card", "CARD_DEBIT", 5.0),
            ],
        );
        assert!(turnover(&[money, training]).payments.is_empty());
    }

    #[test]
    fn the_user_report_credits_a_correction_to_the_employee_who_signed_it() {
        let l = vec![line("1", ("10", "A"), 1, &[("A", 52.0)])];
        let events = vec![
            event(
                EventLabel::N,
                sale(U1, "2026-07-03T12:00:00+02:00", l, 52.0, cash(52.0)),
                signed("SALE", 1, Value::Null),
            ),
            correction(
                U2,
                EventLabel::F,
                &[
                    ("1", "Espèces", "CASH", -52.0),
                    ("2", "Card", "CARD_DEBIT", 52.0),
                ],
            ),
        ];
        let u = users(&events);
        let seller = u.iter().find(|x| x.employee_id == U1).unwrap();
        let manager = u.iter().find(|x| x.employee_id == U2).unwrap();
        assert_eq!(seller.payments.len(), 1);
        assert_eq!(seller.payments[0].amount[0].corrections_count, 0);
        assert_eq!(manager.payments.len(), 2);
        assert_eq!(manager.payments[0].amount[0].negative_corrections, -52.0);
        assert_eq!(manager.payments[1].amount[0].positive_corrections, 52.0);
        // The correction is no sale: the manager's turnover stays 0.
        assert_eq!(manager.total_amount, 0.0);
    }

    #[test]
    // covers: FAQ_Q303
    fn negative_lines_are_counted_per_reason() {
        let neg = |id: &str, qty: i64, price: f64, reason: &str| {
            let mut l = line(id, ("10", "A"), qty, &[("A", price)]);
            l["mainProduct"]["negQuantityReason"] = json!(reason);
            l
        };
        let events = vec![
            // A refund ticket with two refunded lines.
            event(
                EventLabel::N,
                sale(
                    U1,
                    "2026-07-03T12:00:00+02:00",
                    vec![neg("1", -2, -24.0, "REFUND"), neg("2", -1, -28.0, "REFUND")],
                    -52.0,
                    cash(-52.0),
                ),
                Value::Null,
            ),
            // A sale with an entry and its correction.
            event(
                EventLabel::N,
                sale(
                    U2,
                    "2026-07-03T12:30:00+02:00",
                    vec![
                        line("3", ("12", "Soft"), 1, &[("A", 3.0)]),
                        neg("3", -1, -3.0, "CORRECTION"),
                        line("4", ("12", "Soft"), 1, &[("A", 2.5)]),
                    ],
                    2.5,
                    cash(2.5),
                ),
                Value::Null,
            ),
            // A table correction is counted on its order registration.
            event(
                EventLabel::P,
                sale(
                    U2,
                    "2026-07-03T13:00:00+02:00",
                    vec![neg("5", -1, -4.0, "CORRECTION")],
                    -4.0,
                    Value::Null,
                ),
                json!({"eventOperation": "ORDER"}),
            ),
            // A pre-bill resends the entries: never counted.
            event(
                EventLabel::P,
                sale(
                    U2,
                    "2026-07-03T13:10:00+02:00",
                    vec![
                        line("5", ("12", "Soft"), 1, &[("A", 4.0)]),
                        neg("5", -1, -4.0, "CORRECTION"),
                    ],
                    0.0,
                    Value::Null,
                ),
                json!({"eventOperation": "PRE_BILL"}),
            ),
            // The table sale resends the registered correction, and one
            // registered in an earlier period (table open over a Z): both
            // were counted on their order, so neither counts here.
            ReportEvent {
                registered_negatives: vec![
                    neg("5", -1, -4.0, "CORRECTION"),
                    neg("6", -1, -6.0, "CORRECTION"),
                ],
                ..event(
                    EventLabel::N,
                    sale(
                        U2,
                        "2026-07-03T13:20:00+02:00",
                        vec![
                            line("5", ("12", "Soft"), 1, &[("A", 4.0)]),
                            neg("5", -1, -4.0, "CORRECTION"),
                            line("6", ("12", "Soft"), 1, &[("A", 6.0)]),
                            neg("6", -1, -6.0, "CORRECTION"),
                            line("7", ("12", "Soft"), 1, &[("A", 1.0)]),
                        ],
                        1.0,
                        cash(1.0),
                    ),
                    Value::Null,
                )
            },
        ];
        let n = turnover(&events).neg_quantities;
        let got: Vec<_> = n
            .iter()
            .map(|x| {
                (
                    x.neg_quantity_reason,
                    x.neg_quantity_count,
                    x.ticket_count,
                    x.amount,
                )
            })
            .collect();
        assert_eq!(
            got,
            vec![
                (NegQuantityReason::Correction, 2, 2, -7.0),
                (NegQuantityReason::Refund, 2, 1, -52.0),
            ]
        );
    }

    #[test]
    fn only_drawer_openings_count_as_drawers_opened() {
        let f = |op: &str| {
            event(
                EventLabel::F,
                json!({"employeeId": U1}),
                json!({"eventOperation": op}),
            )
        };
        let events = vec![f("DRAWER_OPEN"), f("MONEY_IN_OUT"), f("DRAWER_OPEN")];
        assert_eq!(turnover(&events).drawers_open_count, 2);
        // A training drawer opening is labelled T.
        let training = vec![event(
            EventLabel::T,
            json!({}),
            json!({"eventOperation": "DRAWER_OPEN"}),
        )];
        assert_eq!(turnover(&training).drawers_open_count, 0);
    }

    #[test]
    fn users_have_their_sales_payments_presence_and_activity() {
        let l = |p: f64| vec![line("1", ("10", "A"), 1, &[("A", p)])];
        let events = vec![
            event(
                EventLabel::S,
                json!({"employeeId": U2, "posDateTime": "2026-07-03T09:00:00+02:00"}),
                json!({"eventOperation": "WORK_IN"}),
            ),
            event(
                EventLabel::P,
                sale(U1, "2026-07-03T08:30:00+02:00", l(52.0), 52.0, Value::Null),
                Value::Null,
            ),
            event(
                EventLabel::N,
                sale(U1, "2026-07-03T08:40:00+02:00", l(52.0), 52.0, cash(52.0)),
                Value::Null,
            ),
            event(
                EventLabel::N,
                sale(
                    U1,
                    "2026-07-03T09:40:00+02:00",
                    l(-52.0),
                    -52.0,
                    cash(-52.0),
                ),
                Value::Null,
            ),
            event(
                EventLabel::N,
                sale(U2, "2026-07-03T12:57:00+02:00", l(99.0), 99.0, cash(99.0)),
                Value::Null,
            ),
            event(
                EventLabel::S,
                json!({"employeeId": U2, "posDateTime": "2026-07-03T13:30:00+02:00"}),
                json!({"eventOperation": "WORK_OUT"}),
            ),
            // The report itself is not activity of the user who runs it.
            event(
                EventLabel::R,
                json!({"employeeId": U1, "posDateTime": "2026-07-03T14:00:00+02:00"}),
                json!({"eventOperation": "REPORT_TURNOVER_X"}),
            ),
        ];
        let u = users(&events);
        assert_eq!(u.len(), 2);

        assert_eq!(u[0].employee_id, U1);
        assert_eq!(u[0].total_amount, 0.0);
        assert_eq!(u[0].first_pos_date_time, "2026-07-03T08:30:00+02:00");
        assert_eq!(u[0].last_pos_date_time, "2026-07-03T09:40:00+02:00");
        assert!(u[0].social_events.is_empty());
        assert_eq!(u[0].payments[0].amount[0].normal_amount, 0.0);

        assert_eq!(u[1].employee_id, U2);
        assert_eq!(u[1].total_amount, 99.0);
        assert_eq!(u[1].first_pos_date_time, "2026-07-03T09:00:00+02:00");
        assert_eq!(u[1].last_pos_date_time, "2026-07-03T13:30:00+02:00");
        let presence: Vec<_> = u[1].social_events.iter().map(|s| s.in_out).collect();
        assert_eq!(presence, vec![InOut::In, InOut::Out]);
    }

    #[test]
    fn activity_ranges_compare_instants_not_strings() {
        // 10:30+02:00 is before 09:00Z.
        let events = vec![
            event(
                EventLabel::S,
                json!({"employeeId": U1, "posDateTime": "2026-07-03T09:00:00Z"}),
                json!({"eventOperation": "WORK_OUT"}),
            ),
            event(
                EventLabel::S,
                json!({"employeeId": U1, "posDateTime": "2026-07-03T10:30:00+02:00"}),
                json!({"eventOperation": "WORK_IN"}),
            ),
        ];
        let u = &users(&events)[0];
        assert_eq!(u.first_pos_date_time, "2026-07-03T10:30:00+02:00");
        assert_eq!(u.last_pos_date_time, "2026-07-03T09:00:00Z");
    }

    #[test]
    fn devices_cover_every_terminal_and_fdm_range() {
        let at = |pos: &str, term: &str, t: &str| json!({"posId": pos, "terminalId": term, "employeeId": U1, "posDateTime": t});
        let events = vec![
            event(
                EventLabel::P,
                at("CPOS0031234567", "TER-2-DIN", "2026-07-03T11:45:00+02:00"),
                signed("ORDER", 53, Value::Null),
            ),
            event(
                EventLabel::P,
                at("CPOS0031234567", "TER-1-BAR", "2026-07-03T08:30:00+02:00"),
                signed("ORDER", 49, Value::Null),
            ),
            event(
                EventLabel::N,
                at("CPOS0031234568", "POS-2-REC", "2026-07-03T09:40:00+02:00"),
                signed("SALE", 51, Value::Null),
            ),
            event(
                EventLabel::N,
                at("CPOS0031234567", "TER-2-DIN", "2026-07-03T12:57:00+02:00"),
                signed("SALE", 56, Value::Null),
            ),
        ];
        let pos: Vec<_> = pos_devices(&events)
            .into_iter()
            .map(|d| {
                (
                    d.pos_id,
                    d.terminal_id.unwrap(),
                    d.first_pos_date_time,
                    d.last_pos_date_time,
                )
            })
            .collect();
        assert_eq!(
            pos,
            vec![
                (
                    "CPOS0031234567".into(),
                    "TER-1-BAR".into(),
                    "2026-07-03T08:30:00+02:00".into(),
                    "2026-07-03T08:30:00+02:00".into()
                ),
                (
                    "CPOS0031234567".into(),
                    "TER-2-DIN".into(),
                    "2026-07-03T11:45:00+02:00".into(),
                    "2026-07-03T12:57:00+02:00".into()
                ),
                (
                    "CPOS0031234568".into(),
                    "POS-2-REC".into(),
                    "2026-07-03T09:40:00+02:00".into(),
                    "2026-07-03T09:40:00+02:00".into()
                ),
            ]
        );
        let fdm = fdm_devices(&events);
        assert_eq!(fdm.len(), 1);
        assert_eq!(
            (fdm[0].first_total_counter, fdm[0].last_total_counter),
            (49, 56)
        );
        assert_eq!(fdm[0].first_fdm_date_time, "2026-07-03T10:49:00Z");
        assert_eq!(fdm[0].last_fdm_date_time, "2026-07-03T10:56:00Z");
    }

    #[test]
    fn unreadable_archive_rows_are_rejected() {
        let row = ArchivedEvent {
            id: 1,
            event_type: EventLabel::N,
            sequence_number: 1,
            order_id: None,
            pos_id: "P".into(),
            terminal_id: "T".into(),
            user_insz: U1.into(),
            event_data_json: "{not json".into(),
            fdm_response_json: None,
            fdm_id: None,
            fdm_datetime: None,
            event_label: None,
            short_signature: None,
            qr_data: None,
        };
        assert!(ReportEvent::from_archive(&row).is_none());
        let ok = ArchivedEvent {
            event_data_json: "{}".into(),
            ..row
        };
        let parsed = ReportEvent::from_archive(&ok).unwrap();
        assert!(parsed.response.is_null());
    }

    /// The UC223 sale (50 % ticket discount over a B and an A line),
    /// then a P carrying the same discount: only the sale counts in
    /// `priceChanges` and `departments`; both count in `transactions`.
    #[test]
    fn price_changes_net_the_amounts_and_are_summed_per_label() {
        let discounted = |product: &str, dept: (&str, &str), label: &str, price: f64, off: f64| {
            let mut l = line(product, dept, 1, &[(label, price)]);
            l["mainProduct"]["vats"][0]["priceChanges"] = json!([
                {"id": "10", "name": "Discount50P", "scope": "EVENT", "type": "PUBLIC", "amount": off, "groupingId": 1},
                {"id": "99", "name": "Recalc", "scope": "LINE", "type": "INTERNAL", "amount": 0.0, "groupingId": 2},
            ]);
            l
        };
        let lines = vec![
            discounted("20001", ("20", "Starters"), "B", 40.0, -20.0),
            discounted("10006", ("10", "Aperitifs"), "A", 12.0, -6.0),
        ];
        let tx = json!({ "transaction": { "transactionLines": lines, "transactionTotal": 26.0 } });
        let events = vec![
            event(EventLabel::N, tx.clone(), json!({})),
            event(EventLabel::P, tx, json!({})),
        ];
        let totals = turnover(&events);

        let n = totals
            .transactions
            .iter()
            .find(|t| t.event_label == EventLabel::N)
            .unwrap();
        assert_eq!(n.amount, 26.0);
        let p = totals
            .transactions
            .iter()
            .find(|t| t.event_label == EventLabel::P)
            .unwrap();
        assert_eq!(p.amount, 26.0);
        let starters = totals
            .departments
            .iter()
            .find(|d| d.department_id == "20")
            .unwrap();
        assert_eq!(starters.amount, 20.0);

        assert_eq!(
            totals.price_changes.len(),
            1,
            "internal changes are left out"
        );
        let change = &totals.price_changes[0];
        assert_eq!(
            (change.id.as_str(), change.name.as_str()),
            ("10", "Discount50P")
        );
        let by_label: Vec<_> = change
            .amount
            .iter()
            .map(|a| (a.label, a.negative, a.positive))
            .collect();
        assert_eq!(
            by_label,
            vec![(VatLabel::A, -6.0, 0.0), (VatLabel::B, -20.0, 0.0)]
        );
    }

    #[test]
    fn invoices_of_the_period_are_listed() {
        let none = turnover(&[]);
        assert_eq!(none.invoices.as_deref().map(<[_]>::len), Some(0));

        let invoice = ReportEvent {
            invoice_amount: Some(54.5),
            ..event(
                EventLabel::I,
                json!({"invoiceNo": "20240005", "fdmRefs": []}),
                Value::Null,
            )
        };
        let report = turnover(&[invoice]);
        let invoices = report.invoices.unwrap();
        assert_eq!(invoices.len(), 1);
        assert_eq!(invoices[0].invoice_no, "20240005");
        assert_eq!(invoices[0].amount, 54.5);
        // An I event has no transaction: never counted as a sale.
        assert!(report.transactions.is_empty());
    }
}
