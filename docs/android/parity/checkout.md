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
| CheckoutView.vue (summary: change rows, Sous-total, Remise) | feature/checkout/CheckoutPanels.kt (SummaryLine, SummaryFoot), CheckoutState.kt (CheckoutSummary.discountCents) | - |
| CheckoutView.vue (`held-warning`, `sendHeldNow`: "Envoyer maintenant") | feature/checkout/CheckoutPanels.kt (HeldWarning), CheckoutViewModel.kt (sendHeldNow) over ui/state/OrderStore.kt | `send_next_course` |
| CheckoutView.vue ("Bon cadeau" method, `GiftVoucherDialog.vue`, voucher notes `gv-prints` on the done screen) | feature/checkout/GiftVoucherDialog.kt, CheckoutPanels.kt (MethodRow), CheckoutScreen.kt (VoucherPrints), CheckoutViewModel.kt (voucherKey, lookupVoucher, applyVoucher) with ui/components/GiftVouchers.kt | `lookup_gift_voucher` `checkout` |
| components/TicketMediumPicker.vue | ui/components/TicketMediumPicker.kt (TicketMediumPicker, TicketMediumKeypad) bound to ui/state/DeliveryStore.kt | - |
| components/EmailKeypad.vue | ui/components/EmailKeypad.kt, drawn in the scaffold overlay of the checkout route | - |
| components/DeliveryOutcomePanel.vue | ui/components/DeliveryOutcomePanel.kt with `DeliveryStore.printMissingOriginal` | `print_missing_original` |
| stores/delivery.ts | ui/state/DeliveryStore.kt (refreshMailStatus on entering the pay step, request, reset) | `check_mail_status` |

## Quick checkout (UX audit P1, P2, P6)

| Vue | Compose | Commands |
| --- | --- | --- |
| CheckoutView.vue (`prefill`, `collectCents`, `validateLabel`, `receivedHint`; strings `collect_*`) | feature/checkout/CheckoutViewModel.kt (`Own.prefilled`), CheckoutState.kt (`collectCents`, `showReceived`), CheckoutPanels.kt (`validateLabel`, `ValidateButton`) | - |
| CheckoutView.vue (`showRest`, `offeredMethods`, `rest-methods-label`, string `rest_methods`) | CheckoutState.kt (`showRest`, `offeredMethods`), CheckoutPanels.kt (`MethodRow`, tag `checkout_rest_label`) | - |
| CheckoutView.vue (`online`, `wasOnline`, `beginOnlineCollect`: an unpaid online order paid at the checkout with Exact pre-filled and the change; "Carte" selected when the card button was tapped; the sale is `webshop_handover_order` and the flow returns to the online orders; UX audit O2) | feature/checkout/CheckoutViewModel.kt (`online`, `startOnCard`, `ensureOrder`, `startCheckout`, `BackToBoard`, `BoardAfterSale`), CheckoutRoutes.kt (`checkoutOnline`, args `online` and `card`), CheckoutEntry.kt, CheckoutScreen.kt, CheckoutPanels.kt (SummaryHead title `online_orders`) | `webshop_handover_order` |
| CheckoutView.vue (`back-to-order` always shown, `parts-clear` "Annuler les parts" only with parts taken by hand, `restoreParts`, `reconcileHeld`, toast `parts_dropped`; UX audit P4), utils/heldParts.ts, stores/order.ts (`heldParts`, `holdParts`), CartSidebar.vue (`held-parts`, "Parts déjà saisies : N") | feature/checkout/CheckoutPanels.kt (FootActions, tags `checkout_back`, `checkout_clear_parts`), CheckoutViewModel.kt (`restoreHeld`, `toHeld`, `toTaken`), ReconciledParts.kt (`reconcileHeld`, the sums through `CashDisplay.sumCents`), ui/state/OrderStore.kt (`heldParts`, `holdParts`), feature/order/MenuViewModel.kt, OrderCartColumn.kt (tag `cart_held_parts`) | - |
| CheckoutView.vue (done step: `holdDone`, `onDoneTap`, `finish`, `done-auto-return`), utils/checkoutDone.ts (`doneNeedsAttention`, `DONE_AUTO_RETURN_MS`) | CheckoutState.kt (`CheckoutDone.needsAttention`), CheckoutViewModel.kt (`scheduleAutoReturn`, `finish`, `DONE_AUTO_RETURN_MS`), CheckoutScreen.kt (`DoneStep`, `ChangeBanner`) | - |

