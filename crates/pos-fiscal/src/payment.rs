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
//! - a sale of 0,00 EUR is a cash payment of 0 (FAQ Q206).

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
            PaymentMethod::App => PaymentType::App,
            PaymentMethod::Online => PaymentType::Online,
        }
    }

    /// Whether the operator can take it at the till. App and online
    /// payments only come with online orders.
    pub fn is_offered_at_till(self) -> bool {
        matches!(
            self,
            PaymentMethod::Cash | PaymentMethod::Card | PaymentMethod::MealVoucher
        )
    }
}

/// The `id` and `name` of a payment line (spec §2.2.2
/// `financials.id`). The reports total the payments per id, name and
/// type, so each method keeps the same pair on every event.
pub fn payment_line_identity(payment_type: PaymentType) -> (&'static str, &'static str) {
    match payment_type {
        PaymentType::Cash => ("1", "Cash"),
        PaymentType::CardDebit => ("2", "Card"),
        PaymentType::ChequeMeal => ("3", "Meal voucher"),
        PaymentType::App => ("4", "App"),
        PaymentType::Online => ("5", "Online"),
        _ => ("9", "Payment"),
    }
}

/// One part of a payment as the operator takes it. A cash part has no
/// amount: it pays the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct PaymentPart {
    pub method: PaymentMethod,
    #[serde(default)]
    pub amount_cents: Option<i64>,
}

impl PaymentPart {
    pub fn cash() -> Self {
        Self {
            method: PaymentMethod::Cash,
            amount_cents: None,
        }
    }

    pub fn of(method: PaymentMethod, amount_cents: i64) -> Self {
        Self {
            method,
            amount_cents: Some(amount_cents),
        }
    }
}

/// A settled payment line: the amount this method pays of the sale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettledPayment {
    pub method: PaymentMethod,
    pub amount_cents: i64,
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
            payments: vec![SettledPayment {
                method,
                amount_cents: total_cents,
            }],
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
            payments: vec![SettledPayment {
                method: PaymentMethod::Cash,
                amount_cents: 0,
            }],
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
            (PaymentMethod::Cash, None) if !cash => cash = true,
            (PaymentMethod::Cash, _) => return Err(PaymentError::Invalid),
            (method, Some(amount)) if amount > 0 => {
                non_cash += amount;
                payments.push(SettledPayment {
                    method,
                    amount_cents: amount,
                });
            }
            _ => return Err(PaymentError::Invalid),
        }
    }
    if non_cash > total_cents {
        return Err(PaymentError::ExceedsTotal);
    }

    let rest = total_cents - non_cash;
    let mut rounding_cents = 0;
    if rest > 0 {
        if !cash {
            return Err(PaymentError::Incomplete);
        }
        rounding_cents = round_to_5_cents(rest);
        payments.push(SettledPayment {
            method: PaymentMethod::Cash,
            amount_cents: rest,
        });
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
                    amount_cents: None
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
}
