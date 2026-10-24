# Merge of the parallel lanes into android/int (2026-10-09)

> Correction (2026-10-10): the Linux build is not filed as v1.1 under Art. 113. Nothing was filed, so Linux and Android are both version 1.0.0 initial filings (Art. 99), and the freeze tag is `linux-v1.0.0-rc1`, not `linux-v1.1.0-rc1` (`docs/android/DECISIONS.md`, "Decided by the user"). The report below is unchanged.

Integrator report. The three lanes that forked from `android/int` at `ba57adf` (P4 exit) are merged back with `git merge --no-ff`, in the order lane-export, lane-ui, lane-plat. Nothing was pushed, `main` was not touched, no tag was set.

## What was merged

| Order | Branch | Head | Content | Merge commit |
| --- | --- | --- | --- | --- |
| 1 | `android/lane-export` | `8a678ff` | X1 nightly export: `crates/pos-export`, `pos-export-mock`, engine worker, cert commands `export_status` and `export_run_now`, FFI, Linux settings panel, Android `nightly` package (X1-exit.md) | `da9cb80` |
| 2 | `android/lane-ui` | `f18438c` | P5 and P5b UI lane: shared components, theme, every view (login, tables, order, checkout, reports, settings, webshop), parity pages (P5-exit.md, P5b-exit.md) | `0baf18b` |
| 3 | `android/lane-plat` | `b7834d8` | P6 platform lane: SPF export, About, declared-only updater, device-owner restrictions, reproducible `:app` APK, release tooling (P6-exit.md) | `9a6d29e` |

X1-T10 (dossier, branch `android/X1-X1-T10`, `86088e4`) is not part of `android/lane-export` and was not merged here (X1-exit.md, it waits for its re-review).

## Conflicts

All conflicts were append-on-both-sides; every resolution keeps both sides.

- Merge 1 (lane-export): none (fast path, the fork point was the `android/int` head).
- Merge 2 (lane-ui): `docs/android/DECISIONS.md` (append-only): the X1 block, then the UI lane blocks.
- Merge 3 (lane-plat):
  - `docs/android/DECISIONS.md` (append-only): X1, UI, then the P6 blocks.
  - `docs/android/LANES.md`: the `nightly` owner rules of X1-T9 and the new `platform` rules of P6 both kept, in the rules block and in the sample table. `lane_paths.py --check` PASS (3887 files, 80 rules, 74 probes).
  - `docs/android/LINUX_FREEZE.md`: the "X1 exit check" and "P6 exit check" sections both kept.
  - `android/app/src/test/kotlin/dev/nuagemagique/pilipos/nav/FeatureEntriesTest.kt`: the route set now holds `NightlyRoutes.NIGHTLY` (X1) and `PlatformRoutes.EXPORT`, `PlatformRoutes.UPDATE` (P6); the entry count stays 9 (the platform entry carries several routes, so only the nightly entry adds one).

No fix commit was needed: the merged tree passed every non-freeze check on the first run with a working environment.

## Gate results (android/int at `9a6d29e`)

Environment: `$HOME/.cargo/bin` first on PATH, no `RUSTC_WRAPPER`, `ANDROID_HOME`, `ANDROID_NDK_HOME` (27.1.12297006) and `JAVA_HOME` (17) as in BUILD.md, Gradle `-Xmx3g`. The first X1 run failed every Gradle check with "SDK location not found" (the shell had no `ANDROID_HOME`) and `root:clippy` on a stale `smallvec` artifact in the moved `target/`; both were environment, the rerun below passed them.

