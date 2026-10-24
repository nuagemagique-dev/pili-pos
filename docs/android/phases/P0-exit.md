# P0 exit report: decisions, baselines, spikes

> Correction (2026-10-10): the Linux build is not filed as v1.1 under Art. 113. Nothing was filed, so Linux and Android are both version 1.0.0 initial filings (Art. 99), and the freeze tag is `linux-v1.0.0-rc1`, not `linux-v1.1.0-rc1` (`docs/android/DECISIONS.md`, "Decided by the user"). The report below is unchanged.

Written by the P0 planner on 2026-10-06.
- **Base:** main `96728e1` (`feat(menu): show products by their menu code`).
- **Phase branch:** `android/p0`. It still points at `96728e1`; no task has
  been merged into it yet.
- **Exit rule (plan):** the user approves `docs/android/DECISIONS.md` (draft
  delivered with this report).

## Verdict

- **Five of six tasks are done.** P0.1, S1, S2, S3 and S5 are done, and the
  reviewer approved each of them after rerunning their gates.
- **S4 has not run.** There is no `android/p0-S4` branch, worktree or report.
  The AVD `sunmi_d3_pro_api33` exists on the host.
- **P0.7 is drafted.** `DECISIONS.md` is delivered with this report.

**Planner recommendation:**
- approve DECISIONS.md with its PENDING-S4 items left open;
- start P1a (engine extraction), which needs nothing from S4;
- run S4 before the P4 planner starts, since P4 provisioning and P6 depend on it.

The alternative is to hold the P0 exit until S4 reports. The user picks
(question Q1).

## Task status

| Task | Branch, commits | Work | Review | Merge into `android/p0` |
|------|-----------------|------|--------|-------------------------|
| P0.1 Baselines | `android/p0-P0.1`: `679430b chore(android): add test inventory and FDM journal scripts`, `778926a docs(android): record P0 baselines` | done | APPROVE | full branch (`--no-ff`): `scripts/android/*` are kept tooling, not spike code |
| S1 Cross-compile | `android/p0-S1`: `1d0f3da chore(spike): ...` (spike), `60c02b4 docs(android): S1 cross-compile spike report` | done | APPROVE | report commit `60c02b4` only |
| S2 UniFFI hello world | `android/p0-S2`: `673d0c5 chore(android): add S2 UniFFI hello-world spike` (spike), `367e8f1 docs(android): add S2 UniFFI hello-world spike report` | done | APPROVE | report commit `367e8f1` only |
| S3 TLS | `android/p0-S3`: `ef19691 chore(spike): S3 TLS spike ...` (spike), `04f4b4c docs(android): S3 TLS spike report and recommendation` | done | APPROVE | report commit `04f4b4c` only |
| S4 D3 Pro on the emulator | none | **not run** | none | nothing |
| S5 Reproducible APK | `android/p0-S5`: `2afb7db`, `7c5b020` (spike), `2e8317b docs(android): add S5 reproducible APK spike report` | done | APPROVE | report commit `2e8317b` only |
| P0.7 DECISIONS.md | planner draft | draft | user | after approval |

**Spike code.** The plan says spike code is never merged. For S1 to S5, only
the report commits are taken. Each of them touches only
`docs/android/spikes/Sn.md`, as the reviewers checked.

**Keeping the evidence reachable.** The reports cite files and commits under
`spikes/` on their own branches. Proposal: keep the four spike branches, or tag
their heads locally as `android/p0-spike-S1` to `android/p0-spike-S5`, so
those references stay valid. They are not pushed without the user's go-ahead
(question Q9).

## Gate evidence and key numbers

### P0.1 Baselines (`docs/android/BASELINE.md`)

**Test inventory** (`scripts/android/test_inventory.sh`, from the real test
binaries):

| Measure | Value |
|---------|-------|
| `ws` | 1355 tests (binaries report 1355). `cargo test --workspace` gives 1342 passed, 13 ignored, 0 failed |
| `cert` | 491 tests (strict subset of `ws`; only the two `spf_e2e_basic::m150_*` invoice tests are absent) |
| Lines / distinct names | 1846 lines, 1355 distinct names |
| sha256 | `586834f05390ed4535e34b22747d2a34c0f003542676684e0de8d8b0ff7f99a2` |

- The reviewer got the same file twice from a cold target dir.
- `--diff` behaves as specified: 0/0 with exit 0; a deleted line counts as
  added (exit 0); an extra line counts as removed (exit 1).
- There are no doctests.

**The plan's 1,356 does not hold.** The measured total is 1355 and no source
for 1,356 was found. DECISIONS §11.1 replaces the figure with `--diff` plus
`ws >= 1355`.

**Linux SHA-1** (`scripts/check_repro.sh` from a clean clone of `96728e1`):

