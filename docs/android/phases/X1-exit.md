# X1 exit report: Nightly export (export lane)

> Correction (2026-10-10): the Linux build is not filed as v1.1 under Art. 113. Nothing was filed, so Linux and Android are both version 1.0.0 initial filings (Art. 99), and the freeze tag is `linux-v1.0.0-rc1`, not `linux-v1.1.0-rc1` (`docs/android/DECISIONS.md`, "Decided by the user"). The report below is unchanged.

Written by the X1 planner on 2026-10-09, at the end of the X1 run, with the user away.

- **Base (fork point):** `ba57adf06191a1f386863f98f24930b6f5aa543c` (`docs(android): P4 phase cards, exit report and exit decisions`).
- **Exit head:** `android/lane-export` at `a364ad7e3e345ff005b678e6c73f19268743c7fd` (`docs(android): record the X1 wave 6 integration`) before the exit commit, then the docs-only commit that adds `phases/X1.md`, this report, the exit decisions (X1D23 to X1D29) and the X1 exit record in `LINUX_FREEZE.md`.
- **Cards:** `docs/android/phases/X1.md`. Decisions: `docs/android/DECISIONS.md`, "Decided while the user was away (review)" (X1D1 to X1D16 from the planner, X1D17 to X1D22 from the integrator, X1D23 to X1D29 at this exit).
- `main` (`96728e1`), `cert/v1.0` (`d6210ca`) and `android/int` (`ba57adf`) were not touched. Nothing was pushed, no GitHub Actions run, no live module2, HubRise or receiver call, no `declare.sh`, no release key, no tag.

## Verdict

- **Nine of ten cards are APPROVE and merged** into `android/lane-export` with `--no-ff` (T1 to T9), no conflict. One integration fix commit (`dc29f43`, rust-embed `deterministic-timestamps`, X1D20) made the Linux build reproducible again after pos-export entered the cert graph.
- **X1-T10 (dossier) is NOT merged.** Its review was REWORK on two text points. The planner applied exactly those two fixes on the card branch (`86088e4`, `docs(dossier): qualify the local-only answer and the export port`, PDFs rebuilt, `check_dossier.py` and its 19 self-tests green, pdf-freshness clean), but the merge of `android/X1-X1-T10` into `android/lane-export` was refused by the session's permission check, so the branch waits for a re-review and a merge (X1D23). The card worktree `moyun-pos-wt/X1-X1-T10` is kept.
- **`android/lane-export` builds and the phase gate passes** at `a364ad7`: the planner's `x1_gate.sh --exit --base ba57adf` ends **GATE | total | PASS | 0 failed (98 checks, 6 waived)**, the six waived checks being exactly the X1D2 freeze checks. The first two planner runs failed one check, `root:tests-cert-pos-engine` (441 passed, 150 failed in 20 targets), because sccache served a `pos-engine-testkit` rlib compiled in the removed X1-X1-T9 worktree, whose embedded `CARGO_MANIFEST_DIR` no longer exists (`fixtures.rs:56`, `workspace_root().canonicalize()`); a tooling issue, not a code defect (X1D29). After `cargo clean -p pos-engine-testkit` and a run without `RUSTC_WRAPPER` the check gives 591 passed, 0 failed.
- **Linux reproducibility:** the integrator's `x1_gate.sh --fast --linux-repro` at `a364ad7` (integrate-6) built the Linux cert binary twice in Docker from a clean clone: SHA-1 `d4fb09a2ae2cf2501fe4d6847eb72e954d0b95ed`, two identical builds. Docker Desktop was down during this exit, so the planner did not rebuild it; the exit commit and `86088e4` change no freeze path (`git diff a364ad7 HEAD -- <freeze paths>` is empty), so that SHA-1 stands for the re-freeze.
- **No freeze step at this exit** (X1D3, X1D27): the re-freeze `linux-v1.0.0-rc1` and the rename to 1.0.0 are the user's. `linux-v1.1.0-rc1` stays the only freeze tag; `check_linux_frozen.sh` against it fails on this lane by design.

**Planner recommendation:** after T10 is re-reviewed and merged, merge `android/lane-export` into `android/int` last, after the UI and platform lanes (X1D3), and gate `android/int` with `x1_gate.sh`; then the user re-freezes. `main`, pushes and the dossier's legal framing wait for the user.

## Task status

