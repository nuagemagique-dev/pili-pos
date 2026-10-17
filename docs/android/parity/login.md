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
  PIN all end with `navigator.replaceAll(TablesRoutes.TABLES)`. Vue pushes the `order` route, whose screen bounces to
  Tables when there is no order. The login screen is removed from the back stack, so Back does not return to it.
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
- **Language choice.** Three buttons (Francais, English, Chinese) choose the language through
  `PosSettingsStore.setLanguage` and are kept on the device. Vue has no such control on this screen (its language
  lives in Settings); the till needs one before the first login, so it is added here.
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
