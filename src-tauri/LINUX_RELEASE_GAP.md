# Linux release gap

Internal note (English). It lists what stands between the repository and a
certification filing of the Linux product `pili-pos` (Tauri shell `src-tauri/`,
Vue UI `ui/`, `deploy/`, dossier `docs/SUBMISSION_ARTIFACTS/`), measured against
the head of branch `android/int` (the integration branch, freeze commit
`8c53acc3` plus the freeze record `28920b2a`). Every figure below was read from
git or from a script run on that head on 2026-10-18; re-run the commands in the
last section before relying on a number.

## 1. Status

- **Decision (user, 2026-10-18, INT-D41, `docs/android/DECISIONS.md`, section
  "Lane rc3"):** the v1.0.0 certification submits only `pili-pos-android` 1.0.0
  (dossier `docs/android/dossier`). The Linux product stays in the repository,
  keeps building and passing, and is not submitted. Nothing in the Android
  dossier depends on it any more (INT-D42, INT-D43).
- **Submitted product:** Android, freeze tag `android-v1.0.0-rc3` (local,
  never pushed) on `8c53acc3c7a0b2424b6cabb15e613e0d43753663`; record in
  [`docs/android/ANDROID_FREEZE.md`](../docs/android/ANDROID_FREEZE.md).
- **Last Linux freeze:** local annotated tag `linux-v1.0.0-rc2`, tag object
  `3f9908d9`, record in [`docs/android/LINUX_FREEZE.md`](../docs/android/LINUX_FREEZE.md)
  (built commit `f0c3078d`, reproducible SHA-1
  `ed9489009301cf909d60bbef46d7348da0be9323`, set 2026-10-15).
- **What rc2 does not contain:** it predates all five lanes. Compared with the
  rc2 tag, `android/int` changes 294 files under the Linux frozen paths
  (`git diff --stat linux-v1.0.0-rc2 HEAD -- Cargo.toml Cargo.lock rust-toolchain.toml src-tauri ui deploy/build-release.sh scripts/build_and_hash.sh crates`:
  294 files, 65823 insertions) and `scripts/android/check_linux_frozen.sh`
  reports "frozen paths differ from linux-v1.0.0-rc2". The engine crates that
  changed are pos-accounting, pos-catalog, pos-core, pos-db, pos-eft,
  pos-engine, pos-engine-testkit, pos-export, pos-fdm-mock, pos-fiscal,
  pos-hubrise, pos-peppol and pos-print. In feature terms the rc2 binary lacks:
  till discounts, payment correction (M132), table transfer (M122), manager
  presence registration, B2B invoices with the Peppol queue, sold-out products,
  kitchen notes, the menu editor, the non-fiscal sales report, the accountant
  export tool, owner alerts, allergens, courses, multi-purpose gift vouchers,
  staff badges, meal-voucher cards on the linked terminal, and every rc3 fix
  listed in section 3. **The rc2 binary is
  therefore not the Linux product as it stands in the repository; a new Linux
  freeze is mandatory.**
- The Linux version in `Cargo.toml`, `src-tauri/tauri.conf.json` and
  `ui/package.json` is still `1.0.0`; the Linux `posSwVersion` is
  `pili-pos/1.0.0` (`src-tauri/src/main.rs`, from `CARGO_PKG_VERSION`).

## 2. Changes made for the Android-only submission that Linux would mirror or undo

Commits `980dcbd6..HEAD` (17 commits). `980dcbd6` itself is only an About
golden re-record. Decisions: INT-D41 to INT-D50.

### 2.1 Dossier documents absorbed into the Android dossier

- Commit `7a7b4f95` (+ `e963b857`, `db8ca994`): the six engine documents
  `SCHEMA_BD_FR`, `SCHEMA_ER_FR`, `STRATEGIE_COMPTEURS_FR`, `COUVERTURE_FIXTURES_FR`,
  `COUVERTURE_RAPPORTS_FR`, `TICKET_NUMERIQUE_FR` were copied from
  `docs/SUBMISSION_ARTIFACTS/` into `docs/android/dossier/` and adapted to
  Android (annex numbers 11, 13, 17, 24, 25, 32 unchanged; MANIFEST lost the
  `commun:` prefix). The FDM error JSON (`evidence/fdm_errors`) and
  `pdf_header.tex` were copied too (INT-D42).