| Card | Wave | Card commits (base..tip) | Review | Merge on `android/lane-export` |
|------|------|--------------------------|--------|--------------------------------|
| X1-T1 lane gate `x1_gate.sh` + self-test (73 cases), X1D1 to X1D16 | 1 | `ba57adf..c470656` (2) | APPROVE | `70f9a5e` |
| X1-T2 `ExportConfig`, pos-export base (naming, settings, snapshot, crypt, manifest), mock skeleton, age `=0.12.1` | 1 | `ba57adf..14be22b` (3) | APPROVE | `61cf510` |
| X1-T3 statistics copy v1, pseudonyms, Z lookup, `STATS_SCHEMA.md`, privacy proof | 2 | `2d6ae0d..525ffb5` (4) | APPROVE | `fa88efa` |
| X1-T4 upload client, mock receiver (library, binary, test controls) | 2 | `fa88efa..89539df` (2) | APPROVE | `a229e36` |
| X1-T5 outbox (prepare, backoff, retry now, retention, tmp) and `ExportStatus` | 3 | `6256167..b97b233` (2) | APPROVE (after one rework round) | `a3e951a` |
| X1-T6 engine glue: `ExportState`, loop, wake after Z, two commands, check-config, `export_nightly` tests | 4 | `df34e33..e8a32a2` (2) | APPROVE | `f6e44eb`, then fix `dc29f43` |
| X1-T7 Tauri wrappers, FFI methods and record, FFI tests, `pos-core.api`, FFI.md | 5 | `4a0688c..2ce0fc0` (2) | APPROVE | `2397854` |
| X1-T8 Vue `ExportStatusPanel`, `i18n-export.ts`, 9 Playwright tests | 5 | `2397854..a15d7ee` (1) | APPROVE | `1ee78bf` |
| X1-T9 Android `nightly` screen, strings fr/en/zh, tests and goldens, LANES owner | 6 | `2819253..81979f2` (4) | APPROVE | `0711932` |
| X1-T10 dossier `EXPORT_NOCTURNE_FR.md` and updates, MANIFEST, PDFs | 6 | `2819253..86088e4` (8, the last by the planner) | REWORK, fixed, re-review pending | not merged |

Integration records: `2d6ae0d` (X1D17), `6256167` (X1D18), `df34e33` (X1D19), `4a0688c` (X1D20), `2819253` (X1D21), `a364ad7` (X1D22). Across the merged cards: 29 non-merge commits, 89 files, +15421/-83 lines against `ba57adf`. The card branches are kept; every task worktree except X1-T10 is removed.

### What each card delivered (from the reviews)

- **T1.** `x1_gate.sh` (runs p1b and p4 gates, waives exactly the six freeze checks, adds `x1-paths`, `x1-scope`, `x1-lock-additive`, `x1-parity`, `--linux-repro`), `test_x1_gate.sh` 73 cases; shellcheck clean at warning level.
- **T2.** `ExportConfig` with `export_url_problem`, the crate `pos-export` (token never in `Debug` or errors, streamed age encryption to `.partial` then rename, atomic manifest, path traversal refused, SQLite online backup 256 pages per step with a pause, `journal_mode DELETE`, `integrity_check`), `pos-export-mock` skeleton. Cargo.lock: 78 packages added, 0 changed.
- **T3.** Statistics copy v1 (tables per X1D9), channel mapping, refund sign, sum of tickets equal to the Z total, VAT per rate equal to the Z VAT, training and non-sale events excluded, pseudonyms `u-` + 16 hex. The privacy proof checks column names, every TEXT value (11-digit runs, `@`, bcrypt prefixes, planted secrets) and the raw file bytes. 25 tests.
- **T4.** `upload.rs` (512 MiB limit checked before and after the read, Bearer token marked sensitive, `X-Content-SHA256`, `pili-export/1`, 401/403 Unauthorized), mock receiver with `fail_next`, `set_delay`, `set_unauthorized`, observer, binary.
- **T5.** Outbox: backoff 1, 2, 4 ... 60 min, upload order backup, stats, manifest, `retry_now`, retention with `dropped`, token absent from every file and log; the first version's global lock removed in rework. 118 pos-export tests.
- **T6.** Engine: `ExportState`, a separate loop (spawned only when enabled), `wake_after_z` after a real Z only, `do_export_status`/`do_export_run_now` (Admin), check-config of the section; `export_nightly` 12 scenarios. The Z took 21 ms and the next sale 20 ms with a 30 s receiver delay.
- **T7.** `export_status` and `export_run_now` in both Tauri handler lists, `PosEngine.exportStatus()`/`exportRunNow()` (suspend), record mirror, 6 FFI tests, `pos-core.api` additive only, cert command count 86.
- **T8.** Settings panel, admin only, 30 s poll created in `onMounted` and cleared in `onUnmounted`, Brussels times, tokens only; 9 Playwright tests (UI e2e 305).
- **T9.** `dev.nuagemagique.pilipos.nightly` (route, FeatureEntry, SettingsEntry `nightly.export` managerOnly order 800, ViewModel, Screen polling through `LifecycleStartEffect`), strings fr/en/zh, 34 tests and 3 goldens, Kover LINE 95.7% BRANCH 75.4%; LANES owner `nightly`.
- **T10.** `EXPORT_NOCTURNE_FR.md` (annex 34) and updates of the data sheet, certification answers, database security, data acquisition sheet, company profile, manuals, assembly schema, MANIFEST and PDFs; every cited test exists, every figure matches the code.

