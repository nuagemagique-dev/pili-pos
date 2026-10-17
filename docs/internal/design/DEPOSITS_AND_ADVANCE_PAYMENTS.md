# Deposits and advance payments (design note, UC4110, UC4111, UC4113, UC4114)

Status: design only, no code. Target: a later certified update after v1.0.0.
Audience: the planner of the next lane. Language: English (internal).

Note on numbering: `UC4112` of the Advanced Set 1.6 is the weighted product, not a deposit. The four
deposit cases are UC4110, UC4111, UC4113 and UC4114.

## 1. Goal

Support three money-in-advance situations that the till cannot express today:

1. A **prepayment for a party or a reservation** (UC4110, UC4111, UC4113): the customer pays before the
   meal; on the day the prepayment is deducted from the bill, and the bill is corrected for the people who
   did not come.
2. A **bottle (or glass) deposit** (UC4114): a deposit line is sold with the drink (VAT label X, "no VAT"),
   and returned later, possibly in another sale.
3. The **return of a deposit on a card** (UC4114 part 2), which today is blocked because refunds are cash only
   (see `CARD_REFUND_TO_CARD.md`).

## 2. What the spec requires

| Source | Requirement |
| --- | --- |
| `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/UC4110_advancedPayment/` (three steps) | 1. Sale (N) of two prepayment products on a CUSTOMER cost center (`id 100`, reference `offertnr`): Prepayment Food 750 (B) and Prepayment Drinks 500 (A), cash 1250. 2. `signOrder` (P) of the party: 20 Menu x 75 (B) and 20 Wine pairing x 50 (A), plus two **negative** lines, one per prepayment product, `negQuantityReason REFUND`, -750 (B) and -500 (A); `transactionTotal` 1250. 3. Final sale (N): the order lines again, then `CORRECTION` lines (-2 Menu, -12 Menu drink) and 8 bottles of water; `transactionTotal` 540, cash 540. |
| `UC4111_advancedPayment%of price/` | Same flow, the prepayment is one product "PREPAYMENT" for 1250 (A), a percentage of the offer; the order and the sale refund it with one negative line of the same product and VAT label. |
| `UC4113_prepaymentReservation/` (five steps) | Reservation for 4: step 1 sale (N) with no cost center, 4 x PREPAYMENT 100 (A) = 400 in cash; step 2 `signOrder` on TABLE 212: 4 Menu 200 (B) and 4 Wine menu 75 (A), total 1100; step 3 `signOrder` with one line PREPAYMENT x -4 `REFUND`, total -400 (the deduction is its own order registration); step 4 `signPreBill`: the lines and the product "DEPOSIT" x -4 -400, total 700; step 5 sale (N) cash 700. |
| `UC4114_festivalDeposit/` | 1. Sale of 5 Coca-Cola (C, 3.04) and 5 "Bottle S" (department Bottle Deposits, VAT label X, 0.01 each), total 15.25, paid by card (`CARD_DEBIT`, provider VISA). 2. Sale (N) with `Bottle S` x -10 `REFUND`, `transactionTotal` -0.10, payment **negative on the card**: `CARD_DEBIT / PAYMENT / -0.1`. |
| `docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md` | VAT label X (no VAT), `negQuantityReason` values `REFUND` and `CORRECTION`, the cost center types `CUSTOMER` and `TABLE`, the payment amount "may be positive, negative or null" (around line 1790). |
| `docs/legislation/20240429_AM_aspects_techniques_certification_FR.md` art. 29 (reports) | Negative quantities are reported once, per reason, by the event that first transmitted them (`neg_quantities` in `report_totals.rs` implements the FAQ Q303 rule). |
| `docs/20260717_FAQ_PRODUCTEURS_FR.md` | To read before coding: the questions on prepayments, vouchers and deposits (search "acompte", "consigne", "prépaiement"); this note does not claim an answer. |

## 3. Today in the code

- Nothing models a prepayment or a deposit. Any product can have a negative quantity only through the
  refund and correction paths; a free price must be above zero (`price_positive`) and a menu price is never negative.
- `commands/refund.rs`: a refund is bound to one original sale (`refund_sale`), with `RefundSpec::cash`;
  `payment_type` can be `APP` or `ONLINE` for online orders. The refunded lines are products of that sale.