| Gate | Result |
| --- | --- |
| `x1_gate.sh --exit --base ba57adf` | `1 failed (98 checks, 6 waived)`. The only FAIL is `x1-scope`: `android/docker/build_in_container.sh` and `android/docker/prefetch.gradle`, changed by the platform lane (P6-T10, `b41ed01`, `6c0142e`; owner `platform` in LANES.md), not by the export lane. The six waivers are the X1D2 freeze checks. Root: fmt, clippy, 1627 workspace tests, 8 + 591 cert tests, test inventory (ws 1640 >= 1355), dossier, UI build, 305 Playwright e2e. Android: 242 Rust tests, Kotlin inventory removed 0 (1957 at head), all `:app` checks, `plan-gradle` PASS. |
| `PILI_X1_FORK=b7834d8 x1_gate.sh --fast --base ba57adf` | `PASS, 0 failed (40 checks, 5 waived)`: `x1-scope` 61 guarded files changed since the platform head, all in the export scope; `x1-lock-additive` (Cargo.lock 577 kept, 78 added; android/rust/Cargo.lock 369 kept, 80 added); `x1-parity` missing none. |
| `p5_gate.sh --exit --parity-strict --base ba57adf` (P5b exit) | `5 failed (35 checks)`: `hygiene-paths`, `hygiene-root-lock`, `linux-frozen`, `freeze-tag` (the export lane's reopened freeze, the X1D2 set) and `app-only-scope` (12 files under `android/rust`, `android/pos-core`, `android/docker`: the export lane's FFI and the platform lane's Docker build, the UI card gate's "use the full gate" case, which the full gates above cover). UI checks PASS: `ui-parity` strict `called 75, excluded 9, pending 0, stale 0, missing 0 (84 commands)`, `ui-contracts`, `ui-money`, `ui-strings`, both self-tests; `:app` lint, ktlint, detekt, 1913 unit tests, Roborazzi verify, Kover LINE 97.4 percent BRANCH 74.7 percent, release APK and its checks, `plan-gradle` PASS. |
| `p6_gate.sh --exit --android --base ba57adf` (P6 exit) | `4 failed (63 checks)`: `hygiene-paths` (also PD223), `hygiene-root-lock`, `linux-frozen`, `freeze-tag`, the X1D2 freeze set. Every P6 check PASS (`plat-manifest`, `plat-backup-rules`, `plat-strings`, `plat-boundary`, `plat-keys`, `plat-engine-ref`, `selftest-p6-gate` 104, `selftest-declare` 42). |
| `scripts/android/check_linux_frozen.sh` | differs from `linux-v1.1.0-rc1` (65b567f) on 46 files, all export lane files: `git diff android/lane-export HEAD` over the frozen paths (Cargo.toml, Cargo.lock, rust-toolchain.toml, crates, src-tauri, ui, deploy, the build scripts, android/rust, android/pos-core) is empty. Expected (freeze reopened for X1 only). |
| `./gradlew lint ktlintCheck detekt test koverVerifyDebug assembleRelease` | BUILD SUCCESSFUL (291 tasks), then `./gradlew --stop` (no other Gradle client was running). |
| `check_command_parity.py --strict` | `parity: 86/86`, `missing: none` |
| `check_ui_parity.py --strict` | PASS, `called 75, excluded 9, pending 0, stale 0, missing 0 (84 commands)` |
| `check_ipc_wrappers.py` | `commands: 88 (86 without peppol), single-call: 88/88` |

Verdict: green. Every failing check is either the reopened Linux freeze (X1D2, X1D3: export files only) or a lane-local scope check that sees another lane's files after the merge, and each was shown clean on its own terms (x1-scope from the platform head, frozen paths equal to lane-export).

Not run here: the emulator runs (`--app-e2e`, `--plat-e2e`) and the Docker reproductions (`--linux-repro`, `check_repro.sh`); the lanes ran them at their heads and the merge changes no build input beyond theirs.

## After the merge

- The lane worktrees `moyun-pos-wt/lane-export`, `lane-ui` and `lane-plat` were removed; the branches `android/lane-export`, `android/lane-ui` and `android/lane-plat` are kept. The export lane's Rust `target/` (51 GB) was moved into `moyun-pos-wt/int` and reused.
- `scripts/android/p6_gate.sh`, `test_p6_gate.sh` and `plat_e2e.sh` are committed without the executable bit; run them with `bash` (left for a platform card).
- The freeze stays as it is: `linux-v1.1.0-rc1` is the only freeze tag. The re-freeze as `linux-v1.0.0-rc1` (export lane candidate SHA-1 `d4fb09a2ae2cf2501fe4d6847eb72e954d0b95ed`) is the user's, and after it a foundation card points `p4_gate.sh` and `plat_about_engine_ref` at the new tag.

## The SD card question

Unchanged by the merge (X1D14, X1D28, PD220, PD248): `pos.db`, its backups, the exports, the bundle and the logs stay in the app's private internal storage on the D3 Pro (`files/engine`), never on the SD card. The manifest has no `installLocation` and no storage permission (gate `plat-manifest`), so the card cannot become adoptable storage for the app. The card is only a target of the manager's SAF export (and of an update from a stick). Size: one export set is about 7 database copies plus logs; at the X1 working figure (about 30 MB of `pos.db` a month at 150 tickets a day) a 16 or 32 GB A1 card covers years of exports. This is an estimate until research card SD-1 measures it.