| Measure | Value |
|---------|-------|
| Result | reproducible |
| SHA-1 | `83313be28027d2e8d9e84cbddff4e584799c2fbc` (both runs, both reports) |
| SHA-256 | `90255b297176fc33e909e035ed25a2767d5d4e6c5b641d035b87df2c2584e459` |
| Size | 36,879,344 bytes |
| Time | 4656 s in all (about 36 min per run plus the image pull), exit 0 |

This is the SHA-1 of main `96728e1`, not of `cert/v1.0`, which main is 47
commits ahead of (question Q6).

**FDM journals** (e2e_hubrise flow, local mocks only):

| Measure | Value |
|---------|-------|
| Runs | two, each passing 8/8 |
| Calls per run | 9: `Status, signWorkIn, signSale, Status, signSale, Status, signOrder, signSale, Status` |
| Normalised journals | byte-identical, sha256 `68a09ee31caa69a39864de47bb71313f949034b7da86d93fbd3af3b01c073473` |

The raw journals also show identical `eventCounter` and `totalCounter`
sequences across the two runs. They are kept as parity data for P1 (DECISIONS
§11.2).

### S1 Cross-compile (`docs/android/spikes/S1.md`)

**Build.**
- All six crates build in release for `aarch64-linux-android` (Rust 1.99.0,
  NDK r27.1, cargo-ndk 4.1.2) with no source change. The crates are `pos-db`
  (`--features hubrise`), `pos-fiscal`, `pos-print`, `pos-mail`, `pos-hubrise`
  and `pos-catalog`.
- aws-lc-sys and the bundled SQLite build with the NDK clang. No cmake or
  bindgen was needed.
- None of the six crates depends on `tauri`.
- The combined build takes 136 s cold and 10 s warm.

**Tests on the pili_api33 emulator** (counts match the host `--list`):

| Crate | Result |
|-------|--------|
| pos-db | 124 pass |
| pos-catalog | 30 pass |
| pos-print | 151 pass |
| pos-mail | 74 pass |
| pos-hubrise | 68 pass |
| pos-fiscal, as built | 215 pass, 35 fail, 2 ignored (252) |
| pos-fiscal, fixtures redirected | 250 pass, 0 fail, 2 ignored |

All 35 pos-fiscal failures have one cause: fixture paths baked in through
`CARGO_MANIFEST_DIR`.

**The one real Android difference** was the rustls-platform-verifier panic,
which S3 handles. mTLS FDM with aws-lc-rs works (`fdm_tls` 6/6,
`fdm_https_mock` 3/3).

### S2 UniFFI hello world (`docs/android/spikes/S2.md`)

**Verdict: GO.** It ran in the R8 release APK on pili_api33 (ALL OK) and in
`:jvmtest:test` (9/9) against the host dylib. Both were rerun by the reviewer.
Proven:
- the engine's own runtime through `rt.spawn` and `spawn_blocking`;
- the error enum mapped to sealed exceptions;
- `EventStream` with `Resync(missed=6)`;
- panics surfacing as `InternalException`, with the process surviving;
- a cancelled coroutine not aborting the Rust task;
- the main thread not blocked.

**Sizes:**

| Artifact | Bytes |
|----------|------:|
| `libhello_ffi.so` unstripped | 6,575,928 |
| `libhello_ffi.so` `--strip-debug` | 1,203,296 |
| `libhello_ffi.so` packaged (`--strip-unneeded`) | 628,272 |
| `libjnidispatch.so` | 165,992 |
| Release APK | 1,202,791 |

Every LOAD segment is aligned to `0x4000`, and `zipalign -c -P 16 -v 4`
passes.

**Gotchas carried into DECISIONS §11:**
- `uniffiEnsureInitialized()` is required, or a mismatched `.so` causes a JVM
  SIGSEGV;
- bindgen must read the unstripped `.so`;
- the uniffi 0.32 `--config` format;
- the R8 rule set;
- panic text is not forwarded to logcat.

### S3 TLS (`docs/android/spikes/S3.md`)

- **Baseline panic confirmed** (`rustls-platform-verifier-0.7.1/src/android.rs:90`).
- **All options reach both services.** A, B1, B1c, B2 and B2c each get
  HubRise `HTTP/2.0 404` and Scaleway `HTTP/2.0 200` on pili_api33. Every B
  variant still refuses the four badssl negative controls.
- **Root counts.** The emulator system store holds 125 roots; `webpki-roots`
  holds 121.
- **Recommendation adopted in DECISIONS §4:** B1c (`tls_certs_only` with
  `webpki-root-certs` 1.0.9, which is already in the root lock). Linux stays on
  `Platform`.
