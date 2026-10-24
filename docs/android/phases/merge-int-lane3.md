# Merge of lane 3 (android/lane-v11) into android/int (2026-10-17)

Integrator report. Lanes 1, 2 and 5 were already on `android/int` (head `a882bf18`). Lane 3 (invoices and
Peppol, sold-out, menu editor, sales reports) is merged with `git merge --no-ff android/lane-v11` as `cb9d39a1`.
Nothing was pushed, `main` was not touched, no tag was set. Decisions: `docs/android/DECISIONS.md`,
"Lane int (2026-10-17)", INT-D1 to INT-D11.

## Commits

| Commit | Content |
| --- | --- |
| `cb9d39a1` | Merge commit, every conflict resolved by keeping both sides (INT-D1 to INT-D4, INT-D8) |
| `6dbbc963` | `pos-core.api` regenerated with `:pos-core:apiDump` |
| `209aa15f` | Settings route test: scroll to the Peppol panel, stop the polling in a `finally` (INT-D9) |
| `fd369b5b` | Goldens re-recorded: `reports_tickets_manager`, `reports_payment_correction`, `settings_peppol` |
| `ed4780c7` | UC evidence recaptured on the merged code, M122 and M132 embedded (INT-D5) |
| `7145c11b` | Dossier reconciliation of both dossiers (INT-D6, INT-D7), matrices, PDFs, decisions, this plan |
| `cd4052c6` | Vue settings columns 1 and 3 scroll as a whole: the merged panels overlapped (INT-D10) |
| `3bd92f88` | `SettingsRouteTest` stops the Peppol poll of the manager route it draws (INT-D11) |
| next | This gate section |

## Conflicts

- Rust: `pos-db` migrations (till 30, 31 and invoicing 40 to 42 both registered), `signing.rs` (payment
  correction and invoice arms), `engine.rs` and `pos-ffi/src/core.rs` (both timers), `report_totals.rs`,
  `error_codes.rs`, the FFI orders, reports and records modules.
- Tauri: one handler list, 128 wrappers.
- Vue: `ipc.ts`, `i18n-reports.ts`, `SettingsView.vue`, `ReportsView.vue` (invoice tick box and payment
  correction on the same tickets row).
- Android: order feature (both dialog sets), reports feature (correction and invoice callbacks, the tickets row),
  generated strings (894 keys), `pos-core.api`.
- Docs: `DECISIONS.md` (lane 1, lane 2, lane 5, lane 3), `parity/order.md`, `CLAUDE.md`, both dossiers.

## Captures to retake

No emulator and no kiosk in this lane: the Robolectric goldens under `android/app/src/test/screenshots/` are
renders, not device captures. Both UIs (Vue on the Linux kiosk, Compose on the D3 Pro), French, then zh where the
runbook asks for it:

1. Reports, Tickets tab: the row with the invoice tick box and the "Corriger le paiement" slot together, the
   "Facture" button and the "Ventes" tab in the tab bar (the existing captures of this tab changed on both UIs).
2. Reports, Tickets tab, payment correction dialog over the merged tab bar.
3. Settings, manager view: the third column now holds the badge reader panel above Informations, Intégrations,
   Peppol and Tickets par email; the employees panel shows the presence-only buttons ("Arrivée", "Présence seule").
   Retake the settings captures, including the Peppol panel with a parked delivery and "Réessayer".
   On the Linux kiosk, columns 1 and 3 of the settings screen now scroll as a whole (INT-D10): retake every
   settings capture of the Vue UI, top and scrolled to the bottom of each column.
4. Every capture listed in `LANE_V11.md` section 12 (invoice flow, sold-out, menu editor, sales report) and in
   `LANE_NFC.md` (badge panel, enrolment, login by badge), now on the merged build.

## Gate

Run on `3bd92f88` with a private `CARGO_TARGET_DIR` (`target/gate`), rustup toolchain 1.99.0, every heavy
command through `scripts/android/heavy.sh`.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` (root and `android/rust`) | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo clippy -p pili-pos -p pos-engine --no-default-features --features cert --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | 2234 passed, 0 failed, 13 ignored |
| `cargo test -p pili-pos --no-default-features --features cert` | 14 passed |
| `cargo test -p pos-engine --no-default-features --features cert` | 966 passed, 0 failed, 11 ignored |
| FFI tests (`android/rust`, `cargo test`) | 293 passed |
| `check_ipc_wrappers.py` / `check_command_parity.py` | 128/128 / 128/128 |
| `check_ui_parity.py` | PASS, 126 commands (called 117, excluded 9) |
| `check_lock_parity.sh` | 343 packages, identical |
| `ui`: `npm run build` / `npx playwright test` | PASS / 624 passed |
| `p5_gate.sh` | 34 checks, 2 failed: linux-frozen, freeze-tag (allowed) |
| `p6_gate.sh` | 57 checks, 2 failed: linux-frozen, freeze-tag (allowed) |
| `check_dossier.py`, Linux and Android (`PILI_DOSSIER_DIR`) | PASS, no finding |

In both gates kotlin-tests, hygiene-root-lock and hygiene-paths pass; `:app` unit tests 2577, kover LINE 97.7 %,
BRANCH 74.9 %. The Kotlin test inventory against each parent removes nothing (`a882bf18`: 2420 to 2624,
`android/lane-v11`: 2285 to 2624). `SettingsRouteSignedInTest` and `PeppolScreenshotTest` passed three forced
reruns in a row.

`check_dossier.py --final --rules manifest` on the Android dossier reports the six `commun:pdf_out/...` entries of
`MANIFEST.txt` as missing: the rule does not resolve the `commun:` prefix, the six PDFs exist in
`docs/SUBMISSION_ARTIFACTS/pdf_out/`, and the same entries are on `a882bf18`. Not a merge finding.
