# Engine tests on the Android emulator

Phase P3, task P3-T5 (plan P3.3, DECISIONS 11.12, PD82). The Rust tests of the fiscal
engine and of the crates of the certified build graph run on the validation device, the
`sunmi_d3_pro_api33` emulator (API 33, arm64), not only on the host. Result in one line:
**1384 tests in 88 binaries; 1178 pass; 193 fail only because they
read fixtures from the build machine (with `--fixtures`, 1371 pass and none fails);
13 are `#[ignore]` live tests that stay ignored; no Android behaviour difference.**

## What runs and why

The certified logic (signing, counters, tickets, reports, HubRise flows) is the same Rust
code on Linux and on Android. A green host run says nothing about the Android build of
that code: another libc (bionic), another filesystem layout, another SQLite build, another
TLS provider setup, another thread scheduler. The spike S1 had run six crates by hand;
this card turns that into a repeatable script plus a gate check (`p3_gate.sh --device-tests`).

What is built and run, all with `--locked`, from the root workspace (the root `Cargo.lock`
and every manifest stay untouched, `git diff --quiet Cargo.lock` is checked after the build):

- `pos-engine` with `--no-default-features --features cert` (the certified and Android
  feature set): the unit tests and every target under `crates/pos-engine/tests`, which
  use the harness `pos-engine-testkit` (in-memory SQLite, in-process mocks of the FDM, the
  printer and HubRise, `wiremock`).
- The other crates of the cert build graph, with the features the graph gives them
  (`cargo tree -p pili-pos --no-default-features --features cert -e features`): `pos-core`,
  `pos-db --features hubrise`, `pos-fiscal`, `pos-print`, `pos-mail`, `pos-hubrise`,
  `pos-catalog`, `pos-hubrise-types`.
- Not run: doc-tests (no crate has any), `#[ignore]` tests (the `live_*` tests and the
  `e2e_staging` tests call module2 or HubRise; `--ignored` is never passed), `pos-peppol`
  (not in the cert graph), the Tauri shell.

Each test binary is cross-built for `aarch64-linux-android` (API 33, NDK 27.1), stripped
(`llvm-strip --strip-debug`), pushed with `adb` to `/data/local/tmp/pili-p3-device-tests/<crate>/`,
listed (`--list`, `--list --ignored`), run with `TMPDIR=/data/local/tmp/pili-p3-device-tests/tmp`
and a timeout of 1800 s, and deleted again. Every failed test is rerun once alone
(`--exact --test-threads=1`); a test that passes alone counts as passed and is reported as
flaky (none was). The mocks of the tests are served in-process on the device, so no
`adb reverse` and no host service is needed.

## How to run

```bash
scripts/android/device_engine_tests.sh --check            # build, run, verdict (default mode)
scripts/android/device_engine_tests.sh --run-only --check # reuse the last build
scripts/android/device_engine_tests.sh --build-only       # cross-build only
scripts/android/device_engine_tests.sh --check --host-list      # also compare with the host --list counts
scripts/android/device_engine_tests.sh --fixtures --check # run the tests with their fixtures (see below)
scripts/android/device_engine_tests.sh --report-only --check    # classify the raw results of the last run again
scripts/android/p3_gate.sh --base android/int --allow '<regex>' --device-tests   # as a gate check
```

Options: `--crates LIST` (comma or space separated), `--keep` (keep the device directory
and the staging directory). Needs the emulator (`scripts/android/emulator.sh ensure` is
called; an already running emulator is reused and never stopped), `cargo-ndk`, the
`aarch64-linux-android` Rust target, NDK 27.1 (`ANDROID_NDK_HOME`) and `python3`.

Output: one `DEVICE | <crate>/<target> | PASS|FAIL | passed N, failed N, excluded N, ignored N, flaky N`
line per binary, `DEVICE | WARN | ...` lines (android-difference entries, flaky tests, host
count differences), then `DEVICE | total | PASS|FAIL | ...`. "failed" counts every test that
still fails after the lone rerun, excluded ones included; "excluded" is the part of "failed"
listed in `scripts/android/device_tests_exclude.txt`, so `passed + failed + ignored` equals
the `--list` count of the binary. Failure details go to stderr; a machine readable copy is
`target/android-device-tests/results/report.json`. The check exits 1 when a test fails that
the exclusion file does not list, when an excluded test passes or does not exist (stale
entry), when `passed + failed + ignored` differs from the `--list` count, or when a binary
did not run to its end (crash, timeout).

