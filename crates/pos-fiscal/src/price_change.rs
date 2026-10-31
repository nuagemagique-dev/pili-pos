//! Price changes on transaction lines (POS spec v2.2, `priceChange`;
//! Advanced Set UC220 to UC225): a discount (negative) or a surcharge
//! (positive) on a line's VAT entry. `vats[].price` stays the price
//! before the change; `lineTotal` and `transactionTotal` are after it.
//!
//! The POS keeps its lines as they are and passes their changes beside
//! them, keyed by the id of each [`OrderLine`] given to the event
//! builder (an entry's id for the journal, a line's for the
//! consolidated order). With no changes every event is unchanged.

use std::collections::BTreeMap;

use pos_core::order::OrderLine;
use serde::{Deserialize, Serialize};

use crate::inputs::{PriceChangeInput, TransactionInput};
use crate::ticket::TicketLine;
use crate::types::{PriceChangeScope, PriceChangeType};

/// One price change on one line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineChange {
    /// Groups a change applied to several lines at once; successive
    /// changes increment it (producers FAQ Q304).
    pub grouping_id: i32,
    /// The change's id in the POS, whatever the language.
    pub id: String,
    /// As the customer sees it: "Promo -10 %".
    pub name: String,
    pub scope: PriceChangeScope,
    pub change_type: PriceChangeType,
    /// Negative for a discount.
    pub amount_cents: i64,
}

impl LineChange {
    /// The same change, the other way: what a correction or a refund
    /// of its line carries.
    #[must_use]
    pub fn negated(&self) -> Self {
        Self {
            amount_cents: -self.amount_cents,
            ..self.clone()
        }
    }
}

/// The changes of each line, by the line's id.
pub type LineChanges = BTreeMap<i64, Vec<LineChange>>;

use pos_core::money::eur;

/// Σ of a line's changes, in cents.
pub fn line_change_cents(changes: &LineChanges, line_id: i64) -> i64 {
    changes
        .get(&line_id)
        .map_or(0, |c| c.iter().map(|c| c.amount_cents).sum())
}

/// A line's total after its changes, in cents.
pub fn net_line_cents(line: &OrderLine, changes: &LineChanges) -> i64 {
    line.line_total() + line_change_cents(changes, line.id)
}

/// The lines' total after their changes, in cents.
pub fn net_total_cents(lines: &[OrderLine], changes: &LineChanges) -> i64 {
    lines.iter().map(|l| net_line_cents(l, changes)).sum()
}

/// Put the changes on the transaction built from `lines` (same lines,
/// same order): each line's VAT entry carries its changes, its
/// `lineTotal` and the `transactionTotal` are after them.
pub fn apply(transaction: &mut TransactionInput, lines: &[OrderLine], changes: &LineChanges) {
    if changes.is_empty() {
        return;
    }
    debug_assert_eq!(transaction.transaction_lines.len(), lines.len());
    let mut total = 0;
    for (tx_line, line) in transaction.transaction_lines.iter_mut().zip(lines) {
        let line_changes = changes.get(&line.id).filter(|c| !c.is_empty());
        if let Some(line_changes) = line_changes {
            // A single-product line has one VAT entry.
            if let Some(vat) = tx_line.main_product.vats.first_mut() {
                vat.price_changes = Some(
                    line_changes
                        .iter()
                        .map(|c| PriceChangeInput {
                            grouping_id: Some(c.grouping_id),
                            id: c.id.clone(),
                            name: c.name.clone(),
                            scope: c.scope,
                            price_change_type: c.change_type,
                            amount: eur(c.amount_cents),
                        })
                        .collect(),
                );
            }
        }
        let net = net_line_cents(line, changes);
        tx_line.line_total = eur(net);
        total += net;
    }
    transaction.transaction_total = eur(total);
}

/// The ticket rows of the lines' public changes, after each line's own
/// row: "Promo -10 %  -2,00  C". Internal recalculations are not shown.
pub fn ticket_rows(line: &OrderLine, changes: &LineChanges) -> Vec<TicketLine> {
    changes
        .get(&line.id)
        .into_iter()
        .flatten()
        .filter(|c| c.change_type == PriceChangeType::Public)
        .map(|c| TicketLine {
            quantity: 1.0,
            description: c.name.clone(),
            unit_price: eur(c.amount_cents),
            total: eur(c.amount_cents),
            vat_code: line.vat_code.as_str().to_string(),
            weighed: false,
        })
        .collect()
}

/// The ticket rows of `lines`: each line, then its public changes.
pub fn ticket_lines(lines: &[OrderLine], changes: &LineChanges) -> Vec<TicketLine> {
    let mut rows = Vec::with_capacity(lines.len());
    for l in lines {
        rows.push(TicketLine {
            quantity: l.fiscal_quantity(),
            description: l.name.clone(),
            unit_price: eur(l.unit_price_cents),
            total: eur(l.line_total()),
            vat_code: l.vat_code.as_str().to_string(),
            weighed: l.is_weighed(),
        });
        rows.extend(ticket_rows(l, changes));
    }
    rows
}

pub use pos_core::money::distribute;

// ── Discounts taken at the till ────────────────────────────────

/// Change id of an order discount in percent or amount.
pub const TILL_DISCOUNT_ID: &str = "TILL-DISCOUNT";
/// Change id of an order given for free ("Offert", 100 %).
pub const TILL_FREE_ID: &str = "TILL-FREE";
/// Change id of one line given for free (scope LINE, UC229B).
pub const TILL_FREE_LINE_ID: &str = "TILL-FREE-LINE";

/// Whether `change_id` is one of the till's discount ids.
pub fn is_till_change(change_id: &str) -> bool {
    matches!(
        change_id,
        TILL_DISCOUNT_ID | TILL_FREE_ID | TILL_FREE_LINE_ID
    )
}