- `commands/tables.rs`: `do_sign_table_order` (P ORDER) and `do_sign_pre_bill` (P PRE_BILL) take the
  order's journal. A P event carries negative lines only as corrections of earlier entries.
- Cost centers: the order has a table (`CostCenterType::Table`) or none. `CUSTOMER` exists in
  `CostCenterType` and is used by invoices (V11), with the customer's reference.
- VAT label X exists (`VatLabel::X`); the Basic Set has the "Bottle S" deposit product (department 90).
- `report_totals` already counts negative lines per reason, once, on the event that first sent them
  (`registered_negatives`), and the Z turnover per department includes the "Prepayments" department as
  any other.

## 4. Proposal

### 4.1 Model

Treat a prepayment as an ordinary product in a dedicated department, sold in a normal sale (N), and its
deduction as an ordinary negative line of the same product and VAT label. This is exactly what the four
fixtures do: the FDM sees nothing special, only products, labels and cost centers. What the till adds is
the **link** between the prepayment sale and the later order, which the spec leaves to the POS.

New concepts (all in the till, none in the signed message):

- `prepayment` record: id, the fiscal event id of the sale (N), the customer reference (offer number or
  reservation name), the product lines it holds (product id, VAT label, amount), the amount still unused,
  status `open`, `applied`, `refunded`, `partially_applied`.
- A product flag `deposit_kind`: `none`, `prepayment`, `bottle_deposit`. A `prepayment` product has a free
  price and a VAT label chosen at sale time (A, B, C, or the label of the menu it will pay); a
  `bottle_deposit` has a fixed price and label X.
- On an order or table: an optional `prepayment_id`. When set, the order carries the negative lines in the
  P registration (UC4113 step 3 and UC4110 step 2) and they come back unchanged in the pre-bill and the
  sale.

### 4.2 Flows

Prepayment (UC4110/4111/4113):

1. "Acompte": the cashier picks the prepayment product(s), enters the amount per VAT label, the customer
   reference (offer or reservation), takes the payment. Event N, cost center CUSTOMER with the reference
   (UC4110, UC4111) or none (UC4113). The ticket is a normal VAT ticket.
2. At the table or the party: "Utiliser un acompte" on the order, choose the open prepayment (by reference).
   The engine adds the negative lines (quantity -1 or -n, `negQuantityReason REFUND`, same product, same
   VAT label, price of the prepayment line) and registers them with the next P ORDER (UC4113 step 3) or
   together with the first order (UC4110/4111 step 2).
3. Sale: the order total is positive or zero after the deduction. Corrections after the order (people who
   did not come) are the existing `CORRECTION` lines. If the bill is below the prepayment, the balance must
   be returned: the sale total is negative or zero, which needs a negative payment (a cash refund; on card,
   see `CARD_REFUND_TO_CARD.md`). Conservative rule for the first version: refuse to close a sale with a
   negative total and ask for a refund of the prepayment instead (open question Q2).

Bottle deposit (UC4114):

1. The product "Bottle S" is added automatically by a link `menu_item -> deposit_item` (one per bottle), VAT
   label X, 0.01 EUR in the fixture (real deposits are a few cents to a few euros).
2. Return: "Retour de consigne": choose the deposit product and a quantity, the engine signs an N with
   negative quantity, `REFUND`, and a negative payment in the method the customer picks (cash, or card
   when `CARD_REFUND_TO_CARD.md` is done). No original sale is needed (UC4114 step 2 has none).

### 4.3 Reports

The Z already shows the lines by department (prepayments and deposits in their own departments), the VAT
(label X has rate 0 and shows in `vats`), `negQuantities` (REFUND, once) and `payments` (a negative card
payment appears in `PAYMENT` as a negative amount). Nothing new is needed in `report_totals.rs`, but each
case needs a fixture test because a prepayment spans two booking dates when the party is another day: the
deduction is counted on the day it is registered, and the Z of the prepayment day shows the cash, not the
use. The sales report (v1.0.0) shows the same split (prepayment lines are products of their own).

## 5. Data model

Next free migration after 42 (a v1.2 number, not reserved here):

- `prepayments(id, fiscal_event_id, reference, customer_id NULL, created_by, created_at, status,
  amount_cents, used_cents)` and `prepayment_lines(prepayment_id, menu_item_id, vat_label, amount_cents,
  applied_cents)`, append-only for the signed facts, `status` and `applied_cents` updated only through a
  trigger-checked path like `sold_out`.
