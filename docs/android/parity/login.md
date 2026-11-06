# Parity: login

Port of `ui/src/views/LoginView.vue` (card P5-T9). The screen is a native Compose view of module `:app`, package
`dev.nuagemagique.pilipos.feature.login`, laid out for 1920 x 1080 dp. The engine owns every rule (clock-in, PIN
checks, end of shift); the screen only calls it through the shared stores and the gateway.

## Rows

| Vue | Compose | Commands |
| --- | --- | --- |
| ui/src/views/LoginView.vue | feature/login/LoginScreen.kt (LoginRoute, LoginContent), LoginBrand.kt, LoginPeople.kt, LoginPinPanel.kt, LoginViewModel.kt, LoginStartupChecks.kt, LoginState.kt, LoginEntryPoint.kt | `list_users` `list_open_shifts` `check_printer` `check_kitchen_printer` `check_mail_status` `hubrise_get_connection_status` |
| stores/auth.ts (login, loginWithNewPin, endShift driven by the tiles, the PIN pad, the INSZ entry and the first PIN) | ui/state/SessionStore.kt | `login` `login_with_new_pin` `end_shift` |
| stores/fdm.ts (checkNow, read by the start-up checks) | feature/login/LoginStartupChecks.kt (StoreFdmProbe over ui/fdm/FdmStore.kt, which makes the call) | `fdm_check_now` |
| LoginView.vue (badge hints, a manager's badge opens the PIN pad, a badge login, the end-of-shift mode mirrored for the scans) | feature/login/LoginViewModel.kt (`observeBadges`), LoginPeople.kt (BadgeHint), LoginPinPanel.kt over ui/badge/BadgeStore.kt | `get_badge_settings` `badge_tap` |
| LoginView.vue (start-up rows are buttons; a tap opens the status sheet, retry runs that check only; `probes` / `retryOpen`) | feature/login/LoginBrand.kt (CheckRow), LoginDeviceSheet.kt, LoginViewModel.kt (openDevice, closeDevice, retryDevice), LoginStartupChecks.kt (runOne) over ui/scaffold/DeviceStatusSheet.kt | `check_printer` `check_kitchen_printer` `check_payment_terminal` `check_mail_status` `hubrise_get_connection_status` `fdm_check_now` |
| components/PinDialog.vue (first login) | ui/components/PinPad.kt (PinDialog in PinMode.Choose), drawn inside the screen layout | - |
| components/PiliLogo.vue | feature/login/LoginBrand.kt (wordmark as text) | - |

## Differences

- **Presence-only staff have no tile (lane 1-v1-fixes).** Both screens drop the users flagged `presence_only` from the
  tiles (kitchen staff never log in; a manager clocks them in and out from Settings). Typing their INSZ in "Autre
  (INSZ)" shows the engine's `presence_only_user` text. When only presence-only users exist, the INSZ entry shows.
- **No redirect on start.** Vue has no redirect here, and the Compose screen adds none either (PD205): it never
  navigates when it starts, only after an action of the operator succeeded. A test proves that no effect is sent
  while the screen loads and while the operator only browses (tile, PIN panel, INSZ entry, language).
- **Tables after a login (PD211).** A successful login, a PIN entered on the 4th digit, the INSZ path and the first
  PIN all end with `navigator.replaceAll(TablesRoutes.TABLES)`. Since the UX audit (L5) Vue goes to Tables
  directly too (`utils/postLogin.ts`), without drawing the order screen only to bounce; the one exception is an order
  the Vue order store still holds (a resumed order), which keeps the `order` route. The login screen is removed from
  the Compose back stack, so Back does not return to it.
- **Back to the table after an idle lock (UX audit T6).** When the engine locks the till for inactivity, the operator
  who was logged in and the open order on screen are remembered (`SessionStore.takeIdleLockedUser`,
  `OrderStore.takeLockedOrderId`; Vue: `auth.rememberLockedOrder` / `takeLockedOrder`, `utils/postLogin.ts`). The same
  operator logging back in gets the order back (`engine.getOrder`, status open, then `OrderStore.loadOrder`; effect
  `GoToOrder`, which goes to Tables first so Back returns there); another operator, a manual lock, a closed, voided or
  unreadable order and any later login go to Tables. Nothing new is asked of the engine.
- **Idle lock line (UX audit L4).** After the engine's idle lock the login screen shows "Caisse verrouillée après
  N min d'inactivité" above the tiles (`idle-notice` / tag `idle_locked_notice`, amber) until someone logs in. N comes
  from the `idleAfterMs` of the `session-locked` payload (`SessionStore.idleLockedAfterMin`, `idleLockedMinutes`:
  rounded, at least 1); a manual lock or a payload without a delay shows nothing.
- **Engine not ready.** Every engine failure is caught: with the engine closed the screen shows the INSZ entry and
  the checks as "not configured" instead of failing. The root carries the tag `placeholder_login` (PD204: the tag
  means "feature root") so the foundation tests keep finding the screen.
- **Start-up checks wait for the engine, then appear together.** As in Vue the list is empty until all checks ran.
  HubRise is only checked when the integration is switched on; the screen loads the switch itself when the shared
  store has not yet (the store loads when the engine turns ready, which may be after the screen started).
- **FDM check through the shared store.** The screen asks `FdmStore.checkNow()` (`StoreFdmProbe`), which is the
  only caller of `fdm_check_now`, and reads the severity the store then holds. The severity is mapped as Vue does:
  blocking is an error "offline", warning is a warning, info is ok ("connected"). An FDM that did not answer (the
  store returned false), an engine that is not running, or a command that failed gives "not configured"
  (`check_not_configured`, PD244): an unanswered FDM is never shown as ok. A command that failed also raises the
  store's error toast, as in Vue; an engine that is not running does not.
- **Software version (art. 47).** The version of `PosSettings.posSwVersion` is shown under the checks
  (`login_version`), as in Vue.
- **Language choice.** The screen is in the terminal language (Settings, "Langue par défaut du terminal"). Three
  buttons (Francais, English, Chinese) in the footer choose that terminal language through
  `PosSettingsStore.setLanguage`; Vue has no such footer control (its terminal language lives in Settings), the till
  needs one before the first login, so it is kept here.
- **Language per user (UX audit R5).** When a tile is selected, a "FR / 中文 / EN" choice (tags
  `login_user_lang_<code>`; Vue `login-user-lang`) sets that person's language
  (`LoginViewModel.setPanelUserLanguage`, `PosSettingsStore.setUserLanguage`, kept per INSZ on the terminal in the
  preferences / localStorage). It is applied at login (`UserLanguageBinder`; Vue `pos.applyUser`) and the header user
  menu offers the same choice. Tickets and signed texts do not follow it.
- **Login and end of shift (UX audit L1).** An operator on shift sees two buttons once selected, "Se connecter" and
  "Terminer le service" (tags `login_mode_login`, `login_end_shift`; Vue `shift-choice`), instead of the small toggle;
  the pad title says which one runs (`pinTitleKey`). The badge hint and the S event (M141) are unchanged.
- **PIN panel layout.** The PIN pad is the shared `PinPad` (96 x 72 dp keys, the hint above the dots) instead of the
  128 x 88 px keys of the Vue panel. The end-shift button sits under the pad instead of beside "back", because the
  pad carries its own back button. An operator on shift without a PIN gets login, back and end-shift buttons, as in
  Vue.
- **First PIN dialog.** `PinDialog` has no "who" line, so the operator name leads its hint text. The toast
  "PIN saved" is raised after the engine accepted the PIN, as in Vue.
- **Tiles.** Two columns of tiles in a scrolling grid; only active users are listed. The selected tile (the one whose
  PIN panel is open) gets a Brume ring and a Brume initial, never Volt.
- **Wordmark.** `PiliLogo.vue` is an SVG; the Compose screen draws the word "pili" and the hanzi as text until the
  bundled brand font and logo asset land (PD106).
- **Fonts and sizes.** Text sizes use the theme tokens only; the Vue sizes that have no token (40, 26, 18 px) use the
  nearest token or a multiple of one.
- **Badge scan on the login screen (lane 5-nfc, Vue side).** The scan handling is global (`stores/badge.ts`, one
  capture-phase keyboard listener while the reader is `usb`, installed before any other keyboard listener); the login
  screen only adds three things. (1) The hint `login_badge_hint` ("Ou présentez votre badge", tag `badge-login-hint`)
  under the tiles, when the reader is on and badge login is enabled. (2) Its "Fin de service" mode is mirrored into
  the store (`endShiftMode`), so a scan in that mode is sent as `badge_tap(token, true)`; the pad shows
  `end_shift_badge_hint` (tag `end-shift-badge-hint`) while the reader is on, and closes when the scan registered a
  departure. (3) The outcome `pin_required` (a manager's badge) selects that manager's tile and opens their PIN
  pad (the same path as a tap on the tile, so a manager without a PIN still gets the first-PIN dialog); a badge never
  opens a manager session. The outcome `logged_in` stores the user and goes where a PIN login goes
  (`goAfterLogin`). A registered arrival or departure reloads the tiles' shift state. Refusals and presence toasts are
  the ones of the whole app (`badge_clock_in`, `badge_clock_out`, `badge_presence_failed`, the `err_` texts) with a
  high or a low tone (Web Audio on Vue, `AudioTrack` on Android).
- **Badge scan on the login screen (lane 5-nfc, Android side).** The scan handling is global too (`BadgeStore`, fed by
  `MainActivity.dispatchKeyEvent` for the USB wedge reader and by the NFC reader mode); the login screen adds the same
  three things as Vue. (1) The hint `login_badge_hint` (tag `login_badge_hint`) under the tiles, when the reader is on
  and badge login is enabled (`LoginUiState.badgeLoginOn`). (2) Its "Fin de service" mode is mirrored into the store
  (`setEndShiftMode`), so a scan is sent as `badge_tap(token, true)`; the pad shows `end_shift_badge_hint` (tag
  `login_end_shift_badge_hint`) while the reader is on, and closes when the scan registered a departure. (3) The
  outcome `pin_required` selects that manager (the INSZ entry gives way to the tiles) and opens their PIN pad through
  the tile path (a manager without a PIN gets the first-PIN dialog); a badge never opens a manager session. The
  outcome `logged_in` has already opened the session in the engine: the store keeps the user (`SessionStore.remember`)
  and the screen sends `GoToTables`, the same way out as a PIN login. A registered arrival or departure reloads the
  shift state of the tiles. Toasts and the high or low tone (`AudioTrackBadgeCue`) are the ones of the whole app.
- **Status sheet on a start-up row (UX audit L2).** Each row is a 48 dp button (a transparent button over the row,
  so the answer text keeps its own test tag). A tap opens the shared `DeviceStatusSheet` with the device, the state
  in words, the reason and the time of that row's last check; "Réessayer maintenant" calls
  `LoginStartupChecks.runOne(id)` and replaces that row only, in place, keeping the order. The FDM row's reason is the
  message of the code of the last FDM reading (`LoginUiState.fdmCode`).
