# Merge of lane 4 (android/lane-v12) into android/int (2026-10-17)

Integrator report. Lanes 1, 2, 3 and 5 were already on `android/int` (head `1ac3eae7`). Lane 4 (accountant export,
owner alerts, allergens, courses, gift vouchers) is merged with `git merge --no-ff android/lane-v12` as `612c9ba1`.
Nothing was pushed, `main` was not touched, no tag was set. Decisions: `docs/android/DECISIONS.md`, second
"Lane int (2026-10-17)" section, INT-D12 to INT-D23.

## Plan (section 13 of `LANE_V12.md`, mandatory interactions)

| Interaction | Where | Test |
| --- | --- | --- |
| Allergen editor in the menu editor, both UIs (INT-D20) | `MenuEditorView.vue`, `MenuEditorKitchenFields.kt` | `menu-editor.spec.ts`, `MenuEditorKitchenTest`, goldens `menu_editor_product_allergens`, engine `lane_interactions.rs` |
| Course picker 0 to 3 in the category panel (INT-D20) | same | same, golden `menu_editor_category` |
| `pili:%` system rows hidden from the editors (INT-D17) | `pos-db` `menu_admin.rs`, `allergens.rs` | `lane_interactions.rs` |
| Sales report, Z and X with VAT X voucher lines and `VOUCHER_STORE` (INT-D21) | engine | `lane_interactions.rs`, lane 4 Z/X tests |
| Invoice of a voucher sale refused (INT-D18) | `invoice.rs` | `lane_interactions.rs` |
| Discounts and voucher lines (INT-D13) | `discount.rs`, both UIs | `lane_interactions.rs`, `till-discount.spec.ts`, `VoucherLineActionsTest` |
| M132 refuses a `VOUCHER_STORE` line (INT-D19) | `payment_correction.rs`, both UIs | `lane_interactions.rs`, `payment-correction.spec.ts`, `ReportsViewModelTest`, `ReportsContentTest` |
| Kitchen notes with vouchers and courses (INT-D15) | `courses.rs` | `lane_interactions.rs` |
| PRICE_CHANGE never held or printed (INT-D14) | `courses.rs` | `lane_interactions.rs` |
| Table transfer carries no course data | `table_transfer.rs` | `lane_interactions.rs` (request has no course, held course kept, release names the target table) |
| Goldens re-recorded and reviewed (INT-D23) | Robolectric | 2 new (menu editor), 18 re-recorded |

## Commits

| Commit | Content |
| --- | --- |
| `612c9ba1` | Merge commit, conflicts resolved by keeping both sides (INT-D12) |
| `fd760a2e` | System rows out of the menu and allergen editors (INT-D17) |
| `d56affdb` | PRICE_CHANGE entries never held behind a course (INT-D14) |
| `995d380d` | Gift vouchers out of invoices and payment corrections (INT-D18, INT-D19) |
| `c377c22c` | `lane_interactions.rs`, 16 cross-lane tests |
| `3a00344a` | `MenuAdmin.allergens` for the editor (INT-D20) |
| `a9ac7432` | Allergens and course in the menu editor, both UIs (INT-D20) |
| `3f7e0706` | Voucher lines in the discount, note and correction flows, both UIs (INT-D13, INT-D19) |
| `47bdaf07` | Schema documents and engine command map (INT-D22) |
| `0f9c8dc1` | `SettingsRouteTest` scrolls to the badge nodes, 18 goldens re-recorded (INT-D23) |
| `28cc209c` | Decisions INT-D12 to INT-D23, code references renumbered, this plan |
| `5773c754` | Vue manager panels of the employees column capped at 42 % (INT-D24) |
| `9a464cf7` | clippy `cloned_ref_to_slice_refs` in `lane_interactions.rs` |
| `1a8807c2` | ktlint and detekt after the merge (`MenuViewModel`, `MenuEditorViewModel`, two composables) |
| next | This gate section |

## Captures to retake

No emulator and no kiosk in this lane: the Robolectric goldens are renders, not device captures. Both UIs (Vue on
the Linux kiosk, Compose on the D3 Pro), French, then zh where the runbook asks for it:

1. Menu editor: a category with the course picker (courses on), a product and an option with the allergen grid
   (declared, contains, may contain) and the "information non disponible" state.
2. Order screen: the "Allergènes" toggle and legend next to the "Épuisé" bar, the "Bon cadeau" button, a cart with a
   voucher line (badge, no "Offert" and no "Note") and a course chip.
3. Checkout: a sale paid with a gift voucher (code entry, balance) and a voucher sale.
4. Reports: the "Bons cadeaux" tab next to "Ventes"; the sales report with the "Bons cadeaux" department; the
   payment correction dialog with a locked voucher line and its note.
5. Settings, manager view: phone alerts, "Service en plusieurs temps" and "Bons cadeaux" panels beside the badge
   reader and the employees panel with the presence-only buttons.
6. Tables: a table with a held course ("Suite : Plats") and the "Déplacer une table" button; the release slip after
   a transfer names the target table.
7. Every capture listed in `LANE_V12.md` section 11, now on the merged build.

## Gate

Run on `1a8807c2` with a private `CARGO_TARGET_DIR` (`target/gate`), rustup toolchain first on `PATH` (Gradle too,
INT-D23), every heavy command through `scripts/android/heavy.sh`.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` (root and `android/rust`) | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (after `9a464cf7`) |
| `cargo clippy -p pili-pos -p pos-engine --no-default-features --features cert --all-targets -- -D warnings` | PASS (after `9a464cf7`) |
| `cargo test --workspace` | 2555 passed, 0 failed, 13 ignored |
| `cargo test -p pili-pos --no-default-features --features cert` | 14 passed |
| `cargo test -p pos-engine --no-default-features --features cert` | 1155 passed, 0 failed, 11 ignored |
| FFI tests (`android/rust`, `cargo test`) | 315 passed |
| `check_ipc_wrappers.py` / `check_command_parity.py` | 148/148 / 148/148 |
| `check_ui_parity.py` | PASS, 146 commands (called 136, excluded 10) |
| `check_lock_parity.sh` | 343 packages, identical |
| `ui`: `npm run build` / `npx playwright test` | PASS / 711 passed (first run 710 + 1 failed, fixed by INT-D24) |
| `p5_gate.sh` | 34 checks, 2 failed: linux-frozen, freeze-tag (allowed) |
| `p6_gate.sh` | 57 checks, 2 failed: linux-frozen, freeze-tag (allowed) |
| `check_dossier.py`, Linux and Android (`PILI_DOSSIER_DIR`) | PASS, no finding |

In both gates `:app` unit tests 2806, kover LINE 97.8 %, BRANCH 74.7 %, kotlin-tests removes nothing (2851 to
2851). The first p5 run also failed app-ktlint and app-detekt on merged code; fixed in `1a8807c2`.
`lane_interactions.rs` rerun after the clippy fix: 16 passed.
