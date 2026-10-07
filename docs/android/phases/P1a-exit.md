# P1a exit report: engine extraction

Written by the P1a planner on 2026-10-07, at the end of the P1a run.

- **Base:** `android/int` at `aeee4e8` (`docs(android): S4 exit report`).
- **Exit head:** `android/int` at `240e4a6` before the exit commits (P1a cards, merges and the PD18 record), then the commit that adds `phases/P1a.md`, this report and the exit decisions (PD19 to PD29).
- **Cards:** `docs/android/phases/P1a.md`. Decisions: `docs/android/DECISIONS.md`, "Decided while the user was away (review)" (PD1 to PD17 from P1a-T3, PD18 from the wave 9 integrator, PD19 to PD29 at this exit). Engine reference: `docs/android/ENGINE.md`.
- main (`96728e1`) and `cert/v1.0` (`d6210ca`) were not touched. No `linux-v*` tag exists. Nothing was pushed, no GitHub Actions run, no live module2, HubRise or SPF call.

## Verdict

- **All eleven cards are APPROVE and merged** into `android/int` with `--no-ff`, one merge per card, no conflicts, no rework round, no escalation. P1a-T2 and P1a-T9 were reported "partial" by their workers for reasons outside their allowed paths (a decision to record; the journal byte hash); both reviewers approved, and the planner settles both items here (PD22, PD18/PD19).
- **`android/int` builds and the phase gate passes**, rerun by the planner at `240e4a6`: `p1a_gate.sh --base aeee4e8...` gives `GATE | total | PASS | 0 failed (29 checks)`, with the only SKIP the freeze check (no freeze tag, by design).
- **The engine is out of the Tauri crate.** `crates/pos-engine` holds the commands, state, engine, events, background loops and install API with no Tauri, no `cfg(target_os)` and no global; `src-tauri` is a shell of 79 one-call wrappers plus `main.rs`, `cli.rs`, `paths.rs` and `events.rs`.
- **No behaviour change was found.** Same IPC contract (wrapper signatures identical to the pre-P1a commands), same tests (0 removed), same FDM business requests (journal identical once the timer-driven `Status` polls are left out; see "FDM journal" and PD19), `posSwVersion` still `pili-pos/1.0.0`.
- **One exit criterion holds only under the PD19 rule:** the normalised journal hash is `d59a081d...`, not the baseline `68a09ee3...`, because one `Status` poll lands in another position on this host today. The P1a base commit gives the same `d59a081d...`, so P1a is not the cause.

**Planner recommendation:** go on with P1b on `android/int`. Merging into main waits for the user (PD17).

## Task status

| Card | Wave | Branch tip | Worker | Review | Merge on `android/int` |
|------|------|------------|--------|--------|------------------------|
| P1a-T1 engine functions, wrappers to `src-tauri/src/ipc`, wrapper checker | 1 | `android/P1a-P1a-T1` `d5b7384` | done | APPROVE | `c1be3e6` |
| P1a-T2 gate tooling, inventory across moves, freeze check, exclude | 1 | `android/P1a-P1a-T2` `749459f` | partial | APPROVE | `2180c24` |
| P1a-T3 dossier path rewrite tool, PD1 to PD17 | 1 | `android/P1a-P1a-T3` `7d336bb` | done | APPROVE | `fe7f7d2` |
| P1a-T4 `EngineConfig`, `Engine::open` in src-tauri | 2 | `android/P1a-P1a-T4` `5cffa73` | done | APPROVE | `9f9df19` |
| P1a-T5 move commands, state, engine to `crates/pos-engine` | 3 | `android/P1a-P1a-T5` `0cbe63d` | done | APPROVE | `06a91e3` |
| P1a-T6 tests to `pos-engine/tests`, harness to `pos-engine-testkit` | 4 | `android/P1a-P1a-T6` `4f55da6` | done | APPROVE | `8b96305` |
| P1a-T7 `EventSink`, background loops into the engine | 5 | `android/P1a-P1a-T7` `3a480e1` | done | APPROVE | `9a7e9df` |
| P1a-T8 globals removed, `engine.lock` | 6 | `android/P1a-P1a-T8` `d7308f2` | done | APPROVE | `8675928` |
| P1a-T9 `SoftwareIdentity`, `HttpTls` | 7 | `android/P1a-P1a-T9` `c926471` | partial | APPROVE | `db8d9ab` |
| P1a-T10 device id override, support program, bundle config | 8 | `android/P1a-P1a-T10` `d6dff13` | done | APPROVE | `c99d297` |
| P1a-T11 dossier prose, CLAUDE.md, README.md, ENGINE.md | 9 | `android/P1a-P1a-T11` `c004386` | done | APPROVE | `2196b1a` |