- Consequence for Linux: the Android copies are now the maintained ones where
  they differ (Android wording). The Linux copies are not behind on engine
  facts (they were regenerated on the merged code by `07b481b7`, `4a1ba557`,
  `47bdaf07`), but they carry no Android-only edits. Nothing to undo; the Linux
  dossier is still self-contained. Provenance comments on line 1 of the Android
  files (`<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/<NAME>.md @ 86088e4 -->`) are
  history only (`docs/android/DOSSIER_EVIDENCE.md`, "Linux source (history)").
- The Linux dossier received one change: a comment note in
  `docs/SUBMISSION_ARTIFACTS/MANIFEST.txt` saying that v1.0.0 submits Android only
  (commit `ccc7441c`). **Undo that note when filing Linux.** No Linux PDF was
  rebuilt for it.
- The Android request framing (`CADRE_DEMANDE_FR.md`, `INDEX_DOSSIER_FR.md`,
  `REPONSES_MANUEL_CERT_FR.md`, `RAPPORT_TESTS_BASELINE_FR.md`, commit
  `1595f448`) was rewritten as one initial demand for `pili-pos-android`, with
  no "commun" or Linux sibling. The two-filing Art. 99 framing only ever lived
  in the Android dossier (`git show 1595f448^:docs/android/dossier/CADRE_DEMANDE_FR.md`).
  The Linux dossier has no request framing or index document:
  `CADRE_DEMANDE_FR.md` and `INDEX_DOSSIER_FR.md` do not exist in
  `docs/SUBMISSION_ARTIFACTS/` and never did, and its
  `REPONSES_MANUEL_CERT_FR.md` mentions neither Art. 99 nor Android. A Linux
  filing must create both documents (and list them in `MANIFEST.txt`), possibly
  starting from the pre-`1595f448` Android text (see section 6, step 8).

### 2.2 Evidence items removed or redefined

- **EV-07** (Android versus Linux journal and FDM request comparison) is retired
  and its id never reused (INT-D43). The `p7_gate.sh` journal comparison still
  calls `compare --linux target/p7-gate/linux --android ...`; to restore the
  evidence run it again and give the Linux dossier its own EV row.
- **EV-04** was redefined as the commit of the Android freeze tag, with no
  Linux binary SHA-1 (INT-D43, INT-D47). It stays "À effectuer" until the final
  `android-v1.0.0`. A Linux filing needs its own EV-04 equivalent: the SHA-1
  from `scripts/check_repro.sh` on the Linux freeze (rc2 value
  `ed9489009301cf909d60bbef46d7348da0be9323` is obsolete).
- The Linux IPC command parity and Linux UI parity are no longer certification
  arguments (INT-D43); the scripts stay as internal checks and are green
  (`check_ipc_wrappers.py`: 148/148, `check_command_parity.py --strict`: 148
  commands, `check_ui_parity.py --strict`: called 136, excluded 10, pending 0).
- Host engine runs (cargo test, `pos-uc-check` replay) are described in the
  Android dossier as "engine tests on the development host"; the 27/09/2026
  module2 run under the Linux product identity is no longer cited by the
  Android dossier (the Linux dossier still cites it, see section 5); the
  Android live run is EV-14. ROUND1-5 JSON became EV-06 (INT-D43). The Linux dossier
  was not edited for any of these, so it still describes its own evidence.

### 2.3 Gate checks retargeted or retired

| Script | Before | Now | Restore for Linux |
|--------|--------|-----|--------------------|
| `scripts/android/p2_gate.sh` | `linux-frozen` check (`check_linux_frozen.sh`) | SKIP "retired", plus `android-frozen` | run `bash scripts/android/check_linux_frozen.sh --tag <new linux tag>` by hand, or re-enable the gate line |
| `scripts/android/p4_gate.sh` | `freeze-tag` checks the Linux tag; `--linux-repro` | `freeze-tag` retargeted to `android-v1.0.0-rc3`; `--linux-repro` kept as history with `LINUX_REPRO_TAG=linux-v1.0.0-rc2` | change `LINUX_REPRO_TAG` to the new tag |
| `scripts/android/p6_gate.sh` | `plat-engine-ref` pinned to the Linux freeze tag (`PILI_P6_FREEZE_TAG`) | default `android-v1.0.0-rc3`, About string must equal `pili-pos <V>, <TAG>` | not needed for Linux (Linux has no About engine reference) |
| `scripts/android/p7_gate.sh` | `linux-frozen` | `android-frozen` | as p2 |
| `scripts/android/check_android_frozen.sh` | did not exist | new (commit `ac06ab21`), guards the APK inputs (23 paths) | n/a |
| `scripts/android/check_linux_frozen.sh` | active guard | kept, unchanged, currently FAILS against rc2 | see below |

