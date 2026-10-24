# P1b: UI fiscal triggers into the engine

Phase P1b of the Android port of moyun-pos (pili-pos) to the Sunmi D3 Pro (plan `~/.claude/plans/i-d-like-you-to-vast-koala.md`, section "P1b: UI fiscal triggers into the engine", the invariants and the orchestration protocol). The fiscal rules that still live in the Vue UI move into `crates/pos-engine`, so the Linux Vue UI and the later Android Compose UI only call the engine and never re-implement a rule. Each engine card is paired with a Vue card that switches the UI to it.

- **Base:** `android/int` at `7b8a2da406a676d5839e46f07c56db7af4fdd75b` (`docs(android): P1a phase cards, exit report and exit decisions`). main (`96728e1`) and `cert/v1.0` (`d6210ca`) are not touched in this run. Nothing is pushed.
- **Rule of the phase:** the moved rules behave as before (same FDM requests, same tickets, same database, same existing IPC contract). The IPC contract only grows (seven commands, three event topics). The Linux version becomes 1.1.0 at the end of the phase, then the soft freeze `linux-v1.1.0-rc1` (local tag).
- **Inputs read by the planner:** the plan, `docs/android/phases/P1a-exit.md`, `docs/android/DECISIONS.md` (sections 2, 11, 12 and the "Decided while the user was away" list up to PD29), `docs/android/spikes/S4.md` section 5, `docs/android/ENGINE.md`, the code at `7b8a2da`.
- **Measured starting point (7b8a2da):** 79 Tauri wrappers (77 cert), all single calls; Rust inventory ws 1405 tests, cert 527, 1932 lines, 0 removed against the baseline; Playwright 280 tests in 22 files (`npx playwright test --list --project=e2e`); workspace version 1.0.0 (`Cargo.toml`, `src-tauri/tauri.conf.json`, `ui/package.json`); no `linux-v*` tag; Linux baseline SHA-1 `83313be28027d2e8d9e84cbddff4e584799c2fbc` (main `96728e1`, P0.1); disk 37 GB free, `int/target` 48 GB.
- **The UI rules to move (files at 7b8a2da):**
  - P event when leaving an order screen, takeaways excluded (FAQ Q205): `ui/src/router/index.ts` (beforeEach), `ui/src/stores/order.ts` (`registerPending`), `ui/src/components/AppHeader.vue` (`startTakeaway`, `lock`), `ui/src/components/CartSidebar.vue` (`sendToKitchen`);
  - clock-in (event S) at login unless already on shift, clock-out at end of shift: `ui/src/stores/auth.ts` (`clockInIfOffShift`, `registerPresence`, `endShift`);
  - 5 minute idle lock: `ui/src/composables/useIdleTimer.ts`, `ui/src/App.vue`;
  - FDM status polling (30 s, backoff to 5 min) and the status classification (blocking codes, MANDATORY messages that must not auto-dismiss): `ui/src/stores/fdm.ts`, `StatusLights.vue`, `FdmAlerts.vue`, `LoginView.vue`;
  - the software version on the login screen (AM Art. 47): not shown today.

## Cards and waves

