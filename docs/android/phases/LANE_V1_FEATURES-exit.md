# Lane 2-v1-features: exit report (T6, full gate)

Branch `android/lane-v1-features`, base `android/int` at `b3248d2d`. Gate run on 2026-10-17 by the T6 worker (Sonnet 5.5), every heavy step through `scripts/android/heavy.sh`. Refreshed after the round 2 review fixes (section 8). Every step below is green except the checks named under "Expected FAILs".

## 1. Gate steps

| Step | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo clippy -p pili-pos --no-default-features --features cert --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS, 1927 passed, 0 failed |
| `cargo test -p pili-pos --no-default-features --features cert` | PASS, 8 passed |
| `cargo test -p pos-engine --no-default-features --features cert` | PASS, 759 passed |
| android/rust `cargo fmt --all --check` | PASS |
| android/rust `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| android/rust `cargo test --workspace --locked` | PASS, 261 passed (24 + 237) |
| `check_ipc_wrappers.py` | PASS, 102 commands (100 without peppol), single-call 102/102 |
| `check_command_parity.py`, `--strict`, `--hygiene` | PASS, 100/100, hygiene 115 exports and 126 types |
| `check_ui_parity.py`, `--strict` | PASS, called 89, excluded 9, pending 0, stale 0, missing 0 (98 commands) |
| `vue_to_android.mjs --check` | PASS, 630 keys, error codes 115 + 23 |
| `lane_paths.py --check` | PASS, 4507 files, 90 rules, 91 probes |
| `ui`: `npm run build && npx playwright test` | PASS, 404 passed |
| `p5_gate.sh` | FAIL, see section 2 (hygiene-paths, linux-frozen, app-only-scope, freeze-tag, ui-money, and the kotlin-tests artefact of section 3) |
| `p6_gate.sh` | FAIL on hygiene-paths, linux-frozen, freeze-tag, plus the three cleared items of section 3 (api-check fixed, selftest-declare flaky and green alone, kotlin-tests artefact) |
| `check_dossier.py` (Linux) | PASS |
| `PILI_DOSSIER_DIR=docs/android/dossier check_dossier.py` | PASS |

## 2. Expected FAILs and the one pre-existing FAIL

- `linux-frozen` (p2) and `freeze-tag` (p4): by design (V1F-D22), the lane changes paths frozen at `linux-v1.0.0-rc2`. The orchestrator's rc3 freeze (F4) clears them.
- `hygiene-paths`: same cause, 86 changed paths outside android/, scripts/android/, docs/android/ (crates/pos-db, pos-engine, pos-engine-testkit, pos-fiscal, pos-print, src-tauri, ui, docs/SUBMISSION_ARTIFACTS, scripts/uc_scope.toml, CLAUDE.md). All are lane paths by design (V1F-D58).
- `app-only-scope` (p5 only): p5 is the app-only gate, the lane touches android/rust and pos-core. The full gate is p6.
- `ui-money` (p5 only): `WebshopOrdersStore.kt:239` (`sumOf` over `amountCents`) and `:240` (`-` on `totalCents`). Code from commit `31908ed6` (card payments of online orders), not touched by the lane; `p6_gate.sh` does not run this check. Left for the orchestrator (F6).
- Every other `GATE |` line is PASS or SKIP (`frozen-provisional` SKIP: retired, tag rc2 exists).

## 3. GATE lines

Final run (round 2 review fixes, 2026-10-17). The `api-check` FAIL and the `selftest-declare` FAIL of the `p6_gate.sh` run below were fixed or cleared afterwards: `api-check` by `:pos-core:apiDump` (commit `chore(android): update the pos-core public API dump for CartLine.online`, `:pos-core:apiCheck` now PASS), `selftest-declare` is a flaky apksigner case (`empty-release-key-refused`, "a v1 signature is present") that passes alone (`TESTDECLARE | total | PASS | 42 cases`). `kotlin-tests` FAIL is an artefact: `android/int` moved on to `492505e4` (lane 1 merged, 23 tests that exist only there), and the gate compares with `android/int` by name; against the lane base `b3248d2d` the inventory says `KTESTS | total | PASS | base 2081, head 2211, removed 0`.

### p5_gate.sh

```
GATE | hygiene-paths | FAIL | 91 changed path(s) outside the allowed set
GATE | hygiene-emdash | PASS | no em-dash in added lines
GATE | hygiene-artifacts | PASS | no tracked build output, key or local file
GATE | hygiene-subjects | PASS | 49 commit subject(s) are Conventional Commits
GATE | hygiene-root-lock | PASS | Cargo.lock and Cargo.toml unchanged
GATE | frozen-provisional | SKIP | retired: tag linux-v1.0.0-rc2 exists (PD100)
GATE | linux-frozen | FAIL | x frozen paths differ from linux-v1.0.0-rc2
GATE | app-only-scope | FAIL | 9 file(s) under android/rust, pos-core, smoke or docker: use the full gate
GATE | freeze-tag | FAIL | x frozen paths differ from linux-v1.0.0-rc2
GATE | hygiene-keys | PASS | no secret key file under android/, scripts/android/ or docs/android/
GATE | lanes | PASS | LANES / total / PASS / 4508 files, 90 rules, 91 probes
GATE | vue-strings | PASS | VUE / total / PASS / 630 keys (core 254, errors 140, checkout 40, reports 45, settings 27, webshop 124), fdm 49, error codes 115 + 23
GATE | kotlin-tests | FAIL | base 2104, head 2211, removed 23
GATE | ffi-boundary | PASS | no native entry point outside engine/ and provisioning/
GATE | app-manifest | PASS | manifest ok: 6 permissions, exported components restricted
GATE | selftest-p4-gate | PASS | TEST / summary / PASS / 79 passed, 0 failed
GATE | selftest-heavy | PASS | TEST / summary / PASS / 11 passed, 0 failed
GATE | selftest-kotlin-tests | PASS | OK
GATE | selftest-lanes | PASS | OK
GATE | selftest-vue-strings | PASS | ℹ duration_ms 2309.312209
GATE | app-lint | PASS | 74 actionable tasks: 7 executed, 67 up-to-date
GATE | app-ktlint | PASS | 11 actionable tasks: 11 up-to-date
GATE | app-detekt | PASS | 1 actionable task: 1 up-to-date
GATE | app-unit-tests | PASS | 2164 tests
GATE | app-screenshots | PASS | 59 actionable tasks: 2 executed, 57 up-to-date
GATE | app-kover | PASS | LINE 97.4%, BRANCH 74.8%
GATE | app-assemble-release | PASS | 90 actionable tasks: 1 executed, 89 up-to-date
GATE | app-apk-check | PASS | 6 checks passed, e1a2b99e495f
GATE | ui-parity | PASS | UIPARITY / total / PASS / called 89, excluded 9, pending 0, stale 0, missing 0 (98 commands)
GATE | ui-contracts | PASS | no contract violation in the ui/ and feature/ sources
GATE | ui-money | FAIL | 2 money arithmetic violation(s): android/app/src/main/kotlin/dev/nuagemagique/pilipos/ui/state/WebshopOrdersStore.kt:239 sumOf uses amountCents; android/app/src
GATE | ui-strings | PASS | 9 strings_ui file(s) with en and zh twins and the ui_<area>_ prefix
GATE | selftest-p5-gate | PASS | SELFTEST / total / PASS / 193 passed, 0 failed
GATE | selftest-ui-parity | PASS | SELFTEST / total / PASS / 46 tests
GATE | total | FAIL | 6 failed (34 checks)
```

### p6_gate.sh

```
GATE | hygiene-paths | FAIL | 91 changed path(s) outside the allowed set
GATE | hygiene-emdash | PASS | no em-dash in added lines
GATE | hygiene-artifacts | PASS | no tracked build output, key or local file
GATE | hygiene-subjects | PASS | 49 commit subject(s) are Conventional Commits
GATE | hygiene-root-lock | PASS | Cargo.lock and Cargo.toml unchanged
GATE | frozen-provisional | SKIP | retired: tag linux-v1.0.0-rc2 exists (PD100)
GATE | linux-frozen | FAIL | x frozen paths differ from linux-v1.0.0-rc2
GATE | rust-fmt | PASS | ok
GATE | rust-clippy | PASS | Finished `dev` profile [unoptimized + debuginfo] target(s) in 58.10s
GATE | rust-tests | PASS | 261 passed
GATE | ffi-deps | PASS | pos-engine yes, pos-peppol no, tauri no
GATE | lock-parity | PASS | lock parity: 341 packages, identical
GATE | command-parity | PASS | missing: none
GATE | ffi-hygiene | PASS | hygiene: ok (115 exports, 126 types)
GATE | selftest-frozen | PASS | PASS: all cases
GATE | selftest-lock-parity | PASS | OK
GATE | selftest-command-parity | PASS | OK (skipped=1)
GATE | jvm-tests | PASS | 20 actionable tasks: 4 executed, 16 up-to-date
GATE | selftest-check-apk | PASS | TEST / summary / PASS / 12 passed, 0 failed, 0 skipped
GATE | selftest-p3-gate | PASS | TEST / summary / PASS / 17 passed, 0 failed
GATE | gradle-lint | PASS | 103 actionable tasks: 13 executed, 90 up-to-date
GATE | gradle-ktlint | PASS | 23 actionable tasks: 23 up-to-date
GATE | gradle-detekt | PASS | 3 actionable tasks: 3 up-to-date
GATE | unit-tests | PASS | 22 tests
GATE | mismatch-test | PASS | 22 actionable tasks: 2 executed, 20 up-to-date
GATE | api-check | FAIL | 12 actionable tasks: 2 executed, 10 up-to-date
GATE | aar-check | PASS | aar ok: jni/arm64-v8a/libpos_ffi.so only, 4 consumer rules
GATE | assemble-release | PASS | 85 actionable tasks: 12 executed, 73 up-to-date
GATE | apk-check | PASS | 6 checks passed, 9123caa967df
GATE | freeze-tag | FAIL | x frozen paths differ from linux-v1.0.0-rc2
GATE | hygiene-keys | PASS | no secret key file under android/, scripts/android/ or docs/android/
GATE | lanes | PASS | LANES / total / PASS / 4508 files, 90 rules, 91 probes
GATE | vue-strings | PASS | VUE / total / PASS / 630 keys (core 254, errors 140, checkout 40, reports 45, settings 27, webshop 124), fdm 49, error codes 115 + 23
GATE | kotlin-tests | FAIL | base 2104, head 2211, removed 23
GATE | ffi-boundary | PASS | no native entry point outside engine/ and provisioning/
GATE | app-manifest | PASS | manifest ok: 6 permissions, exported components restricted
GATE | selftest-p4-gate | PASS | TEST / summary / PASS / 79 passed, 0 failed
GATE | selftest-heavy | PASS | TEST / summary / PASS / 11 passed, 0 failed
GATE | selftest-kotlin-tests | PASS | OK
GATE | selftest-lanes | PASS | OK
GATE | selftest-vue-strings | PASS | ℹ duration_ms 4017.384125
GATE | app-lint | PASS | 74 actionable tasks: 1 executed, 73 up-to-date
GATE | app-ktlint | PASS | 11 actionable tasks: 11 up-to-date
GATE | app-detekt | PASS | 1 actionable task: 1 up-to-date
GATE | app-unit-tests | PASS | 2164 tests
GATE | app-screenshots | PASS | 59 actionable tasks: 2 executed, 57 up-to-date
GATE | app-kover | PASS | LINE 97.4%, BRANCH 74.8%
GATE | app-assemble-release | PASS | 90 actionable tasks: 1 executed, 89 up-to-date
GATE | app-apk-check | PASS | 6 checks passed, e1a2b99e495f
GATE | plat-manifest | PASS | backup off, no installLocation, 6 permissions, none for storage or install
GATE | plat-backup-rules | PASS | both sections exclude root, file, database, sharedpref, external; no include
GATE | plat-strings | PASS | 3 file(s), 114 names, same keys in fr, en and zh
GATE | plat-boundary | PASS | 56 source file(s) under platform/, no engine internals, no force-stop, no plain http
GATE | plat-keys | PASS | debug key equals dev-minisign.pub, release key empty
GATE | plat-engine-ref | PASS | pili-pos 1.0.0, linux-v1.0.0-rc2 (922a158)
GATE | selftest-p6-gate | PASS | TESTP6 / total / PASS / 110 tests
GATE | selftest-declare | FAIL | TESTDECLARE / total / FAIL / 1 of 42 cases failed
GATE | total | FAIL | 6 failed (57 checks)
```

## 4. Fixture deviations per UC (`crates/pos-engine/tests/spf_e2e_fixtures.rs`)

Every listed deviation is asserted both ways (an unlisted difference fails, and a listed one that no longer occurs fails).

| UC | Deviations |
|---|---|
| M122 (table transfer) | none |
| M132 (payment correction) | one listed: the new card line is `CARD_DEBIT` against the fixture `CARD_CREDIT` (no linked terminal, the operator cannot tell debit from credit; V1F-D34). Declared, not compared: the comparator checks only type, amount and amountType of each payment line, so the new card line is `MANUAL` where the fixture has `AUTOMATIC`, and the cash line has no drawer (optional field). Stated in `RESULTATS_TESTS_UC_FR.md` (M132 section) |
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

## 8. Review round 2 fixes

| # | Finding | Result |
|---|---|---|
| 1 | Late TableTransfer commit could put two open orders on one table | Fixed (V1F-D60): the order stays on its source table, retry returns `table_transfer_not_moved` (fr, en, zh on both UIs), no moved notice printed. Tests `a_late_answer_never_puts_two_open_orders_on_one_table` and `the_background_tick_keeps_the_order_put_when_the_target_was_taken` (LossyFdm) |
| 2 | Training refund missed by `referencing` | Already closed by V1F-D59 (refunds found by FDM `eventOperation`, not label); recorded V1F-D61, test `training_events_are_told_apart_by_their_fdm_operation` |
| 3 | 0,00 EUR sale payload | FAQ Q206 b allows `PAY 0` CASH 0 or an empty array; the engine now sends the empty array (V1F-D62), fixtures and tests aligned |
| 4 | Discount button on an online order (Vue) | Fixed on both UIs (V1F-D64): `CartLine.online`, Playwright and Kotlin tests. Note: Android did not hide it either, contrary to the finding; both now do |
| 5 | Dossier truth | Refund lost-answer sentence added to both manuals (the 9 bis text and FLUX_REMBOURSEMENT already told the pending request); REPONSES row XV already ticked for N Refund and P; evidence counts recomputed for the regenerated set (21 UC, 55 pairs, 6 Advanced UC with 10 pairs; the current `evidence/uc` is stale until F1); LISTE_FONCTIONNALITES and DECLARATION_FONCTIONS_INTERDITES already correct; M132 deviation declared; `uc_scope.toml` reasons of UC220, UC221A, UC221B, UC4116, UC4118 reworded; V1F-D28 aligned to the slip (V1F-D63); `UC_MATRIX_ANDROID.md` states that the tests added by the lane have not run on the device; PDFs of MANUEL_UTILISATEUR, REPONSES_MANUEL_CERT, RESULTATS_TESTS_UC rebuilt in both dossiers |
| 6 | CLAUDE.md transfer entry | Entry existed; now mentions the late-answer rule and the slip content |
| 7 | list_sale_payments per ticket row | No batch without a new IPC command: recorded as a known limit with numbers (V1F-D65) |
| 8 | Exit report | This file |