How to restore a Linux freeze check: after the new freeze (section 6), set the
default tag of `check_linux_frozen.sh` (it defaults to `linux-v1.0.0` when that
tag exists, else `linux-v1.0.0-rc2`) or pass `--tag`, then put its call back in
`p2_gate.sh` (replace the `skip linux-frozen "retired ..."` line, around line
302) and `p7_gate.sh`. Its frozen path list (19 paths) is printed by the script;
it includes `Cargo.lock`, `src-tauri`, `ui`, `deploy/build-release.sh`,
`scripts/build_and_hash.sh` and every path crate of the cert build graph.

### 2.4 About strings, CLAUDE.md and docs

- `plat_about_engine_ref` (`android/app/src/main/res/values/strings_plat_about.xml`)
  is `pili-pos 1.0.0, android-v1.0.0-rc3` (no commit part, INT-D45). Android only.
- `CLAUDE.md` gained the "Certification v1" paragraph at the top (commit
  `d8e614f0`). When Linux is filed, reword it.
- Freeze docs updated: `docs/android/ANDROID_FREEZE.md` (new), `LINUX_FREEZE.md`
  (history line added), `APP.md`, `BUILD.md`, `LANES.md`, `PLATFORM.md`,
  `REPRO.md`, `DOSSIER_EVIDENCE.md`.
- `scripts/check_dossier.py` accepts letter-suffixed capture names such as
  `A127b` (INT-D48, commit `459f463d`). Linux captures have no such names; no effect.

## 3. Android work since rc2: what the Linux UI/shell has and what its dossier shows

Method: Vue components and views in `ui/src` (the files exist, 14 to 30 files
mention each feature), Playwright specs in `ui/e2e`, `check_ui_parity.py --strict`
(only 10 engine commands have no caller on Android, none of them a lane feature
except `reprint_course_slip`), and the feature annexes and captures under
`docs/SUBMISSION_ARTIFACTS`. I did not re-run the Vue build or Playwright for
this note.

| Feature (lane) | Linux UI and shell | Linux dossier |
|----------------|--------------------|---------------|
| Till discounts, payment correction M132, table transfer M122, presence registration, B2B invoices + Peppol (lane 1-2, 3-v11) | present (`DiscountDialog`, `PaymentCorrectionDialog`, `TablePickerDialog`, `InvoiceDialog`, `InvoiceList`, `PeppolPanel`, e2e specs `till-discount`, `payment-correction`, `table-transfer`, `invoice`, `peppol`); Tauri wrappers in `src-tauri/src/ipc` (148/148) | described (`FACTURATION_PEPPOL_FR.md`, `FLUX_REMBOURSEMENT_FR.md`, etc.), folded by `7145c11b`, `4a1ba557`, `484cb0ed` |
| Sold-out, kitchen notes, menu editor, non-fiscal sales report (lane 3-v11) | present (`SoldOutDialog`, `NoteDialog`, `MenuEditorView`, `MenuEditorPanel`, `SalesReportPanel`) | `PRODUIT_EPUISE_FR.md`, `EDITEUR_CARTE_FR.md`, `RAPPORTS_VENTES_FR.md`; Linux capture `80_reports_sales.png` is a mock-data screen |
| Owner alerts, allergens, courses, gift vouchers (lane 4-v12) | present (`OwnerAlertsSettings`, `AllergenBadges`, `AllergenSheetDialog`, `CoursePickerDialog`, `CoursesSettings`, `GiftVoucher*`) | annexes `ALERTES_PROPRIETAIRE_FR`, `ALLERGENES_FR`, `SERVICES_CUISINE_FR`, `BONS_CADEAUX_FR`; captures `81` to `92` in `docs/SUBMISSION_ARTIFACTS/screenshots` |
| Meal-voucher cards on the linked terminal (lane 1-v1-fixes item B, L1F-B1 to B7): an Edenred, Pluxee, Monizze or Payflip brand is signed as `CHEQUE_MEAL` `AUTOMATIC`, shown on the ticket and its copies, labelled on both checkouts | present, in the cert binary (`ui/e2e/eft.spec.ts`) | capture `09l_eft_meal_voucher.png` (retaken by `a595579b`, picked up by glob in `make_captures_pdf.sh`); covered in `MANUEL_UTILISATEUR_FR.md` (lines 601-605, capture 09l at line 562), `LISTE_FONCTIONNALITES_FR.md` (line 165) and the 'Chèque repas' checklist row of `REPONSES_MANUEL_CERT_FR.md` (line 515). `TICKETS_EXEMPLES_FR.md` only shows the manual 'Chèques-repas' voucher (ROUND5 section 10), with no brand: the Linux dossier has no ticket example of a branded terminal meal voucher, so a Linux filing may need to add one through `scripts/generate_ticket_examples.sh` |
| Accountant export (lane 4 item A, `crates/pos-accounting`) | CLI outside the binary and outside the frozen app inputs; no Vue or Tauri surface, nothing to port | not in the Linux dossier; a Linux filing decides whether to cite it (it is not part of the certified binary) |
| Nightly export (lane X1, `crates/pos-export`) | present (`ExportStatusPanel` in Settings, managers only) | annex `EXPORT_NOCTURNE_FR` (annex 42) and its mentions in the manuals, data flows, data sheet (section 8 added), security, GDPR profile and certification answers, merged from PR #5 (branch `android/X1-X1-T10`) onto the current Linux dossier; no Linux capture of the export panel yet |
| Staff badges (lane 5-nfc) | USB wedge reader only (`BadgeSettings`, `BadgeEnrolDialog`, `ui/src/utils/badgeWedge.ts`); a stored `nfc` reader is not a reader on Linux (commit `a2b0d156`). NFC is Android only (section 4) | covered inside the user manual and annexes; no dedicated Linux annex was checked |

