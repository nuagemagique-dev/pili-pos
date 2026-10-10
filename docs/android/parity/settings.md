# Parity: settings

Port of `ui/src/views/SettingsView.vue` and of the parts it hosts (`IntegrationSwitch.vue`, `HubriseSettings.vue`,
`HubriseConnectDialog.vue`, the PIN dialogs of `PinDialog.vue`), card P5b-T10. Native Compose views of module
`:app`, package `dev.nuagemagique.pilipos.feature.settings`, laid out for 1920 x 1080 dp. The engine owns every rule
(who may create a user, the PIN checks, the refusal to switch HubRise off while online orders are unsettled); the
screen only calls it through the shared stores and the gateway.

## Rows

| Vue | Compose | Commands |
| --- | --- | --- |
| ui/src/views/SettingsView.vue (employees: list, add, deactivate, PIN) | feature/settings/SettingsScreen.kt (SettingsRoute, SettingsContent, SettingsDialogs), SettingsViewModel.kt | `list_users` `create_user` `deactivate_user` `set_pin` |
| SettingsView.vue (training mode) | feature/settings/SettingsScreen.kt (TrainingPanel) over ui/state/PosSettingsStore.kt | `set_training_mode` |
| SettingsView.vue (table plan, managers) | feature/settings/SettingsScreen.kt (TablePlanPanel), SettingsViewModel.kt | `get_table_plan` `set_table_plan` |
| SettingsView.vue (ticket deliveries and the mail status) | feature/settings/SettingsScreen.kt (DeliveriesPanel) over ui/state/DeliveryStore.kt | `list_ticket_deliveries` `check_mail_status` |
| ui/src/components/IntegrationSwitch.vue | feature/settings/SettingsScreen.kt (IntegrationPanel) over ui/state/IntegrationsStore.kt | `set_hubrise_enabled` |
| ui/src/components/HubriseSettings.vue (connection, health, catalog, adoption, channels) | feature/settings/HubriseSection.kt, HubriseViewModel.kt | `hubrise_get_connection_status` `hubrise_status` `hubrise_disconnect` `hubrise_sync_catalog` `hubrise_adoption_preview` `hubrise_adopt_catalog` `hubrise_list_channels` `hubrise_set_channel` |
| ui/src/components/HubriseConnectDialog.vue | feature/settings/HubriseConnectDialog.kt, HubriseViewModel.kt | `hubrise_start_oauth_flow` `hubrise_complete_oauth` |

## Differences

- **Remote support is not ported.** `SettingsView.vue` has a remote support column (`SupportBanner`, the support store,
  the PIN confirmation to start it). The Android till has no remote support in v1, so none of it is drawn and no
  support command is called.
- **The rows section (UI-PD308).** The third area of the screen is new: the settings rows that other features
  contribute through the Hilt set (`NavRegistry.settingsEntries`, sorted by order then id, manager only rows hidden
  from other roles). Each row is a button tagged `settings_row_<id>` and a tap calls `navigator.navigate(row.route)`;
  the screen imports nothing from `platform/` and names no platform route. It takes the place of the remote support
  panel in the first column, and comes first on a window narrower than 900 dp.
- **Employees are for managers.** The employees panel (list, add form, PIN and deactivate buttons) is drawn for a
  manager only; Vue draws it for any role and lets the engine refuse. The other panels are drawn for every role.
- **Deactivation asks in a dialog.** Vue arms the button for a second tap. The view model keeps the two taps (the
  first arms, the second deactivates) and the screen shows the armed state as a `ConfirmDialog` naming the employee.
- **PIN dialogs.** The three Vue `PinDialog`s become the `PinDialog` of `ui/components/` in the scaffold's overlay
  slot: own PIN typed twice and saved at once, another employee's or a new manager's chosen first and then confirmed
  with the manager's own PIN (`Verify`, hint `pin_confirm_manager`). A refusal is shown in the dialog.
- **Disconnect asks first.** Vue disconnects at the tap; the Android view model asks for a confirmation
  (`ConfirmDialog`, destructive) before it forgets the HubRise connection.
- **Connect dialog and QR (PD218).** The HubRise login link is drawn as a QR code from `EngineHelpers.qrMatrix`
  through `QrMatrices` and `QrCode`, never from the engine's `qr_svg` (the Vue `v-html`). The code HubRise shows is
  typed in a text field and confirmed with the button or the keyboard's done key; a blank code is refused. The link
  is not opened in a browser (the till has none).
- **Channel selects.** The two `select` elements of a channel (kind, service charge) are two segmented buttons.
- **No redirect and no start-up call.** The screen renders with the engine closed (no user, no data), never
  navigates when it starts, and reads the mail status once when it opens.
- **Times.** Delivery dates, the switch log and the last catalog sync are engine strings (UTC) shown in Europe/Brussels
  through `PosDateTime`.
- **Layout.** Three columns on a window of 900 dp or more (training, language, product label and the rows; the
  employees; the information, the HubRise switch, the HubRise panel and the deliveries), a single scrolling column
  otherwise. The third column scrolls as a whole instead of each panel growing and scrolling inside.
