# Lane 5 (NFC staff badges): plan

Branch `android/lane-nfc`, worktree `moyun-pos-wt/lane-nfc`, base `android/lane-v1-fixes` at `05a8b6c5` (lane 1, presence registration). Planner: Opus 5.5. Workers: Sonnet 5.5. Reviewer: Opus 5.5. Written 2026-10-17.

Everything in this lane ships in the certified v1.0.0 (Linux and Android). There is no later update planned before certification: the v1.0 dossiers (`docs/SUBMISSION_ARTIFACTS`, `docs/android/dossier`) are updated directly and their changed PDFs rebuilt. This lane never tags, pushes or merges; the orchestrator merges and makes the next freeze.

## 1. Goal

Staff identify themselves with an NFC badge (card or key fob):

1. to register their presence (arrival, departure: S events signed by the FDM in their own INSZ) from any screen, without logging out the cashier who is using the till;
2. optionally, to log in to the till instead of typing the PIN (cashier switch on the login screen), managers keeping their PIN.

All of it sits behind a per-installation setting, OFF by default, "Lecteur de badges": Désactivé / NFC intégré / Lecteur USB, set by a manager in Settings on both UIs. With the setting off nothing reads badges and every badge command is refused.

### 1.1 Hardware facts (checked)

- Sunmi D3 Pro (model F3510, Android 13): the main unit has NO NFC. Only the optional 10.1 inch customer display has NFC ("Tap on Glass", meant for SoftPOS payments). Source: Sunmi spec sheet `https://file.cdn.sunmi.com/newebsite/products/d3-pro/spec/d3-pro-en-new.pdf` (customer display "Option with NFC, Tap on Glass to support SoftPOS"). Whether that NFC is exposed through the standard Android `NfcAdapter` of the main unit is UNKNOWN (open question Q1, a device test).
- The Linux till (Penetek A3216 M82 / Advantech USC-360J) has no NFC.
- Therefore badge reading must work without native NFC: the main source is a USB HID "keyboard wedge" reader (an NFC reader that types the badge number followed by Enter, no driver, works on Linux and Android). Android built-in NFC is a second source, offered only when the device reports `PackageManager.FEATURE_NFC` (covers the D3 Pro customer display if it exposes standard NFC, and any other Android hardware with NFC). This is the "flag for hardware that does not have it natively" the user asked for: the setting picks the source per installation.

## 2. Scope and non-goals

In scope: `crates/pos-core`, `crates/pos-db`, `crates/pos-engine`, `src-tauri`, `android/rust/pos-ffi`, `android/pos-core/api/pos-core.api`, both UIs (`ui/`, `android/app`), their tests (cargo, FFI, Playwright, Kotlin unit, Roborazzi goldens that change), both dossiers and their PDFs, `CLAUDE.md`, `docs/android/{DECISIONS,PARITY,ENGINE,FFI,DEVICE_TESTS}.md`, `docs/android/parity/{settings,login}.md`, `docs/internal/CAPTURES_ECRAN_RUNBOOK_FR.md` (and the Android runbook rows), `docs/android/LANES.md` only if a new path needs an owner (section 7.4 says none should).

Many of these paths are `frozen` in `docs/android/LANES.md` (Linux build inputs, FFI, dossier, the foundation `MainActivity.kt` and `AndroidManifest.xml`). The user asked for this lane for v1.0.0, so the lane edits them, as lane 1 did; say so in the commit bodies. Never touched: `scripts/build_and_hash.sh`, `scripts/check_repro.sh`, `scripts/android/check_repro.sh`, `docs/android/baseline/`, `LINUX_FREEZE.md`, release scripts.

Non-goals (record them in DECISIONS, do not build them):
- PC/SC readers (pcsc-lite, for example ACR122U) on Linux: recorded as a possible source for a later certified update; not built (it needs the `pcsc` crate, the `pcscd` daemon in the image and a polling thread: not trivial). A USB wedge reader covers Linux today.
- Reading anything but the UID: no NDEF, no sector data, no writing to badges, no MIFARE keys.
- Several badges per employee, temporary or visitor badges, badge expiry dates.
- A badge-only manager session (a manager logged in by badge with reduced rights until the PIN): rejected, see L5-D6.
- Switching the cashier from a screen other than the login screen (a scan elsewhere is always a presence toggle).
- A screen listing every scan (the scan log is stored for audit, the Settings panel shows only the link/revoke/settings log).
- Retaking dossier screenshots (section 11 lists them for the orchestrator).
- Version bumps, tags, freeze files, `plat_about_engine_ref`.

## 3. Spec references

