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
| next | `SettingsRouteTest` stops the Peppol poll of the manager route it draws (INT-D11) |

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

Filled in after the full gate run.
