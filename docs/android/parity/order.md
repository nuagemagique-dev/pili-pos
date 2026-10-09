# Parity: order

Port of `ui/src/views/OrderView.vue`, `ui/src/stores/menu.ts` and the menu components `CategoryTabs.vue`,
`MenuGrid.vue`, `MenuItemButton.vue`, `ModifierDialog.vue` and `PriceDialog.vue` (already ported as
`ui/components/AmountDialog.kt`), card P5b-T7 (was P5-T11), and of the cart `CartSidebar.vue`, `OrderLineItem.vue` and
`VatSummary.vue`, card P5b-T12 (was P5-T12). Native Compose views of module `:app`, package
`dev.nuagemagique.pilipos.feature.order`, laid out for 1920 x 1080 dp. The engine owns every rule (the catalog, the
option groups and their limits, the price of a line, the VAT, the total, the unsent quantities, the registration when
an order is sent or left, the closing unpaid); the screen only calls it through the shared stores and the gateway.

## Rows

| Vue | Compose | Commands |
| --- | --- | --- |
| ui/src/views/OrderView.vue | feature/order/OrderScreen.kt (OrderRoute, OrderContent, OrderDialogs), MenuViewModel.kt, OrderState.kt, OrderEntryPoint.kt, OrderEntry.kt | `needs_z_report` |
| OrderView.vue (handleAddItem, handlePriceConfirmed) over the order store | feature/order/MenuViewModel.kt over ui/state/OrderStore.kt (addItem) | `add_order_line` `update_order_line_quantity` |
| stores/menu.ts | feature/order/MenuViewModel.kt (loadMenu, selectCategory; reload on language, CatalogUpdated, Resync) | `get_categories` `get_menu_items` |
| components/CategoryTabs.vue | feature/order/CategoryTabs.kt | - |
| components/MenuGrid.vue | feature/order/MenuGrid.kt (MenuGrid) | - |
| components/MenuItemButton.vue | feature/order/MenuGrid.kt (MenuItemButton, ItemLabel) over ui/format/ProductNames.kt | - |
| components/ModifierDialog.vue, api/ipc-modifiers.ts | feature/order/ModifierDialog.kt (ModifierDialog, ModifierPicks), MenuViewModel.kt (confirmModifiers over OrderStore.addLineWith) | `get_item_modifiers` `add_order_line_with_modifiers` |
| components/PriceDialog.vue | ui/components/AmountDialog.kt, opened by feature/order/OrderScreen.kt (OrderDialogs) | - |
| components/CartSidebar.vue | feature/order/OrderCartColumn.kt (OrderCartColumn), MenuViewModel.kt (sendToKitchen, printPreBill, askCloseUnpaid, closeUnpaid, unpaidClosed), OrderScreen.kt (CloseUnpaidDialog over ui/components/PinDialog.kt) over ui/state/OrderStore.kt | `leave_order` `sign_pre_bill` `close_unpaid` |
| components/OrderLineItem.vue | feature/order/OrderLineItem.kt (OrderLineItem, lineNotes), MenuViewModel.kt (incrementLine, decrementLine, removeLine) over ui/state/OrderStore.kt | `update_order_line_quantity` `remove_order_line` |
| components/VatSummary.vue | ui/components/VatSummary.kt over ui/format/CartAmounts.kt (vatRows), drawn by feature/order/OrderCartColumn.kt | - |

## Differences

- **No redirect on start.** Vue sends the operator to the login screen without a user, and to Tables when there is no
  current order. The Compose screen never navigates when it starts (PD205): without a user or without an order it
  shows an empty state (`order_empty`). Once the engine is ready and somebody is logged in, the order of the route
  (`order/{orderId}`) is loaded through `OrderStore.loadOrder` when another order, or none, is current. A test hosts
  the route with the engine closed and proves that the root `placeholder_order` is displayed and nothing is navigated.
- **Reloads (UI-PD303).** The Vue menu store is read on mount and reloaded by `reloadWithLanguage` after Settings. The
  menu now reloads by itself when the language of the till changes and on `PosEvent.CatalogUpdated` and `Resync`; the
  selected category is kept, and the first one is selected when it is gone.
- **Z reminder.** `needs_z_report` is checked when the screen opens and every 5 minutes while it is composed; a
  failure leaves the banner as it was and shows nothing.