## Phase gate evidence

Planner's runs on `a364ad7` (clean tree except the uncommitted `phases/X1.md`; `ui` built by the gate; logs in the session scratchpad `x1exit-planner-gate{,2,3}.log`):

| Step | Result |
|------|--------|
| `git merge-base --is-ancestor ba57adf HEAD` | PASS |
| `x1_gate.sh --exit --base ba57adf` (third run, no sccache) | **GATE \| total \| PASS \| 0 failed (98 checks, 6 waived)** |
| `test_inventory.sh --diff` | `ok no test disappeared (ws 1640 >= 1355)` |
| `check_command_parity.py --strict` / `--hygiene` | `missing: none` / `hygiene: ok (101 exports, 111 types)` |
| `check_ipc_wrappers.py` | `commands: 88 (86 without peppol), single-call: 88/88` |
| `check_lock_parity.sh` | `lock parity: 339 packages, identical` |
| `lane_paths.py --check` | `LANES \| total \| PASS \| 3440 files, 76 rules, 68 probes` |
| `vue_to_android.mjs --check` | `481 keys ... fdm 47, error codes 63 + 22` (unchanged) |
| `check_dossier.py` | PASS |
| `check_dossier.py --final --rules pdf-freshness,manifest` | one finding, `MANIFEST.txt:64 listed file does not exist: evidence/build`, pre-existing and identical on `android/int` (the directory is created by the final build) |
| `cargo tree -p pili-pos --features cert` | `pos-export v1.1.0` only, never `pos-export-mock` |
| `x1_gate.sh --fast --app-e2e` | not rerun by the planner; integrator at `a364ad7` (integrate-6): 41 checks, 0 failed, e2e 11 of 11 on `sunmi_d3_pro_api33` |
| `x1_gate.sh --fast --linux-repro` | not rerun (Docker Desktop down); integrator at `a364ad7`: two identical builds, SHA-1 `d4fb09a2ae2cf2501fe4d6847eb72e954d0b95ed` |

Key lines of the passing exit run: `root:tests` 1627 passed; `root:tests-cert-pili-pos` 8; `root:tests-cert-pos-engine` 591; `root:ui-e2e` 305 passed, inventory +25 / -0; `root:ipc-signatures-additive` 86 kept, 2 added; `root:version` 1.1.0; `android:rust-tests` 242; `android:ndk-build` `libpos_ffi.so` 74155856 bytes; `android:kotlin-tests` base 318, head 352, removed 0; `android:app-unit-tests` 309; `android:app-kover` LINE 95.7%, BRANCH 75.4%; `x1-scope` 61 guarded files, all in scope; `x1-lock-additive` Cargo.lock 577 kept, 78 added, android/rust/Cargo.lock 369 kept, 80 added; `x1-parity` missing none; `x1-selftest` 73 cases.

For X1-T10 (`86088e4`, not merged): `check_dossier.py` PASS, `test_check_dossier.py` 19 OK, pdf-freshness clean, the same single manifest finding, every changed path inside the T10 regex, no em-dash; it changes no build input, so the root and android gate results above hold for it.

## Measured sizes and the space estimate

| Export of one test day | Bytes |
|------------------------|-------|
| `pos-*.db.age` (engine test day, X1-T6 scenario a) | 311767 |
| `stats-*.sqlite` | 81920 |
| `manifest-*.json` | 783 |
| fixture day (X1-T5): backup / source `pos.db` | 254338 / 253952 |

The encrypted backup is the size of `pos.db` plus a few hundred bytes (age header and chunk tags), and the statistics file starts at 80 KB (SQLite pages of ten empty tables) and then grows with the day's tickets. So the local exports take about `keep_days x (size of pos.db + a statistics file)`.