rc3 fixes:

| Fix | Layer | On Linux | Linux dossier |
|-----|-------|----------|---------------|
| Badge key creation race (`7f96ae02`) | engine (`badge_key.rs`) | yes, shared engine | not Linux-specific; engine tests only |
| PIN-pad refusal text (`a9a50108`) | Android `ReportsViewModel` only | not needed; the Vue payment-correction dialog maps engine errors through `ui/src/i18n.ts` (`err_payment_correction_method`) | n/a |
| Menu editor accepts stored names in languages the till does not offer (`94131505`) | engine | yes | nothing to change |
| Sales report "dont remboursements" counts refund tickets only (`49d07289`, INT-D38) | engine; both UIs only display the figure | yes | `RAPPORTS_VENTES_FR.md` already states the rule (lines 41 and 119) |
| "Chèques-repas" rename (`a595579b`, INT-D30) | engine labels, both UIs, ticket label | yes (`ui/src/i18n-checkout.ts`, `i18n-reports.ts`, `i18n.ts`) | 15 Linux annexes already say "chèques-repas"; the quoted manual checklist row keeps the SPF wording. `a595579b` hand-edited 3 Linux ticket evidence files (`ROUND4_ticket.txt`, `ROUND5_ticket.txt`, `ticket_N_paiement_mixte.txt` under `docs/SUBMISSION_ARTIFACTS/evidence/tickets`): they now read "Chèques-repas" but are not output of `scripts/generate_ticket_examples.sh`. INT-D33 reverted the same kind of hand edit on Android as wrong; the Linux ones were never reverted. Remaining action: regenerate those 3 files with the generator |
| Gift voucher line takes no kitchen course or note (INT-D29) | engine | yes (`err_gift_voucher_line_no_kitchen` in `ui/src/i18n.ts`) | `BONS_CADEAUX_FR.md` lines 27 and 152 |
| Dev minisign key rotation (`920a19ec`, INT-D37) | Android debug provisioning only (`android/app/provisioning/dev-minisign.pub`) | not applicable | n/a |

### Linux dossier check output (this head)

- `python3 scripts/check_dossier.py` exits 0 with no finding.
- `python3 scripts/check_dossier.py --final` exits 1 with 46 findings:
  - 38 `[pending]`: "A effectuer" left in `docs/SUBMISSION_ARTIFACTS/JOURNAL_TESTS_PRE_SOUMISSION_FR.md`
    (lines 88 to 180; the pre-submission test journal template, to be filled on
    the hardware);
  - 1 `[pdf-freshness]`: `pdf_out/MATRICE_TRACABILITE_FR.pdf` is older than its
    markdown;
  - 7 `[screenshots]`: `docs/internal/CAPTURES_ECRAN_RUNBOOK_FR.md` lines 651 to
    669 cite `50_ticket_n.jpg` to `56_report_z.jpg`, absent on disk (the printed
    ticket and report photos of the real printer).