Other commit on `android/int`: `240e4a6` `docs(android): record the FDM journal Status ordering drift at the P1a exit` (PD18). The card branches are kept locally; all task worktrees are removed.

### What each card delivered (from the reviews)

- **T1.** Eight engine functions with verbatim bodies plus a ninth, `do_get_item_modifiers_localized` (PD20). 79 wrappers in `src-tauri/src/ipc`, `check_ipc_wrappers.py` (`single-call: 79/79`, `--map`, `--signatures`), signature diff against the pre-P1a `commands/` empty. Dossier: about 20 references repointed, all targets exist.
- **T2.** `test_inventory.sh` with `test_moves.tsv`, multi-package cert pass and `--min-ws`; `p1a_gate.sh` (fast and full, fails closed on a bad base); `check_linux_frozen.sh` (exit 2 with no tag, 15 frozen paths in a tagged scratch clone); `exclude = ["android"]`; test.yml `paths-ignore`. The multi-package cert pass first ran for real at T5 and the totals check held.
- **T3.** `rewrite_dossier_paths.py`: `--plan` 168 references in 25 files, `--unmapped` 0; an end-state simulation rewrote all 168 and left `--strict` at 0. PD1 to PD17 appended to DECISIONS.md.
- **T4.** `EnginePaths`, `ConfigSource`, `EngineConfig`, `Engine::open` replace `AppState::new` step by step with the same log texts; Tauri manages the `Engine`; backups take the engine's backups directory; per-TestApp temporary data directory (PD16). 8 engine tests.
- **T5.** 29 renames (all R100) into `crates/pos-engine`; features mirrored (`cert = [hubrise]`, no peppol in cert); `install.rs` with its 6 tests; pos-engine in the pili-pos cert graph, no tauri in its tree; Cargo.lock changes only in workspace entries.
- **T6.** 47 test files and 9 harness files moved (renames R076 to R100); testkit `publish = false` and outside the cert graph; scripts and generators follow; UC evidence regenerated identical except date fields; `uc_matrix.py --write` changes only the date line.
- **T7.** `EngineEvent` with the six topics, `EventSink`, `NullSink`, `start_background`/`shutdown`, `TauriEventSink` with an instance `OnceLock`; same intervals, warn text, `select!` on shutdown and payloads; no fiscal loop left in `main.rs`.
- **T8.** Status cache in `AppState` (same key and 20 s TTL); per-client sign-message sink in `FdmClient`, `on_sign_messages` removed; `engine.lock` with `File::try_lock` ("engine already open: <path>"); engine-globals 0, allowlist empty. Dossier: `engine.lock` row checked against `deploy/pili-pos.service` (pos:pos, 0640).
- **T9.** `SoftwareIdentity` (1 to 36 characters) replaces `POS_SW_VERSION`; Linux passes `concat!("pili-pos/", CARGO_PKG_VERSION)`; `HttpTls::{Platform, RootsOnly}` at the three production client sites, empty or invalid roots refused before the data directory is touched.
- **T10.** `device_id_override` (validated before the lock, applied before the configuration is read), support program resolved by the shell, `PosConfig::resolve_relative_to` over all seven path fields, `ConfigSource::Bundle`, `install::check_config_source`. No absolute Linux path and no environment read in the engine besides PD13.
- **T11.** Dossier prose on the new structure (French, brand-neutral, no fiscal statement, figure, result or evidence changed), CLAUDE.md and README.md, `docs/android/ENGINE.md` with the 79-row command map equal to `check_ipc_wrappers.py --map`.

