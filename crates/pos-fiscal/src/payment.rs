//! How a sale is paid: the parts the operator takes (card, meal
//! vouchers, cash), settled into the payment lines sent to the FDM.
//!
//! Rules (POS description v2.2 §2.2.2 `financials`, certification
//! manual "Arrondissement des méthodes de paiement", producers FAQ
//! Q402 and Q403):
//! - a card or meal voucher part is an exact amount, never rounded, and
//!   no change is given on it;
//! - cash pays what is left; the 5-cent rounding applies to that rest
//!   only and goes on its own ROUNDING line;
//! - the payments add up to the sale total, the rounding apart;
//! - a sale of 0,00 EUR is a cash payment of 0 (FAQ Q206);
//! - a card part taken on a linked terminal (`eft_payment_id`) carries
//!   what the terminal returned ([`TerminalCard`]): the payment line is
//!   `AUTOMATIC` (spec v2.2, `inputMethod`), of the card type the
//!   terminal gave, with the brand as `provider`. It is an exact amount
//!   like any card part;
//! - a multi-purpose gift voucher part (`VOUCHER_STORE`, spec v2.2 §5
//!   and `tb_paymentTypes`) is an exact amount like a meal voucher: never
//!   rounded, no change given on it, never above what is left to pay. It
//!   names the voucher it redeems (`gift_voucher_id`).

use serde::{Deserialize, Serialize};

use crate::ticket::round_to_5_cents;
use crate::types::PaymentType;

/// A payment method the operator can take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    Cash,
    Card,
    /// Electronic meal vouchers, paid on the card terminal.
    MealVoucher,
    /// A multi-purpose gift voucher sold by the restaurant (`VOUCHER_STORE`).
    GiftVoucher,
    /// Paid in an ordering platform's app (an online order signed at
    /// acceptance). Never offered at the till.
    App,
    /// Paid online on the restaurant's own webshop. Never offered at
    /// the till.
    Online,
}

impl PaymentMethod {
    pub fn payment_type(self) -> PaymentType {
        match self {
            PaymentMethod::Cash => PaymentType::Cash,
            PaymentMethod::Card => PaymentType::CardDebit,
            PaymentMethod::MealVoucher => PaymentType::ChequeMeal,
            PaymentMethod::GiftVoucher => PaymentType::VoucherStore,
            PaymentMethod::App => PaymentType::App,
            PaymentMethod::Online => PaymentType::Online,
        }
    }

    /// Whether the operator can take it at the till. App and online
    /// payments only come with online orders.
    pub fn is_offered_at_till(self) -> bool {
        matches!(
            self,
            PaymentMethod::Cash
                | PaymentMethod::Card
                | PaymentMethod::MealVoucher
                | PaymentMethod::GiftVoucher
        )
    }
}

/// The `id` and `name` of a payment line (spec §2.2.2
/// `financials.id`). The reports total the payments per id, name and
/// type, so each method keeps the same pair on every event.
///
/// Cards: a card typed by hand is `CARD_DEBIT` ("2", "Card", `MANUAL`).
/// A linked terminal returns its own card type; a debit card from the
/// terminal is the same method as the manual one ("2", "Card": the Z
/// report counts both in one row, they differ by `inputMethod` only),
/// while credit and unspecified cards get a pair of their own, so the
/// reports (which count the lines of a payment by id and name) never
/// mix two card types in one count.
pub fn payment_line_identity(payment_type: PaymentType) -> (&'static str, &'static str) {
    match payment_type {
        PaymentType::Cash => ("1", "Cash"),
        PaymentType::CardDebit => ("2", "Card"),
        PaymentType::CardCredit => ("6", "Credit card"),
        PaymentType::CardUnknown => ("7", "Card (type unknown)"),
        PaymentType::ChequeMeal => ("3", "Meal voucher"),
        PaymentType::App => ("4", "App"),
        PaymentType::Online => ("5", "Online"),
        PaymentType::VoucherStore => ("8", "Gift voucher"),
        _ => ("9", "Payment"),
    }
}

