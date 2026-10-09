# Parity: order, menu part

Port of `ui/src/views/OrderView.vue`, `ui/src/stores/menu.ts` and the menu components `CategoryTabs.vue`,
`MenuGrid.vue`, `MenuItemButton.vue`, `ModifierDialog.vue` and `PriceDialog.vue` (already ported as
`ui/components/AmountDialog.kt`), card P5b-T7 (was P5-T11). Native Compose views of module `:app`, package
`dev.nuagemagique.pilipos.feature.order`, laid out for 1920 x 1080 dp. The engine owns every rule (the catalog, the
option groups and their limits, the price of a line, the VAT, the total); the screen only calls it through the shared
stores and the gateway.

`CartSidebar.vue` and `OrderLineItem.vue` come with P5b-T12: until then the cart column is the minimal
`OrderCartColumn` (lines, engine total, pay button), the seam that card replaces.

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