/// The customer-visible name of a till change, in the establishment's
/// receipt language (`fr`, `nl`, `de`; `en` for completeness, the
/// receipt language is never English). A percent discount says its rate
/// ("Remise 10 %"); an amount discount is "Remise"; both kinds of free
/// are "Offert".
pub fn till_change_name(lang: &str, change_id: &str, percent: Option<i64>) -> String {
    let (discount, free) = match lang {
        "nl" => ("Korting", "Gratis"),
        "de" => ("Rabatt", "Gratis"),
        "en" => ("Discount", "Free"),
        _ => ("Remise", "Offert"),
    };
    match (change_id, percent) {
        (TILL_DISCOUNT_ID, Some(p)) => format!("{discount} {p} %"),
        (TILL_DISCOUNT_ID, None) => discount.to_string(),
        _ => free.to_string(),
    }
}

/// Mark the negative lines of a `PRICE_CHANGE` pair: the correction that
/// takes a line already transmitted back before its re-entry carries
/// the new price (POS spec v2.2 `negQuantityReason`, FAQ Q303).
/// `entry_ids` are the journal entries marked `till_op` = `price_change`;
/// `entries` are the lines the transaction was built from, in order.
pub fn mark_price_change_reason(
    transaction: &mut TransactionInput,
    entries: &[OrderLine],
    entry_ids: &std::collections::BTreeSet<i64>,
) {
    if entry_ids.is_empty() {
        return;
    }
    debug_assert_eq!(transaction.transaction_lines.len(), entries.len());
    for (tx_line, entry) in transaction.transaction_lines.iter_mut().zip(entries) {
        if entry.quantity < 0 && entry_ids.contains(&entry.id) {
            tx_line.main_product.neg_quantity_reason =
                Some(crate::types::NegQuantityReason::PriceChange);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use pos_core::vat::VatCode;

    fn line(id: i64, qty: i32, unit: i64, vat: VatCode) -> OrderLine {
        OrderLine {
            id,
            order_id: 1,
            menu_item_id: id,
            quantity: qty,
            unit_price_cents: unit,
            vat_code: vat,
            name: format!("P{id}"),
            modifiers_json: None,
            created_at: Utc::now(),
            department_id: "1".into(),
            department_name: "D".into(),
            weight_grams: None,
            weighed_amount_cents: None,
        }
    }

    fn change(amount_cents: i64) -> LineChange {
        LineChange {
            grouping_id: 1,
            id: "10".into(),
            name: "Discount50P".into(),
            scope: PriceChangeScope::Event,
            change_type: PriceChangeType::Public,
            amount_cents,
        }
    }

    #[test]
    fn till_change_names_follow_the_receipt_language() {
        assert_eq!(
            till_change_name("fr", TILL_DISCOUNT_ID, Some(10)),
            "Remise 10 %"
        );
        assert_eq!(till_change_name("fr", TILL_DISCOUNT_ID, None), "Remise");
        assert_eq!(till_change_name("fr", TILL_FREE_ID, None), "Offert");
        assert_eq!(till_change_name("fr", TILL_FREE_LINE_ID, None), "Offert");
        assert_eq!(
            till_change_name("nl", TILL_DISCOUNT_ID, Some(5)),
            "Korting 5 %"
        );
        assert_eq!(till_change_name("nl", TILL_FREE_ID, None), "Gratis");
        assert_eq!(till_change_name("de", TILL_DISCOUNT_ID, None), "Rabatt");
        assert_eq!(till_change_name("de", TILL_FREE_LINE_ID, None), "Gratis");
        assert_eq!(
            till_change_name("en", TILL_DISCOUNT_ID, Some(20)),
            "Discount 20 %"
        );
        assert_eq!(till_change_name("en", TILL_FREE_ID, None), "Free");
        // A language the receipt cannot be in falls back to French.
        assert_eq!(till_change_name("zh", TILL_FREE_ID, None), "Offert");
        assert!(is_till_change(TILL_FREE_LINE_ID) && !is_till_change("10"));
    }

    #[test]
    fn only_the_negative_lines_of_a_pair_get_the_price_change_reason() {
        let entries = [
            line(1, 2, 2000, VatCode::B),
            line(2, -2, 2000, VatCode::B),
            line(3, 2, 2000, VatCode::B),
            line(4, -1, 2000, VatCode::B),
        ];
        let mut tx = crate::event::build_transaction_for_tests(&entries);
        let ids = std::collections::BTreeSet::from([2, 3]);
        mark_price_change_reason(&mut tx, &entries, &ids);
        let reasons: Vec<_> = tx
            .transaction_lines
            .iter()
            .map(|l| l.main_product.neg_quantity_reason)
            .collect();
        assert_eq!(
            reasons,
            vec![
                None,
                Some(crate::types::NegQuantityReason::PriceChange),
                None,
                Some(crate::types::NegQuantityReason::Correction),
            ]
        );
    }

    #[test]
    fn negated_flips_only_the_amount() {
        let c = change(-250);
        assert_eq!(c.negated().amount_cents, 250);
        assert_eq!(c.negated().id, c.id);
    }

    #[test]
    fn ticket_rows_show_public_changes_only() {
        let l = line(7, 2, 2000, VatCode::B);
        let mut internal = change(-100);
        internal.change_type = PriceChangeType::Internal;
        let changes = LineChanges::from([(7, vec![change(-2000), internal])]);
        let rows = ticket_rows(&l, &changes);
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].total, rows[0].vat_code.as_str()), (-20.0, "B"));
        assert_eq!(net_line_cents(&l, &changes), 4000 - 2100);
    }
}