/// One part of a payment as the operator takes it. A cash part has no
/// amount: it pays the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentPart {
    pub method: PaymentMethod,
    #[serde(default)]
    pub amount_cents: Option<i64>,
    /// The card payment the linked terminal approved for this part
    /// (`eft_payments.id`). Absent: the card was typed by hand on a
    /// standalone terminal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eft_payment_id: Option<i64>,
    /// The gift voucher this part redeems (`gift_vouchers.id`), from
    /// `lookup_gift_voucher`. Only a gift voucher part names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gift_voucher_id: Option<i64>,
}

impl PaymentPart {
    pub fn cash() -> Self {
        Self {
            method: PaymentMethod::Cash,
            amount_cents: None,
            eft_payment_id: None,
            gift_voucher_id: None,
        }
    }

    pub fn of(method: PaymentMethod, amount_cents: i64) -> Self {
        Self {
            method,
            amount_cents: Some(amount_cents),
            eft_payment_id: None,
            gift_voucher_id: None,
        }
    }

    /// A gift voucher part redeeming `amount_cents` of voucher `voucher_id`.
    pub fn gift_voucher(amount_cents: i64, voucher_id: i64) -> Self {
        Self {
            gift_voucher_id: Some(voucher_id),
            ..Self::of(PaymentMethod::GiftVoucher, amount_cents)
        }
    }

    /// A card part settling the terminal payment `eft_payment_id`.
    pub fn terminal(amount_cents: i64, eft_payment_id: i64) -> Self {
        Self {
            eft_payment_id: Some(eft_payment_id),
            ..Self::of(PaymentMethod::Card, amount_cents)
        }
    }
}

/// What a linked terminal returned for a card payment, as it goes on
/// the payment line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCard {
    /// `CARD_DEBIT`, `CARD_CREDIT` or `CARD_UNKNOWN`.
    pub card_type: PaymentType,
    /// The card brand: the payment line's `provider`.
    pub brand: Option<String>,
    /// The terminal's transaction id: the payment line's `reference`.
    pub reference: Option<String>,
}

/// A settled payment line: the amount this method pays of the sale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettledPayment {
    pub method: PaymentMethod,
    pub amount_cents: i64,
    /// The terminal payment it settles, from [`PaymentPart::eft_payment_id`].
    pub eft_payment_id: Option<i64>,
    /// The gift voucher it redeems, from [`PaymentPart::gift_voucher_id`].
    pub gift_voucher_id: Option<i64>,
    /// What that terminal payment returned; the caller fills it in
    /// (this module knows nothing of the database).
    pub terminal: Option<TerminalCard>,
}

impl SettledPayment {
    /// The payment type of its line: the method's, or, for a card the
    /// linked terminal took, the card type it returned (a terminal card
    /// type that is not a card type is `CARD_UNKNOWN`).
    pub fn payment_type(&self) -> PaymentType {
        match (&self.terminal, self.method) {
            (Some(t), PaymentMethod::Card) => match t.card_type {
                PaymentType::CardDebit | PaymentType::CardCredit => t.card_type,
                _ => PaymentType::CardUnknown,
            },
            _ => self.method.payment_type(),
        }
    }

    /// What the terminal returned, for a card it took.
    pub fn terminal_card(&self) -> Option<&TerminalCard> {
        self.terminal
            .as_ref()
            .filter(|_| self.method == PaymentMethod::Card)
    }

    fn plain(method: PaymentMethod, amount_cents: i64) -> Self {
        Self {
            method,
            amount_cents,
            eft_payment_id: None,
            gift_voucher_id: None,
            terminal: None,
        }
    }
}

/// How a sale is settled: its payment lines (non-cash parts first, in
/// the order taken, then cash) and the cash rounding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settlement {
    pub payments: Vec<SettledPayment>,
    /// Delta added to the cash rest (negative when rounded down).
    pub rounding_cents: i64,
}

