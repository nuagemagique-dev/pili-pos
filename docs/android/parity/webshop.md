# Parity: online orders (webshop)

Port of `ui/src/views/WebshopOrdersView.vue` (card P5b-T11) on the shared state of card P5b-T5
(`ui/state/WebshopOrdersStore.kt`). The screen is a native Compose view of module `:app`, package
`dev.nuagemagique.pilipos.feature.webshop`, laid out for 1920 x 1080 dp. The engine owns every rule (what is signed,
the VAT, the refund, the printed tickets); the screen only reads the records it lists and calls the commands through
the store.

## Rows

| Vue | Compose | Commands |
| --- | --- | --- |
| ui/src/views/WebshopOrdersView.vue (board, cards, actions, delays, prints, resync) | feature/webshop/WebshopScreen.kt (WebshopRoute, WebshopContent), WebshopCard.kt, WebshopDialogs.kt, WebshopViewModel.kt, WebshopRules.kt, WebshopEntryPoint.kt | `hubrise_sync_catalog` |
| stores/webshopOrders.ts (list, refetch on events) | ui/state/WebshopOrdersStore.kt (fetchOrders) | `hubrise_list_webshop_orders` |
| stores/webshopOrders.ts (acceptOrder, rejectOrder) | ui/state/WebshopOrdersStore.kt (acceptOrder, rejectOrder) | `webshop_accept_order` `webshop_reject_order` |
| stores/webshopOrders.ts (startPreparation, markReady, markOutForDelivery, delayByMinutes) | ui/state/WebshopOrdersStore.kt (patchStatus, delayByMinutes) | `hubrise_patch_order_status` `hubrise_set_confirmed_time` |
| stores/webshopOrders.ts (collectOrder, completeOrder, refundOrder, closeCancelledOrder) | ui/state/WebshopOrdersStore.kt (collectOrder, completeOrder, refundOrder, closeCancelledOrder) | `webshop_handover_order` `webshop_complete_order` `webshop_refund_order` `webshop_close_cancelled_order` |
| stores/webshopOrders.ts (printKitchen, printCustomer) | ui/state/WebshopOrdersStore.kt (printKitchen, printCustomer) | `print_webshop_kitchen` `print_webshop_customer` |
| components/OnlineOrdersNavButton.vue | ui/scaffold/AppHeader.kt (online orders button, PendingBadge) over ui/state/WebshopOrdersStore.kt (pendingCount) | - |
| composables/useOrderChime.ts | ui/feedback/OrderChime.kt (OrderChimeOverlay, OrderChimePlayer) | - |

## Differences

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
- **Confirmations (added).** Vue refuses an order, and sends a refund, with one tap. Here "Refuser" asks for a
  confirmation first (the refusal goes to the platform and cannot be undone), and "Rembourser" asks the logged-in
  manager to confirm the engine's amount; the manager's INSZ is the one sent to `webshop_refund_order`. Without a
  manager nothing is sent and the warning `webshop_manager_needed` is shown.
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
