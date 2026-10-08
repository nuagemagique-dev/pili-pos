# P2 exit report: pos-ffi facade + JVM tests

Written by the P2 planner on 2026-10-08, at the end of the P2 run.

- **Base:** `android/int` at `175d3ef` (`docs(android): P1b phase cards, exit report and exit decisions`).
- **Exit head:** `android/int` at `005a953` (`chore(android): merge P2-T12`) before the exit commit, then the commit that adds `phases/P2.md`, this report and the exit decisions (PD67 to PD75).
- **Cards:** `docs/android/phases/P2.md`. Decisions: `docs/android/DECISIONS.md`, "Decided while the user was away (review)" (PD51 to PD66 from P2-T2, PD67 to PD75 at this exit). FFI reference: `docs/android/FFI.md` (new, P2-T12); engine reference: `docs/android/ENGINE.md` (pointer to FFI.md added).
- main (`96728e1`) and `cert/v1.0` (`d6210ca`) were not touched. Nothing was pushed, no GitHub Actions run, no live module2, HubRise or SPF call, no `declare.sh`, no release key, no emulator needed.

## Verdict

- **All twelve cards are APPROVE and merged** into `android/int` with `--no-ff`, one merge per card, in the planned order, no conflicts, no rework round, no escalation.
- **`android/int` builds and the phase gate passes.** The planner reran the full exit gate itself on `005a953`: `p2_gate.sh --exit --root --android --base 175d3ef...` gives **0 failed of 24 checks** (only `linux-frozen` SKIP, no freeze tag), and a forced rerun of `:pos-core:test`, `:pos-core:mismatchTest`, `:pos-core:apiCheck` and `:pos-core:check` gives BUILD SUCCESSFUL with 23 JVM tests passing.
- **Parity 84/84.** Every cert Tauri command has its wrapper, its engine function and an async method of the same name on `PosEngine`, calling the same engine function (`check_command_parity.py --strict`: `parity: 84/84 (wrapper, engine fn, FFI method)`, `missing: none`). The two peppol commands are excluded (PD52).
- **The engine is untouched.** `git diff 042d911..005a953` on the 16 freeze paths is empty; P2 changed only `android/`, `scripts/android/` and `docs/android/` (not `docs/android/baseline/`); the root `Cargo.toml` and `Cargo.lock` are unchanged; the shared pos-engine dependency tree is identical in both workspaces (240 packages).
- **The arm64 library builds** with `cargo ndk --profile android-release`, every LOAD segment aligned to 0x4000 (16 KB pages), and the android-flavour Kotlin bindings generate from it.
- **No freeze step at this exit** (PD66, PD74): no `linux-*` tag, no `check_repro.sh`, no `docs/android/LINUX_FREEZE.md`. The soft freeze still waits for the user's answer on the mutation scan (PD47).

**Planner recommendation:** P3 can start on `android/int` at the exit commit. `:pos-core` is ready to become an Android library (PD60); `042d911` stays the provisional freeze point (PD48, PD63). Merging into main waits for the user.

## Task status

| Card | Wave | Branch tip | Worker | Review | Merge on `android/int` |
|------|------|------------|--------|--------|------------------------|
| P2-T1 `android/rust` workspace, `pos-ffi` core, stubs, `uniffi-bindgen` | 1 | `android/P2-P2-T1` `205520f` | done | APPROVE | `bf66c0d` |
| P2-T2 `p2_gate.sh`, `check_linux_frozen.sh --ref`, lock parity, PD51 to PD66 | 1 | `android/P2-P2-T2` `125f3cc` | done | APPROVE | `6ec3f5e` |
| P2-T3 `check_command_parity.py` (parity, hygiene, markdown) | 1 | `android/P2-P2-T3` `a9b3a33` | done | APPROVE | `d30f3ac` |
| P2-T4 type survey, `records/common.rs`, `PosEvent` | 2 | `android/P2-P2-T4` `ef15747` | done | APPROVE | `fe743ad` |
| P2-T5 `qr_matrix`, `verify_minisign`, `verify_bundle`, `install_init_db`, `check_config_bundle` | 2 | `android/P2-P2-T5` `950ed07` | done | APPROVE | `b1f7a0c` |
| P2-T6 Gradle build, `:pos-core` JVM tests | 2 | `android/P2-P2-T6` `2059e5c` | done | APPROVE | `18f6a2e` |
| P2-T7 orders/menu/tables/session (20) + `vat_split` | 3 | `android/P2-P2-T7` `74a9b1e` | done | APPROVE | `d454a8c` |
| P2-T8 checkout/unpaid/refund/copy/delivery/print (15) + `settle_preview`, `cash_due_cents` | 3 | `android/P2-P2-T8` `031a072` | done | APPROVE | `93f3e47` |
| P2-T9 reports/social/financial/settings/fdm_monitor (16) | 3 | `android/P2-P2-T9` `b2c2b70` | done | APPROVE | `1aca9d8` |
| P2-T10 auth/integrations/support (14) | 4 | `android/P2-P2-T10` `46cf56f` | done | APPROVE | `214493a` |
| P2-T11 hubrise/webshop (19) | 4 | `android/P2-P2-T11` `f8bc82a` | done | APPROVE | `7dcf674` |
| P2-T12 JVM end-to-end tests, apiCheck, FFI.md, `ffi-doc` | 5 | `android/P2-P2-T12` `35efff8` | done | APPROVE | `005a953` |