impl Settlement {
    /// One method paying the whole total (cash is rounded).
    pub fn single(method: PaymentMethod, total_cents: i64) -> Self {
        let part = match method {
            PaymentMethod::Cash => PaymentPart::cash(),
            other => PaymentPart::of(other, total_cents),
        };
        settle(total_cents, &[part]).unwrap_or_else(|_| Settlement {
            payments: vec![SettledPayment::plain(method, total_cents)],
            rounding_cents: 0,
        })
    }

    /// True when part of the sale is paid in cash (the drawer opens).
    pub fn has_cash(&self) -> bool {
        self.payments
            .iter()
            .any(|p| p.method == PaymentMethod::Cash && p.amount_cents != 0)
    }

    /// The cash amount to hand over, rounding included.
    pub fn cash_due_cents(&self) -> i64 {
        self.payments
            .iter()
            .filter(|p| p.method == PaymentMethod::Cash)
            .map(|p| p.amount_cents)
            .sum::<i64>()
            + self.rounding_cents
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentError {
    /// A part without an amount, a zero or negative amount, a cash part
    /// with an amount, or two cash parts.
    Invalid,
    /// The card and meal voucher parts come to more than the total.
    ExceedsTotal,
    /// Something is left to pay and no cash part pays it.
    Incomplete,
}

/// Settle `total_cents` with the operator's `parts`.
pub fn settle(total_cents: i64, parts: &[PaymentPart]) -> Result<Settlement, PaymentError> {
    if total_cents == 0 {
        return Ok(Settlement {
            payments: vec![SettledPayment::plain(PaymentMethod::Cash, 0)],
            rounding_cents: 0,
        });
    }
    if total_cents < 0 || parts.is_empty() {
        return Err(if total_cents < 0 {
            PaymentError::Invalid
        } else {
            PaymentError::Incomplete
        });
    }

    let mut payments = Vec::new();
    let mut cash = false;
    let mut non_cash = 0_i64;
    for part in parts {
        match (part.method, part.amount_cents) {
            (PaymentMethod::Cash, None)
                if !cash && part.eft_payment_id.is_none() && part.gift_voucher_id.is_none() =>
            {
                cash = true
            }
            (PaymentMethod::Cash, _) => return Err(PaymentError::Invalid),
            // A gift voucher part names its voucher; no other part does.
            (method, Some(amount))
                if amount > 0
                    && (method == PaymentMethod::GiftVoucher) == part.gift_voucher_id.is_some() =>
            {
                // Checked per part so the sum cannot wrap past the total.
                non_cash = non_cash
                    .checked_add(amount)
                    .filter(|sum| *sum <= total_cents)
                    .ok_or(PaymentError::ExceedsTotal)?;
                payments.push(SettledPayment {
                    eft_payment_id: part.eft_payment_id,
                    gift_voucher_id: part.gift_voucher_id,
                    ..SettledPayment::plain(method, amount)
                });
            }
            _ => return Err(PaymentError::Invalid),
        }
    }
    let rest = total_cents - non_cash;
    let mut rounding_cents = 0;
    if rest > 0 {
        if !cash {
            return Err(PaymentError::Incomplete);
        }
        rounding_cents = round_to_5_cents(rest);
        payments.push(SettledPayment::plain(PaymentMethod::Cash, rest));
    }
    Ok(Settlement {
        payments,
        rounding_cents,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use PaymentMethod::*;

    fn lines(s: &Settlement) -> Vec<(PaymentMethod, i64)> {
        s.payments
            .iter()
            .map(|p| (p.method, p.amount_cents))
            .collect()
    }

    #[test]
    fn cash_alone_is_rounded() {
        let s = settle(1152, &[PaymentPart::cash()]).unwrap();
        assert_eq!(lines(&s), [(Cash, 1152)]);
        assert_eq!(s.rounding_cents, -2);
        assert_eq!(s.cash_due_cents(), 1150);
    }

    #[test]
    fn a_card_is_never_rounded() {
        let s = settle(1152, &[PaymentPart::of(Card, 1152)]).unwrap();
        assert_eq!(lines(&s), [(Card, 1152)]);
        assert_eq!(s.rounding_cents, 0);
        assert!(!s.has_cash());
    }

    // covers: ROUND4
    #[test]
    fn round4_meal_vouchers_then_cash_rounds_the_cash_rest_only() {
        let s = settle(
            1486,
            &[PaymentPart::of(MealVoucher, 1152), PaymentPart::cash()],
        )
        .unwrap();
        assert_eq!(lines(&s), [(MealVoucher, 1152), (Cash, 334)]);
        assert_eq!(s.rounding_cents, 1);
        assert_eq!(s.cash_due_cents(), 335);
    }

    // covers: ROUND5
    #[test]
    fn round5_meal_vouchers_card_and_cash() {
        let s = settle(
            1486,
            &[
                PaymentPart::of(MealVoucher, 313),
                PaymentPart::of(Card, 397),
                PaymentPart::cash(),
            ],
        )
        .unwrap();
        assert_eq!(lines(&s), [(MealVoucher, 313), (Card, 397), (Cash, 776)]);
        assert_eq!(s.rounding_cents, -1);
        assert_eq!(s.cash_due_cents(), 775);
    }

    // covers: UC4119
    #[test]
    fn uc4119_three_thirds_cash_last() {
        let s = settle(
            9312,
            &[
                PaymentPart::cash(),
                PaymentPart::of(Card, 3104),
                PaymentPart::of(MealVoucher, 3104),
            ],
        )
        .unwrap();
        assert_eq!(lines(&s), [(Card, 3104), (MealVoucher, 3104), (Cash, 3104)]);
        assert_eq!(s.rounding_cents, 1);
    }

    #[test]
    fn meal_vouchers_for_the_whole_sale_need_no_cash() {
        let s = settle(1486, &[PaymentPart::of(MealVoucher, 1486)]).unwrap();
        assert_eq!(lines(&s), [(MealVoucher, 1486)]);
        assert_eq!(s.rounding_cents, 0);
        // A cash part with nothing left to pay is dropped.
        let s = settle(
            1486,
            &[PaymentPart::of(MealVoucher, 1486), PaymentPart::cash()],
        )
        .unwrap();
        assert_eq!(lines(&s), [(MealVoucher, 1486)]);
    }

    #[test]
    fn a_zero_sale_is_a_cash_payment_of_zero() {
        let s = settle(0, &[PaymentPart::of(Card, 100)]).unwrap();
        assert_eq!(lines(&s), [(Cash, 0)]);
        assert!(!s.has_cash());
    }

    #[test]
    fn refused_settlements() {
        use PaymentError::*;
        let card = |a| PaymentPart::of(Card, a);
        assert_eq!(settle(1000, &[card(1001)]), Err(ExceedsTotal));
        assert_eq!(
            settle(1000, &[card(600), PaymentPart::of(MealVoucher, 500)]),
            Err(ExceedsTotal)
        );
        assert_eq!(settle(1000, &[card(600)]), Err(Incomplete));
        assert_eq!(settle(1000, &[]), Err(Incomplete));
        assert_eq!(settle(1000, &[card(0), PaymentPart::cash()]), Err(Invalid));
        assert_eq!(settle(1000, &[card(-5), PaymentPart::cash()]), Err(Invalid));
        assert_eq!(
            settle(
                1000,
                &[PaymentPart {
                    method: Card,
                    amount_cents: None,
                    eft_payment_id: None,
                    gift_voucher_id: None,
                }]
            ),
            Err(Invalid)
        );
        assert_eq!(settle(1000, &[PaymentPart::of(Cash, 500)]), Err(Invalid));
        assert_eq!(
            settle(1000, &[PaymentPart::cash(), PaymentPart::cash()]),
            Err(Invalid)
        );
        assert_eq!(settle(-100, &[PaymentPart::cash()]), Err(Invalid));
        // Parts that would wrap an i64 sum are refused, not settled.
        assert_eq!(
            settle(1000, &[card(i64::MAX), card(i64::MAX), PaymentPart::cash()]),
            Err(ExceedsTotal)
        );
        assert_eq!(
            settle(1000, &[card(i64::MAX), PaymentPart::of(MealVoucher, 2)]),
            Err(ExceedsTotal)
        );
    }

    #[test]
    fn single_method_settlements() {
        assert_eq!(Settlement::single(Cash, 608).rounding_cents, 2);
        assert_eq!(lines(&Settlement::single(Card, 608)), [(Card, 608)]);
        assert_eq!(
            lines(&Settlement::single(MealVoucher, 608)),
            [(MealVoucher, 608)]
        );
    }

    #[test]
    fn every_method_has_its_own_line_identity() {
        let ids: Vec<_> = [Cash, Card, MealVoucher]
            .iter()
            .map(|m| payment_line_identity(m.payment_type()))
            .collect();
        assert_eq!(ids, [("1", "Cash"), ("2", "Card"), ("3", "Meal voucher")]);
    }

    #[test]
    fn a_terminal_part_is_an_exact_card_amount_that_keeps_its_payment_id() {
        let s = settle(1486, &[PaymentPart::terminal(1000, 7), PaymentPart::cash()]).unwrap();
        assert_eq!(lines(&s), [(Card, 1000), (Cash, 486)]);
        assert_eq!(s.payments[0].eft_payment_id, Some(7));
        assert_eq!(s.payments[1].eft_payment_id, None);
        assert!(s.payments.iter().all(|p| p.terminal.is_none()));
        assert_eq!(s.rounding_cents, -1, "only the cash rest is rounded");
        // The whole sale on the terminal: exact, no cash needed.
        let s = settle(1152, &[PaymentPart::terminal(1152, 3)]).unwrap();
        assert_eq!(lines(&s), [(Card, 1152)]);
        assert_eq!(s.rounding_cents, 0);
    }

    #[test]
    fn a_cash_part_cannot_name_a_terminal_payment() {
        let cash = PaymentPart {
            eft_payment_id: Some(1),
            ..PaymentPart::cash()
        };
        assert_eq!(settle(1000, &[cash]), Err(PaymentError::Invalid));
    }

    #[test]
    fn the_terminal_id_is_not_written_when_absent() {
        let json = serde_json::to_value(PaymentPart::of(Card, 100)).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"method": "card", "amount_cents": 100})
        );
        let json = serde_json::to_value(PaymentPart::terminal(100, 4)).unwrap();
        assert_eq!(json["eft_payment_id"], 4);
        let part: PaymentPart =
            serde_json::from_str(r#"{"method":"card","amount_cents":100}"#).unwrap();
        assert_eq!(part, PaymentPart::of(Card, 100));
    }

    #[test]
    fn a_gift_voucher_is_vat_store_type_8_and_offered_at_the_till() {
        assert_eq!(GiftVoucher.payment_type(), PaymentType::VoucherStore);
        assert!(GiftVoucher.is_offered_at_till());
        assert_eq!(
            payment_line_identity(GiftVoucher.payment_type()),
            ("8", "Gift voucher")
        );
    }

    // covers: UC261, UC262
    #[test]
    fn a_gift_voucher_part_is_exact_and_never_rounded() {
        // UC261: 116,00 EUR, 50,00 EUR of voucher then the rest in cash.
        let s = settle(
            11600,
            &[PaymentPart::gift_voucher(5000, 9), PaymentPart::cash()],
        )
        .unwrap();
        assert_eq!(lines(&s), [(GiftVoucher, 5000), (Cash, 6600)]);
        assert_eq!(s.payments[0].gift_voucher_id, Some(9));
        assert_eq!(s.rounding_cents, 0);
        // UC262: the voucher pays the whole 42,50 EUR, no cash, no rounding.
        let s = settle(4250, &[PaymentPart::gift_voucher(4250, 3)]).unwrap();
        assert_eq!(lines(&s), [(GiftVoucher, 4250)]);
        assert_eq!(s.rounding_cents, 0);
        // An odd cent stays exact on the voucher; only the cash rest rounds.
        let s = settle(1152, &[PaymentPart::gift_voucher(1152, 1)]).unwrap();
        assert_eq!(lines(&s), [(GiftVoucher, 1152)]);
        let s = settle(
            1486,
            &[PaymentPart::gift_voucher(1152, 1), PaymentPart::cash()],
        )
        .unwrap();
        assert_eq!(lines(&s), [(GiftVoucher, 1152), (Cash, 334)]);
        assert_eq!(s.rounding_cents, 1);
    }

    #[test]
    fn two_vouchers_and_a_card_settle_in_the_order_taken() {
        let s = settle(
            10000,
            &[
                PaymentPart::gift_voucher(3000, 1),
                PaymentPart::gift_voucher(2000, 2),
                PaymentPart::of(Card, 5000),
            ],
        )
        .unwrap();
        assert_eq!(
            lines(&s),
            [(GiftVoucher, 3000), (GiftVoucher, 2000), (Card, 5000)]
        );
        assert_eq!(
            s.payments
                .iter()
                .map(|p| p.gift_voucher_id)
                .collect::<Vec<_>>(),
            [Some(1), Some(2), None]
        );
    }

    #[test]
    fn a_gift_voucher_part_never_exceeds_the_total_and_always_names_a_voucher() {
        assert_eq!(
            settle(1000, &[PaymentPart::gift_voucher(1001, 1)]),
            Err(PaymentError::ExceedsTotal)
        );
        // Without a voucher id, or with one on another method: refused.
        assert_eq!(
            settle(1000, &[PaymentPart::of(GiftVoucher, 1000)]),
            Err(PaymentError::Invalid)
        );
        let named_card = PaymentPart {
            gift_voucher_id: Some(1),
            ..PaymentPart::of(Card, 1000)
        };
        assert_eq!(settle(1000, &[named_card]), Err(PaymentError::Invalid));
        let named_cash = PaymentPart {
            gift_voucher_id: Some(1),
            ..PaymentPart::cash()
        };
        assert_eq!(settle(1000, &[named_cash]), Err(PaymentError::Invalid));
        assert_eq!(
            settle(
                1000,
                &[PaymentPart::gift_voucher(0, 1), PaymentPart::cash()]
            ),
            Err(PaymentError::Invalid)
        );
        // Something left and no cash: incomplete.
        assert_eq!(
            settle(1000, &[PaymentPart::gift_voucher(400, 1)]),
            Err(PaymentError::Incomplete)
        );
    }

    #[test]
    fn the_voucher_id_is_written_only_when_present_and_defaults_when_absent() {
        let json = serde_json::to_value(PaymentPart::gift_voucher(500, 4)).unwrap();
        assert_eq!(json["gift_voucher_id"], 4);
        assert_eq!(json["method"], "gift_voucher");
        let json = serde_json::to_value(PaymentPart::of(Card, 100)).unwrap();
        assert!(json.get("gift_voucher_id").is_none());
        let part: PaymentPart = serde_json::from_str(
            r#"{"method":"gift_voucher","amount_cents":500,"gift_voucher_id":4}"#,
        )
        .unwrap();
        assert_eq!(part, PaymentPart::gift_voucher(500, 4));
    }

    #[test]
    fn card_types_have_their_own_line_identity_except_the_manual_debit() {
        use PaymentType::*;
        assert_eq!(payment_line_identity(CardDebit), ("2", "Card"));
        assert_eq!(payment_line_identity(CardCredit), ("6", "Credit card"));
        assert_eq!(
            payment_line_identity(CardUnknown),
            ("7", "Card (type unknown)")
        );
        let ids: std::collections::HashSet<_> = [
            Cash,
            CardDebit,
            CardCredit,
            CardUnknown,
            ChequeMeal,
            App,
            Online,
            VoucherStore,
        ]
        .into_iter()
        .map(payment_line_identity)
        .collect();
        assert_eq!(ids.len(), 8, "one pair per payment type");
    }
}