The growth of `pos.db` was not measured on real traffic. A development database (`pos-20261003-171447.db`, 19 fiscal events) stores about 1.5 KB of JSON per fiscal event (an N event 1.7 KB, a Z 2.4 KB), before the order rows, lines, indexes and print records. A rough working figure is 5 to 10 KB per ticket all in. At 150 tickets a day that is about 1 MB a day, 30 MB a month, 350 MB a year. This is more than "megabytes per month" in `X1.md` section 2, and it has two consequences the user should know:

- with `keep_days` 14 the exports keep 14 full copies: about 0.4 GB after one month of a 30 MB database, about 5 GB after one year of a 350 MB database; the internal storage of the D3 Pro was not checked;
- a `pos.db` above 512 MiB can no longer be uploaded (X1D8), which this estimate reaches after about 18 months.

See Risks and Questions 4 and 6.

## Hashes and references

| Item | Value |
|------|-------|
| exit head before the exit commit | `a364ad7e3e345ff005b678e6c73f19268743c7fd` |
| fork point | `ba57adf06191a1f386863f98f24930b6f5aa543c` |
| X1-T10 tip (not merged) | `86088e4` (`android/X1-X1-T10`) |
| freeze tag (unchanged) | `linux-v1.1.0-rc1` -> `65b567fdd2497277300340979c2c61ff17abc918`, SHA-1 `9d230ef1bba90a4edde0157bcb107e4bb4d23d6e` |
| new Linux cert binary, two identical Docker builds (integrate-6, at `a364ad7`) | SHA-1 `d4fb09a2ae2cf2501fe4d6847eb72e954d0b95ed` |
| earlier repro on the lane | `f28d684c65eb087c8776bf9530bc2f909ef954cf` at `4a0688c` (before T7 and T8 added the two commands and the panel) |
| `main` / `cert/v1.0` | `96728e1325f5e11d69b7571d17280e3027974e06` / `d6210caabb32ca09f54be0f15410ce8447960be2` |
| age | `=0.12.1`, default features off (X1D15) |
| Cargo.lock / android/rust/Cargo.lock | +78 / +79 packages, none changed; lock parity 339 packages identical |
| cert commands | 86 (84 + `export_status`, `export_run_now`) |

## Decisions taken while the user was away

X1D1 to X1D16 (planner, verbatim in `X1.md`), X1D17 to X1D22 (integrator, one per wave) and, at this exit:

- **X1D23** X1-T10: the planner applied the reviewer's two fixes on the card branch (`86088e4`); the merge needs a re-review and was refused by the permission check in this run; T10 is carried to the next step, not dropped.
- **X1D24** accepted scope widenings, each unavoidable: X1-T7 `scripts/android/test_check_command_parity.py` (84 to 86), X1-T9 `android/app/src/test/kotlin/dev/nuagemagique/pilipos/nav/FeatureEntriesTest.kt` (8 to 9 entries, expect a textual conflict with the platform lane), X1-T3 `tests/common/mod.rs` additions in X1-T5.
- **X1D25** accepted design details from the reviews: STATS_SCHEMA v1 may gain tables and CHECK values as well as columns; an INSZ missing from `users` maps to role `cashier`; the first pass after start-up calls `retry_now`; the export loop is spawned only when enabled.
- **X1D26** follow-ups for a later card (none blocks the exit), listed under Risks.
- **X1D27** no freeze step at this exit; the candidate SHA-1 for `linux-v1.0.0-rc1` is `d4fb09a2...`, recorded in `LINUX_FREEZE.md`.
- **X1D28** the SD card answer and the space estimate of this report replace the "megabytes per month" guess of `X1.md` section 2.
- **X1D29** sccache can serve an artifact compiled in another worktree whose `env!("CARGO_MANIFEST_DIR")` path no longer exists (seen for `pos-engine-testkit` from the removed X1-X1-T9 worktree): a check that fails on a missing workspace path after a worktree removal is rerun after `cargo clean -p <crate>` without `RUSTC_WRAPPER` before it counts as a code failure.

## Risks