| Card | Wave | Depends on | Lane | Scope | Cargo heavy |
|------|------|------------|------|-------|-------------|
| P1b-T1 | 1 | none | tooling | `p1b_gate.sh`, `p1a_gate.sh --allow-ui`, Playwright inventory, journal compare, capture volume per checkout, PD30 to PD46 into DECISIONS.md | gate run only |
| P1b-T2 | 1 | none | engine | `leave_order`, `set_active_order`, `lock_session`, event `session-locked` | yes |
| P1b-T3 | 1 | none | engine | DLE EOT seam (`status_known`, `printer_status_read`), `posSwVersion` in `get_settings` | yes |
| P1b-T4 | 2 | T1, T2, T3 | engine | clock-in inside login (presence worker), `end_shift`, event `presence-failed` | yes |
| P1b-T5 | 2 | T1, T2, T3 | Vue | router guard, order store, lock, send to kitchen on the engine; version on the login screen | gate only |
| P1b-T6 | 3 | T4 | engine | engine idle lock, `user_activity`, `EngineTimers.idle_lock` | yes |
| P1b-T7 | 4 | T6 | engine | FDM monitor, classification, `fdm_health`, `fdm_check_now`, event `fdm-status`, `EngineTimers.fdm_poll` | yes |
| P1b-T8 | 4 | T4, T5, T6 | Vue | auth store on the engine presence, `end_shift`, activity forwarding, `useIdleTimer.ts` deleted | gate only |
| P1b-T9 | 5 | T7, T8 | Vue | FDM store on `fdm-status` / `fdm_check_now`, UI poller and classification removed | gate only |
| P1b-T10 | 6 | T9 | release | version 1.1.0, ENGINE.md, dossier prose, FDM journal capture and compare (Docker) | yes + Docker |

Ten cards, six waves. The engine lane is sequential (T2 and T3 touch disjoint hunks; from T4 on every engine card edits `session.rs`, `background.rs`, `events.rs`, the handler lists and ENGINE.md). The Vue lane trails the engine lane by one wave. No card needs a human (needs-human: no on all cards).

**Merge order inside a wave:** wave 1: T1, T2, T3 (the gate script lands first). Wave 2: T4, T5. Wave 4: T7, T8.

**Cargo load:** wave 1 has three cards that build the workspace; workers use `CARGO_BUILD_JOBS=3` in wave 1 and 4 later, `CARGO_INCREMENTAL=0` always.

## Planner decisions (decided while the user was away)

P1b-T1 copies these bullets into `docs/android/DECISIONS.md` under "Decided while the user was away (review)".

