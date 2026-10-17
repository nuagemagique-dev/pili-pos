# Tips on the sale (design note, UC292 and UC296)

Status: design only, no code. Target: a later certified update after v1.1.
Audience: the planner of the next lane. Language: English (internal).

## 1. Goal

Let the cashier record a tip as part of a sale: the customer pays cash and leaves the change as a
tip (UC292), pays by card and adds a tip on the terminal, or pays in a foreign currency and leaves
the change as a tip (UC296, optional). Today the only tip the till knows is the cash tab reason
"Pourboire" (a MONEY_IN_OUT movement of the drawer, event F). A tip taken inside a sale is not
recorded at all, so the Z payments never show it and the drawer count does not explain the extra cash.

## 2. What the spec requires

| Source | Requirement |
| --- | --- |
| `docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md` section on `paymentLine` (around lines 1775 to 1830) | `amountType` is `PAYMENT`, `TIP` or `ROUNDING`. "Tip: used to record the tips received. A tip may be received as part of a transaction or be recorded as a money movement." A tip line is a payment line of a sale (event N) with the same `type`, `id`, `name` as the payment it comes with. |
| Same file, `moneyInOutLine` (around lines 1876 to 1890) | `MONEY_IN_OUT`, `TIP`, `DRAWER_DECLARATION` for event F; the tip in a movement is the second way of recording a tip. |
| Same file, `foreignCurrency` (around line 1819) | The `foreignCurrency` object only if the POS supports foreign currencies, and never together with `ROUNDING`. |
| Same file, Z report `payments[].amount[]` (around lines 2085 to 2130) | Totals per `PaymentLineType` (`PAYMENT`, `TIP`, `ROUNDING`), each with `normalAmount`, corrections and `totalAmount`, and `normalForeign` per currency. |
| `docs/legislation/20240429_AM_aspects_techniques_certification_FR.md` art. 1, 5 (entered data) | "les paiements enregistrés, y compris les pourboires et les arrondissements" are entered data the FDM must sign. |
| `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/UC292_Tip/` | Direct sale of 1 Perrier (A, 2.50) and 1 Burger of the chef (B, 28.00), total 30.50. The customer gives 35.00 in cash, the change 4.50 is the tip. Financials: `CASH / PAYMENT / 30.5` and `CASH / TIP / 4.5`, both with the same drawer. `transactionTotal` stays 30.5. |
| `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/UC296_PaymentForeignCurrencyTip/` | Sale of 40.02 EUR (carpaccio B, deposit bottles X) paid with 45 USD at 1.1152. Financials: `CASH / PAYMENT / 40.02` with `foreignCurrency {amount 44.63, iso USD}` and `CASH / TIP / 0.33` with `foreignCurrency {0.37, USD}`. 0.33 EUR is 45 USD converted (40.35) minus 40.02. |

UC291, UC294, UC295 and UC290 (foreign currency without tip) are out of scope here; UC296 is the only
foreign currency case that carries a tip, and it is the optional part of this note.

## 3. Today in the code

- `pos_fiscal::types::PaymentLineType` already has `Tip`. `PaymentLineInput` and `ForeignCurrencyInput`
  exist in `crates/pos-fiscal/src/inputs.rs`. Nothing creates a `TIP` payment line.
- `crates/pos-fiscal/src/payment.rs` settles a sale: card and voucher parts are exact, cash pays the
  rest, the 5 cent rounding goes on a `ROUNDING` line after the cash payment (`financial_lines` in
  `crates/pos-fiscal/src/event.rs`). The payments always add up to the total, rounding apart.
- `report_totals::payments` (in `crates/pos-fiscal/src/report_totals.rs`) already groups financial lines per
  payment method and per `amountType`, so a `TIP` line reaches the Z `payments` totals with no change
  there. Tips are not part of `transactionTotal` or of the turnover per department and VAT.
- The checkout screens compute the change for cash. The cash tab "Pourboire" reason is an F event
  (`commands/financial.rs`, `MoneyInOutLineType::MoneyInOut`, the reason is the line name).
- The linked card terminal drivers (`crates/pos-eft`) ask for an exact amount; none sends a tip amount.
- Foreign currency is not supported (`foreignCurrency` is never set; Z `normalForeign` is empty).

## 4. Proposal

Three levels, deliverable one after the other.

### 4.1 Tip from the change (cash), UC292 (core)

The cashier enters the cash received. If it is above the amount due, the change screen offers
"Rendre la monnaie" (default) or "Pourboire" (all of the change) or a typed tip amount. The sale
carries `PAYMENT` for the amount due and a second line of the same cash method with `amountType: TIP`.
The drawer receives both (the cash really entered the drawer), the change given back is the rest.

Rules:

- A tip is a positive amount, a multiple of 0.01. Cash rounding is computed on the amount due only, before
  the tip, and the tip is never rounded (FAQ Q403 reading: the rounding applies to the payment).
- `transactionTotal` is unchanged; the sum of `PAYMENT` and `ROUNDING` lines still equals it.
- Refunds: a tip is never refunded with the sale (the refund N reuses the original products only).
  Open question Q3.
- The ticket prints the tip under the payment rows ("Pourboire 4,50") so the customer's copy adds up.