Cost on the Apple M1 with 16 GB (the emulator running, `CARGO_BUILD_JOBS=2`,
`CARGO_PROFILE_DEV_DEBUG=0` set by the script):

| | cold | warm |
|---|---|---|
| cross-build, default mode | about 3.5 min (pos-engine 108 s, the 8 other crates 84 s) | 1 to 6 s |
| cross-build, `--fixtures` (own target dir, rebuilds everything) | 166 s | a few s |
| run, 88 binaries | 129 s (sum of the test times 80 s, the rest is strip, push and `--list`) | same |
| host build for `--host-list` (not kept) | 178 s | not kept |
| disk | 3.7 GB in `target/android-device-tests`, the same again for `-fx`; the binaries are 2.4 GB stripped (about 48 MB each for the largest engine tests), pushed and deleted one at a time | |
| device | at most one binary (48 MB) plus 9.8 MB of fixtures at a time, directory removed at the end | |

Stripping debug info gains little (2.45 GB to 2.39 GB) because the build already uses
`debug = 0`; it stays in the script as the documented step.

## Why `cargo ndk test` is not used verbatim

`cargo ndk ... test --no-run --message-format=json` prints nothing: cargo-ndk 4.1.2
consumes cargo's JSON output, so the executables cannot be listed from it (S1 found them
by scanning `deps/`). The script therefore sources `cargo ndk-env -t arm64-v8a --platform 33`
(the compiler, `ar`, bindgen and sysroot variables cargo-ndk itself sets) and runs
`cargo test --no-run --locked --target aarch64-linux-android --message-format=json` directly,
with the NDK clang `aarch64-linux-android33-clang` as linker (cargo-ndk as a linker wrapper
fails outside `cargo ndk`: `running the file ...rcgu.o requires -Zscript`). The executables
are the `compiler-artifact` entries whose `profile.test` is true. This is the same
compilation as `cargo ndk` does; the test run is not affected.

## The run

| | |
|---|---|
| Date | 2026-10-08 |
| Device | AVD `sunmi_d3_pro_api33` (Sunmi D3 Pro profile, 1920x1080, 160 dpi, 4 GB, API 33, arm64-v8a, google_apis), `emulator-5554`, shared with another worker during the run |
| Host | Apple M1, 16 GB, Rust 1.99.0, cargo-ndk 4.1.2, NDK 27.1.12297006 |
| Script commit | `9dfb6d5` on `android/P3-P3-T5` (from `android/int` at `03fcd92`) |
| Runs | default mode: build and run (310 s with the host build), then `--run-only --check`, both `DEVICE | total | PASS`, same numbers; `--fixtures --check`: PASS |
| Flaky | none in any run |
| Host comparison | `--host-list`: the `--list` count of all 88 binaries is identical on the host and on the device (1384 = 1384) |

### Per crate

| Crate | Features | Binaries | --list | passed | failed (excluded) | ignored | passed with --fixtures |
|---|---|---:|---:|---:|---:|---:|---:|
| pos-engine | --no-default-features --features cert | 55 | 587 | 426 | 150 (150) | 11 | 576 |
| pos-core | default | 1 | 64 | 64 | 0 (0) | 0 | 64 |
| pos-db | --features hubrise | 1 | 124 | 124 | 0 (0) | 0 | 124 |
| pos-fiscal | default | 17 | 263 | 226 | 35 (35) | 2 | 261 |
| pos-print | default | 5 | 157 | 157 | 0 (0) | 0 | 157 |
| pos-mail | default | 3 | 74 | 74 | 0 (0) | 0 | 74 |
| pos-hubrise | default | 2 | 68 | 68 | 0 (0) | 0 | 68 |
| pos-catalog | default | 2 | 30 | 30 | 0 (0) | 0 | 30 |
| pos-hubrise-types | default | 2 | 17 | 9 | 8 (8) | 0 | 17 |
| **total** | | **88** | **1384** | **1178** | **193 (193)** | **13** | **1371** |

### Per binary