- **PD30 Card layout.** One sequential engine lane (P1b-T2 and P1b-T3 in wave 1, then T4, T6, T7) and a Vue lane that trails it by one wave (T5, T8, T9), plus tooling (T1) and the release preparation (T10). Between waves `android/int` may apply a rule twice (engine and Vue, for example the clock-in) or not yet use an engine rule; such intermediate states are never built for a till, and the exit gate checks that the Vue rules are gone.
- **PD31 New engine commands.** Seven commands, each with an engine function and a one-call wrapper: `leave_order`, `set_active_order`, `lock_session`, `user_activity` (module `session`), `end_shift` (module `social`), `fdm_health`, `fdm_check_now` (module `fdm_monitor`). The wrapper count goes from 79 (77 cert) to 86 (84 cert); the plan's "77/77" parity target for P2 becomes 84/84. `work_in`, `work_out`, `check_fdm_status`, `sign_table_order` and `logout` stay cert commands with the same contract, even where the Vue UI stops calling them (P5 records them as exclusions in `check_ui_parity`).
- **PD32 New events.** Three topics with camelCase payloads: `session-locked` `{reason: "manual"|"idle", orderError: string|null}`, `presence-failed` `{insz, clockIn: bool, error}`, `fdm-status` (`FdmHealth`). The six P1a topics and their payloads are unchanged.
- **PD33 Leaving an order.** `do_leave_order(order_id, reason)` registers the order's pending entries (event P, POS description v2.2 section 1.3) when it is an open dine-in order, and is a no-op for a takeaway (FAQ Q205) or an order no longer open. Reasons: `navigate`, `switch_order`, `send_to_kitchen` (the order stays active), `lock`, `idle`. `do_set_active_order` tells the engine which order the screen shows, so a lock registers it. `do_lock_session` registers the active order (a failure is reported in `orderError` and never blocks the lock; the entries stay pending), closes the session (the operator stays on shift) and emits `session-locked`.
- **PD34 Presence.** A successful `login` or `login_with_new_pin` queues a clock-in (event S) that a background worker signs unless the employee is already on an open shift (when the shift lookup fails the worker clocks in anyway; a second WorkIn is harmless), so the login is never blocked by the FDM; a failure emits `presence-failed`. `do_end_shift(insz, pin)` checks the PIN exactly like `login`, signs the clock-out without a clock-in and always closes the session.
- **PD35 Idle lock.** The engine owns the 5 minute lock (`EngineTimers.idle_lock`, default 300 s), reset by `user_activity` and by every login; on expiry with a session open it runs the `lock_session` path with reason `idle`. The Vue UI forwards touch, mouse and key events at most once per second (a transport throttle, not a rule).
- **PD36 FDM monitor.** The engine polls the FDM status while a session is open (an immediate check when a session opens), every `EngineTimers.fdm_poll` (default 30 s) while the FDM answers and `base * 2^n` up to `base * 10` (5 min) while it does not, through the existing 20 s status cache, and emits `fdm-status`. The classification of the Vue store (blocking codes `BUFFER_FULL`, `FDM_LOCKED`, `FDM_NOT_OPERATIONAL`, `UNKNOWN_POS`; MANDATORY first; `FDM_NOT_INITIALIZED`; a transport failure is blocking and mandatory) moves 1:1 into the engine. `fdm_check_now` replaces the explicit checks of the login screen and of the retry button.
- **PD37 DLE EOT (settles PD12 and the P1a exit question 2).** No Linux behaviour change: `pos-print` gets a `status_known` flag on `PrinterStatus` (true only when both DLE EOT answers were read) and a status read timeout chosen by the caller; the engine passes `EngineTimers.printer_status_read`, 500 ms by default as today. Android will pass about 2 s and show "printer state unknown" when `status_known` is false (S4 section 5). Done now because after the P3 hard freeze it would be a Linux v1.2 change.
- **PD38 Version.** `get_settings` returns `posSwVersion` (the engine identity), shown on the login screen (AM Art. 47). P1b-T10 bumps the Linux version to 1.1.0 (workspace `Cargo.toml`, `src-tauri/tauri.conf.json`, `ui/package.json` and `ui/package-lock.json`). The 146 `pili-pos/1.0.0` strings in `docs/SUBMISSION_ARTIFACTS` describe the v1.0 evidence and are not edited in P1b; P8 regenerates the evidence for the v1.1 filing.
- **PD39 Not moved in P1b.** The once-per-bookingDate display of sign messages (a display convenience: POS description v2.2 asks for "at least once"), cash rounding and the VAT split (P2.7 helpers `settle_preview`, `vat_split`, `cash_due_cents`), the support banner poll and the printer and HubRise status lights stay in the UI.
- **PD40 Journal rule for P1b (extends PD19; answers the P1a exit question 1 while the user is away).** The P1b exit accepts the FDM journal when its business view (normalised journal without `Status` entries) equals baseline run1 after mapping `posSwVersion` `pili-pos/1.0.0` to the current identity, the `(eventLabel, eventCounter, totalCounter)` triples equal run1 (`S 1 1`, `N 1 2`, `N 2 3`, `P 1 4`, `N 3 5`) and every `posSwVersion` is the current identity. The `Status`-only list is reported, not required (the engine monitor may move or add `Status` polls). Checked by `scripts/android/compare_fdm_journals.sh`.
- **PD41 Playwright inventory.** The Vue suite (280 tests in 22 files at `7b8a2da`) is a gate like the Rust inventory: `docs/android/baseline/playwright_list.txt` (written once by P1b-T1, never edited after) and `docs/android/baseline/playwright_moves.tsv` (rewritten tests declared old title, new title); `scripts/android/playwright_inventory.py --diff` fails on a removed title.
- **PD42 Gate.** `scripts/android/p1b_gate.sh` runs `scripts/android/p1a_gate.sh --allow-ui` (`ui` is no longer untouchable; every other P1a check stays) plus the UI build, the Playwright suite and its inventory, the additive IPC signature check, and with `--exit` the checks that the moved rules are gone from `ui/src` and that the version is 1.1.0.
- **PD43 Journal capture volume (settles PD29).** `capture_fdm_journals.sh` uses one Docker target volume per checkout (name suffixed with a hash of the checkout path); it never removes a volume.
- **PD44 Soft freeze (settles PD28 and the P1a exit question 3 while the user is away).** At the P1b exit, once `scripts/check_repro.sh` gives the same SHA-1 for both clean builds of the exit commit, the integrator creates the local annotated tag `linux-v1.1.0-rc1` on that commit (never pushed) and the planner records it in `docs/android/LINUX_FREEZE.md`. If the builds differ, no tag is created and the run stops for the user. The user reviews the tag; the hard freeze `linux-v1.1.0` stays at the P3 exit with the user.
- **PD45 P1a exit questions 4 and 5 (while the user is away).** src-tauri size: accepted (PD25 stands; P1b adds seven one-call wrappers). Merge into main: not in this run; `android/int` is merged into main by the user after reviewing P1a and P1b.
- **PD46 Flakes and disk.** A failing cert test or Playwright test is rerun once; a repeat is a real failure (PD26). The integrator deletes each merged card's worktree target, deletes `int/target/debug/incremental` before any Docker run, and keeps at least 25 GB free.