## Phase gate evidence (rerun by the planner on 2026-10-07)

Run in the integration worktree (`moyun-pos-wt/int`) at `240e4a6`, clean tree (except the uncommitted exit files), `ui/dist/index.html` present.

`PATH="$HOME/.cargo/bin:$PATH" scripts/android/p1a_gate.sh --base aeee4e8b6624dcf48d1c6a79a44d615ef6106f2e`:

| Check | Result |
|-------|--------|
| ui-dist | PASS |
| hygiene-emdash, hygiene-artifacts, hygiene-untouchable, hygiene-cargo-lock | PASS |
| hygiene-subjects | PASS (34 subjects conform) |
| engine-tauri-free | PASS (no Tauri or `target_os` in `crates/pos-engine/src`) |
| engine-tree | PASS (no `tauri*` package under pos-engine) |
| engine-globals | PASS (0 globals) |
| fmt, clippy (workspace, `-D warnings`) | PASS |
| tests (workspace) | PASS: 1392 passed, 0 failed |
| cert-members | PASS: pili-pos, pos-engine |
| clippy-cert-pili-pos, tests-cert-pili-pos | PASS: 8 passed |
| clippy-cert-pos-engine, tests-cert-pos-engine | PASS: 508 passed |
| test-inventory | PASS (ws 1405 >= 1355) |
| test-inventory-selftest, linux-frozen-selftest | PASS |
| dossier-check, dossier-check-selftest | PASS |
| uc-matrix, uc-matrix-selftest | PASS (`MANQUANT: 0`) |
| ipc-wrappers, ipc-wrappers-selftest | PASS: `commands: 79 (77 without peppol), single-call: 79/79` |
| dossier-paths, dossier-paths-selftest | PASS: `check: 0 references to rewrite` |
| linux-frozen | SKIP: no freeze tag yet (PD14) |
| **total** | **PASS, 0 failed (29 checks)** |

Exit-only checks, run by the planner:

| Check | Result |
|-------|--------|
| `rewrite_dossier_paths.py --strict` | PASS: `strict: 0 references remain` |
| `src-tauri/src/commands`, `src-tauri/src/state.rs`, `src-tauri/tests` gone | PASS |
| `crates/pos-engine`, `crates/pos-engine-testkit` present | PASS (`pos-engine/tests`: 48 files, the 47 moved plus `background.rs`; testkit: 9 source files) |
| no `POS_SW_VERSION`, no `on_sign_messages` in `crates/` or `src-tauri/` Rust sources | PASS (0 hits) |
| main at `96728e1`, `cert/v1.0` at `d6210ca`, no `linux-v*` tag | PASS (only the local `android/p0-spike-S1` to `-S5` tags exist) |
| untouchable files unchanged since `aeee4e8` (`ui`, `rust-toolchain.toml`, `deploy/build-release.sh`, `scripts/build_and_hash.sh`, `scripts/check_repro.sh`, `test_inventory.txt`) | PASS |
| no `source`/`checksum` change in `Cargo.lock` since `aeee4e8` | PASS (0 lines) |
| no em-dash in the P1a diff | PASS (0 hits) |
| workspace version | `1.0.0` (PD3) |
| FDM journal capture (`capture_fdm_journals.sh`, local mocks only) | PASS under PD19: suite 8/8, 9 calls, without `Status` identical to run1, counters identical, `posSwVersion` `pili-pos/1.0.0` (5 of 5); full normalised hash `d59a081d...`, not `68a09ee3...` (see below) |

The integrator ran the same phase gate after every wave: 27 of 27 checks after waves 1 and 2, 29 of 29 after waves 3 to 9. Two incidents, neither caused by the merge: at the T5 merge one cert test run had 1 unidentified failure that did not repeat (PD26), and at the T9 merge the disk filled up (ENOSPC) until the T9 worktree target (76 GB) and `int/target/debug/incremental` were deleted.

## Test inventory

Baseline `docs/android/baseline/test_inventory.txt` (never edited): 1846 lines, ws 1355, cert 491, sha256 `586834f0...`.

