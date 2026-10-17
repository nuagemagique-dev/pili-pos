# Lane 2-v1-features: exit report (T6, full gate)

Branch `android/lane-v1-features`, base `android/int` at `b3248d2d`. Gate run on 2026-10-17 by the T6 worker (Sonnet 5.5), every heavy step through `scripts/android/heavy.sh`. No source change was needed: every step below is green except the checks named under "Expected FAILs".

## 1. Gate steps

| Step | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo clippy -p pili-pos --no-default-features --features cert --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS, 1922 passed, 0 failed |
| `cargo test -p pili-pos --no-default-features --features cert` | PASS, 8 passed |
| `cargo test -p pos-engine --no-default-features --features cert` | PASS, 756 passed |
| android/rust `cargo fmt --all --check` | PASS |
| android/rust `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| android/rust `cargo test --workspace --locked` | PASS, 261 passed |
| `check_ipc_wrappers.py` | PASS, 102 commands (100 without peppol), single-call 102/102 |
| `check_command_parity.py`, `--strict`, `--hygiene` | PASS, 100/100, hygiene 115 exports and 126 types |
| `check_ui_parity.py`, `--strict` | PASS, called 89, excluded 9, pending 0, stale 0, missing 0 (98 commands) |
| `vue_to_android.mjs --check` | PASS, 629 keys, error codes 114 + 23 |
| `lane_paths.py --check` | PASS, 4507 files, 90 rules, 91 probes |
| `ui`: `npm run build && npx playwright test` | PASS, 403 passed |
| `p5_gate.sh` | FAIL, see section 2 (hygiene-paths, linux-frozen, app-only-scope, freeze-tag, ui-money) |
| `p6_gate.sh` | FAIL only on hygiene-paths, linux-frozen, freeze-tag (3 of 57) |
| `check_dossier.py` (Linux) | PASS |
| `PILI_DOSSIER_DIR=docs/android/dossier check_dossier.py` | PASS |

## 2. Expected FAILs and the one pre-existing FAIL

- `linux-frozen` (p2) and `freeze-tag` (p4): by design (V1F-D22), the lane changes paths frozen at `linux-v1.0.0-rc2`. The orchestrator's rc3 freeze (F4) clears them.
- `hygiene-paths`: same cause, 86 changed paths outside android/, scripts/android/, docs/android/ (crates/pos-db, pos-engine, pos-engine-testkit, pos-fiscal, pos-print, src-tauri, ui, docs/SUBMISSION_ARTIFACTS, scripts/uc_scope.toml, CLAUDE.md). All are lane paths by design (V1F-D58).
- `app-only-scope` (p5 only): p5 is the app-only gate, the lane touches android/rust and pos-core. The full gate is p6.
- `ui-money` (p5 only): `WebshopOrdersStore.kt:239` (`sumOf` over `amountCents`) and `:240` (`-` on `totalCents`). Code from commit `31908ed6` (card payments of online orders), not touched by the lane; `p6_gate.sh` does not run this check. Left for the orchestrator (F6).
- Every other `GATE |` line is PASS or SKIP (`frozen-provisional` SKIP: retired, tag rc2 exists).

## 3. GATE lines

### p5_gate.sh

```
GATE | hygiene-paths | FAIL | 86 changed path(s) outside the allowed set
GATE | hygiene-emdash | PASS | no em-dash in added lines
GATE | hygiene-artifacts | PASS | no tracked build output, key or local file
GATE | hygiene-subjects | PASS | 39 commit subject(s) are Conventional Commits
GATE | hygiene-root-lock | PASS | Cargo.lock and Cargo.toml unchanged
GATE | frozen-provisional | SKIP | retired: tag linux-v1.0.0-rc2 exists (PD100)
GATE | linux-frozen | FAIL | x frozen paths differ from linux-v1.0.0-rc2
GATE | app-only-scope | FAIL | 9 file(s) under android/rust, pos-core, smoke or docker: use the full gate
GATE | freeze-tag | FAIL | x frozen paths differ from linux-v1.0.0-rc2
GATE | hygiene-keys | PASS | no secret key file under android/, scripts/android/ or docs/android/
GATE | lanes | PASS | LANES / total / PASS / 4507 files, 90 rules, 91 probes
GATE | vue-strings | PASS | VUE / total / PASS / 629 keys (core 254, errors 139, checkout 40, reports 45, settings 27, webshop 124), fdm 49, error codes 114 + 23
GATE | kotlin-tests | PASS | base 2081, head 2210, removed 0
GATE | ffi-boundary | PASS | no native entry point outside engine/ and provisioning/
GATE | app-manifest | PASS | manifest ok: 6 permissions, exported components restricted
GATE | selftest-p4-gate | PASS | TEST / summary / PASS / 79 passed, 0 failed
GATE | selftest-heavy | PASS | TEST / summary / PASS / 11 passed, 0 failed
GATE | selftest-kotlin-tests | PASS | OK
GATE | selftest-lanes | PASS | OK
GATE | selftest-vue-strings | PASS | ℹ duration_ms 2235.894208
GATE | app-lint | PASS | 74 actionable tasks: 1 executed, 73 up-to-date
GATE | app-ktlint | PASS | 11 actionable tasks: 11 up-to-date
GATE | app-detekt | PASS | 1 actionable task: 1 up-to-date
GATE | app-unit-tests | PASS | 2163 tests
GATE | app-screenshots | PASS | 59 actionable tasks: 2 executed, 57 up-to-date
GATE | app-kover | PASS | LINE 97.4%, BRANCH 74.8%
GATE | app-assemble-release | PASS | 90 actionable tasks: 1 executed, 89 up-to-date
GATE | app-apk-check | PASS | 6 checks passed, 0be3425f0736
GATE | ui-parity | PASS | UIPARITY / total / PASS / called 89, excluded 9, pending 0, stale 0, missing 0 (98 commands)
GATE | ui-contracts | PASS | no contract violation in the ui/ and feature/ sources
GATE | ui-money | FAIL | 2 money arithmetic violation(s): android/app/src/main/kotlin/dev/nuagemagique/pilipos/ui/state/WebshopOrdersStore.kt:239 sumOf uses amountCents; android/app/src
GATE | ui-strings | PASS | 9 strings_ui file(s) with en and zh twins and the ui_<area>_ prefix
GATE | selftest-p5-gate | PASS | SELFTEST / total / PASS / 193 passed, 0 failed
GATE | selftest-ui-parity | PASS | SELFTEST / total / PASS / 46 tests
GATE | total | FAIL | 5 failed (34 checks)
```

### p6_gate.sh

```
GATE | hygiene-paths | FAIL | 86 changed path(s) outside the allowed set
GATE | hygiene-emdash | PASS | no em-dash in added lines
GATE | hygiene-artifacts | PASS | no tracked build output, key or local file
GATE | hygiene-subjects | PASS | 39 commit subject(s) are Conventional Commits
GATE | hygiene-root-lock | PASS | Cargo.lock and Cargo.toml unchanged
GATE | frozen-provisional | SKIP | retired: tag linux-v1.0.0-rc2 exists (PD100)
GATE | linux-frozen | FAIL | x frozen paths differ from linux-v1.0.0-rc2
GATE | rust-fmt | PASS | ok
GATE | rust-clippy | PASS | Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.57s
GATE | rust-tests | PASS | 261 passed
GATE | ffi-deps | PASS | pos-engine yes, pos-peppol no, tauri no
GATE | lock-parity | PASS | lock parity: 341 packages, identical
GATE | command-parity | PASS | missing: none
GATE | ffi-hygiene | PASS | hygiene: ok (115 exports, 126 types)
GATE | selftest-frozen | PASS | PASS: all cases
GATE | selftest-lock-parity | PASS | OK
GATE | selftest-command-parity | PASS | OK (skipped=1)
GATE | jvm-tests | PASS | 20 actionable tasks: 20 up-to-date
GATE | selftest-check-apk | PASS | TEST / summary / PASS / 12 passed, 0 failed, 0 skipped
GATE | selftest-p3-gate | PASS | TEST / summary / PASS / 17 passed, 0 failed
GATE | gradle-lint | PASS | 103 actionable tasks: 3 executed, 100 up-to-date
GATE | gradle-ktlint | PASS | 23 actionable tasks: 23 up-to-date
GATE | gradle-detekt | PASS | 3 actionable tasks: 3 up-to-date
GATE | unit-tests | PASS | 22 tests
GATE | mismatch-test | PASS | 22 actionable tasks: 22 up-to-date
GATE | api-check | PASS | 12 actionable tasks: 1 executed, 11 up-to-date
GATE | aar-check | PASS | aar ok: jni/arm64-v8a/libpos_ffi.so only, 4 consumer rules
GATE | assemble-release | PASS | 85 actionable tasks: 1 executed, 84 up-to-date
GATE | apk-check | PASS | 6 checks passed, 8f0161f06e06
GATE | freeze-tag | FAIL | x frozen paths differ from linux-v1.0.0-rc2
GATE | hygiene-keys | PASS | no secret key file under android/, scripts/android/ or docs/android/
GATE | lanes | PASS | LANES / total / PASS / 4507 files, 90 rules, 91 probes
GATE | vue-strings | PASS | VUE / total / PASS / 629 keys (core 254, errors 139, checkout 40, reports 45, settings 27, webshop 124), fdm 49, error codes 114 + 23
GATE | kotlin-tests | PASS | base 2081, head 2210, removed 0
GATE | ffi-boundary | PASS | no native entry point outside engine/ and provisioning/
GATE | app-manifest | PASS | manifest ok: 6 permissions, exported components restricted
GATE | selftest-p4-gate | PASS | TEST / summary / PASS / 79 passed, 0 failed
GATE | selftest-heavy | PASS | TEST / summary / PASS / 11 passed, 0 failed
GATE | selftest-kotlin-tests | PASS | OK
GATE | selftest-lanes | PASS | OK
GATE | selftest-vue-strings | PASS | ℹ duration_ms 2075.7065
GATE | app-lint | PASS | 74 actionable tasks: 1 executed, 73 up-to-date
GATE | app-ktlint | PASS | 11 actionable tasks: 11 up-to-date
GATE | app-detekt | PASS | 1 actionable task: 1 up-to-date
GATE | app-unit-tests | PASS | 2163 tests
GATE | app-screenshots | PASS | 59 actionable tasks: 2 executed, 57 up-to-date
GATE | app-kover | PASS | LINE 97.4%, BRANCH 74.8%
GATE | app-assemble-release | PASS | 90 actionable tasks: 1 executed, 89 up-to-date
GATE | app-apk-check | PASS | 6 checks passed, 0be3425f0736
GATE | plat-manifest | PASS | backup off, no installLocation, 6 permissions, none for storage or install
GATE | plat-backup-rules | PASS | both sections exclude root, file, database, sharedpref, external; no include
GATE | plat-strings | PASS | 3 file(s), 114 names, same keys in fr, en and zh
GATE | plat-boundary | PASS | 56 source file(s) under platform/, no engine internals, no force-stop, no plain http
GATE | plat-keys | PASS | debug key equals dev-minisign.pub, release key empty
GATE | plat-engine-ref | PASS | pili-pos 1.0.0, linux-v1.0.0-rc2 (922a158)
GATE | selftest-p6-gate | PASS | TESTP6 / total / PASS / 110 tests
GATE | selftest-declare | PASS | TESTDECLARE / total / PASS / 42 cases
GATE | total | FAIL | 3 failed (57 checks)
```

## 4. Fixture deviations per UC (`crates/pos-engine/tests/spf_e2e_fixtures.rs`)

Every listed deviation is asserted both ways (an unlisted difference fails, and a listed one that no longer occurs fails).

| UC | Deviations |
|---|---|
| M122 (table transfer) | none |
| M132 (payment correction) | one: the new card line is `CARD_DEBIT` MANUAL against the fixture `CARD_CREDIT` (no linked terminal, the operator cannot tell debit from credit; V1F-D34) |
| UC223 (event discount at the till) | `TABLE_SALE` only (dine-in sale carries costCenter TABLE) |
| UC225A step 2 (discount after registration) | one: the order discount is an EVENT scope change, the fixture's one-line order uses LINE |
| UC229B (free line at the till) | `TABLE_SALE` plus one: the catalogue has the Coca Cola at VAT A (the scenario says A), the fixture request labels it B |
| UC225B step 2 | replayed in `uc225a_discount_after_registration` (the 2_SALE fixture, the same single listed deviation) |
| Kitchen notes (item D) | no fixture, never in fiscal data |
| Older replays unchanged | M110, M111, M112B, M121, M123, M171, M171B, M172, UC230, UC4119, UC4109 keep their previously listed deviations |

## 5. Consistency pass

- Basic Set counts: both dossiers state 22 UC, 20 with an automated test (`REPONSES_MANUEL_CERT_FR.md` of each); the Linux `RESULTATS_TESTS_UC_FR.md` states 18 fixture replays in `spf_e2e_fixtures.rs` and the 18 listed UC (M110, M111, M112B, M121, M122, M123, M130, M131, M132, M133, M140, M141, M160, M161, M170, M171, M171B, M172) each have a test in that file. The Android dossier does not state a replay count.
- Command counts: `CLAUDE.md` says 102 in all, 100 in the cert build; `check_ipc_wrappers.py` prints 102 (100 without peppol).
- Migrations 30 (till-discounts) and 31 (kitchen-notes): described in `CLAUDE.md` and in `docs/SUBMISSION_ARTIFACTS/SCHEMA_BD_FR.md` (section 5.3 bis, `order_lines` columns, migration table). `docs/android/dossier` has no table-level schema document.
- Decisions: V1F-D1 to D57 all present under "## Lane 2-v1-features (2026-10-17)" at the end of `docs/android/DECISIONS.md`, plus V1F-D58 (gate result) added by T6.
- No em-dash in the lane diff (`git diff b3248d2d...HEAD | grep -n $'\xe2\x80\x94'` finds nothing); no root `Cargo.toml` or `Cargo.lock` change (`hygiene-root-lock` PASS).
- Plan section 11 completed with items 18 and 19.

## 6. Final captures to retake (orchestrator, after the merge)

Plan section 11 items 1 to 19: cart header "Changer de table", table picker, Tables move mode and after-transfer view; Tickets tab "Corriger le paiement", correction dialog and PIN, refused correction and "Paiement corrige" mark, X report with a correction; cart "Remise", discount dialog (percent, amount, Offert, reasons, preview, PIN), cart with an order discount and an offered line, checkout with a discount, VAT ticket and pre-bill with a discount, Z report reductions; note dialog and cart line with a note, bar slip with a note, note slip, transfer notice slip; manual section 1 profiles table; Vue discount reason keypad; Vue note keypad.

## 7. Orchestrator follow-ups

- F1: regenerate and embed the UC evidence (`scripts/capture_uc_evidence.sh`, `scripts/embed_uc_evidence.py`) after the three lanes merge.
- F2: rebuild the dossier PDFs and `CAPTURES_ECRAN_FR.pdf` after the captures of section 6.
- F3: Android device UC scenarios (`android/app/src/androidTest/assets/uc/10_basic/` M122, M132; `40_advanced/` UC223 till path, UC229B) and `docs/android/uc/results.json`, run on the emulator (`uc_e2e.sh`).
- F4: rc3 freeze (tag, `LINUX_FREEZE.md`, freeze SHA in `p4_gate.sh`), which clears `linux-frozen`, `freeze-tag` and `hygiene-paths`.
- F5: check migration numbers 28 to 31 across the three lanes before merging.
- F6 (new): `ui-money` of `p5_gate.sh` fails on `WebshopOrdersStore.kt:239-240` (base code, EFT). Decide between an engine helper and a `money-display-only` exemption; `lane-v1-fixes` may touch this store.
