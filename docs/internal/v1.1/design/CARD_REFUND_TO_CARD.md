# Refund to the card (design note, UC4114 part 2 and EFT reversal)

Status: design only, no code. Target: a later certified update after v1.1.
Audience: the planner of the next lane. Language: English (internal).

## 1. Goal

Today every refund goes back in cash: the refund N carries a cash payment and opens the drawer. A
customer who paid by card expects the money back on the card. This note describes how to refund a card
payment through the linked payment terminal, in a way that stays fiscally correct and recoverable, and
what UC4114 asks for (a refund of deposits on a card with no original sale).

## 2. What the spec requires

| Source | Requirement |
| --- | --- |
| `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/UC4114_festivalDeposit/UC4114_2_RefundDeposit/` | Sale (N) of `Bottle S` x -10 `REFUND` (VAT label X), `transactionTotal` -0.1, payment `CARD_DEBIT / PAYMENT / -0.1` with `provider VISA` and `inputMethod MANUAL` in the fixture. The payment amount is negative; there is no reference to an original ticket. |
| `docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md` `paymentLine` (around line 1790) | `amount` "may be positive, negative or zero"; `inputMethod AUTOMATIC` when the POS is connected to the payment terminal and the terminal returns the card type itself. |
| Same file, event N refunds | A refund is an N event with negative quantities and `negQuantityReason REFUND`; the original sale is referenced through `fdmRef`s in the existing refund path, which the code already builds (`commands/refund.rs`). |
| `docs/legislation/20240429_AM_aspects_techniques_certification_FR.md` art. 14 and following | The till must not take a payment unless the FDM is operational; the same care applies to a card refund (the money moves, so the event must be signed). |
| Certification manual, payment methods | A refund payment line has the method of the money that moves; a card refund is `CARD_*`, a cash refund `CASH`. |

## 3. Today in the code

- `commands/refund.rs`: `RefundSpec { lines, payment_type, open_drawer }`; `RefundSpec::cash` is the only
  constructor used by the till; `APP` and `ONLINE` are used by `commands/webshop.rs` (the platform pays back,
  the till moves no money).
- `crates/pos-eft/src/lib.rs`: `Terminal::pay`, `status`, `check`, `abort`. There is no refund or reversal
  call in the trait, the three drivers (`worldline.rs`, `ccv_cloud.rs`, `ccv_opi.rs`) only implement a
  payment, its status and its abort.
- `eft_payments` (migration 27, `crates/pos-db/src/schema_eft.rs`): one row per payment asked of the terminal,
  stored BEFORE the call, status `sent`, `approved`, `declined`, `cancelled`, `unknown`, `not_sent`, never
  deleted, an approved payment gets `used_event_id` once. `terminal_reference` and `authorisation_code` are
  stored (what a refund refers to, see the `Approval` doc comment).
- `CLAUDE.md` (Payment terminal section): "Card payments are never rounded; refunds stay cash; reversing an
  approved payment is not supported yet."
- `commands/eft.rs`: `eft_pay`, `eft_abort`, `eft_check`, `list_eft_payments`; `eft_below_charged` stops an
  order edit under the approved unused card payments.

## 4. Two different operations

1. **Reversal (annulation)**: cancel an approved card payment the same day, before settlement, because the
   sale is being corrected. On most terminals it is cheaper and invisible on the statement.
2. **Refund (remboursement)**: give money back for a settled payment, later, to the same card (referencing
   the original transaction) or to any card (an unreferenced credit, "refund without reference").

The sale N of the refund is signed once, with a negative `CARD_*` payment line; the operations above are
how the money moves on the terminal. The fiscal event does not say which of the two was used.

## 5. Proposal

### 5.1 Driver interface

Add to `pos_eft::Terminal`:

- `refund(request: &RefundRequest) -> Result<Outcome, EftError>` with `RefundRequest { reference,
  amount_cents, original: Option<OriginalRef>, description, language }` where `OriginalRef` holds the
  original `terminal_reference` and `authorisation_code`.
- `status(reference)` already reads a final answer by the till's reference; the refund reuses it (idempotency
  key = the till's own UUID, as for payments).

Capabilities differ per vendor, to be confirmed from their documents (open question Q1):

| Driver | What to check |
| --- | --- |
| Worldline Terminal API (nexo Retailer 5.1) | Payment Reversal and Refund message types; whether an unreferenced refund is allowed by the acquirer contract. |
| CCV Payment API (`terminal` method) | Refund endpoint, whether it needs the original payment id. |
| CCV OPI (XML over LAN) | `ReversalRequest` and `PaymentRequest` with a refund type; supported by the terminal profile. |

### 5.2 Engine flow

1. The operator chooses "Rembourser" for a sale. If the sale was paid by card (the original sale's financials
   say `CARD_*`, `eft_payments.used_event_id` points to it), the screen offers "Sur la carte" (default) or
   "En espèces". An unreferenced refund (UC4114: no original sale) is "Sur une carte".
