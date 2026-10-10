# UC results: Android run against the reference

Evidence of card P7-T10 (phase P7, end-to-end parity). English, generated from `docs/android/uc/results.json`
and the journals under `docs/android/uc/journals/`.

## The run

| Item | Value |
|------|-------|
| Run date | 2026-10-10, 07:58 to 08:07 (Europe/Brussels), 9 minutes for the Android run, the reference run and the comparison |
| Commit | `778e9968` (branch `android/P7-P7-T10`, child of `android/lane-p7` `ccd059a5`); the run used the tools of that commit, the engine, FFI and app of `ccd059a5` |
| Device | AVD `sunmi_d3_pro_api33` (Sunmi D3 Pro profile: 1920x1080 landscape, 160 dpi, 4 GB, API 33 arm64, google_apis, no Play), serial `emulator-5554`, started by the chain |
| Host | Apple M1, 16 GB, macOS 27.0.1 |
| Android side | app `dev.nuagemagique.pilipos.debug`, `posSwVersion` `pili-pos-android/1.0.0-dev`, host mocks `pos-fdm-mock` and `pos-printer-mock` reached through `adb reverse` on 19380, 19390, 19391; one fresh install, device owner and bundle import per group |
| Reference side | "reference (engine on the host)" (P7D4): `pos-uc-check record` plays the same scenarios with the engine compiled for the host, calling the same `pos_engine::commands::*::do_*` functions as the Tauri wrappers, `posSwVersion` `pili-pos/1.1.0`, the same seed as the Android bundle, a fresh data directory per group. It is not the Linux certified binary in a container; the engine has no `cfg(target_os)`, and the container journals of P0 tie the engine to the certified binary |
| Scenarios | 48 in 8 groups, 48 PASS on both sides, 47 distinct UC ids |
| Comparison | `UCCHECK | total | PASS | 0 failed (48 scenarios)`; `UCE2E | total | PASS | 0 failed (62 checks)` |
| Durations | Android run (build warm, 8 groups, 48 scenarios) about 8 min; reference run 53 s on a warm build (1 min 15 s to build `pos-uc-check` cold); comparison under 5 s |
| Fixture check | PASS on 28 scenarios that name an official fixture, SKIP on the others (no fixture named) |

No volatile path and no ticket normalisation rule was added: `docs/android/UC.md` sections 7 and 8 are unchanged. The
first full run found four departures of the tools from `UC.md`, fixed in the tools (commit `778e9968`), none on a fiscal
field: only the first scenario of each group ran (adb consumed the scenario list), a drawer kick job was left out of
the Android tickets, the host journal set `ok` to "the call returned a value" instead of "the step met its expect", and
the status-shape check compared whole answers instead of key sets. After the fixes every business field, counter,
FDM request and ticket line was equal on the first complete comparison; nothing was escalated.

## How to rerun

```bash
# full run on the emulator, then the reference and the comparison (needs the P7 lock and the emulator)
bash scripts/android/uc_e2e.sh --compare --out target/p7-t10
# compare again without the device: re-record the reference and compare it with the committed Android journals
bash scripts/android/p7_gate.sh --exit --base android/lane-p7 --uc-e2e     # includes uc-replay and the matrix
bash scripts/android/p7_gate.sh --fast --base android/lane-p7             # uc-replay alone is part of this
python3 scripts/android/uc_matrix_android.py --check
```

`uc-replay` (check of `p7_gate.sh`) records the reference of every group into `target/p7-gate/linux` and runs
`pos-uc-check compare --linux target/p7-gate/linux --android docs/android/uc/journals/android --fixtures` against the
committed Android journals.

## Checks

`steps` both journals PASS with the same commands and the same `ok`; `fdm-ops` same operations in order;
`fdm-business` equal `business_view`; `fdm-counters` equal (operation, eventLabel, eventCounter, totalCounter);
`fdm-full` equal full requests once the volatile paths of `UC.md` section 7 are removed; `sw-version` each side
declares its own value; `tickets` equal texts after the normalisation of `UC.md` section 8 (a drawer kick is an
empty entry); `fixture` the request equals the official fixture (with the listed deviations); `status-shape` only
for Q100 and Q110. Verdicts are PASS, FAIL or SKIP.

## One row per scenario

Columns: steps, FDM calls (non-Status), tickets (printer jobs), then the nine checks and the verdict.