`--list` is the same number on the host and on the device for every binary (checked with
`--host-list`). "passed with --fixtures" is the run with `--fixtures` (all binaries pass).

| Binary | --list (device = host) | passed | failed | excluded | ignored | passed with --fixtures | run s |
|---|---:|---:|---:|---:|---:|---:|---:|
| pos-engine/access_control | 20 | 20 | 0 | 0 | 0 | 20 | 22 |
| pos-engine/admission_mock | 4 | 4 | 0 | 0 | 0 | 4 | 0 |
| pos-engine/background | 2 | 2 | 0 | 0 | 0 | 2 | 0 |
| pos-engine/config_check | 2 | 2 | 0 | 0 | 0 | 2 | 0 |
| pos-engine/error_codes | 4 | 4 | 0 | 0 | 0 | 4 | 0 |
| pos-engine/fdm_monitor | 10 | 10 | 0 | 0 | 0 | 10 | 3 |
| pos-engine/fdm_status_cache | 3 | 3 | 0 | 0 | 0 | 3 | 1 |
| pos-engine/hubrise_cancellations | 13 | 13 | 0 | 0 | 0 | 13 | 1 |
| pos-engine/hubrise_catalog | 12 | 12 | 0 | 0 | 0 | 12 | 1 |
| pos-engine/hubrise_config | 4 | 4 | 0 | 0 | 0 | 4 | 1 |
| pos-engine/hubrise_discounts | 4 | 4 | 0 | 0 | 0 | 4 | 1 |
| pos-engine/hubrise_modifiers | 6 | 6 | 0 | 0 | 0 | 6 | 0 |
| pos-engine/hubrise_orders_ingest | 9 | 9 | 0 | 0 | 0 | 9 | 0 |
| pos-engine/hubrise_orders_signing | 16 | 16 | 0 | 0 | 0 | 16 | 1 |
| pos-engine/hubrise_print_access | 1 | 1 | 0 | 0 | 0 | 1 | 0 |
| pos-engine/hubrise_switch | 5 | 5 | 0 | 0 | 0 | 5 | 1 |
| pos-engine/hubrise_transport | 17 | 17 | 0 | 0 | 0 | 17 | 1 |
| pos-engine/idle_lock | 7 | 6 | 1 | 1 | 0 | 7 | 2 |
| pos-engine/lib | 107 | 107 | 0 | 0 | 0 | 107 | 1 |
| pos-engine/live_combined | 2 | 0 | 0 | 0 | 2 | 0 | 0 |
| pos-engine/live_fdm | 6 | 0 | 0 | 0 | 6 | 0 | 0 |
| pos-engine/live_hubrise | 3 | 0 | 0 | 0 | 3 | 0 | 0 |
| pos-engine/order_commands | 22 | 20 | 2 | 2 | 0 | 22 | 0 |
| pos-engine/presence | 8 | 2 | 6 | 6 | 0 | 8 | 0 |
| pos-engine/printer_status_timer | 2 | 2 | 0 | 0 | 0 | 2 | 2 |
| pos-engine/refund_guards | 7 | 7 | 0 | 0 | 0 | 7 | 0 |
| pos-engine/remote_support | 14 | 14 | 0 | 0 | 0 | 14 | 13 |
| pos-engine/screen_state | 3 | 0 | 3 | 3 | 0 | 3 | 0 |
| pos-engine/session_rules | 14 | 10 | 4 | 4 | 0 | 14 | 0 |
| pos-engine/settings_version | 1 | 1 | 0 | 0 | 0 | 1 | 0 |
| pos-engine/spf_e2e_auto_z | 5 | 5 | 0 | 0 | 0 | 5 | 1 |
| pos-engine/spf_e2e_bar_slip | 11 | 11 | 0 | 0 | 0 | 11 | 3 |
| pos-engine/spf_e2e_basic | 15 | 0 | 15 | 15 | 0 | 15 | 1 |
| pos-engine/spf_e2e_concurrency | 3 | 0 | 3 | 3 | 0 | 3 | 0 |
| pos-engine/spf_e2e_copy_refund_tickets | 9 | 9 | 0 | 0 | 0 | 9 | 0 |
| pos-engine/spf_e2e_crash_safety | 2 | 0 | 2 | 2 | 0 | 2 | 0 |
| pos-engine/spf_e2e_digital | 34 | 5 | 29 | 29 | 0 | 34 | 1 |
| pos-engine/spf_e2e_faq | 7 | 6 | 1 | 1 | 0 | 7 | 1 |
| pos-engine/spf_e2e_fdm_failures | 27 | 12 | 15 | 15 | 0 | 27 | 3 |
| pos-engine/spf_e2e_fdm_response_check | 13 | 0 | 13 | 13 | 0 | 13 | 0 |
| pos-engine/spf_e2e_fixtures | 21 | 2 | 19 | 19 | 0 | 21 | 0 |
| pos-engine/spf_e2e_journal | 4 | 0 | 4 | 4 | 0 | 4 | 0 |
| pos-engine/spf_e2e_no_gap | 2 | 1 | 1 | 1 | 0 | 2 | 1 |
| pos-engine/spf_e2e_payments | 8 | 8 | 0 | 0 | 0 | 8 | 0 |
| pos-engine/spf_e2e_peripherals | 18 | 6 | 12 | 12 | 0 | 18 | 0 |
| pos-engine/spf_e2e_report_fixtures | 14 | 0 | 14 | 14 | 0 | 14 | 0 |
| pos-engine/spf_e2e_report_printout | 2 | 2 | 0 | 0 | 0 | 2 | 0 |
| pos-engine/spf_e2e_reports | 4 | 0 | 4 | 4 | 0 | 4 | 0 |
| pos-engine/spf_e2e_rounding | 6 | 6 | 0 | 0 | 0 | 6 | 0 |
| pos-engine/spf_e2e_service_day | 1 | 1 | 0 | 0 | 0 | 1 | 0 |
| pos-engine/spf_e2e_signing_serialized | 14 | 13 | 1 | 1 | 0 | 14 | 2 |
| pos-engine/spf_e2e_split_payment | 9 | 9 | 0 | 0 | 0 | 9 | 2 |
| pos-engine/spf_e2e_unpaid | 10 | 9 | 1 | 1 | 0 | 10 | 8 |
| pos-engine/spf_e2e_validation | 16 | 16 | 0 | 0 | 0 | 16 | 5 |
| pos-engine/ticket_copies | 4 | 4 | 0 | 0 | 0 | 4 | 0 |
| pos-core/lib | 64 | 64 | 0 | 0 | 0 | 64 | 1 |
| pos-db/lib | 124 | 124 | 0 | 0 | 0 | 124 | 3 |
| pos-fiscal/advanced_set_fixtures | 1 | 0 | 1 | 1 | 0 | 1 | 0 |
| pos-fiscal/client_auth | 4 | 4 | 0 | 0 | 0 | 4 | 0 |
| pos-fiscal/client_status | 5 | 5 | 0 | 0 | 0 | 5 | 0 |
| pos-fiscal/e2e_staging | 2 | 0 | 0 | 0 | 2 | 0 | 0 |
| pos-fiscal/fdm_client_errors | 21 | 21 | 0 | 0 | 0 | 21 | 3 |
| pos-fiscal/fdm_https_mock | 3 | 0 | 3 | 3 | 0 | 3 | 0 |
| pos-fiscal/fdm_tls | 6 | 0 | 6 | 6 | 0 | 6 | 1 |
| pos-fiscal/full_service | 1 | 1 | 0 | 0 | 0 | 1 | 0 |
| pos-fiscal/lib | 178 | 178 | 0 | 0 | 0 | 178 | 0 |
| pos-fiscal/module2_recorded | 3 | 0 | 3 | 3 | 0 | 3 | 0 |
| pos-fiscal/price_change_uc | 2 | 1 | 1 | 1 | 0 | 2 | 0 |
| pos-fiscal/report_totals_fixtures | 6 | 0 | 6 | 6 | 0 | 6 | 0 |
| pos-fiscal/reports_fixtures | 8 | 0 | 8 | 8 | 0 | 8 | 0 |
| pos-fiscal/sign_messages | 1 | 1 | 0 | 0 | 0 | 1 | 0 |
| pos-fiscal/sign_request | 6 | 5 | 1 | 1 | 0 | 6 | 0 |
| pos-fiscal/spf_basic_set | 10 | 10 | 0 | 0 | 0 | 10 | 0 |
| pos-fiscal/uc_fixtures | 6 | 0 | 6 | 6 | 0 | 6 | 0 |
| pos-print/lib | 122 | 122 | 0 | 0 | 0 | 122 | 3 |
| pos-print/official_examples | 6 | 6 | 0 | 0 | 0 | 6 | 0 |
| pos-print/receipt_matrix | 9 | 9 | 0 | 0 | 0 | 9 | 0 |
| pos-print/report_matrix | 8 | 8 | 0 | 0 | 0 | 8 | 0 |
| pos-print/ticket_examples | 12 | 12 | 0 | 0 | 0 | 12 | 0 |
| pos-mail/lib | 46 | 46 | 0 | 0 | 0 | 46 | 1 |
| pos-mail/outbox | 13 | 13 | 0 | 0 | 0 | 13 | 1 |
| pos-mail/tem_client | 15 | 15 | 0 | 0 | 0 | 15 | 0 |
| pos-hubrise/lib | 45 | 45 | 0 | 0 | 0 | 45 | 1 |
| pos-hubrise/webshop_resolve | 23 | 23 | 0 | 0 | 0 | 23 | 1 |
| pos-catalog/apply | 18 | 18 | 0 | 0 | 0 | 18 | 0 |
| pos-catalog/lib | 12 | 12 | 0 | 0 | 0 | 12 | 0 |
| pos-hubrise-types/doc_samples | 10 | 2 | 8 | 8 | 0 | 10 | 0 |
| pos-hubrise-types/lib | 7 | 7 | 0 | 0 | 0 | 7 | 0 |

