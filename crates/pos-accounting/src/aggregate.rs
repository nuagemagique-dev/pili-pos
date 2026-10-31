//! One booking period ("day") of signed data and its totals.
//!
//! A [`Day`] holds the sales, invoices and cash movements of a booking period closed
//! by a Z report, as the signed requests and FDM answers say them, and the Z itself.
//! The totals are sums of integer cents; nothing is estimated.

use std::collections::BTreeMap;

use crate::error::Result;
use crate::parse::{
    Channel, Invoice, MoneyMove, RawEvent, Sale, ZReport, parse_invoice, parse_money, parse_sale,
    parse_z,
};
use crate::source::{ClosedPeriod, Source};

/// How a payment is classified in the files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Method {
    Cash,
    Card,
    MealVoucher,
    GiftVoucher,
    App,
    Online,
    Other,
}

impl Method {
    /// Method of a `PaymentType` of the specification.
    pub fn from_type(payment_type: &str) -> Self {
        match payment_type {
            "CASH" => Self::Cash,
            "CHEQUE_MEAL" => Self::MealVoucher,
            "VOUCHER_STORE" => Self::GiftVoucher,
            "APP" => Self::App,
            "ONLINE" => Self::Online,
            t if t.starts_with("CARD") => Self::Card,
            _ => Self::Other,
        }
    }

    /// Value written in the CSV files.
    pub fn code(self) -> &'static str {
        match self {
            Self::Cash => "especes",
            Self::Card => "carte",
            Self::MealVoucher => "cheques_repas",
            Self::GiftVoucher => "bon_cadeau",
            Self::App => "application",
            Self::Online => "en_ligne",
            Self::Other => "autre",
        }
    }
}

/// `payment`, `rounding` or `tip`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Payment,
    Rounding,
    Tip,
}

impl Kind {
    pub fn from_amount_type(amount_type: &str) -> Self {
        match amount_type {
            "ROUNDING" => Self::Rounding,
            "TIP" => Self::Tip,
            _ => Self::Payment,
        }
    }
}

/// Base, VAT and total of a group of tickets, in cents.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Amounts {
    pub base: i64,
    pub vat: i64,
    pub total: i64,
}

/// An invoice with the VAT of the tickets it lists.
#[derive(Debug, Clone)]
pub struct InvoiceDetail {
    pub invoice: Invoice,
    /// Σ of the listed tickets' VAT per label.
    pub vats: BTreeMap<String, Amounts>,
    /// `N<eventCounter>` of each listed ticket (as signed in the reference).
    pub tickets: Vec<String>,
    /// Listed tickets not found in the archive.
    pub unresolved: usize,
}

/// A closed booking period.
#[derive(Debug, Clone)]
pub struct Day {
    pub booking_date: String,
    pub z_report_no: i64,
    pub z: ZReport,
    pub sales: Vec<Sale>,
    pub invoices: Vec<InvoiceDetail>,
    pub moves: Vec<MoneyMove>,
}

/// Read the signed data of one closed period.
pub fn load_day(source: &Source, period: &ClosedPeriod) -> Result<Day> {
    let z = parse_z(&period.z_event)?;
    let mut sales = Vec::new();
    let mut invoices = Vec::new();
    let mut moves = Vec::new();
    for event in source.events(&period.period_id)? {
        match event.label.as_str() {
            "N" => {
                // A refund without a cost center of its own takes the channel of its ticket.
                let original = match event.request.get("fdmRef").filter(|v| v.is_object()) {
                    Some(fdm_ref) if !event.request["costCenter"].is_object() => {
                        source.find_by_fdm_ref(fdm_ref)?
                    }
                    _ => None,
                };
                sales.push(parse_sale(&event, original.as_ref())?);
            }
            "I" => invoices.push(invoice_detail(source, &event)?),
            "F" => moves.extend(parse_money(&event)?),
            _ => {}
        }
    }
    Ok(Day {
        booking_date: period.booking_date.clone(),
        z_report_no: period.z_report_no,
        z,
        sales,
        invoices,
        moves,
    })
}