| Figure | Exit value |
|--------|-----------|
| ws tests listed | 1405 (+50) |
| cert tests listed | 527 (pili-pos and pos-engine; +36) |
| lines | 1932 |
| moves | 9 rules in `test_moves.tsv`, 972 baseline lines renamed |
| diff | 86 added, 0 removed |
| current inventory sha256 | `2adf8b45a49602ff915ff37da366e8336f30ec3a461a98c2d1fa1b663d40f4fa` (written to the planner's scratch directory, not committed) |

Added tests by card (from the reviews' `--diff` lines): T4 +18, T7 +18, T8 +8, T9 +17, T10 +25. No test was deleted, renamed outside a declared move, or newly ignored.

## FDM journal

Baseline (P0.1, `docs/android/baseline/`): run1 and run2 normalised both `68a09ee31caa69a39864de47bb71313f949034b7da86d93fbd3af3b01c073473`.

| Capture | Commit | Calls (order) | Normalised sha256 | Without `Status` | `Status` only |
|---------|--------|---------------|-------------------|------------------|---------------|
| baseline run1 | P0.1 | Status, signWorkIn, signSale, Status, signSale, Status, signOrder, signSale, Status | `68a09ee3...` | `f5c085c3...` | `cfc66440...` |
| P1a base rerun (T9 reviewer) | `aeee4e8` | ..., signOrder, **Status, signSale** | `d59a081d...` | `f5c085c3...` | `cfc66440...` |
| P1a-T9 (reviewer, fresh clone) | `c926471` | ..., signOrder, **Status, signSale** | `d59a081d...` | `f5c085c3...` | `cfc66440...` |
| wave 9 integrator | `2196b1a` | ..., signOrder, Status, signSale | `d59a081d...` | (not kept) | (not kept) |
| planner exit capture | `240e4a6` | ..., signOrder, **Status, signSale** (suite 8/8, 9 calls) | `d59a081d...` (raw `d7fb2f0e...`) | `f5c085c3...` | `cfc66440...` |

`f5c085c3...` is `f5c085c3050bde77114a10f2778efa1d2c206f9766e0e11d6c99c64e89d58bae`; `d59a081d...` is `d59a081d8a4080cd13c3a6db8d53cf6807d91c47d5af8038bd6a3c349f2e750d`. Computed with `jq -S '[.[] | select(.operation != "Status")]'` on the normalised files. Every sign request and response is identical; only the position of one 30 s `Status` poll relative to a step that also runs about 30 s apart has moved (T9 reviewer: the poll used to land about 2.5 s after a sign, now about 0.4 s). In the planner's exit capture the `(eventLabel, eventCounter, totalCounter)` triples of the five sign calls equal run1 (`S 1 1`, `N 1 2`, `N 2 3`, `P 1 4`, `N 3 5`) and `posSwVersion` is `pili-pos/1.0.0` in all 5 occurrences. The raw exit journal hashes to `d7fb2f0eb340c416c7d84d825befa2144b884097a8fb009771d6ace4bac706aa` (raw journals always differ by timestamps and signatures). The first exit capture failed to compile because of a stale container build (PD29); the second, after refreshing the source file times, rebuilt `pos-fiscal` and passed. The planner accepts the exit on this rule (PD19); the baseline files are not edited.

## Hashes and references

| Item | Value |
|------|-------|
| P1a base | `aeee4e8b6624dcf48d1c6a79a44d615ef6106f2e` |
| last card merge (`chore(android): merge P1a-T11`) | `2196b1a` |
| PD18 record | `240e4a6` |
| main | `96728e1325f5e11d69b7571d17280e3027974e06` |
| `cert/v1.0` | `d6210caabb32ca09f54be0f15410ce8447960be2` |
| diff `aeee4e8..240e4a6` | 186 files, 6739 insertions, 2168 deletions, 34 commits |
| `check_repro.sh` | not run in P1a (PD11); it is the P1b exit check |

## Size against the plan

`src-tauri/src/*.rs` plus `src-tauri/src/ipc/*.rs`: 1551 lines (664 shell, 887 wrappers), against the plan's informative target of under 900. Not a gate; kept as is (PD25).

## Decisions taken while the user was away

All in `docs/android/DECISIONS.md` under "Decided while the user was away (review)".

- **PD1 to PD17** (planner, before wave 1): target layout, inventory across moves, version stays 1.0.0, eighth wrapper, managed `Engine`, typed events, where `HttpTls` and `SoftwareIdentity` live, bundle configuration, no new crates, dossier paths, gate cost, DLE EOT left alone, PD13 environment reads, freeze check written but not run, wave 1 merge order, test backups, exit without the user.
- **PD18** (wave 9 integrator): the journal `Status` ordering drift.
- **PD19 to PD29** (this exit): the journal parity rule (PD19), the ninth engine function (PD20), the checker's extra forms (PD21), the `engine-tauri-free` SKIP before the move (PD22), the actual wave 1 merge order (PD23), merge subjects (PD24), src-tauri size (PD25), the cert test flake (PD26), notes for P2 (PD27), no freeze in P1a (PD28), the shared Docker target volume of the journal capture (PD29).

## Risks

1. **Journal byte parity is timing-dependent.** The plan's "byte-identical normalised journal" check cannot hold on this host without a rule for the `Status` polls (PD19). P1b's journal diff must use the business view (no `Status`) or it will fail for reasons that are not regressions.
2. **An unidentified cert test flake** (PD26) in the `spf_e2e_*` targets happened once (T5 merge) and did not repeat in any later gate run.
3. **Disk.** The host had 39 GB free at the exit; `int/target` is 47 GB. A card worktree target can reach 76 GB; the integrator must keep deleting worktree targets after merges, and P1b should clean `int/target/debug/incremental` before Docker or Gradle work.
4. **Engine lifecycle on Android** (PD27a): dropping an `Engine` releases `engine.lock` while background tasks may still hold the database. Harmless on Linux (process exit); P2 must close in order.
5. **Dossier wording.** A few dossier sentences still have small inaccuracies left by the mechanical rewrite (an ASCII box line in `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md:27` wider than its frame; `RAPPORT_TESTS_BASELINE_FR.md:144` still shows the `cargo test -p pili-pos` heading until the next report generation). Not fiscal content; for a later docs pass.
6. **Stale container builds in the journal capture** (PD29): the capture script shares one Docker target volume across checkouts, and cargo's time-based freshness reused an old `pos-fiscal` build on the planner's first exit capture (compile failure, no journal). Touch the sources before a capture after another checkout was captured.
7. **Docker Desktop was force-restarted** by the P1a-T9 worker when it hung (kill of `com.docker.backend`, relaunch). No `pili-e2e` container was left running.

## Open items for the next phases

- **P1b:** the version bump to `1.1.0` and the soft freeze `linux-v1.1.0-rc1` with `docs/android/LINUX_FREEZE.md` (PD3, PD28); two `check_repro.sh` runs with the same new SHA-1; the journal diff on the business view (PD19); the DLE EOT timeout question (PD12) as a possible behaviour-change card before the freeze; P1b adds `idle_lock` and `fdm_poll` to `EngineTimers`.
- **P2:** the PD13 environment reads (`HUBRISE_*` debug fallback, support session HOME); the PD27 notes (close order, `HttpTls` check before `FdmClient`, absolute bundle path, `background.rs` cfg, stale `src-tauri/tests` mentions outside the dossier).
- **Freeze:** P1a has none. `check_linux_frozen.sh` exists and prints SKIP until the first `linux-v*` tag.

## Questions for the user

1. **Journal parity rule (PD19):** do you accept comparing the FDM journal without the timer-driven `Status` polls, or do you want the baseline re-captured on this host (a new `68a09ee3...`-style reference, with the evidence above)?
2. **DLE EOT timeout (PD12, from S4):** change it in P1b, before the soft freeze, or leave it as is for v1.1?
3. **Soft freeze at the P1b exit:** may the chain set `linux-v1.1.0-rc1` as a local tag while you are away (PD28), or should P1b stop before the freeze?
4. **src-tauri size (PD25):** is 1551 lines acceptable against the informative 900 target, given that the excess is documented one-call wrappers?
5. **Merge to main:** P1a stays on `android/int`. When do you want `android/int` reviewed and merged into main (after P1a, or only after P1b with the soft freeze)?