- `docs/SUBMISSION_ARTIFACTS` has 103 screenshot files; they are the host
  Playwright captures retaken by `bf7d83b1` and rebuilt into the PDFs by
  `9671ac30`. They do not predate the label change: `a595579b` retook 7 Linux captures
  (`07`, `08`, `09h`, `09l`, `62`, `88`, `90`) and rebuilt
  `CAPTURES_ECRAN_FR.pdf` and every Linux PDF. They still predate the other rc3
  fixes, which change no screen.
- Open "A effectuer" markers elsewhere: `docs/internal/DOSSIER_SOUMISSION_FR.md`
  (signed request form, "Phase 12"; source archive of the cert tag).

## 4. Android-only features and their Linux equivalent

| Android only | Where | Linux equivalent |
|--------------|-------|------------------|
| Built-in NFC badge reader | `android/app/.../ui/badge/NfcBadgeSource.kt`, `docs/android/PARITY.md` line 86 | USB HID wedge reader (types the UID then Enter), already in `ui/src/utils/badgeWedge.ts`; the engine does not know the hardware |
| Device identity, provisioning bundles signed with minisign (dev key rotation) | `android/app/.../provisioning`, `android/app/provisioning/` | Linux configuration: `/etc/pili-pos/pos.json` and `PILI_POS_CONFIG`; posId in config, provisioning by `deploy/provision/provision.sh` |
| Lock Task kiosk, user activity forwarder | `android/app/.../kiosk` | Cage Wayland kiosk, `deploy/kiosk/`, `deploy/pili-pos.service` |
| Updater (signed update bundles, Play or in-app) | `android/app/.../platform`, `docs/android/PLATFORM.md` | `deploy/update.sh`, `deploy/UPDATE-STRATEGY.md` |
| Nightly export worker on Android | `android/app/.../nightly` | engine background export, same engine |
| UniFFI facade | `android/rust/pos-ffi` | Tauri wrappers in `src-tauri/src/ipc` |
| About engine reference | `strings_plat_about.xml` | none; Linux shows `posSwVersion` on the login screen (`pili-pos/<version>`) |
| Engine commands without a Vue/Android caller (parity exclusions) | `check_ui_parity.py`: `check_fdm_status`, `list_fiscal_events`, `remote_support_*` (3), `reprint_course_slip`, `sign_table_order`, `work_in`, `work_out` | same exclusions apply to Vue; `reprint_course_slip` has no screen on either UI |

## 5. Linux-only open items

- **Kiosk hardware:** the target terminals (Penetek A3216 M82 or Advantech
  USC-360J) were never exercised by this chain; `CHECKLIST_DEPLOIEMENT_KIOSK_FR.md`
  is the manual check list.