- **Option A** is a documented fallback. The AAR is 10,295 bytes, is only
  available from GitHub `maven-archive` (group `org.rustls`), and needs R8
  keep rules.
- **Three production call sites** need `HttpTls`, including the FDM https
  fallback at `fdm_client.rs:427`.

### S4 D3 Pro on the emulator

**Not run.** Its scope (plan P0) is still open:
- device owner and Lock Task;
- the deviceId chain;
- SAF export;
- printing to `pos-printer-mock` with DLE EOT;
- density and CJK font;
- silent PackageInstaller update;
- the `DISALLOW_*` restrictions;
- 24 h survival.

DECISIONS §12 lists what waits on it.

### S5 Reproducible APK (`docs/android/spikes/S5.md`)

**Result: reproducible.** Two fresh linux/amd64 containers (Rosetta) built the
S2 APK from a `git archive` of `367e8f1` and got:

| Artifact | SHA-1 (both runs) |
|----------|-------------------|
| APK (gate run, per-check RSA test key) | `03b3d75c2f44241bfb5eee4ad98471f96d16fb97` |
| `libhello_ffi.so` in the APK | `2e2801339e17d28fae8d40221e6c6ccc3e5a02d3` |
| `libjnidispatch.so` | `ab124d34754c52e0373f7c66bc354db1afb6af78` |

- **Four sessions** (eight builds with path remapping) all matched. That
  includes a run from a different build path.
- **Key-independent content.** The entry manifest
  `3de91ed6731db49b52fa4418d72d80673e320af2528343fa8b458f9903ee1ea9` is the
  same in every session, even though each session used a different key.
- **Signing determinism.** RSA re-signing is deterministic; ECDSA is not.
  This sets the RSA key constraint in DECISIONS §9.
- **Remap negative control.** Without the remap, the unstripped `.so` differs
  between build paths and the stripped one keeps 16 `/opt/cargo` strings.
- **Cost.**
  - The image is 3.99 GB and builds from scratch in 295 s.
  - Each build takes 546 s to 681 s in Gradle, measured under load.
  - The whole check takes 20 min 51 s.

## Reviewer follow-ups (non-blocking, carried forward)

**For later phases**
1. **Plan figures** (P1 planner): lines 96 and 333 of the plan say 1,356.
   Gates use `--diff` and `>= 1355` instead.
2. **Journal counters** (P1): compare `eventCounter` and `totalCounter` from
   the raw journals, not only the normalised ones.
3. **`HttpTls` placement** (P1a): the S3 sketch puts it in `pos-engine`, but
   `pos-fiscal` cannot reach it there. Defining it in `pos-fiscal` is in
   DECISIONS §4.
4. **`ui/dist` race** (all phases): do not run `check_repro.sh` concurrently
   with other builds in one tree.

**Report fixes (optional)**
5. **S1:**
   - S1.md:123-124 gate timings (41 s, 80 s) have no raw log;
   - S1.md:417 says 45 test files where there are 47;
   - S1.md:34 has a Darwin version typo.
6. **S2:**
   - S2.md:219 says `pos-ffi` where the spike crate is `hello-ffi`;
   - the spike's `uniffi.toml` still uses the old flat format;
   - experiment logs are written to `/tmp`.
7. **S3:** the test-only call-site rows leave out a few test files. The
   conclusion is unaffected.
8. **S5:**
   - the committed Dockerfile (with `# syntax=` removed) was never built end
     to end. `docker build --check` is clean, and the RUN layers are identical;
   - `check_repro_apk.sh:126,131` loses diagnostics, though not the verdict,
     when the two runs package different `.so` sets.

## Risks

1. **S4 not done.** Device owner, the deviceId source, silent self-update and
   the restriction set are core assumptions of distribution and compliance,
   and none is proven yet. The plan's known risk applies: device owner is
   refused when an account exists, which forces a reset or QR provisioning.
2. **Emulator only.** Every P0 result is from pili_api33 on an M1. Nothing ran
   on a Sunmi D3 Pro. Hardware residue: the real serial, the SUNMI kiosk
   extension, the recovery keys, OTG, bcrypt timing, the real Epson, and TLS
   on the real image.
3. **Permanent identity.** A wrong application id, or a lost or rotated
   signing key, makes updates impossible without an uninstall, which deletes
   the fiscal DB. Mitigations: decide both before P3, and keep backups of the
   key (DECISIONS §1 and §9).
4. **Supply chain of the APK build.**
   - Maven artifacts are not checksum-pinned yet, the builds use the network,
     and only M1/Rosetta was tested.
   - The real workspace (aws-lc-sys, the full engine) will build much slower
     under emulation than the S2 crate did.
   - P3.4 owns the fixes (DECISIONS §11.13).
