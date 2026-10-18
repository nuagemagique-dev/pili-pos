# Parity: tables and the staff scaffold

Port of `ui/src/views/TablesView.vue` and of the staff frame it sits in (`AppHeader.vue`, `StatusLights.vue`,
`PiliLogo.vue`, `OnlineOrdersNavButton.vue`, the header part of `TrainingBanner.vue` usage and the leave guard of
`router/index.ts`), card P5b-T6 (was P5-T8 and P5-T10). Native Compose views of module `:app`, packages
`dev.nuagemagique.pilipos.ui.scaffold` and `dev.nuagemagique.pilipos.feature.tables`, laid out for 1920 x 1080 dp. The
engine owns every rule (tables, orders, the registration of an order, the lock, the PIN); the screens only call it
through the shared stores and the gateway.

## Rows

| Vue | Compose | Commands |
| --- | --- | --- |
| ui/src/views/TablesView.vue | feature/tables/TablesScreen.kt (TablesRoute, TablesContent), TablesViewModel.kt, TablesEntryPoint.kt | `list_tables` `list_open_takeaways` |
| TablesView.vue (selectTable, resumeTakeaway, newTakeaway through the order store) | feature/tables/TablesViewModel.kt over ui/state/OrderStore.kt | `create_order` `get_order` `list_cart_lines` `set_active_order` |
| TablesView.vue (move mode: toggleMove, pick source then free target, confirmMove) | feature/tables/TablesScreen.kt (MoveBar, MoveConfirmation), TablesViewModel.kt (MoveState, toggleMove, confirmMove) over ui/state/OrderStore.kt (transferOrder) | `transfer_table` `list_tables` |
| TablesView.vue (`held-course-N` tag, "Suite : Plats") | feature/tables/TablesScreen.kt (TableTop, tag `table_N_held`) | `list_tables` |
| components/AppHeader.vue (own PIN dialog, tap on the user) | ui/scaffold/HeaderViewModel.kt (submitPin), PosScaffold.kt (OwnPinDialog), AppHeader.kt | `set_pin` |
| components/AppHeader.vue (lock, new takeaway, workspaces) and the leave guard of router/index.ts | ui/scaffold/HeaderViewModel.kt, HeaderActions.kt over ui/state/SessionStore.kt and OrderStore.leaveIfActive | `lock_session` `leave_order` |
| components/StatusLights.vue | ui/scaffold/StatusLights.kt (StatusLights, DeviceLights, GatewayLightProbe) | `check_printer` `check_kitchen_printer` `hubrise_status` |
| components/OnlineOrdersNavButton.vue | ui/scaffold/AppHeader.kt (online orders button, PendingBadge) over ui/state/WebshopOrdersStore.kt | - |
| components/PiliLogo.vue | ui/scaffold/AppHeader.kt (PiliLogo) and res/drawable/ui_logo.xml, ui_logo_bolt.xml, ui_logo_tones.xml | - |
| components/FdmAlerts.vue (blocking part, `SALES_ROUTES`) | ui/scaffold/PosScaffold.kt (FdmBlockingVeil over the content of the sales areas) | - |

## Differences

- **No redirect on start.** Vue sends the operator to the login screen when there is no user. The Compose screen
  never navigates when it starts (PD205): without a user it shows an empty state (`tables_empty`). A test proves
  that nothing is navigated while the screen loads with the engine closed.
- **Tables after a login (PD211).** A successful login ends with `navigator.replaceAll(TablesRoutes.TABLES)`; Vue
  pushes the `order` route, whose screen bounces to Tables when there is no order. Tables is therefore the first
  screen of a session, and this screen reads its lists each time it comes back to the front (`ON_RESUME`), because
  the navigation back stack keeps it alive under the order screen where Vue re-creates the view.
- **The scaffold wraps the screen.** `PosScaffold(area, navigator, overlay) { content }` draws the 64 dp header, the
  training banner under it and the content; the FDM blocking veil covers the content of Tables, Order and Checkout
  only (PD209), so Reports, Settings and the online orders board stay reachable. The dialogs of a screen go in the
  `overlay` slot above the header (PD214).
- **Leave guard in the header model.** The guard of `router/index.ts` runs in `HeaderViewModel`: leaving the order or
  checkout screen first calls `OrderStore.leaveIfActive(NAVIGATE)`; a refusal shows `order_register_failed : <error>`
  for 6 s and the operator stays (PD210). The lock bypasses the guard (the engine registers the order itself).
  `New takeaway` registers the order in hand with `SWITCH_ORDER` on every screen, not only on the order screen.
- **New takeaway and Tables on their own screen.** A tap on Tables while on Tables, and on New takeaway while a
  takeaway is open on the order screen, do nothing, as the Vue handlers. A second tap while an action runs is ignored.
- **HubRise light.** The HubRise light is shown and checked while HubRise is visible (switched on, or online orders
  still to settle), the condition of the online orders button; Vue ties the light to the switch alone.
- **Order of the lights.** The printers are checked every 30 s and HubRise every 60 s only while the header is on
  screen and the engine is ready; with the engine closed every light is grey and nothing is polled.
- **Logo.** The wordmark is converted from the paths of `PiliLogo.vue` into three vector drawables (ink, bolt, tone
  marks). A vector has no mask, so the gap the bolt cuts in the p is the bolt drawn over the ink in the header colour.
- **Online orders badge.** Amber with the count, as `.nav-pending-badge`, pulsing once (scale 1.3, 180 ms) when the
  count goes up; the first count of a screen is a starting point and does not pulse.
- **Money and time.** Table and takeaway amounts are the engine's `total_cents` through `formatEuro`; the minutes
  open and the start time are time arithmetic in Europe/Brussels through `PosDateTime`. No amount is computed here.
- **Fonts and sizes.** Text sizes use the theme tokens only; the 44 px table number is the largest token times 1.5.
  The dashed outline of a free table is drawn with a dash path effect.