- **Options only with HubRise on.** As Vue, a product asks for its options only when the HubRise switch is on; a
  product without option groups is added plainly. A free-price product never asks for options.
- **No total in the options dialog (PD208, PD219).** Vue previews `price + sum of the picked options`. The Compose
  dialog shows the product price and the own price of each option and no total; the engine prices the line
  (`add_order_line_with_modifiers` sends only ids). The confirm caption is the plain "Ajouter".
- **Every tap counts.** As in `OrderView.vue`, each tap on a product (and each confirmed price or option pick) is sent
  on; `OrderStore` runs the cart actions one after the other, so quick taps queue up and merge into the same line.
  A refusal of a cart action (the order was paid meanwhile, a product is no longer available) is shown as an error
  toast by the order screen, which listens to `OrderStore.failures`.
- **Sizes and flash.** Text sizes use the theme tokens only (the Vue 17, 13 and 24 px names become 16, 14 and 20); the
  200 ms confirmation flash of a tapped button is not reproduced, the quantity badge shows the change. The product
  grid rows are not stretched to the tallest cell.
- **Category colour.** The `#RRGGBB` colour of a category is parsed by `parseCategoryColor`; a missing or malformed
  value gives the neutral border colour.
- **Money.** Prices are the engine's `price_cents` through `formatCents` and `formatEuro`; the cart total is
  `Order.totalCents` and a line amount comes from `CartAmounts`. No amount is computed here.

## Differences of the cart (P5b-T12)

- **One model for the screen.** The cart actions live in `MenuViewModel` next to the menu ones: the Vue component
  talks to the order store directly, the Compose column is stateless (`OrderCartState`, `OrderCartActions`) and the
  model turns each tap into an `OrderStore` call. Every action is ignored while another order than the route's is
  current (PD205).
- **Quantity and removal.** As Vue (UX guide 4.3) the minus stops at 1 and is disabled there; the delete control, set
  apart on the right, removes the line at any quantity (`OrderStore.removeLine`). The plus and the minus send the new
  quantity (`line.quantity` plus or minus one, a count) to `OrderStore.updateQuantity`.
- **Amounts (PD208).** A line shows `CartAmounts.lineAmount` (its total, VAT included, from the engine VAT split),
  not `unit_price_cents * quantity` as Vue does; a line the split leaves out (code X) shows `quantity × unit price`
  as text. The total is `Order.totalCents`, the same figure on the pay button, and the VAT included are the rows of
  `CartAmounts.vatRows` through `VatSummary`. The unsent count is `OrderStore.unsentCount`. No amount is computed.
- **Unsent lines.** On a dine-in order the lines with an unsent quantity go last under the amber label; a takeaway
  shows no label, no send button and no bill button, as Vue.
- **Send to kitchen.** `OrderStore.leave(LeaveReason.SEND_TO_KITCHEN)`, disabled when nothing is unsent and while it
  runs. Success is a toast "Envoyé en cuisine · n articles"; a refusal is the error toast prefixed with
  `order_register_failed` (6 s), through `ErrorPresenter`. A slip that did not print is the store's `slipFailures`.
- **Provisional bill.** `OrderStore.preBill()` for a dine-in order with lines, hidden without the payment right; the
  same two toasts as Vue (`pre_bill_printed`, `pre_bill_failed` prefix).
- **Close unpaid.** The outlined button shows for a manager (administrator) on an open order with lines. It opens the
  shared `PinDialog` in the order window (the hint is the manager's name and `unpaid_hint`); the 4th digit calls
  `OrderStore.closeUnpaid(insz, pin)`, a refusal is shown on the pad by `ErrorPresenter` and the pad is cleared. On
  success the toast `unpaid_closed` is shown and the screen opens the tables, as Vue does.
- **Pay.** The pay button exists only with the payment right (`SessionStore.canTakePayment`), is disabled without a
  line, and opens the checkout of the order of the route; the screen never navigates by itself.
- **Opened at.** "12:00 · 37 min" next to the title is refreshed every 30 s while the cart is composed (as Vue); the
  clock is replaceable (`LocalOrderNow`) so that the goldens are stable.
- **Sizes and flash.** Text sizes use the theme tokens (the 40 px total becomes the 28 sp `sizeXl`); the green flash
  of a new line is not reproduced (the quantity badge of the product button shows the change).