## Exclusions

`scripts/android/device_tests_exclude.txt` has 193 entries, all of the category
`host-fixture`, none of `android-difference`. Every entry names one test exactly (no `*`),
and the check fails when an entry stops matching a failing test.

| Category | Entries | Meaning |
|---|---:|---|
| host-fixture | 193 (pos-engine 150, pos-fiscal 35, pos-hubrise-types 8) | the test fails only because it reads a path of the host checkout |
| android-difference | 0 | any other reproducible failure |

| Binary | excluded tests | what the test reads |
|---|---:|---|
| pos-engine/idle_lock | 1 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/order_commands | 2 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/presence | 6 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/screen_state | 3 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/session_rules | 4 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_basic | 15 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_concurrency | 3 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_crash_safety | 2 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_digital | 29 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_faq | 1 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_fdm_failures | 15 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_fdm_response_check | 13 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_fixtures | 19 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_journal | 4 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_no_gap | 1 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_peripherals | 12 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_report_fixtures | 14 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_reports | 4 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_signing_serialized | 1 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-engine/spf_e2e_unpaid | 1 | `workspace_root()` of the test kit (`crates/pos-engine-testkit/src/fixtures.rs:56`), then `docs/20260821_UC_*` |
| pos-fiscal/advanced_set_fixtures | 1 | `docs/20260821_UC_AdvancedSet_POS_FDM_1.6` |
| pos-fiscal/fdm_https_mock | 3 | `crates/pos-fdm-mock/fixtures/tls/*.crt` |
| pos-fiscal/fdm_tls | 6 | `crates/pos-fiscal/tests/fixtures/module2_tls/*.crt` |
| pos-fiscal/module2_recorded | 3 | `crates/pos-fdm-mock/fixtures/module2/calls.jsonl` |
| pos-fiscal/price_change_uc | 1 | `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/UC223_*` |
| pos-fiscal/report_totals_fixtures | 6 | `docs/20260821_UC_Reports_1.3/M18x_*` |
| pos-fiscal/reports_fixtures | 8 | `docs/20260821_UC_Reports_1.3` (responses, `InputReport`, `Variables.json`) |
| pos-fiscal/sign_request | 1 | `docs/20260821_UC_Reports_1.3/M184_*/..._GQL-Variables.json` |
| pos-fiscal/uc_fixtures | 6 | `docs/20260821_UC_BasicSet_POS_FDM_1.8` |
| pos-hubrise-types/doc_samples | 8 | `crates/pos-hubrise-types/tests/fixtures/*.json` |