### 4.2 Tip on a card payment (EFT or manual)

For a manual card (no linked terminal) the operator types the tip next to the amount. With a linked
terminal the drivers would need a `tip_cents` field in `PaymentRequest` and the terminal would charge
amount plus tip in one approval; the signed lines are `CARD_* / PAYMENT / due` and `CARD_* / TIP / tip`
with `inputMethod AUTOMATIC` and the same `reference` (the terminal's id). Whether each of the three
supported APIs (Worldline Terminal API, CCV Payment API, CCV OPI) can add a tip in the same approval
must be read in their documents before coding (open question Q1). Without it, a second approval on the
same card would be a second `eft_payments` row, which breaks the one-payment-one-line link the sale
check relies on (`eft_below_charged`, `used_event_id`).

### 4.3 Foreign currency tip, UC296 (optional, probably never)

Needs a currency table, a rate input, `foreignCurrency` on the payment and the tip line, no rounding
on those lines, per-currency totals in the Z (`normalForeign`) and a printed rate. No Belgian restaurant
of the first customers takes foreign currency. Recommendation: do not build; keep the data model open
(the `foreignCurrency` object is optional on both lines) and answer "no" to the manual's foreign
currency question.

## 5. Data model

No new table for 4.1 and 4.2 if the tip is stored on the payment line. The checkout today builds the
payment parts from the request (`PaymentPart` in `payment.rs`); a `tip_cents` is added to the part and
to the `Payment` DTO. Nothing is persisted outside the signed request, as for the other payment lines
(the archive is the source for the reports).

Optional: a view `v_tips` over `fiscal_events` (like `v_invoice_journal`) so the manager can ask "tips
of the week" without a new report; the Z already carries the totals.

Migration: none for 4.1 and 4.2. For 4.3 a `currencies` table (code, rate, valid_from, entered_by) with
the append-only triggers used elsewhere.

## 6. Fiscal events and payloads

Event N (`signSale`): the financials of UC292 are the model.

```
financials: [
  { type CASH, amountType PAYMENT, amount 30.50, inputMethod MANUAL, drawer 1 },
  { type CASH, amountType TIP,     amount  4.50, inputMethod MANUAL, drawer 1 }
]
```

Card with a linked terminal: both lines `inputMethod AUTOMATIC`, `provider` the brand, `reference` the
terminal reference. Event F is not used. No change to P, C, R or I payloads. The Z turnover report shows
the tips in `payments[].amount[]` under `type: TIP`; the user Z shows them per user.

## 7. UI

- Vue (`ui/src/views/CheckoutView.vue` and the payment components): a "Pourboire" choice in the cash
  change panel and a tip field on the card step; strings fr, en, zh in `ui/src/i18n*.ts`; a Playwright
  spec `tips.spec.ts`.
- Android (`feature/checkout`): the same two controls with the same keys, the amount through
  `AmountDialog`; parity rows in `docs/android/parity/checkout.md`; new engine fields in the FFI
  `PaymentPart` record and the contract tests.
- Printed ticket (`crates/pos-print/src/receipt.rs`): a "Pourboire" row after the payment rows; the
  certification manual's ticket element list does not forbid it but the SPF should be asked (Q2).

## 8. Tests

`spf_e2e_fixtures.rs` gains `uc292_tip` replayed field by field (financials, drawer, `transactionTotal`
unchanged); `spf_e2e_report_fixtures.rs` style test that a Z after a tip shows `TIP` in `payments`; refund
of a sale with a tip; rounding plus tip; card tip with the scripted EFT driver; FFI contract test of the
new `PaymentPart` field; Playwright and Roborazzi for the two controls.

## 9. Effort (worker-days)

| Layer | 4.1 cash tip | 4.2 card tip | 4.3 foreign currency |
| --- | --- | --- | --- |
| Fiscal crate and engine | 2 | 2 (plus 3 for the drivers, once the APIs are read) | 8 |
| Database | 0 | 0 | 1 |
| Vue | 1.5 | 1 | 3 |
| Android and FFI | 1.5 | 1 | 3 |
| Tests and fixtures | 2 | 2 | 3 |
| Dossier (French) | 0.5 | 0.5 | 1 |
| Total | about 7.5 | about 9.5 (12.5 with the drivers) | about 19 |

## 10. Risks

- A tip charged on the card terminal is real money: the EFT idempotency and recovery rules
  (`unknown` payments, approved-but-unused) must cover the tip amount, which makes 4.2 the risky part.
- The reports: the tip must stay out of the turnover and out of the VAT base. A test per report object.
- Mixing tip and cash rounding on one payment is easy to get wrong (see 4.1 rules).
- The manual's drawer reconciliation: the drawer count must explain the cash that entered with a tip.

## 11. Open questions

- Q1. Which of Worldline, CCV Payment API and OPI accept a tip in the same approval? (read the vendor
  documents; no live call is needed for that.)
- Q2. May the VAT ticket print a tip row, and where? Ask the SPF producers' mailbox.
- Q3. Is a tip returned when the sale is refunded? UC292 says nothing; conservative reading: no.
- Q4. Does the owner want a tip pooling report for the staff? Out of scope for fiscal work.