One integration fix commit: `fd92568` (`fix(android): derive serde on the pos-ffi contract types so the hygiene checker passes`, wave 1). P2-T3's hygiene check (H3) requires the serde derives and `deny_unknown_fields` on every contract type, and P2-T1 had left them off `EngineTimersMs`, `EngineOptions` and `PosEvent`; the integrator added them. The card branches are kept locally; all twelve P2 task worktrees are removed.

### What each card delivered (from the reviews)

- **T1.** Workspace `android/rust` (members `pos-ffi`, `uniffi-bindgen`, `uniffi =0.32.2`), seeded copy of the root lock, `[profile.android-release]` (unwind, opt z, LTO, `strip = "none"` for library-mode bindgen), `PosEngine.open/shutdown/events`, `PosError` (85 codes, asserted in `error.rs`), `convert`, `EventStream`, logging (`logging.rs`, the only `cfg(target_os` and globals), panic hook, `pos_sw_version`; `android-uniffi.toml` and `jvm-uniffi.toml`. `minisign-verify =0.3.0` (PD67).
- **T2.** `p2_gate.sh` (`--fast`, `--android`, `--root`, `--exit`, `--allow`, `--require-modules`), `check_linux_frozen.sh --ref` (18 self-test cases), `check_lock_parity.sh` + `lock_parity.py` (10 tests), empty `lock_parity_allow.txt`, PD51 to PD66 verbatim.
- **T3.** `check_command_parity.py` (parity against the real `check_ipc_wrappers.py` map, `--strict`, `--require-modules`, `--hygiene` H1 to H6, `--markdown`), 52 tests and fixtures; adversarial probes passed in review.
- **T4.** Type ownership table in `records/mod.rs`, 33 common records mirroring the engine (only rename: `Display` to `FdmMessageDisplay`), `PosEvent` with the 9 topics plus `Resync`, exhaustive event conversion, contract tests.
- **T5.** The five free helpers, async, file work on a dedicated thread, PD61 codes each tested, key pairs generated in memory (no committed key); `install_init_db` takes the engine's `engine.lock` (PD68).
- **T6.** `android/` Gradle 8.11.1 build (wrapper byte-identical to moyun-handheld), `:pos-core` Kotlin/JVM, tasks `cargoBuildHost`, `uniffiBindgen`, `cargoBuildMocks`, `mismatchTest` (ABI mismatch dylib); Load, Lifecycle and MocksSmoke tests.
- **T7.** 20 methods, records in `records/orders.rs`, `vat_split` equal to `VatSummary.vue` (checked exhaustively over -200000..200000 cents at 21, 12 and 6 % in review).
- **T8.** 15 methods, `FdmStatusReport`, `FdmDevice`, `FiscalEventSummary` typed from the engine's JSON with no field loss, `settle_preview` and `cash_due_cents` over `pos_fiscal::payment::settle` with the engine's vectors (PD69).
- **T9.** 16 methods, 7 records in `records/reports.rs`, 23 tests (stable under CPU load, 8 runs).
- **T10.** 14 methods (all `self.blocking`), no PIN logged, `User` without `pin_hash`, remote support refused with `support_unavailable` (PD53).
- **T11.** 19 methods, 25 HubRise and webshop types, `ApplyOutcome` custom conversion (PD71), `self.spawn` for the two sync engine functions behind async wrappers (PD70).
- **T12.** `EndToEndTest` (10 tests: checkout, refund, Z, events, cancellation with a detached sign that completes), binary-compatibility-validator 0.18.2 with `android/pos-core/api/pos-core.api` (7097 lines), `docs/android/FFI.md` with the 84-row parity block, `ffi-doc` and `jvm-check` in `p2_gate.sh --exit`.