fn invoice_detail(source: &Source, event: &RawEvent) -> Result<InvoiceDetail> {
    let invoice = parse_invoice(event)?;
    let mut vats: BTreeMap<String, Amounts> = BTreeMap::new();
    let mut tickets = Vec::new();
    let mut unresolved = 0;
    for fdm_ref in &invoice.refs {
        tickets.push(format!(
            "{}{}",
            fdm_ref["eventLabel"].as_str().unwrap_or("N"),
            fdm_ref["eventCounter"].as_i64().unwrap_or(0)
        ));
        match source.find_by_fdm_ref(fdm_ref)? {
            Some(ticket) => {
                let sale = parse_sale(&ticket, None)?;
                for v in sale.vats {
                    let entry = vats.entry(v.label).or_default();
                    entry.base += v.base;
                    entry.vat += v.vat;
                    entry.total += v.total;
                }
            }
            None => unresolved += 1,
        }
    }
    Ok(InvoiceDetail {
        invoice,
        vats,
        tickets,
        unresolved,
    })
}

impl Day {
    /// Sale tickets that are not refunds.
    pub fn tickets(&self) -> usize {
        self.sales.iter().filter(|s| !s.refund).count()
    }

    pub fn refunds(&self) -> usize {
        self.sales.iter().filter(|s| s.refund).count()
    }

    /// Σ of the lines after price changes, refunds negative: the Z report's turnover.
    pub fn turnover(&self) -> i64 {
        self.sales.iter().map(Sale::lines_total).sum()
    }

    /// The FDM's VAT per channel and label.
    pub fn vat_by_channel(&self) -> BTreeMap<(Channel, String), Amounts> {
        let mut out: BTreeMap<(Channel, String), Amounts> = BTreeMap::new();
        for sale in &self.sales {
            for v in &sale.vats {
                let entry = out.entry((sale.channel, v.label.clone())).or_default();
                entry.base += v.base;
                entry.vat += v.vat;
                entry.total += v.total;
            }
        }
        out
    }

    /// The FDM's VAT per label, all channels.
    pub fn vat_by_label(&self) -> BTreeMap<String, Amounts> {
        let mut out: BTreeMap<String, Amounts> = BTreeMap::new();
        for ((_, label), a) in self.vat_by_channel() {
            let entry = out.entry(label).or_default();
            entry.base += a.base;
            entry.vat += a.vat;
            entry.total += a.total;
        }
        out
    }

    /// Payments per method and kind.
    pub fn payments_by_method(&self) -> BTreeMap<(Method, Kind), i64> {
        let mut out = BTreeMap::new();
        for p in self.sales.iter().flat_map(|s| s.payments.iter()) {
            *out.entry((
                Method::from_type(&p.payment_type),
                Kind::from_amount_type(&p.amount_type),
            ))
            .or_insert(0) += p.cents;
        }
        out
    }

    /// Payments per `(PaymentType, amountType)`, as the Z report groups them.
    pub fn payments_by_type(&self) -> BTreeMap<(String, String), i64> {
        let mut out = BTreeMap::new();
        for p in self.sales.iter().flat_map(|s| s.payments.iter()) {
            *out.entry((p.payment_type.clone(), p.amount_type.clone()))
                .or_insert(0) += p.cents;
        }
        out
    }

    /// Gift vouchers sold (VAT X lines of department `GV`), refunds negative.
    pub fn gift_voucher_sold(&self) -> i64 {
        self.sales.iter().map(Sale::gift_voucher_sold).sum()
    }

    /// Sales not collected: the part of the sale totals no payment line covers.
    pub fn unpaid(&self) -> i64 {
        self.sales.iter().map(|s| s.vat_total() - s.paid()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_types_map_to_methods() {
        assert_eq!(Method::from_type("CASH"), Method::Cash);
        assert_eq!(Method::from_type("CARD_DEBIT"), Method::Card);
        assert_eq!(Method::from_type("CARD_UNKNOWN"), Method::Card);
        assert_eq!(Method::from_type("CHEQUE_MEAL"), Method::MealVoucher);
        assert_eq!(Method::from_type("VOUCHER_STORE"), Method::GiftVoucher);
        assert_eq!(Method::from_type("APP"), Method::App);
        assert_eq!(Method::from_type("ONLINE"), Method::Online);
        assert_eq!(Method::from_type("CHEQUE_OTHER"), Method::Other);
    }

    #[test]
    fn amount_types_map_to_kinds() {
        assert_eq!(Kind::from_amount_type("ROUNDING"), Kind::Rounding);
        assert_eq!(Kind::from_amount_type("TIP"), Kind::Tip);
        assert_eq!(Kind::from_amount_type("PAYMENT"), Kind::Payment);
    }
}
