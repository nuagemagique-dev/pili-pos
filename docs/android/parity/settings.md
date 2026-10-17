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
| SettingsView.vue (employees: presence registered by a manager, presence-only profile) | feature/settings/SettingsScreen.kt (EmployeesPanel, WorkerRow), SettingsViewModel.kt (`togglePresence`) over ui/fdm/FdmStore.kt | `clock_in_employee` `clock_out_employee` `list_open_shifts` |
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
- **Presence registered by a manager (lane 1-v1-fixes).** Same behaviour on both screens: the panel loads `list_users`
  and `list_open_shifts` together and reads both again after every action; a manager sees, on each other employee's
  row, "En service depuis HH:MM" (Brussels time of `startedAt`) or "Hors service" and one button, "Arrivée" or
  "Départ" (the operator's own row has none: their presence is the login and "Fin de service"). Success shows
  "Arrivée enregistrée" or "Départ enregistré" with the name; `success: false` shows the FDM error with the FDM
  texts, a refusal code its `err_` text. The buttons are off, with a note under the panel text, while the FDM store
  says not connected (`settings_presence_fdm_down`) and, with the `err_presence_training` text, while training mode
  is on. A presence-only employee shows "Présence seule", has no "Code PIN" button and keeps "Désactiver"; the add
  form has a fourth profile choice "Présence seule" (`tag settings_role_presence`) that sends role `waiter` with
  `presenceOnly`, and opens no PIN dialog. Test tags: `worker_<insz>_presence` (the state line),
  `worker_<insz>_presence_in` and `worker_<insz>_presence_out` (the button), `settings_presence_note`,
  `settings_presence_block`.
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
- **Badge reader and badges (lane 5-nfc, Vue side).** Managers only (the engine refuses the commands to the others,
  the panel is not drawn for other roles). Vue: `components/BadgeSettings.vue` is a panel at the end of the third column, folded by default (its
  header, tag `badge-toggle`, shows the reader and unfolds it: it must not squeeze the other panels of the column), with `get_badge_settings` / `set_badge_settings` / `list_badges` over `stores/badge.ts`. It
  offers two reader choices, "Désactivé" and "Lecteur USB" (no built-in NFC on Linux); a stored `nfc` value
  selects neither and shows `settings_badge_nfc_unavailable`. With a reader on, the "Connexion par badge" switch and
  its note appear; with the USB reader the three timing fields (gap, minimum and maximum length) are typed on the
  `NumPad` and applied by one "Enregistrer" button (the reader and the switch apply at the tap, each one sends the
  whole `BadgeSettingsInput`). A refusal shows its `err_` text and the stored values come back. The `relinkNeeded`
  banner, the privacy note and the six latest `badge_log` rows (action, name, manager, Brussels time) close the panel.
  Test tags: `badge-settings`, `badge-login-switch`, `badge-wedgeMaxGapMs`, `badge-wedgeMinLen`,
  `badge-wedgeMaxLen`, `badge-timing-save`, `badge-nfc-unavailable`, `badge-relink`, `badge-error`, `badge-log`.
  The Android screen has the same content plus a third choice, "NFC intégré", drawn only when the device reports
  `FEATURE_NFC` (and `settings_badge_nfc_disabled` when the adapter is off).
- **Employees and badges (lane 5-nfc, Vue side).** While the reader is on, a manager sees on every row "Badge lié le
  JJ/MM/AAAA" or "Aucun badge" (`badge-state`) and one button (`badge-btn`): "Lier un badge" or "Retirer le badge".
  Linking opens `BadgeEnrolDialog.vue`: "Présentez le badge de {name}" with "Retour", the next scan is captured (never
  sent as a tap), 30 seconds at most (then the toast `badge_capture_timeout`), then the existing PIN dialog asks for the
  MANAGER's own PIN and `link_badge(employeeInsz, token, operatorPin)` runs; success shows "Badge lié · {name}".
  A wrong PIN keeps the dialog; any other refusal (`badge_in_use`, `badge_already_linked`...) closes it with the error
  toast. Removing asks a confirmation dialog first (`badge-revoke-dialog`, `badge-revoke-confirm`), then
  `revoke_badge` and the toast "Badge retiré · {name}". The list and the log are read again after every change, and
  the employees are read again after a registered scan (`presenceVersion`).
