# Parity: checkout

Port of `ui/src/views/CheckoutView.vue` with the shared delivery pieces it uses (`TicketMediumPicker.vue`,
`EmailKeypad.vue`, `DeliveryOutcomePanel.vue`, `stores/delivery.ts`, `warnIfSlipFailed` of `stores/order.ts`), card
P5b-T8. Native Compose view of module `:app`, package `dev.nuagemagique.pilipos.feature.checkout`, laid out for
1920 x 1080 dp inside `PosScaffold(HeaderArea.CHECKOUT)`. The engine owns every rule (settlement, the 5-cent
rounding, the signature, the ticket delivery); the screen only calls it through the shared stores, the gateway and
`EngineHelpers`.

## Rows

| Vue | Compose | Commands |
| --- | --- | --- |
| ui/src/views/CheckoutView.vue | feature/checkout/CheckoutScreen.kt (CheckoutRoute, CheckoutContent), CheckoutPanels.kt, CheckoutViewModel.kt, CheckoutState.kt, CheckoutEntryPoint.kt | `checkout` |
| components/TicketMediumPicker.vue | ui/components/TicketMediumPicker.kt (TicketMediumPicker, TicketMediumKeypad) bound to ui/state/DeliveryStore.kt | - |
| components/EmailKeypad.vue | ui/components/EmailKeypad.kt, drawn in the scaffold overlay of the checkout route | - |
| components/DeliveryOutcomePanel.vue | ui/components/DeliveryOutcomePanel.kt with `DeliveryStore.printMissingOriginal` | `print_missing_original` |
| stores/delivery.ts | ui/state/DeliveryStore.kt (refreshMailStatus on entering the pay step, request, reset) | `check_mail_status` |

## Display arithmetic (UI-PD306)

No amount is computed in this package; there is no sum, difference, product, quotient or rounding of cents.

| What is shown | Where it comes from |
| --- | --- |
| Total | `Order.totalCents` of the order in `OrderStore` |
| Left to pay | the amount of the cash row of `EngineHelpers.settlePreview(total, parts taken + cash part without amount)`, 0 when there is no row |
| Cash due, rounded | `cashDueCents` of the same preview (last part); for a cash part before the last one, of `settlePreview(part, [cash])` |
| Left to pay shown | the rounded cash due when cash pays the last part, else the rest |
| Equal shares | `CashDisplay.equalShares(total, n)` |
| Amount of the current part | the share or the rest, the smaller by comparison |
| Chips of the next parts | the cash row of a preview that holds the parts so far as exact amounts |
| Change | `CashDisplay.changeCents(entered, roundedDue)` |
| Quick amounts | `CashDisplay.quickAmountsCents(roundedDue)` |
| Entered amount | `AmountEntryState` (the digits typed are the cents) |
| Recap lines and VAT | `CartAmounts.lineAmount` and `vatRows` (the engine VAT split) |
| Done screen total | `Order.totalCents` read when the sale was validated |

The sale is `checkout(orderId, parts, request)`: the non-cash parts with their amount in the order taken, then a
cash part without amount (the Vue `payload`), and the `DeliveryStore` request (`null` for paper, as Vue).

## Differences

- **Cash part before the last one.** Vue subtracts the parts in the browser. Here a cash part taken earlier is held
  in the previews as an exact amount, so the engine does the subtraction; the sale itself still sends it last
  without amount.
- **Entry keys.** The pad is the shared `NumPad` (digits, clear, backspace) read as cents like a payment terminal,
  not Vue's comma pad: `2 0 0 0` is 20,00. A shortcut or a prefill is replaced by the first key, as Vue's `fresh`.
- **Quick amounts.** They are shown with their cents (`20,00 €`), as they come from `CashDisplay` in cents.
- **Done screen total.** Vue shows `Math.round(ticket.total * 100)`, a floating point. Here the total is the engine
  order total captured at validation; the change is the one fixed when the cash part was taken.
- **Layout.** The payment panel is on the left and the recap on the right (670 dp, the width of the cart column),
  as `CheckoutView.vue`.
- **No redirect on start.** Vue sends the operator to the tables when there is no order. The screen never navigates
  when it starts (PD205): it loads the route's order through `OrderStore.loadOrder` once the engine is ready and
  somebody is logged in, and shows an empty state meanwhile or when that fails.
- **Payment right.** A role that may not take payments (waiter) sees the screen but cannot validate; the print of
  the missing original is hidden from it, as in `DeliveryOutcomePanel.vue`.
- **Refusals.** The engine codes (`payment_invalid`, `payment_exceeds_total`, `payment_incomplete`, and the others)
  go through `ErrorPresenter` and are shown in the footer, back on the pay step with the parts kept.
- **New order.** The finish button of the done screen goes to the tables (the order is gone); Vue goes to the order
  route, which bounces to the tables when there is no order.