- **Image build:** `deploy/image/` (mkosi) cannot build on the Mac. The USB
  autoinstall work is on local branch `feat/usb-autoinstall` (also seen as
  `remotes/origin/feat/usb-autoinstall` in this worktree's branch list), not
  merged into `android/int`. Linux x86_64 root needed (`ASSEMBLAGE_FINAL_FR.md`,
  step 6).
- **Linux evidence items still "A effectuer"** in
  `JOURNAL_TESTS_PRE_SOUMISSION_FR.md` (38 lines), the real-printer ticket and
  report photos (`50_` to `56_`), `DOSSIER_SOUMISSION_FR.md` (signed form,
  source archive), and the SHA-1 / date / signature fields filled at assembly.
- **On-site EFT probe:** `crates/pos-eft-probe/ESSAI_SUR_SITE_FR.md`; a driver
  fix it calls for changes `crates/pos-eft` and forces a new freeze
  (`LINUX_FREEZE.md`, "Open before filing"). Live probes need the owner's go-ahead.
- **Live FDM run:** the module2 simulator run under the Linux product identity
  (27/09/2026) is still cited by the Linux dossier:
  `REPONSES_MANUEL_CERT_FR.md` line 754 ("rejouées le 27/09/2026") and
  `JOURNAL_TESTS_PRE_SOUMISSION_FR.md` line 59. A Linux filing must redo the
  run on the new Linux freeze (with the owner's go-ahead, section 6 step 6) and
  update those lines.
- **Reproducible build:** Docker emulation on the Mac, from a clean clone
  outside the worktrees (`scripts/check_repro.sh`).

## 6. Checklist to submit Linux

1. Decide the scope with the user and bump versions if the filing is not
   `1.0.0` (`Cargo.toml` `[workspace.package]`, `src-tauri/tauri.conf.json`,
   `ui/package.json`; the Linux `posSwVersion` follows `CARGO_PKG_VERSION`).
   Note that `Cargo.toml` is shared with the frozen Android inputs: if Android
   `android-v1.0.0` is already tagged, a version bump changes its frozen path
   set and needs the Android side to be considered first.
2. Merge or rebase onto the Android release branch so the engine is the one
   Android certified (a different engine for Linux needs its own full evidence).
   Decide what to do with `feat/usb-autoinstall`.
3. Full gate: `cargo fmt --all -- --check`, workspace and cert clippy,
   `cargo test --workspace`, `cargo test -p pili-pos --no-default-features
   --features cert`, `cargo test -p pos-engine --no-default-features --features cert`,
   the three parity scripts, `(cd ui && npm run build && npx playwright test)`.
4. Refresh the Linux evidence: `scripts/capture_uc_evidence.sh`,
   `scripts/capture_fdm_error_evidence.sh`, `scripts/generate_ticket_examples.sh`
   (this regenerates the 3 hand-edited ticket files of section 3) and `scripts/embed_*.py`, then `scripts/generate_test_report.py` for
   `RAPPORT_TESTS_BASELINE_FR.md`.
5. Retake captures: `cd ui && npm run captures` (Playwright `captures` project)
   so screens reflect the current UI; rebuild
   `CAPTURES_ECRAN_FR.pdf` with `scripts/make_captures_pdf.sh`. Take the
   real-printer photos `50_` to `56_` on the hardware.
6. Fill the hardware journal `JOURNAL_TESTS_PRE_SOUMISSION_FR.md` and the
   remaining "A effectuer" fields; restore the `MANIFEST.txt` header (remove the
   Android-only note from `ccc7441c`). Redo the live module2 simulator run on
   the Linux freeze (owner's go-ahead needed) and update the "27/09/2026" lines
   in `REPONSES_MANUEL_CERT_FR.md` (line 754) and
   `JOURNAL_TESTS_PRE_SOUMISSION_FR.md` (line 59).
7. Rebuild PDFs: `scripts/make_dossier_pdfs.sh` (fixes the `MATRICE_TRACABILITE`
   freshness finding), then `python3 scripts/check_dossier.py --final` must
   report 0 findings.
8. Reframe the demand. If Android is certified by then, the Linux product is a
   separate product and version: file it as its own initial demand under
   Art. 99 (one demand per version or model of a cash register system). Art. 113
   (modification of an already certified system) does not apply, because Linux
   is not a modification of the Android product; it does not apply to the first
   Linux version either. Create `CADRE_DEMANDE_FR.md` and `INDEX_DOSSIER_FR.md`
   in `docs/SUBMISSION_ARTIFACTS` (they do not exist there; start from the
   pre-`1595f448` Android text), add them to `MANIFEST.txt`, check
   `REPONSES_MANUEL_CERT_FR.md` is consistent, and do not cite the Android
   evidence as Linux evidence. If Android is not yet certified,
   the earlier plan (two initial filings, INT-D10 of `DECISIONS.md` section 10)
   can be reused, with the Android dossier no longer sharing documents.
9. Linux freeze: set the freeze tag (`linux-v1.0.0-rc3` or the final name), run
   `scripts/check_repro.sh` from a clean clone (two byte-identical builds),
   write a new `docs/android/LINUX_FREEZE.md` section (tag, built commit,
   SHA-1, Cargo.lock sha256, rustc, build image), then
   `bash scripts/android/check_linux_frozen.sh --tag <tag>` must pass. Restore the
   p2/p7 gate call (section 2.3). Set the tag only with the user; never push it
   without them.
10. Fill the dossier placeholders that depend on the freeze (SHA-1, commit, date,
    signatures) in `docs/SUBMISSION_ARTIFACTS`, rebuild the affected PDFs, run
    `scripts/make_submission_zip.sh`, and follow
    `docs/internal/ASSEMBLAGE_FINAL_FR.md` for the signed form and the e-mail.
11. After certification, any change goes through Art. 113 (one month notice);
    re-run `check_linux_frozen.sh` before each release.

## Commands used to build this note

```bash
git log --oneline 980dcbd6..HEAD
git diff --stat linux-v1.0.0-rc2 HEAD -- Cargo.toml Cargo.lock rust-toolchain.toml src-tauri ui deploy/build-release.sh scripts/build_and_hash.sh crates
bash scripts/android/check_linux_frozen.sh
python3 scripts/android/check_ui_parity.py --strict
python3 scripts/android/check_command_parity.py --strict
python3 scripts/android/check_ipc_wrappers.py
python3 scripts/check_dossier.py ; python3 scripts/check_dossier.py --final
```
