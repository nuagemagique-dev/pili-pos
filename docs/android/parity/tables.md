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
| TablesView.vue ("Déplacer" mode, UX audit C8: toggleMove, pick the occupied source, a tap on a free table moves at once with the "Annuler" toast, no confirmation dialog) | feature/tables/TablesScreen.kt (MoveBar), TablesViewModel.kt (MoveState, toggleMove, moveTo; reads the plan again on `OrderStore.tableMoves`) over ui/state/TableMoves.kt (move, undo) and OrderStore.kt (transferOrder, tableMoves) | `transfer_table` `list_tables` |
| components/AppHeader.vue + TableMover.vue (the table chip is a 48 px button on the order and checkout screens, opens the table picker, one tap moves the order) | ui/scaffold/AppHeader.kt (TableBadge `header_table`), HeaderViewModel.kt (openTablePicker, pickTable, closeTablePicker), PosScaffold.kt (TablePickerDialog), HeaderActions.kt over ui/state/TableMoves.kt | `transfer_table` `list_tables` |
| TablesView.vue (`held-course-N` tag, "Suite : Plats") | feature/tables/TablesScreen.kt (TableTop, tag `table_N_held`) | `list_tables` |
| components/AppHeader.vue (own PIN dialog, tap on the user) | ui/scaffold/HeaderViewModel.kt (submitPin), PosScaffold.kt (OwnPinDialog), AppHeader.kt | `set_pin` |
| components/AppHeader.vue (lock, new takeaway, workspaces) and the leave guard of router/index.ts | ui/scaffold/HeaderViewModel.kt, HeaderActions.kt over ui/state/SessionStore.kt and OrderStore.leaveIfActive | `lock_session` `leave_order` |
| components/StatusLights.vue (each light a button, 48 dp hit area) | ui/scaffold/StatusLights.kt (StatusLights, DeviceLights, GatewayLightProbe) | `check_printer` `check_kitchen_printer` `hubrise_status` |
| components/DeviceStatusSheet.vue (state in words, reason, last check, "Réessayer maintenant") and utils/deviceStatus.ts | ui/scaffold/DeviceStatusSheet.kt (DeviceStatusSheet, DeviceStatusSheetHost, DeviceSheetState, DeviceStatus), drawn by PosScaffold.kt in the `deviceSheet` slot | `check_printer` `check_kitchen_printer` `check_payment_terminal` `get_scale_status` `hubrise_status` `fdm_check_now` |
| components/OnlineOrdersNavButton.vue | ui/scaffold/AppHeader.kt (online orders button, PendingBadge) over ui/state/WebshopOrdersStore.kt | - |
| components/PiliLogo.vue | ui/scaffold/AppHeader.kt (PiliLogo) and res/drawable/ui_logo.xml, ui_logo_bolt.xml, ui_logo_tones.xml | - |
| components/FdmAlerts.vue (blocking part, `SALES_ROUTES`; the veil starts under the 64 px header) | ui/scaffold/PosScaffold.kt (FdmBlockingVeil over the content of the sales areas, below the header) | - |
| components/IdleLockBanner.vue, stores/auth.ts (`idleLockAt`, `listenIdleLockWarning`) | ui/components/IdleLockBanner.kt (IdleLockBanner, IdleLockBannerContent), drawn by ui/scaffold/PosScaffold.kt under the header; state in ui/state/SessionStore.kt (`idleWarning`) and ui/scaffold/HeaderViewModel.kt | event `idle-lock-warning` |
| components/ToastStack.vue, stores/toast.ts (`showWithAction`, `runAction`: the "Annuler" button of a toast) | ui/feedback/ToastOverlay.kt (ToastStack, ToastOverlay, `toast_action_<id>`), ToastStore.kt (ToastAction, showWithAction, runAction) | - |

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
- **Idle lock warning (UX audit L4).** The engine says 30 s before the idle lock (`idle-lock-warning`, payload
  `lockInMs`; `null` when the operator was active again) and both UIs only count down what they were told, in an amber
  row under the header (and under the training banner): "Verrouillage dans 30 s · touchez l'écran pour rester
  connecté". Any touch is forwarded to the engine as activity, which ends the warning. Vue also hides the row at once
  when it forwards a touch; Compose waits for the engine's event (a few milliseconds). Compose counts down from the
  frame clock, Vue from `Date.now()` every 250 ms. The row takes no tap and sits in the flow, so the header stays usable.
- **One label for the takeaway (UX audit T4).** The header button and the button of the takeaways panel both read
  "À emporter" (`takeaway_new`); `takeaway_start` is gone. The panel title carries the count of sales in progress,
  "À emporter en cours (2)" (tag `takeaways_title`, test id `takeaways-title`), replacing the separate count badge
  (`takeaways_count`). Neither UI has an icon on these buttons, so the label is what is shared.
- **Status light sheet (UX audit L2).** A tap on a light (header or login start-up row) opens one sheet: device name,
  state in words (`status_state_*`), the reason, the time of the last check (`HH:mm:ss`, Belgian time) and "Réessayer
  maintenant", which re-runs that device's check only (`DeviceLights.retry`, `FdmStore.checkNow` for the FDM). Vue keeps
  the `title` tooltip on the terminal and scale lights and adds an `aria-label`; Compose keeps the reason as the
  accessibility description of those two lights, else "name : state". A light is a transparent button laid over its dot
  and label (48 dp), so the dot and the label remain separate nodes. The FDM time of the last check is the time the
  till last took a reading (`FdmStore.checkedAt`, `fdm.checkedAt`).
- **Toasts (UX audit C5).** Both UIs draw the toasts at the top centre, just under the header (64 px / 64 dp), 560
  wide at most, stacked downwards (newest at the bottom), the newest three only. The stack takes no tap. Info,
  success and warning toasts let taps through and go away by themselves; an error toast never expires, shows
  "Touchez pour fermer" and is the only one that takes a tap, which closes it.
- **FDM veil (UX audit L3).** The Vue veil now starts at `top: 64px`, as the Compose one always did (it is drawn in the
  content box under the header): Tables, Rapports, Config and Verrouiller stay reachable while the FDM blocks.