## Phase gate evidence

Planner's own run on `005a953` (clean tree, `main` `96728e1325f5e11d69b7571d17280e3027974e06`, `cert/v1.0` `d6210caabb32ca09f54be0f15410ce8447960be2`, no `linux-*` tag, 34 GB free, `ui/dist` present):

| Step | Result |
|------|--------|
| `check_linux_frozen.sh --ref 042d911...` | exit 0, `frozen against ref 042d911: 16 paths, clean` |
| `check_linux_frozen.sh` (tag mode) | exit 2, `no freeze tag yet (neither linux-v1.1.0 nor linux-v1.1.0-rc1)` |
| `p2_gate.sh --exit --root --android --base 175d3ef...` | **PASS, 0 failed (24 checks)** |

The 24 checks of that run:

| Check | Result |
|-------|--------|
| hygiene-paths | PASS, 155 changed paths, all allowed |
| hygiene-emdash | PASS |
| hygiene-artifacts | PASS, no tracked build output, key or local file |
| hygiene-subjects | PASS, 37 Conventional Commit subjects (12 merges included) |
| hygiene-root-lock | PASS, root `Cargo.lock` and `Cargo.toml` unchanged |
| frozen-provisional | PASS, 16 paths clean against `042d911` |
| linux-frozen | SKIP, no freeze tag (PD66, PD74) |
| rust-fmt, rust-clippy (`-D warnings`, all targets) | PASS |
| rust-tests | PASS, 234 passed |
| ffi-deps | PASS, pos-engine yes, pos-peppol no, tauri no |
| lock-parity | PASS, `lock parity: 240 packages, identical` |
| command-parity (`--strict`) | PASS, 84/84, missing none |
| ffi-hygiene | PASS, `hygiene: ok (99 exports, 110 types)` |
| selftest-frozen, selftest-lock-parity, selftest-command-parity | PASS |
| jvm-tests | PASS (`:pos-core:test`) |
| ndk-build | PASS, `libpos_ffi.so` built (67676272 bytes) |
| so-align | PASS, 4 LOAD segments, all 0x4000 |
| bindgen-android | PASS, `pos_ffi.kt` 14144 lines with `class PosEngine` |
| test-inventory | PASS, no test disappeared (ws 1479 >= 1355) |
| jvm-check | PASS (`:pos-core:check`: test, mismatchTest, apiCheck) |
| ffi-doc | PASS, the FFI.md parity block is current (88 lines) |

The gate's `jvm-tests` reused up-to-date results of the integrator's run at the same commit, so the planner forced `./gradlew --no-daemon :pos-core:test --rerun :pos-core:mismatchTest --rerun :pos-core:apiCheck --rerun :pos-core:check`: BUILD SUCCESSFUL, both cancellation variants printed `order CLOSED, 1 sale(s)`. No Gradle daemon and no `target/android-mocks` process was left running; the working tree was clean before the exit commit.

The integrator ran the phase gate after every wave: 22 of 22 checks (`--root --android`) after waves 1 to 4, then 22 and 24 of 24 (`--exit`) after wave 5. The one failure was in wave 1 (H3 hygiene, fixed by `fd92568`, and `test-inventory` without `ui/dist`, fixed by building `ui` untracked).

Other figures: `check_ipc_wrappers.py` `commands: 86 (84 without peppol), single-call: 86/86`. The pos-engine dependency subtree (names and versions) is identical in the root (`--no-default-features --features cert`) and `android/rust`; extra versions pulled only by uniffi and minisign (`cargo_metadata`, `cargo-platform`, `cipher`, `inout`) are outside that subtree, so `lock_parity_allow.txt` stays empty.

## Test counts

| Figure | Start (`175d3ef`) | Exit (`005a953`) |
|--------|-------------------|------------------|
| Root Rust ws tests listed | 1479 | 1479, 0 removed (floor 1355) |
| `android/rust` tests passed | none | 234 |
| JVM tests (`:pos-core:test`) | none | 22 (EndToEnd 10, Lifecycle 7, MocksSmoke 3, Load 2) |
| JVM `mismatchTest` | none | 1 (ChecksumMismatch) |
| Gate self-tests | none | frozen 18 cases, lock parity 10, command parity 52 |

Android Rust tests over the run (gate figures): 30 after wave 1, 123 after wave 2, 202 after wave 3, 234 at the exit. No test was removed or ignored.