Why these fail: the tests find their fixtures through `env!("CARGO_MANIFEST_DIR")`, which
cargo expands at compile time to an absolute path of the build machine (`<repo>` below,
the host checkout). That path does not exist on the device. Evidence, verbatim from the
device (`<repo>` replaces the checkout root):

```
thread 'idle_lock_locks_an_unattended_session_once_with_its_order_registered' panicked at crates/pos-engine-testkit/src/fixtures.rs:56:10:
workspace root: Os { code: 2, kind: NotFound, message: "No such file or directory" }

thread 'basic_set_v1_8_fixtures_deserialize_and_match_schema' panicked at crates/pos-fiscal/tests/uc_fixtures.rs:65:29:
cannot read fixture root <repo>/crates/pos-fiscal/../../docs/20260821_UC_BasicSet_POS_FDM_1.8: No such file or directory (os error 2)

thread 'a_missing_file_is_named' panicked at crates/pos-fiscal/tests/fdm_tls.rs:83:30:
the CA alone is enough for HTTPS without mTLS: Read("<repo>/crates/pos-fiscal/tests/fixtures/module2_tls/ca.crt", Os { code: 2, kind: NotFound, message: "No such file or directory" })

thread 'a_product_carries_its_tax_rates' panicked at crates/pos-hubrise-types/tests/doc_samples.rs:11:55:
<repo>/crates/pos-hubrise-types/tests/fixtures/product_upload_doc.json: No such file or directory (os error 2)
```