2. Same safety as a payment: the refund is first written to a new table `eft_refunds` (status `sent`) BEFORE
   the terminal is called; at start a `sent` row becomes `unknown`; nothing is asked of the terminal unless the
   FDM is operational right now.
3. The terminal answers: `approved` -> sign the refund N with the negative card line (`AUTOMATIC`, brand,
   `reference`), mark `used_event_id` in the same commit; `declined` or `cancelled` -> nothing signed, the
   operator may choose cash; `unknown` -> blocks the refund of that sale until `eft_check`, like a payment.
4. The refund N is signed through the existing `refund_sale` path with a new `RefundSpec` whose
   `payment_type` is the card type and `open_drawer` false. If signing fails after an approved terminal
   refund, the approved-but-unused row makes the next attempt reuse it (the same rule as an approved unused
   payment, `eft_below_charged`).

### 5.3 Limits

- A refund on card cannot exceed what the original card payment charged (`eft_payments.amount_cents` minus
  already refunded). Several partial refunds are allowed up to the total.
- A sale paid half cash half card: refund the card part on the card and the rest in cash, in one refund
  event with two negative payment lines (the same `settle` logic as a sale, with negative amounts; to be
  designed, the cash rounding of a refund is an open question Q3).
- A refund after the card terminal was replaced or the acquirer changed falls back to cash.

## 6. Data model

Next free migration after 42:

- `eft_refunds(id, reference UNIQUE, eft_payment_id NULL, order_id NULL, fiscal_event_id NULL,
  terminal_kind, amount_cents, status, card_type, brand, terminal_reference, authorisation_code,
  customer_receipt, detail, user_insz, created_at, resolved_at, used_event_id)` with the same triggers as
  `eft_payments` (never deleted, starts as `sent`, a final answer never changes).
- No change to `eft_payments` except a view `v_card_balance` (charged, refunded, refundable per payment).

## 7. Fiscal events and payloads

Event N, as `refund_sale` builds it today, with `financials` replaced:

```
financials: [
  { type CARD_DEBIT, amountType PAYMENT, amount -0.10, provider VISA, inputMethod AUTOMATIC,
    reference <terminal reference> }
]
```

No new event type. The Z `payments` shows a negative card amount; the user Z likewise. The ticket is the
REMBOURSEMENT VAT ticket (existing) with the card line, and the card terminal's own receipt text is kept in
`customer_receipt`.

## 8. UI

- Vue: the refund dialog in Reports (`ReportsView.vue`) gains the "Sur la carte / En espèces" choice and the
  waiting overlay of the terminal (`EftOverlay.vue` is the model); strings fr, en, zh.
- Android: `ReportsRefundDialog.kt` and the EFT overlay of the checkout, same states; FFI records for the
  refund outcome; goldens.
- Settings: nothing; the feature appears only when `payment_terminal` is configured.

## 9. Tests

The scripted driver (`pos-eft` feature `scripted`) and the three mocks (`pos-eft-mock`) gain refund routes;
engine tests: approved, declined, cancelled, unknown then recovered, crash between terminal answer and signing
(recovery), refund above the charged amount refused, partial refunds, split payment refund, UC4114 part 2
replayed field by field, FDM down refuses before the terminal is called. A live probe in `pos-eft-probe`
(owner go-ahead needed, no live call in a lane).

## 10. Effort (worker-days)

| Layer | Estimate |
| --- | --- |
| Reading the three vendor documents, choosing the message types | 2 |
| `pos-eft` trait and three drivers with their mocks | 8 |
| Database (`eft_refunds`, triggers, view) | 2 |
| Engine (flow, recovery, `RefundSpec`, FDM guard) | 6 |
| Vue and Android with FFI and goldens | 6 |
| Tests and dossier (French) | 6 |
| Total | about 30 |

## 11. Risks

- Money moves on a third-party device: every failure mode (timeout, power cut, double tap) must leave a row
  that tells the truth. This is the largest risk and the reason for the BEFORE-the-call write.
- A reversal and a refund look the same to the customer but not to the acquirer; choosing wrongly costs fees.
- Terminal firmware without refund permission for the merchant: the call is refused with a vendor error that
  the till must show clearly (`eft_rejected` today).
- Fiscal: an approved terminal refund with no signed N is money out without a ticket; the recovery rules above
  exist to prevent it, and the Z must be refused or warned while an approved unused refund exists (as the
  unsettled card payments warning does today).

## 12. Open questions

- Q1. Which refund message types does each vendor support for this merchant contract, with and without reference?
- Q2. Does the SPF want the card brand and the last four digits on the refund ticket? (the payment ticket
  prints the brand today.)
- Q3. Cash rounding on the cash part of a split refund.
- Q4. Who may refund on the card: manager only (as the cash refund), or cashier with a limit?