| Scenario | UC ids | Steps | FDM | Tickets | steps | fdm-ops | fdm-business | fdm-counters | fdm-full | sw-version | tickets | fixture | status-shape | Verdict |
|----------|--------|------:|----:|--------:|---|---|---|---|---|---|---|---|---|---|
| 00_smoke/001_SMOKE | (smoke) | 5 | 3 | 3 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 10_basic/010_M110 | M110 | 5 | 2 | 3 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 10_basic/020_M111 | M111 | 6 | 3 | 5 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 10_basic/030_M112B | M112B | 6 | 2 | 5 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 10_basic/040_M121 | M121 | 5 | 2 | 1 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 10_basic/050_M123 | M123 | 5 | 2 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 10_basic/060_M130 | M130 | 2 | 1 | 0 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 10_basic/070_M131 | M131 | 2 | 1 | 1 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 10_basic/080_M133 | M133 | 2 | 1 | 0 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 10_basic/090_M140 | M140 | 2 | 2 | 0 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 10_basic/100_M141 | M141 | 2 | 1 | 0 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 10_basic/110_M160 | M160 | 6 | 2 | 4 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 10_basic/120_M161 | M161 | 7 | 2 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 10_basic/130_Q100 | Q100 | 2 | 0 | 0 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS | PASS |
| 10_basic/140_Q110 | Q110 | 6 | 0 | 0 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS | PASS |
| 20_training/010_M170 | M170 | 5 | 2 | 0 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 20_training/020_M171 | M171 | 8 | 1 | 3 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 20_training/030_M171B | M171B | 8 | 1 | 3 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 20_training/040_M172 | M172 | 9 | 2 | 4 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 30_reports/010_T1 | T1 | 5 | 2 | 1 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 30_reports/020_T2 | T2 | 3 | 1 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 30_reports/030_T3 | T3 | 4 | 2 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 30_reports/040_T4 | T4 | 4 | 2 | 1 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 30_reports/050_T5 | T5 | 4 | 1 | 0 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 30_reports/060_T6 | T6 | 5 | 1 | 1 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 30_reports/070_T7 | T7 | 3 | 1 | 1 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 30_reports/080_T8 | T8 | 3 | 1 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 30_reports/090_M180 | M180 | 2 | 1 | 1 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 30_reports/100_M181 | M181 | 2 | 1 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 30_reports/110_M182 | M182 | 6 | 2 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 30_reports/120_M183 | M183 | 2 | 1 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 30_reports/130_M184 | M184 | 6 | 2 | 3 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 40_advanced/010_UC201 | UC201 | 4 | 2 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 40_advanced/020_UC223 | UC223 | 5 | 1 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 40_advanced/030_UC230 | UC230 | 5 | 1 | 1 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 40_advanced/040_UC4109 | UC4109 | 8 | 4 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 40_advanced/050_UC4119 | UC4119 | 9 | 2 | 3 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | PASS |
| 50_rounding/010_ROUND1 | ROUND1 | 4 | 2 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 50_rounding/020_ROUND2 | ROUND2 | 4 | 1 | 3 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 50_rounding/030_ROUND3 | ROUND3 | 5 | 1 | 3 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 50_rounding/040_ROUND4 | ROUND4 | 5 | 1 | 3 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 50_rounding/050_ROUND5 | ROUND5 | 5 | 1 | 3 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 60_fdm_errors/010_V_FDM_LOCKED | V_FDM_LOCKED | 7 | 3 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 60_fdm_errors/020_V_BUFFER_FULL | V_BUFFER_FULL | 7 | 2 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 60_fdm_errors/030_V_BUFFER_NEAR_FULL | V_BUFFER_NEAR_FULL | 9 | 2 | 4 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 60_fdm_errors/040_V_UNKNOWN_POS | V_UNKNOWN_POS | 7 | 2 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 60_fdm_errors/050_V_UNAUTHORIZED | V_UNAUTHORIZED | 7 | 2 | 2 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |
| 70_faq/010_FAQ_Q205 | FAQ_Q205 | 9 | 2 | 1 | PASS | PASS | PASS | PASS | PASS | PASS | PASS | SKIP | SKIP | PASS |

## Journals

`docs/android/uc/journals/android/<group>/<scenario>.json` and `docs/android/uc/journals/linux/<group>/<scenario>.json`
(side `linux` is the reference): the raw journals of the final run. They hold only mock data (the seed users, the
FDM mock's signatures and the printer mock's texts); no token, key or credential. `docs/android/uc/results.json` is
the report of the comparison. `docs/android/uc/device_engine_tests.md` records the device run of the engine tests
on the same head.