- **Database size vs keep_days and the 512 MiB limit** (estimate above): 14 full copies of a growing database, and no upload once `pos.db` passes 512 MiB. Options for the user: a lower default `keep_days`, keeping only the newest backup locally while still uploading each one, or a streamed upload (the reqwest stream feature) before the database gets there. Shrinking the database is not an option: the fiscal journal is append-only.
- **Freeze checks on `android/int`** fail for every lane after the merge until the re-freeze (X1D3).
- **`run_now` race** (`commands/export.rs:202`): `retry_now` from the command task can write back a stale `state.json` while the loop uploads; the effect is a second idempotent PUT and a short `pending 1`, no data loss. Fix: a flag the worker applies at the start of its next pass.
- **Upload client edge cases:** a malformed token is reported as Transport and retried forever (validate up front); a base URL with a query or fragment is not rejected by `export_url_problem` (the bracketed-host tail too).
- **Snapshot under continuous writes** restarts the online backup each time; acceptable after a Z, an attempt limit would bound it.
- **Pseudonym key fallback** (`pseudonym.rs`): when `hard_link` fails for another reason than AlreadyExists, `exists` then `rename` can overwrite a concurrent key; unreachable on ext4/f2fs.
- **N events with a NULL label** are exported too; only label `N` would be stricter.
- **UI polish:** the Vue panel can skip the reload after Retry during an in-flight poll; on Android a refused `runNow` replaces the status with the error until the next poll (Vue keeps both); `SystemNightlyLanguage` uses the device locale until the UI lane adds an app language store.
- **Flaky test under load:** `pos-ffi lifecycle::cancelled_shutdown_then_shutdown_ends_the_streams` failed once at load 12 to 20 and passes alone (X1D22).
- **sccache and removed worktrees** (X1D29): a cached artifact can embed the manifest path of a deleted worktree; the symptom is `workspace root` panics in `fixtures.rs:56`.
- **Tooling:** `x1_gate.sh` without `--allow` leaves `x1-paths` SKIP and nothing checks files outside the guarded areas; always pass `--allow`.
- **Disk.** The lane's `target` is 43 GB and the Mac had 14 GB free during the exit; three lanes building at once filled the disk and made Docker read-only twice (X1D17, X1D18).
- **Hardware residue.** No HTTPS run from a real D3 Pro to a real receiver (there is none yet).

## What the next phase must know

- **Merge order:** UI lane, platform lane, then `android/lane-export` (with T10 merged after its re-review). Expected conflicts: `docs/android/DECISIONS.md` (all three lanes append), `FeatureEntriesTest.kt` (the platform lane also counts entries), possibly `LANES.md` and `lane_paths.py`.
- **Gate `android/int` with `x1_gate.sh`** after the merge; the p2, p3 and p4 freeze checks fail by design until the user re-freezes, then a foundation card points the gates at `linux-v1.0.0-rc1`.
- **Keep `rust-embed` `deterministic-timestamps`** on pos-export whenever its dependencies change (X1D20), or the Linux build stops being reproducible.
- **Re-run `--linux-repro` on `android/int`** after the merge; the SHA-1 changes only if a freeze path changed during the merge.
- **check_ui_parity (UI lane):** `export_status` and `export_run_now` have their Kotlin call sites in the `nightly` package.
- **The SD card:** nothing in X1 uses one; P6's SAF export is the only removable-media path.

## Questions for the user

1. Approve the re-freeze plan (X1D3): version 1.0.0, tag `linux-v1.0.0-rc1` on the SHA-1 reproduced after the merge into `android/int` (`d4fb09a2...` on the lane today).
2. Review the dossier's legal framing (X1D13), including the T10 fix: the export as a copy, the legal record local, the receiver holding ciphertext and pseudonymised statistics it can read.
3. Who holds the owner's age private key, and is a producer key wanted (support restores) or not?
4. Default `keep_days` 14, given 14 full copies of a growing database: keep it, lower it, or keep only the newest backup locally?
5. The receiver: who operates it and where?
6. The 512 MiB upload limit: add a streamed upload before v1.0, or accept it and revisit when a database gets near it?

## The relayed SD card question

**Where the SQLite lives.** Not on the SD card. On the D3 Pro the engine opens `files/engine/pos.db` in the app's private internal storage (`ProvisioningPaths.kt:39`), and the nightly exports sit next to it in `files/engine/exports/`. X1 writes nothing to an SD card. A fiscal journal on removable media could be pulled out, swapped, corrupted or read by another app, so it stays inside the app; removable media are only the transfer medium of the manager's SAF export (P6).

**How big a card.** The card holds no live data, only what the manager exports by hand: a backup is about the size of `pos.db`. By the estimate above that is tens of MB in the first months and a few hundred MB after a year, so a good 32 GB card (Class 10 / A1, from a known brand) is ample, and 16 GB is enough. The largest card the D3 Pro accepts was not checked. The internal storage matters more than the card: with `keep_days` 14 the exports hold 14 full copies of the database (question 4).
