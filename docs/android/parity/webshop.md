# Parity: online orders (webshop)

Port of `ui/src/views/WebshopOrdersView.vue` (card P5b-T11) on the shared state of card P5b-T5
(`ui/state/WebshopOrdersStore.kt`). The screen is a native Compose view of module `:app`, package
`dev.nuagemagique.pilipos.feature.webshop`, laid out for 1920 x 1080 dp. The engine owns every rule (what is signed,
the VAT, the refund, the printed tickets); the screen only reads the records it lists and calls the commands through
the store.

## Rows

| Vue | Compose | Commands |
| --- | --- | --- |
| ui/src/views/WebshopOrdersView.vue, components/WebshopRejectDialog.vue, utils/webshopRights.ts (board, cards, actions, reject reasons, rights, delays, prints, resync) | feature/webshop/WebshopScreen.kt (WebshopRoute, WebshopContent), WebshopCard.kt, WebshopDialogs.kt, WebshopViewModel.kt, WebshopRules.kt, WebshopEntryPoint.kt | `hubrise_sync_catalog` |
| stores/webshopOrders.ts (list, refetch on events) | ui/state/WebshopOrdersStore.kt (fetchOrders) | `hubrise_list_webshop_orders` |
| stores/webshopOrders.ts (acceptOrder, rejectOrder) | ui/state/WebshopOrdersStore.kt (acceptOrder, rejectOrder) | `webshop_accept_order` `webshop_reject_order` |
| stores/webshopOrders.ts (startPreparation, markReady, markOutForDelivery, delayByMinutes) | ui/state/WebshopOrdersStore.kt (patchStatus, delayByMinutes) | `hubrise_patch_order_status` `hubrise_set_confirmed_time` |
| stores/webshopOrders.ts (completeOrder, refundOrder, closeCancelledOrder) | ui/state/WebshopOrdersStore.kt (completeOrder, refundOrder, closeCancelledOrder; `collectOrder` and `collectOnTerminal` stay as tested API but the board no longer calls them, see the next row) | `webshop_complete_order` `webshop_refund_order` `webshop_close_cancelled_order` |
| WebshopOrdersView.vue (`collectAtCheckout`: "Encaisser espèces" and "Encaisser carte" of an unpaid order open the checkout for that order, UX audit O2), stores/order.ts (`beginOnlineCollect`, `onlineCollect`) | feature/webshop/WebshopViewModel.kt (`WebshopEffect.OpenCheckout`, `primary`, `collectCard`), WebshopScreen.kt (navigates to `CheckoutRoutes.checkoutOnline`), feature/checkout/CheckoutViewModel.kt (`ensureOrder` -> `OrderStore.beginOnlineCollect`, the handover with the payment parts), ui/state/OrderStore.kt (`beginOnlineCollect`, `onlineCollect`; the online order never becomes the engine's active order) | `webshop_handover_order` |
| stores/webshopOrders.ts (printKitchen, printCustomer) | ui/state/WebshopOrdersStore.kt (printKitchen, printCustomer) | `print_webshop_kitchen` `print_webshop_customer` |
| components/OnlineOrdersNavButton.vue | ui/scaffold/AppHeader.kt (online orders button, PendingBadge) over ui/state/WebshopOrdersStore.kt (pendingCount) | - |
| composables/useOrderChime.ts | ui/feedback/OrderChime.kt (OrderChimeOverlay, OrderChimePlayer) | - |
| composables/useOrderChime.ts (late repeat), utils/webshopLate.ts (`isLateOrder`, `lateCount`, `lateChimeDue`, `LATE_CHIME_EVERY_MS`), stores/webshopOrders.ts (`lateCount`, `now`): the chime repeats every 2 minutes while an online order waited 5 minutes or more (UX audit O3) | ui/feedback/LateOrders.kt (`lateOrderCount`, `lateChimeDue`, `lateSamples`, `playLateChime`) over feature/webshop/WebshopRules.kt (`isLate`, `ALARM_MINUTES`), OrderChime.kt (OrderChimeOverlay, for a logged-in operator only like the Vue chime) | - |
| components/LateOnlineBanner.vue ("N commandes en ligne attendent depuis plus de 5 min" + "Voir", under the header of every staff screen but the board, UX audit O3) | ui/components/LateOnlineBanner.kt (tags `late_online_banner`, `late_online_text`, `late_online_view`), drawn by ui/scaffold/PosScaffold.kt; count in ui/scaffold/HeaderViewModel.kt (`HeaderUiState.lateOrders`, `lateNow`) | - |
| composables/useAddBeep.ts (beep on every add, C9) | ui/feedback/AddBeep.kt (AddBeepOverlay, AddBeepPlayer, `beepSamples`), AddBeepModule.kt | - |

## Differences

- **Late orders (O3).** Vue ticks a `now` ref every 5 s in the webshop store and derives `lateCount`. Compose keeps
  the store unchanged and derives the count where it is used (`ui/feedback/LateOrders.kt`: a 5 s clock that runs only
  while an unanswered order exists, so an idle board runs none). The banner count follows the logged-in operator
  (HeaderViewModel), the repeating chime is in `OrderChimeOverlay`; both are silent with nobody logged in, as Vue.
- **Collecting an unpaid order (O2).** The buttons "Encaisser espèces" and "Encaisser carte" no longer sign a sale
  from the board: they open the checkout of the order (Exact pre-filled, the change, split parts), whose sale is
  `webshop_handover_order`. `WebshopOrdersStore.collectOrder` and `collectOnTerminal` are kept, tested, and unused.
- **No redirect on start.** Vue sends the operator to the login screen when there is no user. The Compose screen never
  navigates when it starts (PD205): without a user it shows an empty state (`webshop_empty`), with HubRise switched off
  and no online order to settle it says so (`webshop_off`, a text Vue does not have). A test proves that nothing is
  navigated while the screen loads with the engine closed. The root carries the tag `placeholder_webshop` (PD204: the
  tag means "feature root").
- **Channel as a text chip (UI-PD309).** Vue shows the Deliveroo, Uber Eats and Takeaway logos. Android shows the
  channel (`source` of the order, or "Webshop" for the restaurant's own) as a text chip, with a Volt outline for the
  own webshop. No third-party artwork is bundled and the feature owns no drawable. The per-brand stripe colours of
  Vue (the logos' colours) are not ported: the colours come from the theme tokens only.
- **Same three columns, same buttons.** New, in the kitchen, ready (or, behind the "Terminées" button, finished). The
  column of an order, its primary action per status, the refund only for a manager, the review block, the
  banners and the 5-minute alarm follow the script of the Vue view one to one (`WebshopRules.kt`); the unit tests
  cover the primary action of every status.
- **Confirmations (added).** "Refuser" asks for a reason and a confirmation first on both UIs (the refusal goes to the
  platform and cannot be undone), and "Rembourser" asks the logged-in manager to confirm the engine's amount; the
  manager's INSZ is the one sent to `webshop_refund_order`. Without a manager nothing is sent and the warning
  `webshop_manager_needed` is shown.
- **Refuser with a reason (UX audit O1).** The dialog (`WebshopRejectDialog.vue`, `WebshopDialogs.kt` `RejectDialog`,
  state `WebshopDialog.RejectConfirm(orderId, reason)`) lists four chips (`webshop_reject_reason_*`: out of stock, too
  many orders, restaurant closed, other); the red `webshop_reject_confirm` button is disabled until one is picked, so
  the tap count is Refuser, a chip, the red button. HubRise has no reject reason field (`pos-hubrise-types` knows only the
  status `rejected`; `seller_notes` is a free note, not a reason, and is not used), so `webshop_reject_order` keeps its one argument and the reason
  stays on the till: it only gates the button. Nothing is signed, a rejected order never is. On the card "Refuser" is
  the first button of the action row (left of the total), outlined red and never filled, at the other end from the
  primary action (`WebshopCard.kt` `RejectButton`; Vue `.btn-reject`).
- **Rights by role (UX audit L6).** Accept, refuse, collect (cash and card), complete and close need the cashier
  rights (engine `Access::Cashier`). For an Apprenti (role `waiter`; on screen "Serveur" is the role `cashier`,
  "Apprenti" the role `waiter`) these buttons are shown disabled with the line `webshop_cashier_only` ("Réservé au
  serveur ou au gérant") above the row instead of failing with `err_forbidden`. Preparing, ready and out for delivery
  stay enabled (engine `Access::Staff`). Vue reads `auth.canTakePayment` (`utils/webshopRights.ts`), Compose
  `SessionStore.canTakePayment` (`WebshopRules.cashierNoteNeeded`, `PrimaryAction.needsCashier`); the view model also
  ignores the taps. The engine still refuses a waiter.
- **Delay in a dialog.** The +15, +30 and +45 chips of the kitchen cards are behind a "Retarder" button that opens a
  dialog with the three choices (one tap on a choice applies it and closes the dialog). The increments stack on the
  current promised time, as in the store (+60 is two taps on +30).
- **One action at a time.** As the `busyId` guard of Vue, a second tap while an action talks to the engine is
  ignored; the buttons of the busy card are disabled. The store adds its own lock per order.
- **Dialogs in the scaffold overlay (PD214).** The dialogs are drawn in the `overlay` slot of `PosScaffold`, inside the
  window, above the header; no `Dialog` window is used.
- **Money and time.** Order totals are the engine's `total_cents` through `formatEuro`; no amount is computed here
  (UI-PD317). The ages ("il y a N min"), the target and ready times are time arithmetic in Europe/Brussels through
  `PosDateTime`; the ages are refreshed every 15 s while the screen is composed.
- **Reading the list.** The list is read when the screen comes to the front with a user and HubRise visible; the store
  also follows the engine events by itself. Vue reads it on mount.
- **Names.** Item names come from `ProductNames.bilingualName` (UI language first, the other underneath); the Chinese
  line gets the Chinese family whichever line it is on.