5. **Root refreshes are filings.** With bundled roots, a root-store refresh
   needs a new APK, so a new declared version and a filing. The likelihood is
   low, because the target services chain to long-lived roots.
6. **Engine extraction size.** `src-tauri/src/commands` has 27 files and 9,909
   lines, and 20 of them reference `tauri::`. P1a moves it as a pure,
   test-guarded move, so any behaviour drift shows in the inventory and
   journal gates.
7. **Linux record ambiguity.** The baseline SHA-1 is for main `96728e1`, and
   `cert/v1.0` is 47 commits behind main. Until the user says which build is
   v1.0 of record, the v1.1 framing (Art. 113) rests on an assumption.
8. **Host disk and time.**
   - A cert-build target dir is about 19 GB, and a repro check takes about
     80 min for Linux and 21 min for the APK.
   - The S5 worker pruned the shared Docker build cache (disclosed), so other
     jobs only rebuild slower.
   - P1 tasks must clean their `CARGO_TARGET_DIR`.

## Questions for the user

1. **Exit timing.** Approve P0 now with S4 deferred (run S4 before P4 starts,
   with P1a allowed to begin), or run S4 first and approve after?
2. **Application id.** `dev.nuagemagique.pilipos` (debug
   `dev.nuagemagique.pilipos.debug`), or another one? (DECISIONS §1)
3. **Version scheme.** Is `pili-pos-android/X.Y.Z` OK, starting at 1.0.0, with
   versionCode `MAJOR*10000 + MINOR*100 + PATCH` and `-dev` for development
   builds? (DECISIONS §2)
4. **deviceId.** Sunmi serial (`Build.getSerial()` as device owner), read once
   at provisioning and checked at every open, with no generated fallback? Or
   should S4 also evaluate the device-owner Wi-Fi MAC
   (`getWifiMacAddress`), which matches Q505 a) and the Linux practice?
   (DECISIONS §3)
5. **TLS.** Do you accept bundled roots (B1c) rather than trusting the
   device's store, knowing that a root refresh means a new declared APK?
   (DECISIONS §4)
6. **Linux v1.0 and v1.1.**
   - Has v1.0 been filed, and is its SHA-1 of record from `cert/v1.0` or from
     main?
   - Should v1.1 be filed at the P3 hard freeze (A), together with Android
     at P8 (B, recommended), or, if v1.0 is not filed, should the extracted
     engine become the first Linux filing (C)?
   - What is the rough deadline and the freeze cadence (still listed under
     "Needs from you" in the plan)? (DECISIONS §10)
7. **Signing key.**
   - Do you confirm you hold it alone, as RSA (4096 after a probe rerun),
     30-year validity, offline, with two encrypted backups, never in CI?
   - Should `declared-android` reuse the existing minisign declaration key, or
     get a second one?
   - When should the key be created (go-ahead needed, before the first
     `android-v*` tag)? (DECISIONS §9)
8. **Updater sources.** Besides the operator-started GHCR pull, should the
   updater also accept the same signed `declared.json` + APK from a USB stick
   through SAF? (DECISIONS §6)
9. **Spike evidence.** May the spike branches stay as local branches, or be
   tagged `android/p0-spike-Sn`, so the reports' references to `spikes/` stay
   resolvable? They are not pushed.
10. **Admission.**
    - For the Android test machine on the module2 simulator, may it use posId
      `CPOS0031234567` with its own terminalId (for example `POS-2-ANDROID`)
      in the one free slot?
    - Will the SPF test machine be a real D3 Pro, or the emulated profile?
      (DECISIONS §8)
11. **Hardware.** Will a real Sunmi D3 Pro (and the Epson TM-m30III) be
    available before filing for the hardware residue checks?
12. **Docker cache.** Is the S5 worker's prune of the shared Docker build
    cache acceptable going forward, or should workers only remove their own
    images?

## Next steps after approval

1. **Merge into `android/p0`:**
   - P0.1 with `--no-ff`;
   - the four report commits (`60c02b4`, `367e8f1`, `04f4b4c`, `2e8317b`) by
     cherry-pick;
   - `docs/android/DECISIONS.md` as approved;
   - this report as `docs/android/phases/P0-exit.md`.
2. **Rerun the phase gate on `android/p0`:**
   - `scripts/android/test_inventory.sh --diff` (0 removed);
   - no em-dash in the tree;
   - only `docs/android/**` and `scripts/android/**` changed against main.
3. **Merge to main.** After a fetch and a comparison with origin, using the gh
   credential helper. No push without the user's go-ahead.
4. **Plan P1a.** Its cards use the DECISIONS §4 `HttpTls` placement and the
   §11 gates.
5. **Schedule S4** per the answer to Q1.
