# Engine tests on the device, rerun on the integration head (rc3)

Run on 2026-10-18 by the rc3 evidence worker, on commit `1d255e62` of `android/int` (all five lanes merged; the
commit holds the badge key fix described below and the refund-only sales report figure of INT-D38), on the emulator `sunmi_d3_pro_api33` (`emulator-5554`,
arm64-v8a, API 33, started by the chain and stopped afterwards).

Command:

```bash
bash scripts/android/device_engine_tests.sh --fixtures --check
```

Result line:

```
DEVICE | total | PASS | 114 binaries, passed 2163, failed 0, excluded 0, ignored 13, flaky 0
```

Rerun after the bench retake (the first run of the day passed 2160 tests on `7f96ae02`; the three added tests are the sales report refund test `a_negative_line_inside_a_sale_is_not_a_refund` and two engine tests merged since (a voucher line takes neither a course nor a kitchen note; a name kept in a language the till does not offer survives an edit)). Cross-build 134 s for the engine, 193 s in all, run 360 s. No `WARN` line: no Android behaviour
difference, no flaky test, no test rerun on the final run.

## A real bug found by this run

A first run on `484cb0ed` ended `114 binaries, passed 2158, failed 1`: `badge_key::tests::concurrent_creators_all_get_the_winners_key`
failed, also on its single rerun. The data directory of the emulator refuses hard links, so the creation of the
badge key took the rename fallback of `load_or_create_with`, which replaces an existing file: eight concurrent
creators each returned their own key and the last rename won. Fixed in `crates/pos-engine/src/badge_key.rs`: a
process-wide lock now serialises the creation, and the new test
`concurrent_creators_without_hard_links_all_get_the_winners_key` runs the same race through the rename fallback on
every host. The second run (this one) is clean.

## Counts per crate

| Crate | Test binaries | Passed | Failed | Excluded | Ignored | Flaky |
|-------|--------------:|-------:|-------:|---------:|--------:|------:|
| pos-engine | 81 | 1160 | 0 | 0 | 11 | 0 |
| pos-core | 1 | 99 | 0 | 0 | 0 | 0 |
| pos-db | 1 | 212 | 0 | 0 | 0 | 0 |
| pos-fiscal | 17 | 288 | 0 | 0 | 2 | 0 |
| pos-print | 5 | 206 | 0 | 0 | 0 | 0 |
| pos-mail | 3 | 74 | 0 | 0 | 0 | 0 |
| pos-hubrise | 2 | 74 | 0 | 0 | 0 | 0 |
| pos-catalog | 2 | 33 | 0 | 0 | 0 | 0 |
| pos-hubrise-types | 2 | 17 | 0 | 0 | 0 | 0 |
| **Total** | **114** | **2163** | **0** | **0** | **13** | **0** |

The 13 ignored tests are the live tests (`live_fdm`, `live_hubrise`, `live_combined`, `e2e_staging`):
they call module2 or HubRise and stay ignored (nothing live is run by the chain).

## Against the P7 run

The P7 run (2026-10-10, commit `778e9968`) ran 89 binaries and 1397 passing tests, 13 ignored. The lanes added 25
binaries and 766 tests since: 2163 passed, 13 ignored, 0 failed.
The matrix claim "Couvert (tests moteur exécutés sur l'appareil)" of `docs/android/UC_MATRIX_ANDROID.md`
rests on this run: `scripts/android/uc_matrix_android.py` reads the date and the commit above and gives
that status only to an id with at least one test that existed at this commit. An id whose tests were
all added later is "Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire)", and the later
tests of a covered id are listed apart; a new device run updates this file and the matrix follows.