## Engine API at the end of P1b

| Command (UI invoke name, args) | Engine function | Call | Replaces (Vue) |
|---|---|---|---|
| `set_active_order {orderId: number or null}` | `commands::session::do_set_active_order` | sync | (new: tells the engine the order on screen) |
| `leave_order {orderId, reason}` | `commands::session::do_leave_order` | async | `stores/order.ts registerPending`, router guard, `startTakeaway`, `sendToKitchen` |
| `lock_session` | `commands::session::do_lock_session` | async | `AppHeader.lock` (navigate, P, logout) |
| `user_activity` | `commands::session::do_user_activity` | sync | `useIdleTimer.ts` reset |
| `end_shift {insz, pin}` | `commands::social::do_end_shift` | async | `stores/auth.ts endShift` (login, work_out, logout) |
| `fdm_health` | `commands::fdm_monitor::do_fdm_health` | sync | `stores/fdm.ts` state |
| `fdm_check_now` | `commands::fdm_monitor::do_fdm_check_now` | async | `stores/fdm.ts checkStatus` |
| `login`, `login_with_new_pin` (unchanged contract) | `commands::auth::do_login*` | sync | now also queue the clock-in (`clockInIfOffShift`) |
| `get_settings` (adds `posSwVersion`) | `commands::settings::do_get_settings` | sync | version on the login screen |

Events added: `session-locked`, `presence-failed`, `fdm-status`. `EngineTimers` at the end of P1b: `auto_z`, `mail_outbox`, `idle_lock` (300 s), `fdm_poll` (30 s), `printer_status_read` (500 ms). New background loops: presence worker, idle lock, FDM monitor. New state: `AppState.session_rules` (active order, presence queue, activity, session-change notifier) and `AppState.fdm_monitor`.

## Shared environment (all cards)

- Worker worktree `$WT` on its card branch from `android/int`. `INT=/Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/int` is read only for workers.
- `export PATH="$HOME/.cargo/bin:$PATH"` (Homebrew cargo shadows rustup); `unset CARGO_TARGET_DIR`; `export CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4` (3 in wave 1).
- Warm start: `cp -cR "$INT/target" "$WT/target"`, `cp -cR "$INT/ui/node_modules" "$INT/ui/dist" "$WT/ui/"`.
- Host: Apple M1, 16 GB RAM, about 37 GB free, shared. One heavy cargo or npm command per agent. Never run `scripts/check_repro.sh` in a card. Docker only in P1b-T10 (journal capture, local mocks only). Playwright runs on a free port (`p1b_gate.sh` picks it) so another checkout's dev server is never reused.
- Workers leave `$WT/target` for the reviewer; the integrator deletes it after the merge.

## Rules (reviewer checks on every card)

