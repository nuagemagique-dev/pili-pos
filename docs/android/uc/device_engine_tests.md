# Engine tests on the device, rerun on the P7 head (P7D18)

Run on 2026-10-10 by card P7-T10, on commit `778e9968` of `android/P7-P7-T10` (a child of the
`android/lane-p7` head `ccd059a5`; `crates/`, `android/rust` and `Cargo.lock` are identical to it),
on the emulator `sunmi_d3_pro_api33` (`emulator-5554`, arm64-v8a, API 33, started by the chain).

Command:

```bash
bash scripts/android/device_engine_tests.sh --fixtures --check
```

Result line:

```
DEVICE | total | PASS | 89 binaries, passed 1397, failed 0, excluded 0, ignored 13, flaky 0
```

Cross-build 183 s (sccache warm), run 119 s. No `WARN` line: no Android behaviour difference, no flaky
test, no test rerun.

## Counts per crate

| Crate | Test binaries | Passed | Failed | Excluded | Ignored | Flaky |
|-------|--------------:|-------:|-------:|---------:|--------:|------:|
| pos-engine | 56 | 591 | 0 | 0 | 11 | 0 |
| pos-core | 1 | 73 | 0 | 0 | 0 | 0 |
| pos-db | 1 | 124 | 0 | 0 | 0 | 0 |
| pos-fiscal | 17 | 261 | 0 | 0 | 2 | 0 |
| pos-print | 5 | 157 | 0 | 0 | 0 | 0 |
| pos-mail | 3 | 74 | 0 | 0 | 0 | 0 |
| pos-hubrise | 2 | 68 | 0 | 0 | 0 | 0 |
| pos-catalog | 2 | 32 | 0 | 0 | 0 | 0 |
| pos-hubrise-types | 2 | 17 | 0 | 0 | 0 | 0 |
| **Total** | **89** | **1397** | **0** | **0** | **13** | **0** |

The 13 ignored tests are the live tests (`live_fdm`, `live_hubrise`, `live_combined`, `e2e_staging`):
they call module2 or HubRise and stay ignored (nothing live is run by the chain).

## Against the P3-T5 run

P3-T5 (`docs/android/DEVICE_TESTS.md`) ran 88 binaries and 1371 passing tests with `--fixtures`, 13
ignored. The export lane added one binary and 26 tests since: 1397 passed, 13 ignored, 0 failed.
The matrix claim "Couvert (tests moteur exécutés sur l'appareil)" of `docs/android/UC_MATRIX_ANDROID.md`
rests on this run.

The build output (`target/android-device-tests-fx`, 4.0 GB) was deleted after the run.
