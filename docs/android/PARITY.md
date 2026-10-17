# Parity index (Vue to Compose)

Owner: `ui-common` (see `LANES.md`). Maintained by the P5 and P5b workers (P5b-T12 closed it); the checker and the P5 gate read it.

## Purpose

One row per Vue component: the Vue file, the Compose code that replaces it, and the engine commands it calls. The Compose UI is a port of the Vue UI (plan section P5), so that every screen, every state and every engine command of the Linux build has a counterpart on the Sunmi D3 Pro. A view is finished when its parity file lists all its Vue components and every command of its rows is called by Compose code.

Paths in the Compose column are relative to `android/app/src/main/kotlin/dev/nuagemagique/pilipos/`. A `-` in the Commands column means the component calls no engine command itself.

## How it is checked

`scripts/android/check_ui_parity.py` (run by `scripts/android/p5_gate.sh`, check `ui-parity`) builds the command set from `ui/src/api/ipc.ts` and `ui/src/api/ipc-modifiers.ts` and prints one line per command:

| Line | Meaning |
| --- | --- |
| `CALLED` | the command is called in a `.call { ... }` lambda of the engine gateway; the line lists `file:line` |
| `EXCLUDED` | declared `excluded` below, with a note; calling it from Compose is an error |
| `PENDING` | declared `pending:<owner>` below; the owner is the card that has to call it |
| `MISSING` | neither called nor declared; always an error |
| `(stale pending:<owner>)` | called although still declared pending; delete the declaration |

Every command must also be a method of the `PosEngineInterface` block of `android/pos-core/api/pos-core.api`. The tables with the header `| Vue | Compose | Commands |`, here and in `docs/android/parity/*.md`, may only name known commands.

The owners of a pending command are the P5 cards (`P5-T<n>`) and the P5b view workers (`P5b-checkout`, `P5b-reports`, `P5b-settings`, `P5b-webshop`). A worker that implements a command removes its declaration line in the same commit. P5b-T12 deleted the last `pending:` lines: since the P5b exit every command is called or excluded (73 called, 9 excluded).

The default run passes while commands are pending. The P5b exit and every later gate run with `--parity-strict` (PD212, UI-PD315): `check_ui_parity.py --strict` fails on any pending or stale command and on any table token that is not called.

## Views

| View | Owner | Parity file |
| --- | --- | --- |
| login | P5-T9 | [parity/login.md](parity/login.md) |
| tables | P5b-T6 | [parity/tables.md](parity/tables.md) |
| order | P5b-T7 and P5b-T12 | [parity/order.md](parity/order.md) |
| checkout | P5b-T8 | [parity/checkout.md](parity/checkout.md) |
| reports | P5b-T3 and P5b-T9 | [parity/reports.md](parity/reports.md) |
| settings | P5b-T4 and P5b-T10 | [parity/settings.md](parity/settings.md) |
| webshop | P5b-T5 and P5b-T11 | [parity/webshop.md](parity/webshop.md) |

## Shared components (ui-common)

Run A (P5) and the P5b ui-common pieces (FDM, scaffold, delivery, webshop orders, chime). The view-specific rows are in `parity/*.md`.

