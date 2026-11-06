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
| components/AllergenBadges.vue, OrderView.vue (Allergènes mode, hint, legend) | feature/order/AllergenBadges.kt, ModeToolbar.kt (Allergènes button, hint, AllergenLegend), MenuViewModel.kt (toggleAllergenMode), MenuGrid.kt, ModifierDialog.kt | `list_allergens` |
| components/AllergenSheetDialog.vue, stores/allergens.ts | feature/order/AllergenSheetDialog.kt, MenuViewModel.kt (openAllergenSheet, printAllergenSheet) | `get_allergen_sheet` `print_allergen_sheet` |
| components/CartSidebar.vue (course groups, `course-label`, "Envoyer la suite"), components/OrderLineItem.vue (course chip), components/CoursePickerDialog.vue | feature/order/OrderCartColumn.kt (CourseHeader, NextCourseButton), OrderLineItem.kt (CourseButton in the detail of the selected line, `_course_tag` chip on the row), CoursePickerDialog.kt, MenuViewModel.kt (openCoursePicker, pickCourse, sendNextCourse) over ui/state/OrderStore.kt | `set_line_course` `list_held_courses` `send_next_course` |
| OrderView.vue ("Bon cadeau" button and amount dialog), components/OrderLineItem.vue (voucher badge, no quantity buttons, no course chip) | feature/order/ModeToolbar.kt (`gift_voucher_sell`), MenuViewModel.kt (askGiftVoucher, confirmGiftVoucher), OrderScreen.kt (`OrderDialog.SellVoucher` over AmountDialog), OrderLineItem.kt (VoucherBadge) over ui/state/OrderStore.kt (addGiftVoucher) | `get_gift_voucher_settings` `add_gift_voucher_line` |
| components/CodePad.vue, utils/codeEntry.ts, OrderView.vue (code pad: codeResults, codePick, submitCode, handleGridTap, selectCategory, `pili-pos-code-pad`), MenuGrid.vue (highlightId, emptyText) | feature/order/CodePad.kt (the 300 dp dock under the grid, C6), CodePadField.kt, CodeEntry.kt, CodeSearch.kt, CodePadFlow.kt (toggle, type, submit, tap, leaveResults), MenuGrid.kt (highlightId, emptyText), MenuViewModel.kt (tapItem, selectCategory), ui/state/PosSettingsStore.kt (codePadOpen) over ui/state/OrderStore.kt | `get_menu_items` `add_order_line` |
| components/ModeToolbar.vue, components/UiIcon.vue, utils/icons.ts, OrderView.vue (`<ModeToolbar>`: the one 56 px row of mode buttons under the category tabs, UX audit C1; Material icons) | feature/order/ModeToolbar.kt (ModeToolbar, ModeToolbarActions, ModeButton, ModeHint, AllergenLegend; `Icons.Filled.Block` `Dialpad` `NoFood` `CardGiftcard` `PlaylistRemove` are the Vue icons `block` `dialpad` `no_food` `card_giftcard` `playlist_remove` of `@material-design-icons/svg`), OrderScreen.kt (OrderContent) | - |
| utils/codeEntry.ts (`stepDigitWindow`, `DIGIT_WINDOW_MS`), CodePad.vue (`takeKey`), OrderView.vue (`addPlain`, `takePadKey`: the digit after an add = quantity, C7) | feature/order/CodeEntry.kt (DigitWindow, stepDigitWindow), CodePadFlow.kt (`openWindow`, `closeWindow`, `takeDigit`; CodeMessage.Counted), CodePadField.kt, CodePad.kt, MenuViewModel.kt (`addPlain`, host `addUnits`) over ui/state/OrderStore.kt (`addProduct`, `addUnits`) | `add_order_line` `update_order_line_quantity` |
| stores/order.ts (`lastAdd`), components/OrderLineItem.vue (`flash`, `.flash-added` 200 ms), CartSidebar.vue (`flashFor`): the flash of the line an add reached (C9) | ui/state/OrderStore.kt (`lastAdd`, LineAdd), feature/order/OrderState.kt (OrderCartState.lastAdd), MenuViewModel.kt, OrderLineItem.kt (`cart_line_<id>_flash`) | - |
| components/CategoryTabs.vue | feature/order/CategoryTabs.kt | - |
| components/MenuGrid.vue | feature/order/MenuGrid.kt (MenuGrid) | - |
| components/MenuItemButton.vue | feature/order/MenuGrid.kt (MenuItemButton, ItemLabel) over ui/format/ProductNames.kt | - |
| components/ModifierDialog.vue (`forWeight`: "Peser"), api/ipc-modifiers.ts | feature/order/ModifierDialog.kt (ModifierDialog, ModifierPicks, `forWeight`), MenuViewModel.kt (confirmModifiers over OrderStore.addLineWith, or WeighFlow.start for a bowl) | `get_item_modifiers` `add_order_line_with_modifiers` |
| components/WeighDialog.vue, OrderView.vue (weigh, weighChosen), OrderLineItem.vue (bowl options, `bowl-option` row) | feature/order/WeighDialog.kt (WeighFlow.ready, start with modifier ids, BowlLink), MenuViewModel.kt (cancelWeighing), OrderLineItem.kt (bowl options, `_bowl`) | `scale_start_weighing`, `scale_cancel_weighing` |
| components/PriceDialog.vue | ui/components/AmountDialog.kt, opened by feature/order/OrderScreen.kt (OrderDialogs) | - |
| components/CartSidebar.vue (contextual primary, `utils/cartPrimary.ts`) | feature/order/OrderCartColumn.kt (OrderCartColumn, KitchenPrimaryButton, PaySecondaryButton), OrderState.kt (OrderCartState.kitchenPrimary, sendableLineCount), MenuViewModel.kt (sendToKitchen, printPreBill, askCloseUnpaid, closeUnpaid, unpaidClosed), OrderScreen.kt (CloseUnpaidDialog over ui/components/PinDialog.kt) over ui/state/OrderStore.kt | `leave_order` `sign_pre_bill` `close_unpaid` |
| components/CartMoreMenu.vue (the cart header's "Plus" menu: Changer de table, Remise, Client parti sans payer) | feature/order/CartMoreDialog.kt (CartMoreDialog), OrderCartColumn.kt (cart_more), OrderState.kt (OrderDialog.CartMore, hasMoreActions), MenuViewModel.kt (openCartMore), OrderScreen.kt (OrderDialogs) over ui/state/OrderStore.kt | - |
| components/TablePickerDialog.vue, TableMover.vue and CartSidebar.vue ("Déplacer", a row of the Plus menu; UX audit C8: every other table, busy ones dimmed with their total, a free one moves the order at once, toast with "Annuler") | feature/order/TablePickerDialog.kt, MenuViewModel.kt (askTransfer, confirmTransfer), CartMoreDialog.kt (cart_transfer) over ui/state/TableMoves.kt and OrderStore.kt (transferOrder) | `transfer_table` `list_tables` |
| components/CartSidebar.vue (UX audit T5: the covers button of the cart header, "Couverts" / "1 couvert" / "4 couverts", a pad from 1 to 30 and 0 to clear; the title reads "Table A-4" with zones, UX audit T1) and stores/order.ts (`covers`, `loadCovers`, `setCovers`) | feature/order/OrderCartColumn.kt (CartHeader, `cart_covers`, `cartTitle`), OrderState.kt (OrderDialog.Covers, `canSetCovers`, `COVERS_MAX`), OrderScreen.kt (the NumberKeypad tagged `covers`), MenuViewModel.kt (askCovers, saveCovers), ui/state/OrderStore.kt (`covers`, `setCovers`) | `get_order_covers` `set_order_covers` |
| components/DiscountDialog.vue and CartSidebar.vue (Remise, Offrir, Retirer la remise) | feature/order/DiscountDialog.kt (DiscountDialog, DiscountEntry), MenuViewModel.kt (askDiscount, chooseDiscount, askRemoveDiscount, confirmDiscountPin, discountDone), CartMoreDialog.kt (cart_discount, cart_discount_remove: rows of the Plus menu), OrderScreen.kt (DiscountPinDialog over ui/components/PinDialog.kt) over ui/state/OrderStore.kt (applyDiscount, removeDiscount) | `apply_discount` `remove_discount` |
| components/OrderLineItem.vue (the 56 px row: kitchen mark, "2×", code, name, chips, price; the detail of the selected line: category, options, whole note, stepper, Note, Offrir, course, Supprimer), components/CartSidebar.vue (`choice`, `newestLineId`, `selectedLineId`, `selectLine`: the dense cart, C2) | feature/order/OrderLineItem.kt (OrderLineItem, LineRow, RowTitle, LineDetail, LineActions, lineNotes), CartLines.kt (rememberCartSelection: newest line by default, tap selects or folds, an add selects its line), MenuViewModel.kt (incrementLine, decrementLine, removeLine) over ui/state/OrderStore.kt | `update_order_line_quantity` `remove_order_line` |
| components/NoteDialog.vue (over TextKeypad.vue, the quick note chips of C11: `chips`, `appendQuickNote`), utils/quickNotes.ts, stores/order.ts (`loadQuickNotes`), CartSidebar.vue (`openNote`) and OrderLineItem.vue (Note button, note on the row) | feature/order/NoteDialog.kt (NoteDialog, NoteChips, noteText on the system keyboard), QuickNotes.kt (appendQuickNote, noteHasChip), MenuViewModel.kt (askNote reads the chips, saveNote), OrderLineItem.kt (cart_line_<id>_note, cart_line_<id>_kitchen_note), OrderScreen.kt (OrderDialogs) over ui/state/OrderStore.kt (quickNotes, setLineNote; addItem opens a new line when the line has a note) | `set_line_note` `get_quick_notes` |
| components/SoldOutDialog.vue, components/SoldOutList.vue, utils/soldOut.ts, the "Épuisé" and "Produits épuisés" buttons of ModeToolbar.vue, stores/menu.ts (loadSoldOut, setAvailability) | feature/order/SoldOutDialog.kt (SoldOutDialog, SoldOutList, endOfDayLabel), MenuViewModel.kt (toggleSoldOutMode, openSoldOutList, setAvailability, the sold-out branches of addItem), OrderState.kt (OrderDialog.SoldOut, SoldOutList, SoldOutChoice) | `list_sold_out` `set_sold_out` `clear_sold_out` |
| components/VatSummary.vue | ui/components/VatSummary.kt over ui/format/CartAmounts.kt (vatRows), drawn by feature/order/OrderCartColumn.kt | - |

## Differences

- **Quick notes and the dense cart (C2, C11).** The chips sit above the keys on both UIs (the Compose `TextKeypad` is the port of
  `TextKeypad.vue`, C13); the rules are the same pure functions on both sides
  (`utils/quickNotes.ts`, `QuickNotes.kt`, same test cases). The selected line is the newest one by default on both UIs, also when an
  existing order opens. The category is shown in the detail of the selected line only.

- **Allergens.** `set_allergens` (manager edit of a product's list) has no screen on either UI in this release; the
  product editor of lane 3 will call it. Both UIs read the lists through the menu items (`allergens`, `traces`,
  `allergens_declared`) and draw the legend of the 14 numbers (Vue from `list_allergens`, Compose from the Vue strings
  `allergen_<code>`, the same texts).

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
- **Options when the product has some (V11-D18).** As Vue, a product asks for its options when the engine says it has
  some (`MenuItem.has_modifiers`: at least one active group with an active option), whatever the HubRise switch; a
  product without options is added plainly. A free-price product never asks for options.
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
- **Contextual primary (C4).** While a dine-in order has lines the kitchen has not received (unsent quantity, not
  `kitchenState` "held", not a bowl option: `OrderCartState.sendableLineCount`, as `utils/cartPrimary.ts`), "Envoyer en
  cuisine" is the Volt primary (80 dp, tag `cart_send_primary`) with a pill "2 lignes" (`cart_send_pill`, lines not
  units) and PAYER is an outlined secondary (`cart_pay_secondary`, still showing the total) next to "Addition". Once
  everything is sent PAYER is the Volt primary again (`cart_pay`) and "Envoyer en cuisine" the plain secondary
  (`cart_send`, amber unit count). A role without the payment right (Apprenti) always has the kitchen button as the
  primary and no secondary row. A takeaway keeps PAYER. PAYER with unsent lines goes to the checkout as before (the
  order is registered on leave, no dialog).
- **Plus menu (C3).** The cart header has a "Plus" button (icon "...", 48 dp, `cart_more`) shown only when the order
  has at least one of: change table, order discount (or its removal), customer left without paying
  (`OrderCartState.hasMoreActions`, the same role rules as before). It opens `OrderDialog.CartMore` (a dialog,
  `CartMoreDialog`, rows of 64 dp); a row replaces the dialog by its own (table picker, discount dialog, manager PIN).
  The footer no longer has the discount and unpaid buttons, the header no longer has "Changer de table".
- **Close unpaid.** The outlined row of the Plus menu shows for a manager (administrator) on an open order with lines. It opens the
  shared `PinDialog` in the order window (the hint is the manager's name and `unpaid_hint`); the 4th digit calls
  `OrderStore.closeUnpaid(insz, pin)`, a refusal is shown on the pad by `ErrorPresenter` and the pad is cleared. On
  success the toast `unpaid_closed` is shown and the screen opens the tables, as Vue does.
- **Pay.** The pay button exists only with the payment right (`SessionStore.canTakePayment`), is disabled without a
  line, and opens the checkout of the order of the route; the screen never navigates by itself.
- **Opened at.** "12:00 · 37 min" next to the title is refreshed every 30 s while the cart is composed (as Vue); the
  clock is replaceable (`LocalOrderNow`) so that the goldens are stable.
- **Sizes and flash.** Text sizes use the theme tokens (the 40 px total becomes the 28 sp `sizeXl`); the green flash
  of a new line is not reproduced (the quantity badge of the product button shows the change).

## Differences of the till discount (T4)

- **Who and when.** As Vue: the "Remise" button shows for a manager on an open order with lines; an order discount
  turns it into "Retirer la remise". "Offrir" shows on a line without change while no order discount applies;
  "Retirer la remise" shows on a line that carries its own (LINE scope) change. The engine re-checks every rule.
  An online order (any line of the cart with `online`, read from the order by `list_cart_lines`) shows none of these
  actions on either UI (the engine refuses with `discount_online_order`); Vue test "is absent on an online order",
  Kotlin test `anOnlineOrderGetsNoDiscountActionEvenForAManager`.
- **Reason text.** The optional reason is typed on the on-screen text keypad on both UIs (`TextKeypad.vue`, Compose
  `TextKeypad` with tag `discount_text`; a tap on the reason button opens it, 120 characters at most).
- **Net amounts without arithmetic (PD208).** A line amount is the `vatSplit` row of the line plus one more line of
  quantity 1 whose unit price is the line's `discount_cents` (negative), so the engine sums them; the VAT summary is
  the same split over all lines. The order total is `Order.totalCents`, which is already net. Vue adds
  `unit_price_cents * quantity + discount_cents` itself (`utils/cartAmounts.ts`).
- **Preview.** The "Total old -> new" preview of the dialog is `CashDisplay.totalAfterDiscountCents` (a display-only
  function with the marker of `p5_gate.sh`, like the checkout shares); the engine recomputes and signs the discount.
- **Quantity rule.** `OrderStore.addItem` opens a new line when the existing line has a price change, as Vue's
  `addItem`: the engine refuses a quantity change on a discounted line.
- **Registration warning.** When the P that registers the change could not be signed the engine still keeps the
  discount; both screens show the warning `discount_registration_failed` (Vue toast, Compose
  `OrderStore.discountWarnings`).

## Differences of the sold-out switch (V11-D16)

- **Same behaviour.** An "Épuisé" button above the product grid (cashier and manager only) turns a mode on: a tap on a
  product opens its availability dialog (until the end of the day, until re-enabled, available) instead of adding it.
  Out of the mode a sold-out product is greyed with an "Épuisé" badge and a tap shows the refusal toast
  (`err_product_sold_out`) without adding anything; the grid is read again in case the end of the day has passed.
  "Produits épuisés (n)" lists the sold-out products and puts one back on sale; the list stays open. No long press.
- **Refusal.** A refused change (the engine answers `forbidden`, for example) is shown as an error toast and the
  availability dialog stays open, as in Vue. A choice equal to the current state just closes the dialog.
- **Role.** `SessionStore.canTakePayment` gates the bar and the sold-out reads (the engine refuses the others); a
  waiter still sees the greyed products and the refusal.
- **End of day label.** `18/10 04:00` in Brussels time: `endOfDayLabel` over `PosDateTime` (Vue: `utils/soldOut.ts`
  over `utils/datetime.ts`), whatever the device zone. The engine computes the end of the day; neither UI does.
- **Reads.** The list is read with the menu (language change, `CatalogUpdated`, `Resync`) and again when the list opens;
  every change returns the new list, and the grid is read again.