| Topic | Reference |
|---|---|
| Any registration routine may produce the S event ("N'importe quelle routine d'inscription peut être utilisée à cette fin, à condition que la caisse enregistreuse crée les données correctes pour l'event") | POS spec v2.2 `docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md` §1.9 "Enregistrement de la présence du personnel", lines 368 to 374 |
| User identification: the NISS is sent to the FDM, never printed; the POS links its internal user identification to the NISS | same spec §6.4 "Identification de l'utilisateur", lines 556 to 560 |
| `employeeId`: NISS, 11 digits, modulo 97 checked by the FDM | same spec, lines 1179 to 1185; WorkInOutInput lines 3205 to 3218 |
| Login and presence are distinct; no operation without a logged-in user; presence must be possible for staff who do not use the POS; the questionnaire asks whether presence is tied to "Work In/Work Out" buttons | Certification manual `docs/20250307_POS_CERTIFICATION_MANUEAL_FRV1.1/20250307_FR - Manuel de certification_POS_V11.md` §II.E "Enregistrement de la présence", lines 799 to 825 |
| POS login method is free (the manual's table only asks for user names and passwords, "Variations de 1234") | same manual §II.C "Utilisateurs & mots de passe", lines 653 to 670 |
| Users are persons identified by their national register or BIS number; the S event records start and end of presence | AM of 29 April 2024 `docs/legislation/20240429_AM_aspects_techniques_certification_FR.md` art. 1, 9° and art. 9 |
| User reports carry the connection times recorded by the S event | same AM, user report contents (point i. "les heures de connexion et de déconnexion ... telles qu'enregistrées via l'event Social") |
| WorkIn/WorkOut payloads, training label T | `docs/20260821_UC_BasicSet_POS_FDM_1.8/M140_signWorkIn/*`, `M141_signWorkOut/*`, `M170_signWorkIn_Training` |
| Training: every registration in training is a T event | POS spec v2.2 §1.5, lines 312 to 320 (lane 1 decision L1F-A4) |
| Reserved migration ranges | `crates/pos-db/src/migrate.rs` (12 to 19 reserved for certified hotfixes; 20 to 28 taken); this lane's brief: other lanes take 30, 31, 40 to 4x and 60 to 6x, this lane takes 70 |

Reading (recorded by T1 as L5-S1): neither the POS spec, the certification manual, the AM nor the producers' FAQ (`docs/20260717_FAQ_PRODUCTEURS_FR.md`, no hit for badge, NFC or biometrics) regulates HOW a user is identified to the POS; what is regulated is the result: every event carries the NISS of a user known to the POS (`employeeId` for S, the operator for the rest), the NISS is never printed, and presence registration is possible for non-POS staff. A badge linked by a manager to an employee record that holds the NISS is an internal identification of that user (§6.4) and a "registration routine" (§1.9). The S event built from a badge scan is byte-for-byte the event of M140/M141 for that employee. Conservative reading kept: a badge never replaces the NISS anywhere, a badge scan never signs anything without the FDM (no buffering), and a badge never grants manager rights alone (L5-D6).

## 4. Design

### 4.1 Decisions (T1 records L5-* in `docs/android/DECISIONS.md`, at the end, under `## Lane 5-nfc (2026-10-17)`; later tasks append below)

- **L5-S1 Spec reading**: section 3 above.
- **L5-D1 One setting, three values, off by default.** `badge_settings.reader` in `off | nfc | usb`, default `off`; `login_enabled` (0/1, default 0); the USB wedge timing `wedge_max_gap_ms` (default 50, 10 to 200), `wedge_min_len` (default 6, 4 to 32), `wedge_max_len` (default 32, min_len to 64). Stored in the database (migration 70), changed by a manager only, every change logged (append-only `badge_log`, action `settings`). No `pos.json` key (unlike HubRise): the hardware is chosen at the till. The engine does not know the hardware: `nfc` is accepted by the engine; the Android UI offers "NFC intégré" only when `FEATURE_NFC` is present, the Vue UI never offers it. A stored `nfc` on a device without NFC means no reading plus a note in Settings.
- **L5-D2 Badge identifier.** The UI sends the raw reader output as `token`. The engine normalizes it (`pos_core::badge::normalize_token`): trim, ASCII uppercase, must match `[0-9A-F]{4,64}`, else `badge_token_invalid`. NFC: the tag id bytes as uppercase hex, in the order Android gives them. USB: the characters typed. No source prefix: a wedge reader configured to type the UID in hex, same byte order, matches an NFC enrolment. Different readers may print the same card differently (hex vs decimal, byte order): an installation uses one reader kind; changing the kind may require re-enrolment (manual note).
- **L5-D3 Only a keyed hash is stored.** `badge_hash = hex(HMAC-SHA256(key, "pili-badge/1\n" || token))` (64 lowercase hex). `hmac 0.12.1` and `sha2 0.10` are already in both `Cargo.lock` files (no new crate). The key: 32 random bytes in `<data_dir>/badge.key` (`EnginePaths.data_dir`), created once on first need with `getrandom`, mode 0600, written beside its name then linked into place, never replaced, never in `pos.db`, so never in the backups nor in the nightly export (same pattern as `pos_export::pseudonym::load_or_create_key`; copy the pattern into `crates/pos-engine/src/badge_key.rs`, do not touch pos-export). Without the key a 4 to 7 byte UID space could be enumerated from a stolen database; with it, it cannot. The raw UID never reaches the database, the logs (tracing included) or an event: logs carry `badge_ref` = the first 12 hex characters of the hash.
- **L5-D4 Lost key.** `badge_settings.key_check` holds the first 16 hex characters of `HMAC(key, "pili-badge-check/1")`. At engine open, when a check is stored and the key file is missing (a restore on new hardware) or gives another check, every active badge is revoked with reason `key_changed` (one `badge_log` row each, `changed_by` NULL) and the new check stored; Settings then show "Liez de nouveau les badges". A key file that exists but cannot be read is NOT a lost key: badge commands answer `badge_key_unavailable`, nothing is revoked.
- **L5-D5 Enrolment rules.** Managers only (`Access::Admin`). Any active user may get a badge, presence-only users included (the main use case), not the system identity. One active badge per employee and one employee per active badge (two partial unique indexes). Linking requires the manager's own PIN (`access::confirmed_operator`), as setting another employee's PIN does: a badge is a login credential. Revocation needs no PIN (it only removes access). Re-issue = revoke then link (two log rows); the same physical badge may be linked again after a revocation. Deactivating an employee revokes their active badge in the same transaction (reason `deactivated`, by the manager). Linking is refused while the reader is off.
- **L5-D6 Manager rights always need the PIN.** A badge alone never opens a manager session: on the login screen a manager's badge only selects the manager and opens the PIN pad for them (outcome `pin_required`); the session is then opened by the existing `login(insz, pin)` with its throttle. Cashiers and waiters log in with the badge alone when `login_enabled` is on. Rejected alternative: a badge-opened manager session with cashier rights until the PIN; it needs a second session state checked by every `Access::Admin` command and by the Android platform code that reads the role directly (`Exporter.kt`, `UpdatePreconditions.kt`), for little gain. So no manager-only action (refund, copy, cash in/out, reports, Z, settings, users, badges) is ever reachable without the manager's PIN.
- **L5-D7 What a scan does (engine decides, `badge_tap(token, end_shift)`).** In this order, after the reader, token, key and repeat checks:
  1. unknown badge: `badge_unknown`; revoked badge (only revoked rows match): `badge_revoked` (both logged in `badge_scans`);
  2. a session is open (any screen but the login screen): presence toggle of the badge's employee; the badge of the logged-in operator is refused with `presence_self` (their presence goes through login and "Fin de service", lane 1 rule);
  3. no session and `end_shift` is true (the login screen is in "Fin de service" mode): clock-out only (`presence_not_in` when not on shift), no session opened;
  4. no session, `login_enabled` on and the employee is not presence-only: a manager gives `pin_required` (nothing else happens); anyone else is logged in exactly like `do_login` without the PIN check (session opened, clock-in queued for the presence worker, which clocks in only when off shift);
  5. otherwise (presence-only employee, or badge login off): presence toggle.
  Presence toggle: training mode on refuses with `presence_training` (L1F-A4: a T event registers no presence, so toggling in training would show "Arrivée" every time); otherwise WorkOut when the local index (`staff_shifts`) says on shift, WorkIn when not, through `social::sign_work_event(state, employee, is_in, None)` (the S event is the employee's: `employeeId` and `fiscal_events.user_insz`; `started_by`/`ended_by` NULL = self-registered; the badge is in `badge_scans`).
- **L5-D8 FDM down.** As L1F-A5: no extra status query; the scan signs; when the FDM is unreachable or refuses, the outcome is `presence_failed` with the FDM error, nothing archived, the index unchanged, and the UI shows the FDM text in an error toast. Badge login works without the FDM, exactly like a PIN login (the presence worker emits `presence-failed`).
- **L5-D9 Training mode.** Presence toggle refused (`presence_training`); badge login allowed (as a PIN login: the presence worker sends a T WorkIn that opens no shift, L1F-A4); enrolment and revocation allowed (no fiscal event).
- **L5-D10 Repeats.** The engine ignores a scan of the same badge hash while its previous scan is still being handled and for `EngineTimers.badge_repeat` (default 10 s) after that scan finished, whatever its outcome (refusals included, so a card left on the reader does not flood the log). The outcome is `ignored`: no toast, nothing logged, nothing signed. Measured from the end of the previous scan so a slow FDM answer followed by a second tap can never clock in and then out. In memory only (a restart clears it). Different badges are never debounced against each other.
- **L5-D11 Not user activity.** A badge scan does not restart the idle lock timer of the logged-in operator (it may be someone else's badge); only a badge login does (it opens a session). The UIs do not forward the swallowed keystrokes as activity.
- **L5-D12 USB wedge detection (both UIs, same algorithm).** Map PHYSICAL keys, never characters: Vue `KeyboardEvent.code` (`Digit0`..`Digit9`, `Numpad0`..`Numpad9`, `KeyA`..`KeyF`), Android `KeyEvent.keyCode` (`KEYCODE_0..9`, `KEYCODE_NUMPAD_0..9`, `KEYCODE_A..F`). Belgian tills often use AZERTY, where a reader's digit keys would produce `&é"'(` as characters; physical codes are layout-independent. Shift, Ctrl, Alt, Meta and key repeats are neutral (ignored, no flush). Every candidate key-down is held (Vue: `preventDefault` + `stopImmediatePropagation` in a window capture listener registered first; Android: consumed in `MainActivity.dispatchKeyEvent` before `super`). A burst is a badge when it ends with Enter/NumpadEnter, every gap between consecutive held keys (and before the Enter) is at most `wedge_max_gap_ms`, and its length is within `[min_len, max_len]`; the Enter is then swallowed too. Anything else is "not a badge" and the held keys are replayed IN ORDER before the key that broke the burst is let through: a gap longer than `max_gap` (flushed by a timer, the first key of a new burst starts over), a non-candidate key, Enter after too short or too long a burst, an over-long burst (flush at `max_len + 1`). Replay: Vue inserts the held characters into the focused editable element (`HTMLInputElement`/`HTMLTextAreaElement`: `setRangeText` at the caret, then an `input` event so `v-model` updates; nothing focused: dropped, as they would have gone nowhere); Android re-dispatches the held `KeyEvent`s through `super.dispatchKeyEvent` with a re-entry flag so the detector does not see them twice. Human typing (about 100 ms or more between keys) is therefore delayed by at most `max_gap` and never taken for a badge; a burst never reaches a text field. With the reader not `usb` the listener is not installed (Vue) and `dispatchKeyEvent` passes everything (Android).
- **L5-D13 Android NFC source.** `NfcAdapter.enableReaderMode` (flags NFC_A, NFC_B, NFC_F, NFC_V, SKIP_NDEF_CHECK) in `MainActivity.onResume` when the setting is `nfc`, `FEATURE_NFC` is present and the adapter is enabled; `disableReaderMode` in `onPause` and when the setting changes. The tag id bytes become the uppercase hex token. Manifest: `<uses-permission android:name="android.permission.NFC" />` (normal permission, not in the p6 forbidden list) and `<uses-feature android:name="android.hardware.nfc" android:required="false" />` (installable without NFC). The platform sound of reader mode is left on (an extra audible cue on NFC); the app cue plays in every case.
- **L5-D14 Feedback.** Every outcome but `ignored` gives a toast (success: "Arrivée · Nom · 14:02" / "Départ · Nom · 14:02" in Brussels time from the outcome's `at`; error: the refusal text or the FDM text) and a short sound: a high two-tone for a registered arrival, departure or login, a low tone for any refusal or failure (`useOrderChime.ts` / `OrderChime.kt` style synthesis, no audio file). The toast is the visual cue.
- **L5-D15 Migration 70** (`staff-badges`), base `MIGRATIONS` list, not feature-gated (the tables exist on every installation and are written only when the setting is on). If another lane takes 70, the orchestrator renumbers at the merge (references: `migrate.rs`, `SCHEMA_BD_FR.md` §6, `SCHEMA_ER_FR.md` §7, `CLAUDE.md` migrations sentence, the base-migrations test).
- **L5-D16 Expected gate failures.** As L1F-G2: `hygiene-paths`, `linux-frozen`, `freeze-tag` (p5, p6) and `app-only-scope` (p5) fail because the lane edits frozen paths on purpose; nothing else may fail.

### 4.2 Data model (migration 70, `MIGRATION_070_STAFF_BADGES` in a new `crates/pos-db/src/schema_badges.rs`, entry `{ version: 70, name: "staff-badges" }` in `MIGRATIONS`)

```sql
CREATE TABLE IF NOT EXISTS badge_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    reader TEXT NOT NULL DEFAULT 'off' CHECK (reader IN ('off', 'nfc', 'usb')),
    login_enabled INTEGER NOT NULL DEFAULT 0 CHECK (login_enabled IN (0, 1)),
    wedge_max_gap_ms INTEGER NOT NULL DEFAULT 50 CHECK (wedge_max_gap_ms BETWEEN 10 AND 200),
    wedge_min_len INTEGER NOT NULL DEFAULT 6 CHECK (wedge_min_len BETWEEN 4 AND 32),
    wedge_max_len INTEGER NOT NULL DEFAULT 32 CHECK (wedge_max_len BETWEEN 4 AND 64),
    -- First 16 hex chars of HMAC(badge key, "pili-badge-check/1"); NULL until a key exists.
    key_check TEXT,
    changed_at TEXT NOT NULL DEFAULT (datetime('now')),
    changed_by TEXT,
    CHECK (wedge_min_len <= wedge_max_len)
);
INSERT OR IGNORE INTO badge_settings (id) VALUES (1);

CREATE TABLE IF NOT EXISTS staff_badges (
    id INTEGER PRIMARY KEY,
    badge_hash TEXT NOT NULL CHECK (length(badge_hash) = 64),
    user_insz TEXT NOT NULL REFERENCES users(insz),
    linked_at TEXT NOT NULL DEFAULT (datetime('now')),
    linked_by TEXT NOT NULL,
    revoked_at TEXT,
    revoked_by TEXT,            -- NULL with reason key_changed
    revoke_reason TEXT CHECK (revoke_reason IS NULL OR revoke_reason IN ('manager', 'deactivated', 'key_changed'))
);
CREATE UNIQUE INDEX IF NOT EXISTS staff_badges_active_badge ON staff_badges(badge_hash) WHERE revoked_at IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS staff_badges_active_user ON staff_badges(user_insz) WHERE revoked_at IS NULL;
-- Never deleted; the only change ever made is one revocation.
CREATE TRIGGER IF NOT EXISTS staff_badges_no_delete BEFORE DELETE ON staff_badges
BEGIN SELECT RAISE(ABORT, 'staff_badges rows are never deleted'); END;
CREATE TRIGGER IF NOT EXISTS staff_badges_revoke_only BEFORE UPDATE ON staff_badges
WHEN OLD.revoked_at IS NOT NULL OR NEW.revoked_at IS NULL OR NEW.revoke_reason IS NULL
  OR NEW.badge_hash IS NOT OLD.badge_hash OR NEW.user_insz IS NOT OLD.user_insz
  OR NEW.linked_at IS NOT OLD.linked_at OR NEW.linked_by IS NOT OLD.linked_by
BEGIN SELECT RAISE(ABORT, 'staff_badges: only one revocation may change a row'); END;

-- Append-only audit of links, revocations and settings changes.
CREATE TABLE IF NOT EXISTS badge_log (
    id INTEGER PRIMARY KEY,
    action TEXT NOT NULL CHECK (action IN ('link', 'revoke', 'settings')),
    badge_id INTEGER REFERENCES staff_badges(id),
    user_insz TEXT,
    detail TEXT,                -- revoke: the reason; settings: JSON of the new values
    changed_by TEXT,            -- manager INSZ; NULL for the engine (key_changed)
    changed_at TEXT NOT NULL DEFAULT (datetime('now'))
);
-- Append-only: every scan that was not an ignored repeat.
CREATE TABLE IF NOT EXISTS badge_scans (
    id INTEGER PRIMARY KEY,
    outcome TEXT NOT NULL CHECK (outcome IN ('clock_in', 'clock_out', 'presence_failed', 'login', 'pin_required', 'unknown', 'revoked', 'refused')),
    user_insz TEXT,
    badge_ref TEXT NOT NULL,    -- first 12 hex chars of the badge hash, never the UID
    detail TEXT,                -- refusal code or FDM error
    scanned_at TEXT NOT NULL DEFAULT (datetime('now'))
);
-- no_update and no_delete triggers on badge_log and badge_scans, like table_plan_log.
```

Tests in `migrate.rs` / `schema_badges.rs`: defaults and the single row; every CHECK (reader value, ranges, min <= max, hash length, reason); both partial unique indexes (second active badge for a user refused, same hash active twice refused, re-link after revocation allowed); the triggers (delete refused, a second revocation refused, changing the hash refused, both logs append-only); the base-migration list test becomes "... then presence, then badges" (rename `the_base_migrations_are_the_certified_ones_then_presence` accordingly); a reopened database does not re-run 70; the `migrate.rs` comments name 70 and the ranges of the other lanes.

### 4.3 Engine

New pure module `crates/pos-core/src/badge.rs`: `BadgeReader { Off, Nfc, Usb }` (serde `off|nfc|usb`, `FromStr`), `BadgeSettings` (reader, login_enabled, wedge_max_gap_ms, wedge_min_len, wedge_max_len) with `validate()`, `normalize_token(&str) -> Result<String, ()>`. Unit tests for both.

`crates/pos-db/src/queries/badges.rs`: settings get/set (+ log row), `active_link_by_hash`, `any_link_by_hash` (revoked detection), `active_link_by_user`, `link` (+ log), `revoke` (+ log; reason), `revoke_all_active(reason)`, `list_active` (with names), `history(limit)`, `log_scan`. Unit tests on an in-memory database.

`crates/pos-engine/src/badge_key.rs`: `load_or_create(data_dir) -> Result<[u8; 32], String>`, `hash_token(key, token) -> String`, `key_check(key) -> String`, plus the open-time check of L5-D4 called from `Engine::open` (after migrations, before the background tasks). Tests: created once, mode 0600 on unix, reused, a wrong-size file is an error and left alone, a known HMAC vector, the check detects another key.

`crates/pos-engine/src/commands/badges.rs` (new), every function returns `Result<_, String>` with codes from `commands/errors.rs`:

| Command | Access | Notes |
|---|---|---|
| `do_get_badge_settings(state) -> BadgeSettingsView` | none (the login screen arms the reader) | `reader`, `login_enabled`, the three wedge values, `relink_needed: bool` (true after a `key_changed` revocation until a badge is linked again). No personal data. |
| `do_set_badge_settings(state, input: BadgeSettingsInput) -> BadgeSettingsView` | Admin | `badge_settings_invalid` on any out-of-range value or unknown reader; a `settings` log row with the new values as JSON when something changed. |
| `do_list_badges(state) -> BadgeOverview` | Admin | `badges`: active links {insz, name, linked_at, linked_by_name}; `log`: latest 30 `badge_log` rows {action, insz, name, by_name, at, detail}. Never the hash. |
| `do_link_badge(state, employee_insz, token, operator_pin) -> BadgeLink` | Admin + own PIN | Refusals in this order, nothing written: `forbidden`; `badge_reader_off`; `badge_token_invalid`; `badge_key_unavailable`; `system_user`; `user_unknown` (unknown or inactive); `badge_already_linked` (the employee has an active badge); `badge_in_use` (the badge is active for anyone); then the PIN (`pin_required`, `pin_wrong`, `pin_locked`, as `confirmed_operator`). Writes the link, its log row and, if absent, `key_check`, in one transaction. |
| `do_revoke_badge(state, employee_insz) -> ()` | Admin | `badge_none` when the employee has no active badge; reason `manager`. |
| `do_badge_tap(state, token, end_shift: bool) -> BadgeOutcome` | none (the badge is the identification, L5-S1) | `badge_reader_off`, `badge_token_invalid`, `badge_key_unavailable` (nothing logged), repeat check (L5-D10), then L5-D7. Async (signs). |

`BadgeOutcome` is a flat struct (simple for TypeScript and UniFFI), `#[serde(rename_all = "camelCase")]`: `kind` (`ignored | clock_in | clock_out | presence_failed | logged_in | pin_required`), `insz: Option<String>`, `name: Option<String>`, `at: Option<String>` (RFC 3339 UTC of the registration, for the toast time), `error: Option<String>` (FDM text of `presence_failed`), `user: Option<User>` (for `logged_in`). Refusals are `Err(code)`: `badge_unknown`, `badge_revoked`, `presence_self`, `presence_not_in`, `presence_training`, `user_unknown`, plus the three of the first line.

Other engine changes:
- `auth::do_deactivate_user`: revoke the active badge in the same transaction (after the `deactivate_on_shift` check).
- `AppState`: `badge_repeats: Mutex<HashMap<String, Repeat>>` (`InFlight` | `Done(Instant)`), pruned on each scan.
- `EngineTimers.badge_repeat: Duration` (default 10 s; update every constructor, the Tauri shell, the FFI and the testkit).
- Badge login: factor from `do_login` a `open_login_session(state, &user)` (open session + `queue_clock_in`) used by both, so the badge path is exactly the PIN path minus the PIN check (`authenticate` minus the PIN; presence-only and system user still refused).
- New codes in `commands/errors.rs` and `errors::ALL`: `badge_reader_off`, `badge_token_invalid`, `badge_key_unavailable`, `badge_unknown`, `badge_revoked`, `badge_in_use`, `badge_already_linked`, `badge_none`, `badge_settings_invalid` (9). The FFI test `every_code_of_both_lists_maps_to_coded` goes from 119 to 128. `crates/pos-engine/tests/error_codes.rs` requires a FR, EN and ZH text per code: add the `err_<code>` texts to `ui/src/i18n.ts` in T1 and regenerate the Android twins (`node scripts/android/vue_to_android.mjs --write`), as lane 1 did (L1F-A8).
- Shell: `src-tauri/src/ipc/badges.rs` with the six wrappers (`get_badge_settings`, `set_badge_settings`, `list_badges`, `link_badge`, `revoke_badge`, `badge_tap`), one engine call each (`badge_tap` async), registered in `main.rs`. Wrapper count 98 to 104 (96 to 102 in the cert build): fix `CLAUDE.md`, the count in `scripts/android/test_check_command_parity.py` or wherever "96 cert commands" is asserted (commit `ba129f77` is the template).
- FFI: `android/rust/pos-ffi/src/api/badges.rs` (or next to auth), same names and parameter order, `self.blocking` for the sync ones and `self.spawn` for `badge_tap`; records in `records/badges.rs` (`BadgeSettingsView`, `BadgeSettingsInput`, `BadgeOverview`, `BadgeLink`, `BadgeLogEntry`, `BadgeOutcome`, enum `BadgeOutcomeKind`), Serialize + Deserialize, `deny_unknown_fields` (hygiene H3), no unsigned integers (H6: the wedge values are `i32`/`i64`); records index comment in `records/mod.rs`; FFI tests in `android/rust/pos-ffi/tests/ffi/badges.rs` (or `auth.rs`); `android/pos-core/api/pos-core.api` regenerated (`(cd android && ../scripts/android/heavy.sh ./gradlew :pos-core:apiDump)`); `docs/android/FFI.md` §11 regenerated with `python3 scripts/android/check_command_parity.py --markdown` and the counts in §1/§11 prose.
- `ui/src/api/ipc.ts`: six typed wrappers and the types (`check_ui_parity.py` only knows commands declared there); `docs/android/PARITY.md` `ui-parity` block: the six commands `pending:P5b-settings` until T3 (non-strict runs accept pending; T3 removes the lines).
- Template for an end-to-end command: commit `c3488d84` (`list_unsettled_card_payments`) and lane 1's `clock_in_employee`.

### 4.4 Vue UI (`ui/`)

- `ui/src/utils/badgeWedge.ts`: pure `WedgeDetector` (config, `onKeyDown(code, timeStamp)` returning hold / pass / badge, a flush callback for the gap timer, the held characters for replay). No DOM inside, so it stays simple; Playwright exercises it through real key events.
- `ui/src/composables/useBadgeReader.ts`: installs one window `keydown` capture listener (registered in `App.vue` before the activity listener) only while `reader === 'usb'`; holds, swallows, replays (L5-D12); hands a badge to the store.
- `ui/src/stores/badge.ts` (Pinia): settings (loaded at start-up and after every change, also from the login screen with no session), `capture` (a pending enrolment: the next scan resolves it instead of calling `badge_tap`), `endShiftMode` (set by `LoginView.vue` while its "Fin de service" mode is on), `onScan(token)` -> `ipc.badgeTap(token, endShiftMode)` -> outcome handling: `logged_in` sets `auth.currentUser` and goes where a PIN login goes (factor the post-login navigation of `LoginView.vue` into one function used by both); `pin_required` sets `pinRequest` which `LoginView.vue` watches to open its PIN pad for that manager (existing `openPinOverlay`); `clock_in` / `clock_out` / `presence_failed` / refusals give the toast and the cue (L5-D14); `ignored` nothing; a `presenceVersion` counter bumped after a registered toggle so the Settings employees panel re-reads the open shifts if shown.
- `ui/src/composables/useBadgeCue.ts`: Web Audio cue, ok and refusal (wrap in try/catch like the chime).
- `ui/src/components/BadgeSettings.vue` (manager only, in `SettingsView.vue`; keep the three-column no-scroll layout, the panel may scroll inside): segmented reader choice "Désactivé / Lecteur USB" (Linux has no "NFC intégré"; a stored `nfc` shows the note `settings_badge_nfc_unavailable`), "Connexion par badge" switch with its note, the USB timing (three numeric fields with the existing `NumPad`), `relink_needed` banner, the GDPR note, and the latest `badge_log` rows.
- Employees panel of `SettingsView.vue`: per row, for a manager and while the reader is on: "Badge lié le 17/10" or "Aucun badge", and one button "Lier un badge" / "Retirer le badge". Linking opens `ui/src/components/BadgeEnrolDialog.vue`: "Présentez le badge de {name}" (waits for the store capture, 30 s timeout, Annuler), then the existing `PinDialog` for the manager's own PIN, then `linkBadge(insz, token, pin)`, toast "Badge lié · {name}", refresh of `list_badges`. Revoking asks a confirmation, then `revokeBadge`.
- `LoginView.vue`: under the tiles, the hint "Ou présentez votre badge" when the reader is on and badge login enabled; in "Fin de service" mode the hint "Présentez votre badge pour terminer le service" whenever the reader is on.
- Strings: settings texts in `ui/src/i18n-settings.ts`, toasts, hints and `err_*` in `ui/src/i18n.ts`; then `node scripts/android/vue_to_android.mjs --write`. Proposed texts (align the ZH with existing terms: 上班 / 下班 as lane 1, 工牌 for badge):

| Key | fr | en | zh |
|---|---|---|---|
| `settings_badges` | Lecteur de badges | Badge reader | 工牌读卡器 |
| `badge_reader_off` | Désactivé | Off | 关闭 |
| `badge_reader_usb` | Lecteur USB | USB reader | USB 读卡器 |
| `badge_reader_nfc` (Android only, strings_ui_settings.xml) | NFC intégré | Built-in NFC | 内置 NFC |
| `settings_badge_login` | Connexion par badge | Badge login | 刷工牌登录 |
| `settings_badge_login_note` | Les Gérants saisissent toujours leur PIN après le badge. | Managers still enter their PIN after the badge. | 经理刷工牌后仍需输入 PIN。 |
| `settings_badge_usb_timing` | Réglages du lecteur USB | USB reader timing | USB 读卡器参数 |
| `settings_badge_gap` | Écart maximal entre caractères (ms) | Maximum gap between characters (ms) | 字符最大间隔（毫秒） |
| `settings_badge_min_len` | Longueur minimale du code | Minimum code length | 卡号最短长度 |
| `settings_badge_max_len` | Longueur maximale du code | Maximum code length | 卡号最长长度 |
| `settings_badge_nfc_unavailable` | Pas de NFC sur cet appareil : choisissez un lecteur USB | No NFC on this device: choose a USB reader | 本设备没有 NFC，请选择 USB 读卡器 |
| `settings_badge_nfc_disabled` (Android only) | Le NFC est désactivé dans les réglages de l'appareil | NFC is turned off in the device settings | 设备设置中的 NFC 已关闭 |
| `settings_badge_relink` | La clé des badges a été renouvelée : liez de nouveau les badges | The badge key was renewed: link the badges again | 工牌密钥已更新，请重新绑定工牌 |
| `settings_badge_privacy` | Seule une empreinte chiffrée du badge est conservée, jamais son numéro. Usage : présence et connexion uniquement. | Only a keyed hash of the badge is kept, never its number. Used for presence and login only. | 仅保存工牌的加密指纹，从不保存卡号，仅用于考勤和登录。 |
| `badge_link` | Lier un badge | Link a badge | 绑定工牌 |
| `badge_revoke` | Retirer le badge | Remove the badge | 解除工牌 |
| `badge_revoke_confirm` | Retirer le badge de {name} ? | Remove {name}'s badge? | 解除 {name} 的工牌？ |
| `badge_linked_on` | Badge lié le {date} | Badge linked on {date} | 工牌绑定于 {date} |
| `badge_none_label` | Aucun badge | No badge | 无工牌 |
| `badge_present_for` | Présentez le badge de {name} | Present {name}'s badge | 请出示 {name} 的工牌 |
| `badge_linked_done` | Badge lié · {name} | Badge linked · {name} | 工牌已绑定 · {name} |
| `badge_revoked_done` | Badge retiré · {name} | Badge removed · {name} | 工牌已解除 · {name} |
| `badge_log_link` / `badge_log_revoke` / `badge_log_settings` | Badge lié / Badge retiré / Réglages modifiés | Badge linked / Badge removed / Settings changed | 绑定工牌 / 解除工牌 / 设置已修改 |
| `badge_clock_in` | Arrivée · {name} · {time} | Clock-in · {name} · {time} | 上班 · {name} · {time} |
| `badge_clock_out` | Départ · {name} · {time} | Clock-out · {name} · {time} | 下班 · {name} · {time} |
| `badge_presence_failed` | Pointage refusé · {name} : {error} | Clock-in or out refused · {name}: {error} | 打卡失败 · {name}：{error} |
| `login_badge_hint` | Ou présentez votre badge | Or present your badge | 或出示您的工牌 |
| `end_shift_badge_hint` | Présentez votre badge pour terminer le service | Present your badge to end your shift | 出示工牌以结束上班 |
| `err_badge_reader_off` | Le lecteur de badges est désactivé | The badge reader is off | 工牌读卡器已关闭 |
| `err_badge_token_invalid` | Badge illisible : présentez-le de nouveau | Badge not readable: present it again | 无法读取工牌，请重新出示 |
| `err_badge_key_unavailable` | Clé des badges inaccessible : contactez l'assistance | Badge key unavailable: contact support | 工牌密钥不可用，请联系技术支持 |
| `err_badge_unknown` | Badge inconnu | Unknown badge | 未知工牌 |
| `err_badge_revoked` | Badge retiré : demandez un nouveau badge au Gérant | Badge removed: ask the manager for a new one | 工牌已解除，请向经理申请新工牌 |
| `err_badge_in_use` | Ce badge est déjà lié à un autre employé | This badge is already linked to another employee | 该工牌已绑定其他员工 |
| `err_badge_already_linked` | Cet employé a déjà un badge : retirez-le d'abord | This employee already has a badge: remove it first | 该员工已有工牌，请先解除 |
| `err_badge_none` | Cet employé n'a pas de badge | This employee has no badge | 该员工没有工牌 |
| `err_badge_settings_invalid` | Réglages du lecteur de badges invalides | Invalid badge reader settings | 工牌读卡器设置无效 |

The existing texts of `presence_self`, `presence_not_in`, `presence_training` and `user_unknown` are reused for those refusals.

### 4.5 Android UI (`android/app`)

- New package `dev.nuagemagique.pilipos.ui.badge` (owner `ui-common`, already covered by the `ui/.*` rule, so `LANES.md` needs no new rule): `BadgeSource` (interface: `scans: Flow<String>`, `start()`, `stop()`), `WedgeDetector` (pure Kotlin, same algorithm and defaults as Vue, injectable clock), `KeyWedgeSource` (fed by `MainActivity.dispatchKeyEvent`; holds and replays `KeyEvent`s, a `Handler` flushes after the gap), `NfcBadgeSource` (reader mode, L5-D13) behind an `NfcSupport` interface (`hasNfc`, `isEnabled`) so tests need no device, `BadgeStore` (settings `StateFlow`, capture for enrolment, `endShiftMode`, `pinRequest`, `onScan` -> engine -> toasts through `ToastStore`, session through `SessionStore.remember`, cue), `BadgeCue` (AudioTrack synthesis like `OrderChime`), `BadgeModule` (Hilt). The active source follows the setting: `off` none, `usb` `KeyWedgeSource`, `nfc` `NfcBadgeSource` when supported (else none, plus the Settings note).
- `MainActivity.kt` (foundation, edited on purpose): `dispatchKeyEvent` offers each event to `KeyWedgeSource` first; `onResume` / `onPause` attach and detach the NFC source. `AndroidManifest.xml`: NFC permission and optional feature (L5-D13).
- Settings (`feature/settings`): a badge section with the same content as `BadgeSettings.vue` (reader choice: "Désactivé", "NFC intégré" only when `FEATURE_NFC`, "Lecteur USB"; the notes), the employees row state and buttons, the enrol dialog (capture then the manager PIN dialog already used for PINs). `SettingsViewModel` / `SettingsState` load `get_badge_settings` and `list_badges` with the employees (all-or-nothing reload, as L1F-A9 (5)).
- Login (`feature/login`): the hints; `LoginViewModel` observes `pinRequest` (opens the PIN panel of that manager), `logged_in` (same navigation as a PIN login) and keeps `endShiftMode` in the store in step with its own mode.
- Android-only strings (`badge_reader_nfc`, `settings_badge_nfc_disabled`) in `strings_ui_settings.xml` with `-en` and `-zh` twins; the rest come from the Vue sources through the generator.
- Gradle notes (L1F-A8 (5)): `$HOME/.cargo/bin` first on PATH, `--no-daemon` if a daemon started with Homebrew's cargo exists.

### 4.6 Parity and docs of the port

- `docs/android/PARITY.md`: rows for the badge settings section, the employees badge actions, the login badge hints and the global scan handling, with their commands; the `pending:` lines of T1 removed by T3 (`check_ui_parity.py --strict` must pass at T5).
- `docs/android/parity/settings.md`, `docs/android/parity/login.md`: the badge behaviour, identical on both UIs except the NFC option (Android only, hardware).
- `docs/android/ENGINE.md` (commands, rules L5-D7 to L5-D10), `docs/android/FFI.md` (table, counts), `docs/android/DEVICE_TESTS.md` (device checks of section 9 Q1 to Q3).
- `CLAUDE.md`: wrapper counts, migrations sentence (70 `staff-badges`), a short "Staff badges" paragraph under the HubRise / table plan / EFT paragraphs (setting off by default, keyed hash, commands, manager PIN rule).

## 5. Tests (mandatory, every behaviour on its layers)

Engine (`crates/pos-engine/tests/badges.rs`, the testkit FDM mock as in `presence_manager.rs`): see the feature to test map (section 12). Assert on the FDM mock requests (one `signWorkIn`/`signWorkOut` with `employeeId` = the badge's employee, label S; none for refusals), on `fiscal_events`, `staff_shifts`, `badge_log`, `badge_scans` and the counters. Add the six commands to `access_control.rs` where they need a role, and the nine codes to `error_codes.rs`. One test reads the raw bytes of the database file (and the WAL) after linking and tapping a known token and asserts the token, in hex and in decimal form, appears nowhere, and that `badge.key` is not inside `pos.db`.

FFI: the six methods, the records (field names, defaults), the error count 128.

Playwright (`ui/e2e/badges.spec.ts`, handlers in `ui/e2e/fixtures/mock-ipc.ts`; keystroke bursts with `page.keyboard.type(token, { delay: 5 })` then `Enter`, human typing with `{ delay: 150 }`; the wedge maps `event.code`, which Playwright sets): reader off ignores bursts; burst detection; normal typing passes to the field in order; a burst never reaches a focused text field; short and long bursts are replayed; the configured gap is honoured; every outcome's toast text (FR, plus one ZH and one EN); login and PIN request on the login screen; end-shift mode flag; settings panel and enrolment and revocation; manager-only visibility.

Kotlin (JVM, Robolectric where a `KeyEvent` or `Context` is needed): `WedgeDetectorTest`, `KeyWedgeSourceTest`, `NfcBadgeSourceTest` (fake `NfcSupport`, tag id to hex), `BadgeStoreTest` (fake source, fake engine: every outcome, capture routing, off = no source started), `SettingsViewModelTest` / `SettingsContentTest` additions, `LoginViewModelTest` additions. Roborazzi: re-record only the goldens that change (expected: `settings/settings_main.png`, `settings/settings_users.png`, new `settings/settings_badges.png`, new `settings/settings_badge_enrol.png`, `login/` goldens that show the hint if the fixture turns the reader on) and LOOK at them.

## 6. Dossier changes (French, brand-neutral, both dossiers unless noted; T4)

- `REPONSES_MANUEL_CERT_FR.md`: §III.E new paragraph "Pointage par badge" (functions `commands::badges::do_badge_tap`, `do_link_badge`, `do_revoke_badge`, `social::sign_work_event`; L5-D7 to L5-D10; the S event identical to M140/M141; FDM and training rules; the badge is an internal identification, §6.4 and §1.9 cited); the "Work In / Work Out buttons" answer stays "Oui" with a note that a badge scan is an equivalent trigger; the "Configuration du POS - C. Utilisateurs & Mots de Passe" section: login by PIN, or by badge for cashiers and waiters when enabled, managers always with their PIN. `scripts/check_dossier.py` checks backticked Rust identifiers: name only real ones.
- `MANUEL_UTILISATEUR_FR.md`: §2.1 (connexion par badge, PIN pad for managers), §2.3 (fin de service par badge), §3 (pointage par badge depuis n'importe quel écran, toast, son, double passage ignoré 10 s, refus FDM et formation), §12.3 (lier, retirer, remplacer un badge), new §12.10 "Lecteur de badges" (réglage, NFC intégré sur Android si l'appareil en a, lecteur USB, réglages de délai, clé renouvelée), §14.1 table (every new message), the RGPD note.
- `PROFILS_UTILISATEURS_FR.md`: who may link and revoke (Gérant, with PIN), badge login rights per profile, the manager PIN rule.
- `LISTE_FONCTIONNALITES_FR.md`: S rows "Pointage par badge (arrivée, départ)" with `crates/pos-engine/src/commands/badges.rs::do_badge_tap`; the training row (badge refused in training).
- `PROFIL_ENTREPRISE_FR.md` §6 (RGPD, both if Android has it): data processed (keyed hash of the badge UID, link history, scan journal with time and INSZ), no biometrics, purpose limited to presence registration and till login, retention (kept with the presence data, the hash is useless without the key, the key never leaves the terminal), legal basis (employer's legitimate interest / social law presence registration), the employee may refuse a badge (PIN and manager registration remain).
- `FEUILLE_ACQUISITION_DONNEES_FR.md`, `SECURITE_BD_FR.md` (the four tables, append-only triggers, revoke-only trigger, `badge.key` outside the database and outside backups and exports), `FLUX_DONNEES_FR.md` (a badge flow: reader -> UI -> engine -> hash -> FDM S event), `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md` (presence section), `MAPPING_CONFORMITE_FR.md` (rows for the S event by badge and the RGPD note).
- Linux only: `SCHEMA_BD_FR.md` (new §3.x for the four tables; §6 the migration list: twenty-one migrations, 1 to 11, 20 to 28, 70), `SCHEMA_ER_FR.md` §7 (same), `MATRICE_TRACABILITE_FR.md` if it lists presence functions.
- `MANUEL_INSTALLATION_FR.md` and `CHECKLIST_DEPLOIEMENT_KIOSK_FR.md` / `CHECKLIST_DEPLOIEMENT_FR.md`: USB HID wedge reader (keyboard emulation, UID then Enter, any layout), test a badge after install; Android: the D3 Pro main unit has no NFC (NFC intégré only with hardware that reports it, the customer display NFC to be confirmed on site), and the Android setting "show the on-screen keyboard while a physical keyboard is connected" must stay on when a USB reader is plugged (Q2). `PLATEFORME_ANDROID_FR.md`: the NFC permission and optional feature.
- `INDEX_DOSSIER_FR.md` (Android) and both `MANIFEST.txt` only if a file is added or a count changes.
- Rebuild the PDFs of every changed annex: `scripts/make_dossier_pdfs.sh <files>` and `PILI_DOSSIER_DIR=docs/android/dossier scripts/make_dossier_pdfs.sh <files>` (an Android `commun:` document has no Android PDF: see its MANIFEST).
- Captures runbook entries (`docs/internal/CAPTURES_ECRAN_RUNBOOK_FR.md`, Android rows) for section 11; `ui/e2e/captures.spec.ts` gets the Linux scenarios (T2), no PNG generated.
- Grep both dossiers for `pointage`, `signWorkIn`, `Présence seule`, `PIN` in the presence and login descriptions and fix every description that would now be incomplete. Never name the restaurant or any client. No version other than 1.0.0.

## 7. Task split (sequential)

Common to every task: the environment prefix of the lane brief on every Bash call; heavy commands through `scripts/android/heavy.sh`; `df -g /` before heavy builds (under 10 GB: `cargo clean --target-dir target` in this worktree and `(cd android && ./gradlew clean)`; under 6 GB after that: stop with `DISK_LOW`); no emulator, no adb, no network to real vendors; Conventional Commits ending with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`; never an em-dash; decisions appended only at the end of `docs/android/DECISIONS.md` under `## Lane 5-nfc (2026-10-17)` (T1 creates the heading); `(cd ui && npm ci)` once if `ui/node_modules` is missing; each task updates the feature to test map (section 12) and adds its "Worker notes" line and its extra captures to section 11.

Gate shorthands (from the worktree root):
- **G-RUST**: `scripts/android/heavy.sh cargo fmt --all -- --check`; `scripts/android/heavy.sh cargo clippy --workspace --all-targets -- -D warnings`; `scripts/android/heavy.sh cargo clippy -p pili-pos --no-default-features --features cert --all-targets -- -D warnings`; `scripts/android/heavy.sh cargo test --workspace`; `scripts/android/heavy.sh cargo test -p pili-pos --no-default-features --features cert`; `scripts/android/heavy.sh cargo test -p pos-engine --no-default-features --features cert`.
- **G-FFI**: `(cd android/rust && cargo fmt --all -- --check && ../../scripts/android/heavy.sh cargo clippy --workspace --all-targets --locked -- -D warnings && ../../scripts/android/heavy.sh cargo test --workspace --locked)` (see `scripts/android/p2_gate.sh` `in_rust` for its environment).
- **G-PARITY**: `python3 scripts/android/check_ipc_wrappers.py`; `python3 scripts/android/check_command_parity.py`; `python3 scripts/android/check_ui_parity.py`; `node scripts/android/vue_to_android.mjs --check`; `python3 scripts/android/lane_paths.py --check`; `python3 -m unittest scripts/android/test_check_command_parity.py scripts/android/test_check_ui_parity.py` if their counts changed.
- **G-VUE**: `(cd ui && npm run build)`; `(cd ui && ../scripts/android/heavy.sh npx playwright test)`.
- **G-ANDROID-LITE**: `(cd android && ../scripts/android/heavy.sh ./gradlew --no-daemon :pos-core:apiCheck :app:compileDebugUnitTestKotlin :app:testDebugUnitTest)`.
- **G-ANDROID**: `bash scripts/android/p5_gate.sh` and `bash scripts/android/p6_gate.sh` (read their headers; expected failures L5-D16 only).
- **G-DOSSIER**: `python3 scripts/check_dossier.py`; `PILI_DOSSIER_DIR=docs/android/dossier python3 scripts/check_dossier.py`.

| Task | Title | Gates before its commits |
|---|---|---|
| T1 | Engine, database, shell and FFI for badges | G-RUST, G-FFI, G-PARITY, G-ANDROID-LITE |
| T2 | Vue: USB wedge reader, badge store, settings, enrolment, login, strings, Playwright | G-PARITY, G-VUE, G-ANDROID-LITE (the generator rewrites Android string files), `scripts/android/heavy.sh cargo test -p pos-engine --test error_codes --test badges` (sanity) |
| T3 | Android: badge sources (USB wedge, NFC), store, settings, enrolment, login, tests, goldens | G-PARITY with `check_ui_parity.py --strict`, G-ANDROID |
| T4 | Dossiers, manuals, GDPR note, port docs, PDFs, captures runbook | G-DOSSIER, `python3 scripts/android/check_ui_parity.py --strict` (sanity) |
| T5 | Full gate + fixes | the complete gate (section 8) |

### 7.1 T1 detail

Migration 70 + `schema_badges.rs` + tests; `pos_core::badge`; `pos_db::queries::badges`; `badge_key.rs` + open-time check; `commands/badges.rs`; deactivation revokes; `EngineTimers.badge_repeat`; `AppState.badge_repeats`; login factoring; nine error codes + `i18n.ts` texts + generator; Tauri wrappers; FFI exports, records, tests, API dump, FFI.md; `ipc.ts` wrappers and types; PARITY pending lines; `crates/pos-engine/tests/badges.rs` covering every engine row of section 12; `access_control.rs`, `error_codes.rs`; DECISIONS L5-S1, L5-D1 to L5-D16 (with any worker refinement). Commits: `feat(db)`, `feat(engine)`, `feat(android)` for the FFI, `docs(android)` for DECISIONS and FFI.md, or as the worker sees fit, each compiling and green.

### 7.2 T2 detail

Section 4.4 and its Playwright tests; `captures.spec.ts` scenarios `19b_admin_badges`, `19c_badge_enrol`, `03b_badge_toast` (names may change, then fix section 11); `docs/android/parity/settings.md` and `login.md` Vue side. Re-run `node scripts/android/vue_to_android.mjs --write` after the strings and commit the generated files with them.

### 7.3 T3 detail

Section 4.5 and its Kotlin tests; Roborazzi goldens re-recorded and looked at; remove the six `pending:` lines; `docs/android/PARITY.md` rows; `docs/android/parity/*.md` Android side; `docs/android/DEVICE_TESTS.md` entries.

### 7.4 T4 detail

Section 6 and `CLAUDE.md`, `docs/android/ENGINE.md`. Rebuild the PDFs. No code.

## 8. Full gate (T5)

`cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo clippy -p pili-pos --no-default-features --features cert --all-targets -- -D warnings`; `cargo test --workspace`; `cargo test -p pili-pos --no-default-features --features cert`; `cargo test -p pos-engine --no-default-features --features cert`; the FFI tests in `android/rust` (G-FFI); `python3 scripts/android/check_ipc_wrappers.py`; `python3 scripts/android/check_command_parity.py`; `python3 scripts/android/check_ui_parity.py --strict`; `node scripts/android/vue_to_android.mjs --check`; `(cd ui && npm run build && npx playwright test)`; `bash scripts/android/p5_gate.sh`; `bash scripts/android/p6_gate.sh`; `python3 scripts/check_dossier.py`; `PILI_DOSSIER_DIR=docs/android/dossier python3 scripts/check_dossier.py`; an em-dash scan of the lines added since `05a8b6c5`; a brand-name scan of the dossiers. Every heavy one through `heavy.sh`. Compare with `moyun-pos-wt/gates-rc2.log` and with the lane 1 results (`docs/android/phases/LANE_V1_FIXES.md` "Results": workspace 1834 passed, FFI 258, Playwright 365, Kotlin 2099): no count may go down, Kotlin tests removed 0. Write the results under "Results" below (PASS/FAIL per step, counts) in the final docs commit.

## 9. Risks and open questions

| # | Risk or question | Recommendation |
|---|---|---|
| Q1 | Does the D3 Pro customer display NFC appear as a standard `NfcAdapter` (FEATURE_NFC) on the main unit? | Unknown until a device test (DEVICE_TESTS.md). The design does not depend on it: USB wedge reader by default on the D3 Pro; "NFC intégré" appears by itself if the device reports it. |
| Q2 | Android hides the on-screen keyboard while a hardware keyboard (the wedge reader) is connected, so Settings text fields (employee name) could become hard to fill. | Installation step: keep "Afficher le clavier virtuel" on (Android physical keyboard setting); check on the device (DEVICE_TESTS.md). Text fields are rare (manager settings only). |
| Q3 | Wedge readers differ (hex vs decimal, byte order, some send a prefix or Tab instead of Enter). | Physical-key mapping, configurable gap and length; a reader that ends with anything but Enter must be configured to send Enter (installation manual). Prefix/suffix stripping is not built; record. |
| R1 | Another lane takes migration 70. | Keep 70; the orchestrator renumbers (L5-D15). |
| R2 | A slow Android USB host stack batches characters with gaps over 50 ms. | `wedge_max_gap_ms` up to 200; default 50 to be checked on the device. |
| R3 | `EngineTimers` gains a field: every constructor (shell, FFI, testkit, tests) must be updated. | T1 compiles every crate and the FFI; prefer `..Default::default()` where already used. |
| R4 | FFI record changes break Kotlin call sites and the API dump. | T1 regenerates `pos-core.api` and compiles the app unit tests in the same commit. |
| R5 | The lane edits frozen paths (Linux build inputs, FFI, dossier, `MainActivity.kt`, manifest). | User-authorized for v1.0.0; say so in commit bodies; expected gate failures L5-D16. |
| R6 | A lost `badge.key` (restore on new hardware) makes every badge unknown. | L5-D4: automatic revocation with reason `key_changed`, Settings banner, manual step. |
| R7 | GDPR: badges are personal data linked to employees. | Hash only, purpose limitation, no biometrics, key never leaves the terminal, note in the manual and the company profile (section 6). |
| R8 | Replay of held keys into a Vue input loses the caret position or a `maxlength`. | `setRangeText` at the selection, then `input` event; Playwright checks the field value and order. |
| R9 | Heavy gates on a shared 16 GB machine. | heavy.sh for everything heavy, disk check first. |

## 10. Worker notes

(Each task adds one line here: deviations from the plan, decisions added, files touched that the plan did not name.)

- **T1 (Sonnet 5.5).** Engine, database, shell and FFI done as planned. Deviations and additions: `link_badge` takes `operator_pin: Option<String>` (a missing PIN is `pin_required`, as `set_pin`); the lost-key check also runs before every scan and link (`badge_key::key_for_use`), not only at open; a clock-out in "Fin de service" mode is refused with `presence_training` in training mode (L5-D9 refinement); `SessionRules::last_activity` is now public (test of L5-D11); `pos-engine/Cargo.toml` gained `hmac`, `sha2`, `hex` and `getrandom` (so the `pos-engine` entry changed in both `Cargo.lock` files); the testkit `AppState` literal and the FFI `timers_from` list `badge_repeat`; `pos-db` `ActiveBadge` and `LogEntry` serialize camelCase and are the types the shell and the FFI return. Decisions: `docs/android/DECISIONS.md`, section `## Lane 5-nfc (2026-10-17)`.
- **T2 (Sonnet 5.5).** Vue UI done as planned. Deviations and additions: the badge panel (`BadgeSettings.vue`) is folded by default behind its header (`badge-toggle`) because the third column is already full and an open panel squeezed the others; the employee row carries the badge state and its button on a second line inside the text block (four buttons on one line crushed the text); new string `badge_capture_timeout` (30 s capture timeout) and `settings_badge_log`, `settings_badge_log_empty`; `utils/postLogin.ts::goAfterLogin` is the factored post-login navigation (PIN, INSZ, first PIN and badge); `useBadgeReader(onReplay)` forwards replayed keys as user activity; the detector keeps the token (physical keys) and the replay text (`event.key`) apart and treats the tail of an over-long burst as typing (decisions L5-W3 to L5-W7); the Playwright mock settings default to a 80 ms gap (the plan default is 50) so a loaded CI machine does not flake; `captures.spec.ts` skips the PNG when `PILI_CAPTURES_DRY` is set. Files the plan did not name: `ui/src/utils/postLogin.ts`, `ui/e2e/fixtures/mock-ipc.ts` (`badgeInit`, `scanBadge`, six handlers).
- **T3 (Sonnet 5.5).** Android UI done as planned (package `ui/badge`, settings, enrolment, login, `MainActivity`, manifest). Deviations and additions: the "NFC intégré" choice and its note are Android-only strings (`ui_settings_badge_reader_nfc`, `ui_settings_badge_nfc_disabled`, fr/en/zh), the choice is drawn only when `FEATURE_NFC` is present (the flag for a D3 Pro main unit without NFC); the panel sits right under the table plan and is folded by default as on Vue; the employee badge line is a full-width second line under the row (not inside the text block, which four buttons crush on the 10-inch layout); `KeyWedgeSource.detach()` drops held keys instead of replaying them into a window that is going away; `MainActivity` carries `@SuppressLint("RestrictedApi")` because lint flags `ComponentActivity.dispatchKeyEvent` (the platform method, restricted by an androidx.core annotation); the Deliveries content test no longer asserts `assertIsDisplayed` (the folded badge header pushes the panel below the fold; it asserts the node exists); `presenceFailedShowsTheFdmText` became `presenceFailedShowsTheRefusalToast` because the raw engine text goes through `ErrorPresenter`, which maps unknown text to the generic error. Decisions L5-A1 to L5-A8 in DECISIONS.md.
- **T4 (Sonnet 5.5).** Dossiers, manuals and port docs done, no code. Deviations and additions: in the Android manual the new section is §12.13 "Lecteur de badges" (its §12.10 to §12.12 are "À propos", "Export pour le SPF" and "Mise à jour"; the Linux manual's §12.10 is free after the table plan), and `REPONSES_MANUEL_CERT_FR.md` cites the right number in each dossier; the Android `FEUILLE_ACQUISITION_DONNEES_FR.md` section is §1.8 (Linux: §1.6 bis); the table-of-contents of both manuals is unchanged (section 12 and 14 already listed); `MATRICE_TRACABILITE_FR.md` is left alone (it maps the UC fixtures M1xx to tests, a badge scan is not a UC; the badge tests are in `MAPPING_CONFORMITE_FR.md` rows 17b and 17c); `INDEX_DOSSIER_FR.md` and both `MANIFEST.txt` need no change (no file added, no annex count changed); `docs/android/ENGINE.md` command map regenerated by hand to 104 wrappers (the `clock_in_employee` and `clock_out_employee` rows moved to their alphabetical place) and a `badge_repeat` timer row and a section 7 row added; `CLAUDE.md` already had 104/102 from T1, T4 added the migration 70 text, the `badge_key.rs` file and the "Staff badges" paragraph. Android `PLATEFORME_ANDROID_FR.md` gets §3.10 and a §9 row for the D3 Pro customer display NFC (Q1). PDFs rebuilt for every changed annex in both dossiers (Android SCHEMA_BD and SCHEMA_ER are `commun:`, no Android PDF). Decisions L5-T4-1 to L5-T4-3 in DECISIONS.md.

- **T5 (Sonnet 5.5).** Full gate run, no code fix needed. One extra expected failure, `hygiene-root-lock` (p5, p6): see L5-G1 in DECISIONS.md. Results table below.

## 11. Captures to retake (orchestrator, after the merge)

Linux (`ui/e2e/captures.spec.ts`, `npm run captures`; workers update the scenarios and mock data, not the PNGs):
- `19_admin_users.png`: employees with the badge state and the "Lier un badge" / "Retirer le badge" buttons.
- New `19b_admin_badges.png` (proposed): the "Lecteur de badges" panel (Lecteur USB, connexion par badge on, timing, log, privacy note).
- New `19c_badge_enrol.png` (proposed): "Présentez le badge de ..." dialog.
- New `03b_badge_toast.png` (proposed): the order screen with an "Arrivée · Nom · HH:MM" toast while a cashier is logged in.
- `01_login.png` (or the login capture in use): the "Ou présentez votre badge" hint, only if the capture data turns the reader on (done as recommended: a separate new `01b_login_badge.png`, `01_login.png` unchanged).
- Runbook entries in `docs/internal/CAPTURES_ECRAN_RUNBOOK_FR.md` for each (T2 wrote them: `19_admin_users` updated, `19b_admin_badges`, `19c_badge_enrol`, `03b_badge_toast`, `01b_login_badge`), and the Linux capture PDF must include the new ones. The scenarios exist in `ui/e2e/captures.spec.ts` under those names; `PILI_CAPTURES_DRY=1 npm run captures` runs them without writing a PNG. The `19b` capture unfolds the panel first.

Android (`docs/android/dossier/screenshots`, emulator; the emulator has no NFC, so the NFC option capture needs a device or is skipped):
- `A46_employes.png` (badge state and buttons).
- Android goldens (Roborazzi, recorded by T3): new `settings/settings_badges.png`, `settings/settings_badge_enrol.png`, `login/login_badge.png`, `login/login_end_shift_badge.png`; re-recorded `settings/settings_main.png`, `settings_users.png`, `settings_hubrise.png`, `settings_hubrise_connect.png`. The dossier PNGs `A46` and `A126` to `A129` need a device or emulator and are retaken by the orchestrator (runbook rows written).
- New `A126_lecteur_badges.png` (badge settings section), `A127_lier_badge.png` (enrol dialog), `A128_pointage_badge.png` (presence toast), `A129_connexion_badge.png` (login hint) (proposed ids; T3 adds the runbook rows).

## 12. Feature to test map

Kept up to date by every task (add the real test names when written). "eng" is `crates/pos-engine/tests`.

| Feature | Layer | Test(s) (planned names) |
|---|---|---|
| Migration 70: tables, defaults, CHECKs, partial unique indexes, revoke-only and append-only triggers, no re-run | db | `pos-db` `schema_badges::tests::staff_badges_migration_*` (7 tests), `migrate::tests::the_base_migrations_are_the_certified_ones_then_presence_then_badges` |
| Token normalization (trim, uppercase, hex/decimal only, 4 to 64) | core | `pos-core` `badge::tests::normalize_*` |
| Settings validation (ranges, min <= max, reader values) | core, engine | `pos-core` `badge::tests::settings_*`; eng `badges.rs` `invalid_settings_are_refused_and_not_logged` |
| Setting off by default; off = no reading: `badge_tap` and `link_badge` refused | engine, Vue, Android | eng `a_fresh_install_has_the_reader_off_and_refuses_every_scan`; Playwright `with the reader off a burst is typed into the field and nothing is sent`; Kotlin `BadgeStoreTest.readerOffStartsNoSource`, `KeyWedgeSourceTest.readerOffPassesEveryKey` |
| Settings changed by managers only, every change logged | engine, Vue, Android | eng `only_a_manager_changes_the_badge_settings_and_each_change_is_logged`, `access_control.rs` `only_a_manager_manages_badges_and_a_scan_needs_no_session`, `error_codes.rs` `the_nine_badge_codes_are_listed_and_translated`; Playwright `the badge panel is for managers only`; Kotlin `SettingsBadgeModelTest.choosingAReaderSendsTheWholeSetting`, `aRefusedSettingShowsItsTextAndKeepsTheStoredValue`, `theLoginSwitchFlipsLoginEnabled` |
| Hash only: HMAC with the installation key, raw UID never stored (DB file bytes), key file 0600, outside pos.db | engine | `badge_key::tests::*` (known HMAC vector, key check, created once, mode 0600 and tightened, wrong size, open check cases); eng `the_raw_badge_number_is_nowhere_in_the_database_file`, `the_token_is_normalized_before_it_is_hashed` |
| Lost key revokes every badge (`key_changed`), unreadable key refuses without revoking | engine | eng `a_new_key_revokes_every_active_badge_with_key_changed`, `an_unreadable_key_refuses_and_revokes_nothing`, `opening_the_engine_without_the_key_revokes_the_badges` |
| Enrolment: manager + own PIN, any active user incl. presence-only, refusal order | engine | eng `a_manager_links_a_badge_to_a_presence_only_employee`, `link_refusals_come_in_the_documented_order`, `a_wrong_manager_pin_links_nothing` |
| Uniqueness: one active badge per employee, one employee per badge | db, engine | db partial index tests; eng `an_employee_cannot_hold_two_badges`, `a_badge_cannot_be_linked_to_two_employees` |
| Revocation and re-issue; deactivation revokes | engine | eng `a_revoked_badge_is_refused_as_revoked_and_can_be_linked_again`, `deactivating_an_employee_revokes_their_badge`, `revoke_without_badge_is_badge_none` |
| Audit log of link/revoke/settings with who did it | engine, Vue, Android | eng `every_link_and_revocation_is_logged_with_the_manager`; Playwright `the badge log lists links and removals`; Kotlin `SettingsBadgeContentTest.theLogListsLinksAndSettingsChanges`, `anEmptyLogSaysSo` |
| Clock-in/out by badge from any screen, S event in the employee's name, cashier stays logged in | engine, Vue, Android | eng `a_scan_while_a_cashier_is_logged_in_clocks_the_badge_holder_in_and_keeps_the_session`, `a_second_scan_after_the_window_clocks_out`; Playwright `a badge on the order screen shows Arrivée with name and time`; Kotlin `BadgeStoreTest.clockInOutcomeShowsTheToast` |
| Own badge while logged in refused (`presence_self`) | engine | eng `the_operators_own_badge_is_refused_as_presence_self` |
| FDM down: `presence_failed`, nothing archived, index unchanged, FDM text shown | engine, Vue, Android | eng `an_fdm_failure_on_a_badge_scan_archives_nothing`; Playwright `an FDM failure shows the FDM text`; Kotlin `BadgeStoreTest.presenceFailedShowsTheRefusalToast` |
| Training: badge presence refused, badge login allowed (T WorkIn, no shift) | engine | eng `in_training_a_badge_scan_is_refused_and_signs_nothing`, `in_training_a_badge_login_opens_no_shift` |
| Debounce: same badge in flight or within the window ignored, other badges not, after the window toggles | engine, Android | eng `a_double_tap_never_clocks_in_and_out`, `a_scan_during_a_slow_fdm_answer_is_ignored`, `two_badges_are_not_debounced_together` (short `badge_repeat` in the test timers) |
| Unknown and revoked badges refused and logged in `badge_scans` | engine, Vue | eng `unknown_and_revoked_badges_are_refused_and_logged`, `refusals_are_debounced_too`; Playwright `the refusal <code> is shown as its text` (8 codes), `an ignored repeat shows nothing` |
| Badge login allowed for cashier/waiter, refused for presence-only (presence toggle instead), off when `login_enabled` is off | engine, Vue, Android | eng `a_cashier_logs_in_with_a_badge_and_is_clocked_in`, `a_presence_only_badge_on_the_login_screen_toggles_presence`, `with_badge_login_off_the_login_screen_toggles_presence`; Playwright `a badge on the login screen logs a cashier in`; Kotlin `LoginBadgeTest.badgeLoginNavigatesLikeAPinLogin`, `BadgeStoreTest.loggedInOpensTheSessionAndAnnouncesIt` |
| Manager badge needs the PIN: `pin_required`, no session, admin commands still refused until the PIN login | engine, Vue, Android | eng `a_manager_badge_opens_no_session_and_manager_commands_stay_refused`, then `the_pin_login_after_pin_required_opens_the_manager_session`; Playwright `a manager badge opens the PIN pad for that manager`; Kotlin `LoginBadgeTest.pinRequiredOpensThePinPanel`, `pinRequiredForAManagerTheTilesDoNotKnowStillOpensThePad`, `BadgeStoreTest.pinRequiredAsksForThatManagersPin` |
| End-shift mode: badge clocks out, no session, `presence_not_in` when off shift | engine, Vue, Android | eng `end_shift_mode_clocks_out_without_a_session`; Playwright `in Fin de service mode the badge is sent with end_shift`; Kotlin `LoginBadgeTest.theEndShiftModeIsToldToTheStoreAndSentWithTheScan`, `aBadgeClockOutClosesTheEndShiftPadAndReloadsTheShifts`, `BadgeStoreTest.endShiftModeIsSentWithTheScan` |
| Badge scan is not user activity | engine | eng `a_presence_scan_does_not_restart_the_idle_timer` |
| Wedge: burst detected (code mapping, Enter, gap, length) | Vue, Android | Playwright `a fast burst ending with Enter is a badge`; Kotlin `WedgeDetectorTest.burstEndingWithEnterIsABadge`, `physicalKeysNotCharacters` |
| Wedge: normal typing never a badge, replayed in order, no leak of a burst into a focused field | Vue, Android | Playwright `human typing reaches the field in order`, `a burst never reaches a focused field`, `a short burst is replayed into the field, Enter included`, `a long burst is replayed whole and its tail is not a badge`, `a key that is not a badge key breaks the burst and keeps the order`, `the configured gap is honoured: slower than the gap is typing` and `...: a wider gap makes a slow reader work`, `the configured length is honoured`; Kotlin `WedgeDetectorTest.humanTypingIsReplayed`, `KeyWedgeSourceTest.heldEventsReplayInOrder` |
| NFC source only with FEATURE_NFC; tag id to uppercase hex; reader mode follows the setting | Android | Kotlin `NfcBadgeSourceTest.*`, `SettingsBadgeContentTest.theNfcChoiceNeedsFeatureNfc`, `aDeviceWithNfcOffersTheBuiltInReader`, `aStoredNfcChoiceOnADeviceWithoutNfcGetsTheUnavailableNote`, `SettingsBadgeModelTest.aDeviceWithoutNfcOffersNoNfcChoice` |
| Enrolment UI (capture, PIN, link, errors) and revocation UI | Vue, Android | Playwright `linking a badge asks for it then for the manager PIN`, `a badge already linked to someone else is refused with its text`, `a wrong manager PIN keeps the dialog for another try`, `no badge within 30 seconds ends the dialog`, `Annuler leaves nothing linked`, `removing a badge asks for a confirmation first`; Kotlin `SettingsBadgeModelTest.linkingAsksForTheBadgeThenForTheManagersPin`, `aWrongManagerPinKeepsTheDialogForAnotherTry`, `aBadgeAlreadyLinkedEndsTheDialogWithItsText`, `noBadgeWithinThirtySecondsEndsTheDialog`, `SettingsBadgeDialogsTest.*` |
| Strings FR, ZH, EN | Vue, Android | Playwright `English texts`, `Chinese texts`, `the texts follow the language (Chinese)`; `vue_to_android.mjs --check`; Android-only twins checked by lint |
| FFI: six methods, records, error count 128 | ffi | `android/rust/pos-ffi/tests/ffi/badges.rs` (`the_settings_view_converts`, `the_settings_input_converts_field_by_field`, `the_overview_converts_with_and_without_optional_fields`, `a_link_converts`, `every_outcome_kind_converts_and_keeps_its_wire_name`, `an_outcome_converts_with_its_fields_and_its_user`, `the_reader_is_off_by_default_and_every_badge_command_says_so`, `settings_enrolment_scan_and_revocation_go_through_the_facade`, `a_manager_badge_on_the_login_screen_asks_for_the_pin`), `every_code_of_both_lists_maps_to_coded` (128) |
| Wedge: physical keys (AZERTY), keypad keys, Shift neutral, a scan is not user activity | Vue, Android | Playwright `the keys are read as physical keys, whatever the layout types`, `the numeric keypad keys count too`, `Shift held by the reader does not break the burst`, `a scan is not user activity`; Kotlin `WedgeDetectorTest.physicalKeysNotCharacters` |
| Settings panel: folded by default, reader choice, login switch, timing on the pad, refused setting, NFC note, relink banner | Vue, Android | Playwright `a manager sees the panel and changes the reader`, `the badge login switch and its note`, `the USB timing is typed on the pad and saved`, `a refused setting shows its text and keeps the stored value`, `a stored NFC choice gets the unavailable note, and the relink banner shows` |
| Employees panel badge state and buttons; registered scan refreshes who is on shift | Vue, Android | Playwright `each row says if it has a badge and offers the right button`, `with the reader off the rows have no badge state and no button`, `a badge registering someone refreshes who is on shift`; Kotlin `SettingsBadgeContentTest.eachRowSaysIfItHasABadgeAndOffersTheRightButton`, `theBadgeButtonCallsBackWithTheEmployee`, `withTheReaderOffTheRowsHaveNoBadgeState`, `SettingsBadgeModelTest.aRegisteredScanReloadsTheEmployees` |
| Login hints (`login_badge_hint`, `end_shift_badge_hint`) | Vue, Android | Playwright `the hint shows when badge login is on, and not otherwise`, `in Fin de service mode the badge is sent with end_shift...`; Kotlin `LoginBadgeTest.theHintsFollowTheReaderAndTheLoginSwitch`, `aReaderWithoutBadgeLoginOnlyHintsTheEndOfShift`, `withTheReaderOffThereIsNoHint` |
| Activity hook: a held wedge key is swallowed and is not user activity; a digit key is untouched with the reader off | Android | `MainActivityTest.aKeyHeldByTheUsbBadgeWedgeIsSwallowedAndIsNotUserActivity`, `withTheReaderOffADigitKeyIsNotTakenByTheBadgeWedge`, `KeyWedgeSourceTest.detachingTheActivityReleasesTheHeldKeysIntoNothing` |
| Cues (ok, refused) and goldens | Android | `BadgeCueTest.*`; Roborazzi `SettingsScreenshotTest.badges`, `badgeEnrol`, `LoginScreenshotTest.tilesWithBadgeHint`, `endShiftBadgeHint` (new goldens; `settings_main`, `settings_users`, `settings_hubrise*` re-recorded for the folded badge panel and the employee badge line) |
| Dossier text consistency | dossier | `scripts/check_dossier.py` both dossiers (`python3 scripts/check_dossier.py`, `PILI_DOSSIER_DIR=docs/android/dossier python3 scripts/check_dossier.py`: rust identifiers `commands::badges::{do_badge_tap,do_link_badge,do_revoke_badge}`, `social::sign_work_event`, paths, brand, em-dash); updated annexes (both dossiers unless noted): `REPONSES_MANUEL_CERT_FR` (§III.E "Pointage par badge", users and passwords), `MANUEL_UTILISATEUR_FR` (§2.1, §2.3, §3, §12.3, new §12.10 Linux / §12.13 Android, §14.1, RGPD), `PROFILS_UTILISATEURS_FR`, `LISTE_FONCTIONNALITES_FR`, `PROFIL_ENTREPRISE_FR` §6, `FEUILLE_ACQUISITION_DONNEES_FR`, `SECURITE_BD_FR` §2.5, `FLUX_DONNEES_FR` §6 ter, `DOCUMENT_PROCESSUS_TECHNIQUE_FR`, `MAPPING_CONFORMITE_FR` rows 17b and 17c, `MANUEL_INSTALLATION_FR` §4.4 bis, `CHECKLIST_DEPLOIEMENT_KIOSK_FR` / `CHECKLIST_DEPLOIEMENT_FR`, Linux only `SCHEMA_BD_FR` §2.6 and §6 and `SCHEMA_ER_FR` §2.10, §4 and §7, Android only `PLATEFORME_ANDROID_FR` §3.10; PDFs rebuilt |

## Results

T5 full gate, run on the branch head before this commit (base `05a8b6c5`). Every heavy step went through `scripts/android/heavy.sh`.

| Step | Result | Counts |
|---|---|---|
| `cargo fmt --all -- --check` | PASS | |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | |
| `cargo clippy -p pili-pos --no-default-features --features cert --all-targets -- -D warnings` | PASS | |
| `cargo test --workspace` (unit and integration targets) | PASS | 1918 passed, 0 failed (base 1800, lane 1 1834) |
| `cargo test -p pili-pos --no-default-features --features cert` | PASS | 8 passed |
| `cargo test -p pos-engine --no-default-features --features cert` | PASS | 739 passed |
| FFI (`android/rust`): fmt, clippy `--locked -D warnings`, test `--locked` | PASS | 267 passed (lane 1 258) |
| `check_ipc_wrappers.py`, `check_command_parity.py` | PASS | |
| `check_ui_parity.py --strict` | PASS | called 91, excluded 9, pending 0, stale 0, missing 0 (100 commands) |
| `vue_to_android.mjs --check` | PASS | 602 keys, error codes 105 + 23 |
| `cd ui && npm run build` | PASS | |
| `cd ui && npx playwright test` | PASS | 423 passed (lane 1 365) |
| `p5_gate.sh` | 5 FAIL, 29 PASS | expected: `hygiene-paths`, `linux-frozen`, `freeze-tag`, `app-only-scope`; plus `hygiene-root-lock` (see L5-G1). Kotlin tests: base 2104, head 2251, removed 0; app unit tests 2204; kover LINE 97.2%, BRANCH 75.2% |
| `p6_gate.sh` | 4 FAIL, 53 PASS | expected: `hygiene-paths`, `linux-frozen`, `freeze-tag`; plus `hygiene-root-lock` (L5-G1). lint, ktlint, detekt, unit tests, Roborazzi verify, release APK checks PASS |
| `check_dossier.py` (Linux) | PASS | |
| `PILI_DOSSIER_DIR=docs/android/dossier check_dossier.py` | PASS | |
| Em-dash scan of lines added since `05a8b6c5` | PASS | 0 (also `hygiene-emdash`) |
| Brand scan of both dossiers | PASS | no brand in any annex (`check_dossier.py` brand check); one untouched evidence file only holds a checkout path |

Notes:
- The first `cargo test --workspace` run had one failure, `pos-eft` `mock_tests::the_wire_trace_shows_bodies_but_never_keys_or_queries` (crate untouched by the lane; it reads a process-global tracing subscriber and is sensitive to load). Two reruns of `cargo test -p pos-eft --lib` passed (66 passed). Not a lane regression.
- `cargo test --workspace` also runs doctests, which fail here only because the shared target dir mixes worktrees (`can't find crate for pos_engine`, stale `pos_core`). Unit and integration targets were run with `--lib --bins --tests` after touching the sources to defeat the cross-worktree mtime freshness; the orchestrator's merged tree builds in its own target.
- Feature to test map (section 12): every named Rust, Kotlin and Playwright test was checked to exist in the tree (script grep); all pass in the runs above. No uncovered behaviour found, no test added.


## Feature to test map

Test round 1 (2026-10-18). Every row names the layer and the tests added for it; the earlier rows of section 12 stay valid for the features they list. "eng" is `crates/pos-engine/tests/badges.rs`, "PW" is `ui/e2e/badges.spec.ts`, Kotlin paths are under `android/app/src/test/kotlin/dev/nuagemagique/pilipos`.

| # | Feature | Layer | Test(s) |
|---|---|---|---|
| 1 | `badge.key` concurrent creation and rename fallback | engine unit | `badge_key::tests::concurrent_creators_all_get_the_winners_key` (8 threads behind a barrier), `without_hard_links_the_key_is_stored_by_rename`, `the_rename_fallback_never_overwrites_an_existing_key` (the link step is a fn pointer, `load_or_create_with`) |
| 2 | Engine open with an unreadable key revokes nothing | engine | eng `opening_the_engine_with_an_unreadable_key_revokes_nothing` |
| 3 | Reader `nfc` accepted by the engine | engine | eng `the_nfc_reader_links_and_scans_like_any_other` |
| 4 | `list_badges` log capped at 30, newest first | engine | eng `the_overview_log_carries_the_thirty_latest_rows_newest_first` |
| 5 | `link_badge` PIN throttle ends in `pin_locked`, nothing linked | engine | eng `repeated_wrong_manager_pins_end_in_pin_locked_and_link_nothing` |
| 6 | Raw UID in no log line (tracing) | engine | eng `the_raw_badge_number_is_in_no_log_line` (TRACE capture over link, tap, unknown, unreadable key, presence_failed) |
| 7 | Active link of a deactivated employee: `user_unknown`, journaled `refused` | engine | eng `a_badge_whose_employee_is_no_longer_active_is_refused` |
| 8 | Precedence: end_shift beats badge login, an open session beats end_shift | engine | eng `end_shift_beats_badge_login_and_an_open_session_beats_end_shift` |
| 9 | Training: enrolment and revocation allowed, no fiscal event | engine | eng `in_training_a_manager_can_link_and_revoke_without_any_fiscal_event` |
| 10 | Internal refusal before signing journaled with its code | engine | eng `an_internal_refusal_before_signing_is_journaled_with_its_code` (`previous_day_open`) |
| 11 | Reader switched to Désactivé at run time; stored `nfc` installs no listener on Linux | Vue | PW `switching the reader to Désactivé releases the keys: a burst is typed again`, `a stored NFC reader installs no listener on Linux: the burst is typed, no tap`, `a stored NFC reader offers no badge login hint and no Lier buttons on Linux` (fix: `stores/badge.ts` `readerOn` is true for `usb` only) |
| 12 | Ctrl chord, key repeat, IME composition | Vue | PW `a Ctrl chord breaks the burst and the held keys are replayed before it`, `a key repeat inside a burst is swallowed and not counted`, `a key during IME composition is left alone` |
| 13 | Timing saved in Settings applies without reload | Vue | PW `a wider gap saved without reload turns a slow burst into a badge`, `a longer minimum saved without reload refuses a 6 key burst` |
| 14 | Replay respects maxlength, read-only, no focus, and the focus at hold time | Vue | PW `a field with a maxlength keeps only what fits`, `a read-only field receives nothing`, `with nothing editable focused the keys are dropped`, `the keys land in the field that was focused when they were held` |
| 15 | Replayed typing is user activity | Vue | PW `held keys that turn out to be typing are reported to the engine` (no Enter, so only the replay can report) |
| 16 | Sound cue: high two-tone, low tone, none on ignored; no audio device | Vue | PW `high two-tone on a registration, low tone on a refusal or FDM failure, none on ignored`, `a badge login plays the high two-tone`, `no audio device: the toast still shows` |
| 17 | `pin_required` for a manager outside the tiles opens the PIN pad | Vue | PW `a manager who is not among the tiles still gets the PIN pad` |
| 18 | A registered scan on the login screen reloads the on-shift tiles | Vue | PW `a registered scan on the login screen reloads who is on shift` |
| 19 | Badge log empty state and 6-row cap | Vue | PW `an empty log says so`, `the panel shows the first six rows only, in order` |
| 20 | Cancel or backdrop during enrolment links nothing and frees the capture | Vue | PW `cancelling at the PIN step after the capture links nothing and frees the reader`, `a tap on the backdrop of the PIN step cancels too`, `a tap on the backdrop of the enrolment closes it and frees the reader` |
| 21 | Refused revocation keeps the row; backdrop closes the dialog | Vue | PW `a refused removal shows the error and keeps the row`, `a tap on the backdrop of the removal dialog closes it without removing` |
| 22 | Setting read again on `Resync`; a failed read keeps the reader | Android | `ui/badge/BadgeStoreTest.aResyncReadsTheSettingAgain`, `aFailedReadKeepsThePreviousSettingAndTheReaderRunning` |
| 23 | `presence_failed` toast carries the FDM text (or the text of a known code) | Android | `BadgeStoreTest.presenceFailedShowsTheRefusalToast` (extended), `presenceFailedWithAKnownCodeShowsItsTextNotTheCode` |
| 24 | `KeyWedgeSource.configure` replays held keys, then applies the new gap | Android | `ui/badge/KeyWedgeSourceTest.aNewTimingReleasesTheHeldKeysThenAppliesTheNewGap` |
| 25 | `MainActivity` lifecycle: wedge replay sink, NFC reader mode follows resume and pause | Android | `MainActivityTest.keysTheWedgeReleasesAreReplayedIntoTheViewHierarchyAndCountAsUserActivity`, `MainActivityNfcTest.theReaderModeRunsWhileTheActivityIsResumedAndEndsWithIt` (Hilt, `BadgeModule` swapped for the same bindings minus the device NFC) |
| 26 | `AndroidNfcSupport`: feature flag, adapter state, null adapter, reader mode | Android | `ui/badge/AndroidNfcSupportTest.*`. The reader flags are not exposed by Robolectric's `ShadowNfcAdapter`: they are read in code review and stay untested (device test list) |
| 27 | Manifest: NFC permission and `uses-feature android.hardware.nfc required=false` | Android | `ui/badge/NfcManifestTest.*` (parses `src/main/AndroidManifest.xml`) |
| 28 | Hilt graph: `BadgeOverlay` in the shell overlays, one wedge and one NFC singleton | Android | `BadgeGraphTest.*` |
| 29 | Badge panel in the narrow layout: managers only, between the plan and the employees | Android | `feature/settings/SettingsBadgeContentTest.onANarrowWindowAManagerGetsTheBadgePanelBetweenThePlanAndTheEmployees`, `onANarrowWindowAWorkerGetsNoBadgePanel` |
| 30 | Badge log: 6 rows, manager name and Brussels time, revoke row | Android | `SettingsBadgeContentTest.theLogShowsAtMostSixRowsNewestFirst`, `aLogRowNamesTheManagerAndTheBrusselsTime`, `aRevocationRowShowsItsLabelAndTheEmployee` |
| 31 | `SettingsRoute` wires the panel and dialog lambdas to the real model | Android | `feature/settings/SettingsRouteTest.theBadgeLambdasReachTheRealBadgeModel` (the route's entry point is handed the test stores with `mockkStatic(EntryPointAccessors)`) |
| 32 | Leaving Settings cancels a waiting capture | Android | `feature/settings/SettingsBadgeModelTest.leavingSettingsWhileTheDialogWaitsCancelsTheCapture` |
| 33 | Login hints render only under their conditions | Android | `feature/login/LoginContentTest.theTilesHintAtTheBadgeOnlyWithBadgeLoginAndNotInInszMode`, `noBadgeHintInInszMode`, `noBadgeHintWithoutBadgeLogin`, `thePinPadHintsAtTheBadgeInEndShiftModeWithAReader`, `noEndShiftBadgeHintOutsideEndShiftMode`, `noEndShiftBadgeHintWithTheReaderOff` |
| 34 | Leaving the login screen resets the store's end-shift mode | Android | `feature/login/LoginBadgeTest.leavingTheLoginScreenResetsTheEndShiftModeOfTheStore` |