- `orders.prepayment_id` nullable.
- `menu_items.deposit_kind` and `menu_item_deposit(menu_item_id, deposit_menu_item_id, quantity_per_unit)`.
- A change log row for every use (who applied which prepayment to which order), append-only.

## 6. Fiscal events and payloads

All events exist already; the new payloads are:

| Step | Event | Specific content |
| --- | --- | --- |
| Prepayment sale | N | product lines of the prepayment products; `costCenter {type CUSTOMER, reference}` or none; positive payments |
| Use on an order | P ORDER | negative lines, `negQuantityReason REFUND`, same `productId`, same VAT label; counted once by `neg_quantities` |
| Pre-bill and final sale | P PRE_BILL, N | the same negative lines again (the sale resends the registered entries, `registered_negatives`) |
| Correction of the party size | N | existing `CORRECTION` lines |
| Deposit sale and return | N | deposit product, VAT label X, positive then negative quantity |
| Return on card | N | negative `CARD_*` payment line (needs the card refund work) |

The invoice of V11 (event I) already handles negative lines (UC4122): a prepayment invoice for a company
is possible once this exists.

## 7. UI

- Vue: an "Acompte" action on the tables screen and in Reports, a "Utiliser un acompte" action in the order
  screen (list of open prepayments with reference, amount, status), a "Consigne" return button on the order
  screen; strings fr, en, zh; Playwright specs per flow.
- Android: the same screens in `feature/tables`, `feature/order`, `feature/checkout`, view models and
  goldens; parity rows in `docs/android/parity/*.md`; FFI records for `Prepayment`.
- Menu editor (V11): the `deposit_kind` selector and the deposit link, locked under HubRise like the rest.

## 8. Tests

Four fixture replays (`uc4110`, `uc4111`, `uc4113`, `uc4114`) in `spf_e2e_fixtures.rs` or a new
`prepayment_advanced.rs`, field by field; Z reports after each flow; a party of 18 instead of 20 (UC4110
step 3); prepayment used on another booking date; double use refused; prepayment larger than the bill;
VAT label per prepayment product; reports of the sales report on those days.

## 9. Effort (worker-days)

| Layer | Prepayment (UC4110, 4111, 4113) | Bottle deposit (UC4114 part 1) | Deposit return (UC4114 part 2) |
| --- | --- | --- | --- |
| Database and queries | 3 | 1.5 | 0.5 |
| Engine and fiscal | 6 | 2 | 1 (cash), see the card note otherwise |
| Vue | 4 | 1.5 | 1 |
| Android and FFI | 4 | 1.5 | 1 |
| Tests and fixtures | 5 | 2 | 1.5 |
| Dossier (French) | 1.5 | 0.5 | 0.5 |
| Total | about 23.5 | about 9 | about 5.5 |

## 10. Risks

- The link between a prepayment and an order is a till rule the FDM cannot check: an operator error can use
  a prepayment twice. The `prepayments` table and its append-only log must make that impossible and visible.
- Mixed VAT: a prepayment taken for a menu split over labels A and B must be deducted per label (the fixtures
  use one product per label). A single "PREPAYMENT" at label A for a B menu (UC4111) is a fiscal choice of the
  restaurant; the till should warn but not decide.
- A prepayment for an event months later crosses Z reports; the customer reference must survive and the report
  must not double count.
- Cancelled reservation: the prepayment is refunded by cash or card; no fixture covers it (open question Q3).
- Art. 44: product price or VAT edits (menu editor) must not change an applied prepayment line.

## 11. Open questions

- Q1. Does the SPF accept a prepayment product at the VAT label of the later meal (UC4111 uses A for a mix)? Ask.
- Q2. What happens when the prepayment exceeds the bill? Return the balance in the same method, or keep it as a
  credit? (The spec's `CUSTOMER_CREDIT` payment type and UC4100 customer accounts are non-goals of v1.0.0.)
- Q3. How is a cancelled reservation refunded (no original bill)? A refund N with the prepayment product
  quantity -n, payment negative, is the natural reading.
- Q4. Bottle deposits sold by the platform (HubRise) orders: the catalog must carry the deposit product.