The 35 `pos-fiscal` failures match the 35 that spike S1 reported. The 150 `pos-engine`
failures all come from `workspace_root()` of the test kit, which `canonicalize`s the
compile-time manifest path: one missing directory takes down every test that builds the
harness, so the default mode runs only the 1178 tests that do not use it. That is a
real gap: the signing, refund, report, crash safety, FDM failure and presence flows of
`spf_e2e_*` are among the excluded tests.

### The `--fixtures` mode (decision for the planner)

To close that gap without touching `crates/`, `--fixtures` rebuilds the same test sources
with `rustc` behind a wrapper that only changes the process environment: the compile-time
`CARGO_MANIFEST_DIR` of a crate under the checkout becomes
`/data/local/tmp/pili-p3-device-tests/repo/<same relative path>`, and the script pushes
`docs/20260821_UC_*` and every `crates/*/tests` and `crates/*/fixtures` directory there (9.8 MB).
No source, fixture or manifest is edited; the root `Cargo.lock` is still untouched; the build
has its own target directory (`target/android-device-tests-fx`). In this mode the
`host-fixture` lines of the exclusion file are not applied, `android-difference` lines still are.

Result: **1178 + 193 = 1371 tests pass, 0 fail, 13 ignored, 0 flaky**, so the
193 excluded tests really fail only for the fixture reason (a test that failed for any other
reason would not pass here). Because the card rules say "no fixture override", this mode is
not the default: the plain `--check` (and so `p3_gate.sh --device-tests`) keeps the
behaviour of the plan, with the 193 exclusions. The recommended step for the planner is to
make `--fixtures` the mode of the gate (one flag in `p3_gate.sh`) and then delete the
`host-fixture` lines, or keep both. Decision recorded for review; nothing else depends on it.

## Flaky tests

None. No test failed in a full run and passed on the lone rerun, in any of the runs.

## Engine questions

None. No `android-difference` was found: every failure of the default mode is a missing
host fixture, and with the fixtures present all 1371 runnable tests pass on the device,
including the FDM and TLS mocks (`fdm_https_mock`, `fdm_tls`), SQLite, the printer mock, the
mail outbox and the HubRise flows. Nothing under `crates/` needs to change, and nothing here
changes Linux behaviour.

## Limits

- The emulator is arm64 on an arm64 host (no instruction translation) but it is not a Sunmi
  D3 Pro: timing, memory pressure and the vendor libc patches of the real device are not
  covered; the real device run belongs to the hardware validation.
- The tests run as the `shell` user in `/data/local/tmp`, not inside an app sandbox; the
  engine under the app sandbox (paths, permissions) is covered by the instrumented tests of
  the Android app, not here.
- Doc-tests are not run; the crates have none.
- Time sensitive tests ran with the default number of test threads while another worker used
  the same emulator; none was flaky, but a loaded host could change that, which is what the
  lone rerun and the flaky report are for.