## Sizes

| Artifact | Size |
|----------|------|
| `libpos_ffi.so` arm64, `android-release` (unstripped, line tables, as the gate builds it) | 67676272 bytes (64.5 MiB), sha256 `dcb87497884f536fda9c8d06d7d57a7bd55cd4e50da46da9ebc0655e6bdf001b` |
| same, `llvm-strip --strip-unneeded` (what AGP will package) | 12620096 bytes (12.0 MiB), 5.0 MB gzip |
| `libpos_ffi.dylib` host debug (unstripped) | 65730416 bytes (62.7 MiB); 30.7 MB after `strip -x` |
| `.so` NEEDED | `liblog.so`, `libdl.so`, `libm.so`, `libc.so` |
| Kotlin bindings (android flavour) | 14144 lines |
| `pos-core.api` snapshot | 7097 lines |
| Code added | `pos-ffi/src` 4931 lines, `pos-ffi/tests` 6836, Kotlin tests 874; diff `175d3ef..005a953` 155 files, +27900, -19 |

The `.so` and dylib are build outputs under `target/android-rust` (not committed; the hash is of this host's build and is not a reproducibility claim).

## Decisions taken while the user was away

All in `docs/android/DECISIONS.md` under "Decided while the user was away (review)".

- **PD51 to PD66** (planner, before wave 1, copied by P2-T2): card layout, parity target and extras, engine options, errors, calls and lifecycle, records, events, logging, workspace and lock, the Kotlin side, bundle format, money helpers, provisional freeze, gate cost, mocks, exit without the user.
- **PD67 to PD75** (this exit):
  - PD67 `minisign-verify =0.3.0` instead of 0.2.x (the newest release).
  - PD68 the bundle verification details chosen by P2-T5; P4.4 and P6 must sign with prehashed minisign.
  - PD69 the till payment guard in `settle_preview` (engine predicate, accepted).
  - PD70 `blocking` or `spawn` follows the Tauri wrapper (amends PD52, PD55).
  - PD71 the engine cannot serialize a rejected catalog outcome; FFI works around it; the engine fix is the user's call.
  - PD72 the two forms of `order_no_lines` (inside `CheckoutResult`, or `Coded` from the pre-bill).
  - PD73 review notes carried, not reworked.
  - PD74 no freeze at the P2 exit; `042d911` stays the provisional point.
  - PD75 exit records.

## Risks

1. **No soft freeze yet** (carried from P1b). Only the provisional point `042d911` and the per-card `--ref` check stop a change to the Linux build inputs. P3 must keep `check_linux_frozen.sh --ref 042d911...` in every gate.
2. **Rejected catalog on Linux** (PD71). The Tauri `hubrise_sync_catalog` and `hubrise_adopt_catalog` fail to return a rejection; the menu is still protected (nothing applied), but the operator sees an error rather than the issues. Fixing it touches a frozen crate.
3. **Shutdown after a cancelled shutdown** (PD73, `core.rs:191`). Open `EventStream`s may not end until `PosEngine` is dropped. Matters for the P4 lifecycle (Activity or service teardown); should be fixed in a P3 or P4 card.
4. **Library size.** 12 MiB stripped per ABI (arm64 only). Acceptable for a device-owner app, but P3 should confirm AGP strips it (`strip = "none"` in cargo is deliberate for bindgen).
5. **Cancellation coverage.** Only the "detached sign completes" branch is exercised in the JVM tests; the "never started" branch is asserted but not reached. A device test in P4 or P7 should cover a coroutine cancelled before dispatch.
6. **FFI success paths through login.** The checkout, refund, copy and HubRise wiring tests in Rust reach only refusals or mocks (written before the FFI login landed); the success paths are covered by the JVM end-to-end tests and the testkit contract tests. A later card can move the Rust fixtures to the FFI login.
7. **Disk.** 34 GB free at the exit, `int/target` 19 GB. P3 adds the AGP build and the emulator; keep the 15 GB floor.

## What the next phase (P3) must know

- **Build inputs:** `android/rust` (own workspace, target `<repo>/target/android-rust`, `--locked`), `cargo ndk -t arm64-v8a build --locked --profile android-release -p pos-ffi`, bindgen in library mode on that `.so` with `android-uniffi.toml` (package `uniffi.pos_ffi`, `SystemCleaner`). The `.so` must be stripped at packaging, not in cargo.
- **Gradle:** wrapper 8.11.1 (pinned sha256), Kotlin 2.2.20, JDK 17, AGP 8.9.1 and R8 8.13.25 already in `libs.versions.toml` but not applied. P3.1 turns `:pos-core` into an Android library, keeps the JVM tests as host unit tests (JNA plain jar, host dylib, mocks built in `target/android-mocks`), and keeps `apiCheck` green (the snapshot changes only deliberately, with `apiDump`).
- **Gate:** reuse `scripts/android/p2_gate.sh` (or extend it) so the parity 84/84, hygiene, lock parity, so-align and `ffi-doc` checks keep running; `ffi-doc` requires the FFI.md parity block to match `check_command_parity.py --markdown`.
- **Runtime contract:** `PosEngine(EngineOptions)` is synchronous: call it on `Dispatchers.IO`. Every other call is `suspend`. Call `shutdown()` before reopening the same data directory (PD55); `engine.lock` refuses a second open. `posSwVersion` is `pili-pos-android/1.0.0-dev` until the version scheme (DECISIONS section 2) is approved.
- **Kotlin never computes money:** totals, cash due, rounding and the VAT display come from `settle_preview`, `cash_due_cents` and `vat_split` (PD62, PD69).
- **Errors:** `PosException.Coded` (85 codes), `Fdm` (raw text to translate like `fdmMessages.ts`), `Internal`, `EngineClosed`; a panic is UniFFI's `InternalException`. A checkout can also come back refused inside `CheckoutResult` (PD72).
- **Events:** one `EventStream` per collector, `Resync { missed }` when it lags; `next()` returns null after shutdown.
- **No emulator was used in P2.** P3's exit ("the engine has been shown to run on Android") is the first run on `sunmi_d3_pro_api33`.
- **Hard freeze at the end of P3** needs the user (plan, "Always needs the user's explicit go-ahead"), and the soft freeze before it is still blocked on PD47.

## Hashes and references

| Item | Value |
|------|-------|
| P2 base | `175d3efe9003d8a4bbd35ec97267d7ea32c074d4` |
| last card merge (`chore(android): merge P2-T12`) | `005a9531c4f83a2a0ee033a5443587c96f826245` (tree `bab7e2235c79398342167e81cd259d1b1047c1a0`) |
| integration fix | `fd9256877622092f70e0701584d98862a9ea4fe5` |
| provisional freeze point | `042d91135940cb3f678a3f5e2947744a9d05ce4d` |
| main | `96728e1325f5e11d69b7571d17280e3027974e06` |
| `cert/v1.0` | `d6210caabb32ca09f54be0f15410ce8447960be2` |
| diff `175d3ef..005a953` | 155 files, 27900 insertions, 19 deletions, 37 commits (12 merges) |
| Linux baseline SHA-1 (P0.1, main `96728e1`) | `83313be28027d2e8d9e84cbddff4e584799c2fbc` |
| Linux 1.1.0 SHA-1 | none: soft freeze still blocked (PD47) |
| local tags | `android/p0-spike-S1` to `-S5` only; no `linux-*` tag |
| pins added | `uniffi`/`uniffi-bindgen` 0.32.2, `minisign-verify` 0.3.0, `minisign` 0.10.0 (dev), JNA 5.19.1, coroutines 1.9.0, JUnit 4.13.2, binary-compatibility-validator 0.18.2 |
| free disk at the exit | 34 GB |

## Questions for the user

1. **Mutation scan and the soft freeze (PD47, still open).** `scripts/build_and_hash.sh` step 3 flags two integration test lines that P1a moved into the scanned tree. How should the scan and these tests relate, and may the chain then run `check_repro.sh` and set the local tag `linux-v1.1.0-rc1`?
2. **Rejected catalog outcome (PD71).** Do you want the `ApplyOutcome::Rejected` serialization fixed in `crates/pos-catalog` before the soft freeze (it changes a frozen path and the Linux behaviour on a rejected HubRise catalog), or kept for Linux v1.2?
3. **Decisions PD67 to PD75.** In particular PD70 (thread choice follows the Tauri wrapper) and PD68 (prehashed minisign for the provisioning and release signatures).
4. **Worktrees.** The clean, merged P1b worktrees `moyun-pos-wt/P1b-P1b-T7` and `moyun-pos-wt/P1b-P1b-T8` are still there (their removal was refused in P1b). May they be removed (branches kept)?
5. **Merge to main.** `android/int` now carries P1a, P1b and P2. When do you want it reviewed and merged into main (PD45, PD66)?