| Vue | Compose | Commands |
| --- | --- | --- |
| components/NumPad.vue | ui/components/NumPad.kt (NumPad) | - |
| components/PinDialog.vue | ui/components/PinPad.kt (PinPadState, PinPad, PinDialog) | - |
| dialog frames | ui/components/PiliDialog.kt (PiliDialog, ConfirmDialog) | - |
| components/PriceDialog.vue | ui/components/AmountDialog.kt | - |
| stores/toast.ts and components/ToastStack.vue | ui/feedback/ToastStore.kt, ToastOverlay.kt | - |
| utils/errorMessages.ts callers | ui/feedback/ErrorPresenter.kt | - |
| QR images | ui/components/QrCode.kt (QrCanvas, QrMatrices, QrCode) | - |
| stores/pos.ts | ui/state/PosSettingsStore.kt | `get_settings` `set_training_mode` |
| stores/auth.ts | ui/state/SessionStore.kt | `login` `login_with_new_pin` `logout` `lock_session` `end_shift` |
| stores/integrations.ts | ui/state/IntegrationsStore.kt | `get_integrations` `set_hubrise_enabled` |
| i18n.ts (useI18n, language) | ui/text/UiText.kt (PosUiRoot, LocalUiLanguage, vueText, uiText) | - |
| stores/order.ts | ui/state/OrderStore.kt | `create_order` `get_order` `list_cart_lines` `set_active_order` `add_order_line` `remove_order_line` `update_order_line_quantity` `leave_order` `sign_pre_bill` `close_unpaid` `list_open_orders` |
| utils/names.ts | ui/format/ProductNames.kt | - |
| stores/fdm.ts | ui/fdm/FdmStore.kt | `fdm_health` `fdm_check_now` |
| components/FdmAlerts.vue | ui/fdm/FdmBanner.kt (FdmBannerOverlay), ui/fdm/FdmBlockingVeil.kt | - |
| App.vue listeners (presence-failed, fdm-sign-messages) and order.ts reported()/warnIfSlipFailed | ui/fdm/UiEventToasts.kt | - |
| utils/format.ts | ui/format/Money.kt | - |
| utils/datetime.ts | ui/format/PosDateTime.kt | - |
| components/VatSummary.vue and the line amount of components/OrderLineItem.vue | ui/format/CartAmounts.kt, ui/components/VatSummary.kt | - |
| components/TrainingBanner.vue and the App.vue watermark | ui/components/TrainingBanner.kt, TrainingWatermarkOverlay | - |
| components/ConfigProblemsBanner.vue | ui/components/ConfigProblemsBanner.kt | - |
| components/AppHeader.vue, PiliLogo.vue, OnlineOrdersNavButton.vue | ui/scaffold/AppHeader.kt (AppHeader, PiliLogo), PosScaffold.kt, HeaderViewModel.kt | `set_pin` |
| views/TablesView.vue, views/ReportsView.vue (card charged badge, Z warning) | feature/tables/TablesScreen.kt, feature/reports/ReportsDayTab.kt | `list_unsettled_card_payments` |
| components/StatusLights.vue | ui/scaffold/StatusLights.kt | `check_printer` `check_kitchen_printer` `check_payment_terminal` `hubrise_status` |
| stores/delivery.ts | ui/state/DeliveryStore.kt | `check_mail_status` `print_missing_original` |
| components/TicketMediumPicker.vue | ui/components/TicketMediumPicker.kt | - |
| components/EmailKeypad.vue | ui/components/EmailKeypad.kt | - |
| components/DeliveryOutcomePanel.vue | ui/components/DeliveryOutcomePanel.kt | - |
| checkout and refund arithmetic of views/CheckoutView.vue and views/ReportsView.vue (UI-PD306) | ui/format/CashDisplay.kt | - |
| stores/webshopOrders.ts | ui/state/WebshopOrdersStore.kt | `hubrise_list_webshop_orders` `webshop_accept_order` `webshop_reject_order` `hubrise_patch_order_status` `hubrise_set_confirmed_time` `webshop_handover_order` `webshop_complete_order` `webshop_refund_order` `webshop_close_cancelled_order` `print_webshop_kitchen` `print_webshop_customer` |
| composables/useOrderChime.ts | ui/feedback/OrderChime.kt | - |
| router/index.ts leave guard | ui/scaffold/HeaderActions.kt and OrderStore.leaveIfActive | `leave_order` |
| App.vue activity forwarding | foundation kiosk/UserActivityForwarder.kt | `user_activity` |

Not ported: components/SupportBanner.vue and stores/support.ts. There is no remote support in v1 (DECISIONS section 7).

## Declarations

One line per command: `<command> <status> <note...>`. The status is `excluded` or `pending:<owner>`. `user_activity` has no line: the foundation already calls it. Lines starting with `#` and blank lines are ignored.

```ui-parity
start_remote_support excluded no remote support in v1, DECISIONS section 7
stop_remote_support excluded no remote support in v1, DECISIONS section 7
remote_support_status excluded no remote support in v1, DECISIONS section 7
remote_support_running excluded no remote support in v1, DECISIONS section 7
check_fdm_status excluded no Vue caller; fdm_health and fdm_check_now replace it
list_fiscal_events excluded no Vue caller
sign_table_order excluded no Vue caller; leave_order registers the P
work_in excluded no Vue caller; login clocks in and end_shift clocks out since P1b (a manager clocks someone else with clock_in_employee)
work_out excluded no Vue caller; login clocks in and end_shift clocks out since P1b (a manager clocks someone else with clock_out_employee)
clock_in_employee pending:P5b-settings employees panel of Settings, lane 1-v1-fixes T2 (both UIs)
clock_out_employee pending:P5b-settings employees panel of Settings, lane 1-v1-fixes T2 (both UIs)
```