- **P1.** The entry starts at the exact amount of the current part: the rounded cash due for cash (with the "exact X ·
  arrondi 5 cts" hint on the Exact button), the part for card and meal vouchers. The first key or a quick note replaces
  it. The primary button names what it collects: "Encaisser 47,30 € en espèces" (also "par carte", "en chèques-repas",
  "par bon cadeau"), "Encaisser la part 2 · 15,77 €" in a split, "Valider" when every part is taken and the sale only
  waits to be signed. For cash the amount is the rounded due; "reçu 50,00 €" is shown under the label only when the
  cash handed over differs from it. Cash is PAYER, then the button: 2 taps.
- **P2.** The done step shows OK and the change very large. It leaves for the tables by itself after 2 s, or at once
  on a tap anywhere or "Terminer" (once, whichever comes first). It stays, and only "Terminer" leaves, when the paper
  ticket was not printed (`needsPaper`), the digital ticket is queued, the FDM was not connected, or a gift voucher
  slip has a note. A takeaway returns to the tables too. Vue "Terminer" and the auto return go straight to the tables
  (no detour through the order screen); Android did already.
- **P6.** After a part leaves a rest, the methods row is led by "Reste 12,30 € :" (the amount of "Reste à payer") and
  offers the methods still allowed: cash is not offered once a cash part is taken (it used to be disabled). Tapping a
  method selects it with the rest entered, so the next part is that tap plus the primary button.

## Display arithmetic (UI-PD306)

No amount is computed in this package; there is no sum, difference, product, quotient or rounding of cents.

| What is shown | Where it comes from |
| --- | --- |
| Total | `Order.totalCents` of the order in `OrderStore` |
| Left to pay | the amount of the cash row of `EngineHelpers.settlePreview(total, parts taken + cash part without amount)`, 0 when there is no row |
| Cash due, rounded | `cashDueCents` of the same preview (last part); for a cash part before the last one, of `settlePreview(part, [cash])` |
| Left to pay shown | the rounded cash due when cash pays the last part, else the rest |
| Subtotal and discount rows | `CashDisplay.sumCents` of the cart lines' `discountCents`, and `CashDisplay.subtotalBeforeDiscountCents(total, discount)`; shown only when a line carries a discount |
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
- **Entry keys.** The pad is the shared `NumPad` (calculator layout 7 8 9 / 4 5 6 / 1 2 3, then clear, 0, backspace; digits, clear, backspace) read as cents like a payment terminal,
  not Vue's comma pad: `2 0 0 0` is 20,00. A shortcut or a prefill is replaced by the first key, as Vue's `fresh`.
  Cash is now prefilled too (P1), so the exact amount is already entered on arrival.
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
- **New order.** The finish button of the done screen goes to the tables (the order is gone), as does Vue since the
  quick checkout (P2).
- **Meal-voucher card on the terminal.** `EftPayment.meal_voucher` (the stored brand is Edenred, Pluxee, Monizze or
  Payflip) is kept on the taken part (Vue `TakenPart.mealVoucher`, Compose `TakenPart.mealVoucher`). The part stays
  method `card` toward the engine (same `eft_payment_id` payload) but is named by the meal-voucher label ("Part 1 ·
  Chèques-repas · Edenred") on the paid-part row and the part chip (Vue `takenLabel`, Compose `displayMethod`). No new
  string.
- **Online order at the checkout (O2).** The route carries `?online=true&card=<bool>` (Vue keeps the order in the
  Pinia store instead). The online order is loaded with `OrderStore.beginOnlineCollect`, which does not make it the
  engine's active order, so a lock or leaving the screen registers nothing on it. The back button and the end of the
  sale go to the online orders board.
- **Parts kept (P4).** Vue keeps the parts of the cart in the order store and restores them with `reconcileHeld`;
  Compose keeps them in `OrderStore.heldParts` and reconciles in `reconcileHeld` (ReconciledParts.kt) with the running
  `CashDisplay.sumCents` (the UI computes no money, UI-PD317). The terminal-card parts are not kept here: the engine
  lists the approved payments (`eft_payments`) and the screen adopts them again, which is the same as Vue. The split
  count is not restored (same on both). Leaving an online checkout drops the kept parts.