- Allowed paths per card (regex in the card's gates).
- Never edited: `rust-toolchain.toml`, `deploy/build-release.sh`, `scripts/build_and_hash.sh`, `scripts/check_repro.sh`, `docs/android/baseline/test_inventory.txt`, `docs/android/baseline/fdm_journal_*`, `docs/android/baseline/repro/*`, and `docs/android/baseline/playwright_list.txt` once P1b-T1 has written it.
- Tests never drop: Rust `test_inventory.sh --diff` 0 removed and ws >= 1355; Playwright `playwright_inventory.py --diff` 0 removed (renamed tests declared in `playwright_moves.tsv`) and total >= 280.
- `Cargo.lock`: no `source` or `checksum` line changes; only workspace member entries (P1b-T10: version lines).
- Engine hygiene: no `cfg(target_os)`, no tauri, no new global in `crates/pos-engine/src` and `crates/pos-fiscal/src`. Every wrapper is a single call (`check_ipc_wrappers.py`). IPC contract additive only (`ipc-signatures-additive`). No new error code.
- No secrets or build outputs committed; no live module2, HubRise or SPF call; no push, no GitHub Actions, no `declare.sh`, no tag (except the integrator's exit tag, PD44).
- No em-dash anywhere; Conventional Commits, merge commits included; dossier French and brand-neutral.

### Gates

- **Wave 1 engine cards (T2, T3):** `scripts/android/p1a_gate.sh` (the P1b gate script lands in the same wave), full on the last commit, `--fast` on every commit, plus the additive IPC signature check and the allowed paths check spelled out in the card.
- **Cards from wave 2 on:** full gate `scripts/android/p1b_gate.sh --base android/int` on the last commit; fast gate on every commit:

```bash
bash -c 'b=$(git rev-parse --abbrev-ref HEAD); for c in $(git rev-list --reverse android/int..HEAD); do git checkout -q $c && PATH="$HOME/.cargo/bin:$PATH" scripts/android/p1b_gate.sh --fast --base android/int || { echo FAIL at $c; git checkout -q $b; exit 1; }; done; git checkout -q $b'
```

- **Freeze invariant:** `scripts/android/check_linux_frozen.sh` prints SKIP (exit 2) until the exit tag exists; after PD44 it must pass on `android/int` and on every later task.

## Integration procedure

1. Merge in the order listed per wave with `git merge --no-ff -m "chore(android): merge P1b-Tn <short title>"` into `android/int` (the subject must contain `merge P1b-Tn`; the phase gate keys its exit checks on `merge P1b-T10`).
2. Rerun the phase gate on `android/int` after each wave; delete the merged card's worktree target; keep at least 25 GB free.
3. REWORK at most twice per card; a third failure, a freeze violation or two escalations stop the run. With the user away, the planner decides and records in DECISIONS.md.

## Phase gate (integrator, on android/int)

```bash
cd /Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/int
test -z "$(git status --porcelain)"
test "$(git rev-parse main)" = 96728e1325f5e11d69b7571d17280e3027974e06
test "$(git rev-parse cert/v1.0)" = d6210caabb32ca09f54be0f15410ce8447960be2
PATH="$HOME/.cargo/bin:$PATH" scripts/android/p1b_gate.sh --ui --base 7b8a2da406a676d5839e46f07c56db7af4fdd75b
# after merge P1b-T10 only:
#   p1b_gate.sh --exit; journal capture + compare (PD40);
#   scripts/check_repro.sh target/repro-p1b (nothing else building);
#   on success the local tag linux-v1.1.0-rc1 (PD44), then check_linux_frozen.sh
```

The exact command list is in the orchestrator's phase gate.

## Exit procedure (planner, after the last wave)

1. Phase gate with the exit-only steps: `p1b_gate.sh --exit` PASS; FDM journal capture of the exit commit and `compare_fdm_journals.sh --sw-version pili-pos/1.1.0` PASS (PD40); `scripts/check_repro.sh` gives one SHA-1 for both clean builds.
2. Soft freeze (PD44): local annotated tag `linux-v1.1.0-rc1` on the built commit, never pushed; `check_linux_frozen.sh` passes.
3. The planner writes `docs/android/LINUX_FREEZE.md` (tag, commit, SHA-1 of both builds, date, frozen paths printed by `check_linux_frozen.sh`, status "soft freeze, for the user's review", what a re-freeze costs before and after the P3 hard freeze), `docs/android/phases/P1b-exit.md` (gate evidence, inventories, journal hashes, risks, questions for the user) and commits them with this file (docs only, outside the frozen paths).

## Card P1b-T1: P1b gate tooling and the planner decisions (wave 1)

- **Depends on:** none. **Lane:** tooling. **Needs human:** no.
- **Allowed:** `^(scripts/android/(p1a_gate|p1b_gate|compare_fdm_journals|test_compare_fdm_journals|capture_fdm_journals)\.sh|scripts/android/(test_)?playwright_inventory\.py|docs/android/baseline/playwright_(list\.txt|moves\.tsv)|docs/android/DECISIONS\.md)$`

**Work.** `p1a_gate.sh --allow-ui` (default unchanged); `playwright_inventory.py` (`--write`, `--diff`, `--current`, `--min 280`, moves file) with unittest; `playwright_list.txt` (280 tests) and an empty `playwright_moves.tsv`; `compare_fdm_journals.sh` (business view with `--sw-version` mapping, counters triples, `posSwVersion`, `Status` info) with a Docker-free self-test; `capture_fdm_journals.sh` target volume per checkout (`--volume-tag`); `p1b_gate.sh [--fast] [--exit] [--ui] [--base REF]`; PD30 to PD46 appended to DECISIONS.md.

**Acceptance.** `p1b_gate.sh --ui --base android/int` PASS with `ui-e2e` 280 passed and `ipc-signatures-additive` 79 kept, 0 added; `--exit` fails only on `ui-rules` and `version`; the self-tests pass; `compare_fdm_journals.sh` on run2 PASS; DECISIONS.md gains exactly the PD30 to PD46 bullet.

## Card P1b-T2: leave order, active order, lock session (wave 1)

- **Depends on:** none. **Lane:** engine. **Needs human:** no.
- **Allowed:** `^(crates/pos-engine/(src|tests)/|crates/pos-engine/Cargo\.toml$|crates/pos-engine-testkit/src/|src-tauri/src/|Cargo\.lock$|docs/android/ENGINE\.md$)`

**Work.** `commands/session.rs` (`LeaveReason`, `LockReason`, `SessionLocked`, `LockOutcome`, `SessionRules`, `do_set_active_order`, `do_leave_order`, `lock`, `do_lock_session`); `AppState.session_rules`; `EngineEvent::SessionLocked` (`session-locked`) and its Tauri arm; `src-tauri/src/ipc/session.rs` with three wrappers in both handler lists; test target `crates/pos-engine/tests/session_rules.rs`; ENGINE.md rows.

**Acceptance.** `commands: 82 (80 without peppol), single-call: 82/82`; additive signatures 79 kept, 3 added; P on leave only for an open dine-in order, never for a takeaway; lock registers the active order before closing the session, never blocked by an FDM failure; full P1a gate PASS.

## Card P1b-T3: DLE EOT seam and the version in get_settings (wave 1)

- **Depends on:** none. **Lane:** engine. **Needs human:** no.
- **Allowed:** `^(crates/pos-print/|crates/pos-printer-mock/|crates/pos-engine/(src|tests)/|crates/pos-engine-testkit/src/|docs/android/ENGINE\.md$)`

**Work.** `PrinterStatus.status_known`, `DEFAULT_STATUS_READ_TIMEOUT` (500 ms), `TcpPrinter::with_status_timeout`, `send_to_printer_with`; `EngineTimers.printer_status_read` passed at every engine print site; `PosSettings.pos_sw_version` (`posSwVersion`); tests; ENGINE.md section 2.

**Acceptance.** Same bytes, same `ready()`, 500 ms by default (no Linux behaviour change); a late answer is caught with a longer timeout and reported as unknown with the default; `get_settings` carries the identity; full P1a gate PASS.

## Card P1b-T4: clock-in inside login, end_shift (wave 2)

- **Depends on:** T1, T2, T3. **Lane:** engine. **Needs human:** no.
- **Allowed:** `^(crates/pos-engine/(src|tests)/|crates/pos-engine/Cargo\.toml$|crates/pos-engine-testkit/src/|src-tauri/src/|Cargo\.lock$|docs/android/ENGINE\.md$)`

**Work.** `auth::authenticate` split out of `do_login` (same checks and order); login and first-PIN login queue a clock-in; presence worker in `background.rs` (`clock_in_if_off_shift`, `presence-failed` on failure); `social::do_end_shift`; `end_shift` wrapper; background task count assertion +1; test target `presence.rs`; ENGINE.md.

**Acceptance.** `login` contract unchanged and never waits for the FDM; off shift one `signWorkIn`, on shift none; `end_shift` refuses a wrong PIN without FDM call, signs one `signWorkOut` and no `signWorkIn`, always ends without a session; `commands: 83 (81 ...)`; full P1b gate PASS.

## Card P1b-T5: Vue on leave, active order, lock; version on the login screen (wave 2)

- **Depends on:** T1, T2, T3. **Lane:** Vue. **Needs human:** no.
- **Allowed:** `^(ui/src/|ui/e2e/|docs/android/baseline/playwright_moves\.tsv$)`

**Work.** `ipc.ts` (`leaveOrder`, `setActiveOrder`, `lockSession`, types, `posSwVersion`); order store `leave(reason)` and `resetLocal`, active order on start, load and clear; router guard, AppHeader, CartSidebar on `leave`; `auth.lock` on `lock_session`; `session-locked` listener in App.vue; version on LoginView; mocks and specs.

**Acceptance.** No `registerPending` or `signTableOrder(` call left in `ui/src` outside `api/ipc.ts`; Playwright all pass, inventory 0 removed, at least 6 new tests; full P1b gate PASS.

## Card P1b-T6: engine idle lock (wave 3)

- **Depends on:** T4. **Lane:** engine. **Needs human:** no.
- **Allowed:** `^(crates/pos-engine/(src|tests)/|crates/pos-engine/Cargo\.toml$|crates/pos-engine-testkit/src/|src-tauri/src/|Cargo\.lock$|docs/android/ENGINE\.md$)`

**Work.** `EngineTimers.idle_lock` (300 s); `SessionRules` activity and `touch`, called by `access::open_session`; `do_user_activity`; idle loop in `background.rs` calling `session::lock(Idle)`; `user_activity` wrapper; tests with a short timer; ENGINE.md.

**Acceptance.** One `session-locked {idle}` after the timeout with the active order registered first; activity keeps the session; nothing without a session; `commands: 84 (82 ...)`; full P1b gate PASS.

## Card P1b-T7: FDM monitor (wave 4)

- **Depends on:** T6. **Lane:** engine. **Needs human:** no.
- **Allowed:** `^(crates/pos-engine/(src|tests)/|crates/pos-engine/Cargo\.toml$|crates/pos-engine-testkit/src/|src-tauri/src/|Cargo\.lock$|docs/android/ENGINE\.md$)`

**Work.** `commands/fdm_monitor.rs` (`FdmHealth`, `FdmSeverity`, `classify`, `poll_delay`, `FdmMonitorState`, `do_fdm_health`, `do_fdm_check_now`); `AppState.fdm_monitor`; `EngineTimers.fdm_poll`; session change notifier; monitor loop; `EngineEvent::FdmStatus`; two wrappers; tests (one per Vue classification branch, the backoff table, the loop); ENGINE.md.

**Acceptance.** Classification equal to the Vue store branch by branch; 30, 60, 120, 240, 300, 300 s; no `Status` request without a session; `commands: 86 (84 ...)`; full P1b gate PASS.

## Card P1b-T8: Vue on the engine presence and idle lock (wave 4)

- **Depends on:** T4, T5, T6. **Lane:** Vue. **Needs human:** no.
- **Allowed:** `^(ui/src/|ui/e2e/|docs/android/baseline/playwright_moves\.tsv$)`

**Work.** `ipc.ts` (`endShift`, `userActivity`, `PresenceFailed`); auth store without `registerPresence`/`clockInIfOffShift`, `presence-failed` toast, `endShift` on `end_shift`; App.vue forwards activity at most once per second; `useIdleTimer.ts` deleted; mocks and specs (renamed titles declared).

**Acceptance.** No `useIdleTimer`, no `ipc.workIn(`/`ipc.workOut(` in `ui/src/stores`; Playwright all pass, inventory 0 removed; full P1b gate PASS.

## Card P1b-T9: Vue FDM store on the engine monitor (wave 5)

- **Depends on:** T7, T8. **Lane:** Vue. **Needs human:** no.
- **Allowed:** `^(ui/src/|ui/e2e/|docs/android/baseline/playwright_moves\.tsv$)`

**Work.** `ipc.ts` (`FdmHealth`, `fdmHealth`, `fdmCheckNow`); FDM store applies engine health from `fdm-status` and `fdm_check_now` (same public refs), UI poller and classification removed; StatusLights, FdmAlerts, LoginView, App.vue; mocks; FDM specs on `FdmHealth` (renamed titles declared).

**Acceptance.** `p1b_gate.sh --exit` fails only on `version`; Playwright all pass, inventory 0 removed.

## Card P1b-T10: version 1.1.0, ENGINE.md, dossier prose, FDM journal (wave 6)

- **Depends on:** T9. **Lane:** release. **Needs human:** no.
- **Allowed:** `^(Cargo\.(toml|lock)|src-tauri/tauri\.conf\.json|ui/package(-lock)?\.json|docs/android/ENGINE\.md|docs/SUBMISSION_ARTIFACTS/[^/]+\.md|CLAUDE\.md|README\.md)$`

**Work.** Version 1.1.0 in the four files (lock: workspace member version lines only); ENGINE.md final pass with a new section 7 "UI rules owned by the engine"; French prose of the dossier where a moved rule is said to live in the UI (no fiscal statement, figure, evidence or `pili-pos/1.0.0` changed); FDM journal capture in Docker and `compare_fdm_journals.sh --sw-version pili-pos/1.1.0`.

**Acceptance.** `p1b_gate.sh --exit` PASS; journal `JOURNAL | total | PASS`; `check_dossier.py`, `uc_matrix.py`, `rewrite_dossier_paths.py --check` PASS; ENGINE.md section 6 equal to `--map` (86/84).

## Risks

1. **Timing tests.** The idle lock and the FDM monitor are timer loops; their tests must use short timers with wide margins (or tokio paused time) to stay stable; a flaky test is rerun once and then fixed, never ignored.
2. **Intermediate double triggers** (PD30): between waves the Vue UI and the engine may both clock in; harmless (the engine skips an employee already on shift; a second WorkIn is harmless) and never built for a till.
3. **Docker under emulation** (journal capture, `check_repro.sh`): each run can take an hour; nothing else heavy runs in the same tree (DECISIONS section 11, item 3); stale builds are prevented by the per-checkout volume (PD43).
4. **Disk.** 37 GB free at the start, `int/target` 48 GB; three cargo cards in wave 1; the integrator cleans worktree targets after every merge.
5. **Reproducibility at the exit.** If `check_repro.sh` gives two SHA-1 values, no freeze tag is created and the run stops for the user (PD44).
6. **Dossier drift.** The dossier text still shows `pili-pos/1.0.0` evidence until P8 (PD38); the exit report lists it.
