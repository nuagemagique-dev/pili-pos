# P8a exit report (documentation lane, rerun)

Written by the P8a planner on 2026-10-10, with the user away. Lane branch `android/lane-p8b`, worktree `moyun-pos-wt/lane-p8b`, base `android/int` at `602544c`. This report replaces the report of the empty first run (`08ea0c2`). `P8a.md` is unchanged since `508c3054`: the computed exit task gave the same text, so it was not rewritten.

## 1. Summary

All twelve cards of `P8a.md` §9 ran, were approved and are merged into `android/lane-p8b`. The Android dossier is complete as a text dossier: 30 annexes of kind A and P, the capture runbook, `MANIFEST.txt` and the evidence registry `DOSSIER_EVIDENCE.md` (36 files, 9,800 lines added on top of `602544c`, documentation only). Every piece of evidence that P7, P8b or the user still has to produce is a placeholder `À effectuer (EV-NN : ...)` with an id of the registry (EV-01 to EV-20).

The lane-level phase gate passes. Two items are not run yet, by design: the tag form of the freeze check (the tag `linux-v1.0.0-rc1` does not exist yet) and the checks that only make sense on `android/int` after both lanes are merged (§4).

One card, P8a-T5, ended its reviews with REWORK on two sentences. The planner applied the reviewer's exact fixes and merged it (P8aD25). That fix was not reviewed again by an independent reviewer.

What is left for P8a to count as filed-ready: nothing in the text itself. The placeholders are filled in P7, P8b and by the user (§7). The PDFs and the ZIP builder are P8b.

## 2. Per task status

| Card | Files | Verdict | Card head | Merge into lane |
|------|-------|---------|-----------|-----------------|
| P8a-T1 | CADRE_DEMANDE, INDEX_DOSSIER, MANIFEST.txt, PROFIL_ENTREPRISE, LISTE_DISTRIBUTEURS, DOSSIER_EVIDENCE.md | APPROVE (after 2 rework rounds) | `440b9e3f` | `9aa751ff` |
| P8a-T2 | PLATEFORME_ANDROID, SCHEMA_ASSEMBLAGE | APPROVE | `845496cc` | `6576236a` |
| P8a-T3 | GARANTIE_MAJ_LANGUES, PROFILS_UTILISATEURS, FLUX_REMBOURSEMENT | APPROVE | `063124db` | `9a4b192e` |
| P8a-T4 | SECURITE_BD, FEUILLE_ACQUISITION_DONNEES | APPROVE | `448aa902` | `82ffcb70` |
| P8a-T5 | EXPORT_NOCTURNE, FLUX_DONNEES, COMMANDES_EN_LIGNE | REWORK, closed by the planner fix `2342e3b5` (P8aD25) | `2342e3b5` | `bda5b6fe` |
| P8a-T6 | REPONSES_MANUEL_CERT | APPROVE | `8468b8f1` | `9cf25a27` |
| P8a-T7 | DOCUMENT_PROCESSUS_TECHNIQUE, LISTE_FONCTIONNALITES, DECLARATION_FONCTIONS_INTERDITES, DECISION_PEPPOL_AP | APPROVE | `0b356279` | `7e06576c` |
| P8a-T8 | MANUEL_INSTALLATION, CHECKLIST_DEPLOIEMENT | APPROVE | `3117490e` | `956ae93a` |
| P8a-T9 | MANUEL_UTILISATEUR | APPROVE | `47afc0c0` | `df4411df` |
| P8a-T10 | RAPPORT_TESTS_BASELINE, RESULTATS_TESTS_UC, JOURNAL_TESTS_PRE_SOUMISSION, TICKETS_EXEMPLES, COUVERTURE_ADVANCED_SET, internal/CAPTURES_ECRAN_RUNBOOK | APPROVE | `f2903e45` | `e3af8ec4` |
| P8a-T11 | MANUEL_PROGRAMMATION, MAPPING_CONFORMITE, LIGNES_PRODUITS | APPROVE | `fecd3aa2` | `99fa2c01` |
| P8a-T12 | consistency pass, capture ids A01 to A98 reconciled | APPROVE (conditional on the T5 merge, now met) | `d1cb2f07` | `c35ef723` |

Integrator entries (waves 1 to 4) and planner commits on the lane: `15ad8a1a` (wave 1 record), `3d53e268`, `80f9ed2b`, `96bd25d2`, then this exit: `ae5cc77e` (review follow-ups across annexes, P8aD26) and the commit of this report with P8aD25 to P8aD29. Merge order: T1 to T4, T6 to T10, T11, T12, T5 (T5 last because it waited for the fix).

## 3. Gate evidence (lane `android/lane-p8b` at `ae5cc77e`, rerun by the planner)

All checks of `P8a.md` §7.1 steps 3 to 7, §7.2 and §7.3 that can run on the lane:

| Check | Result |
|-------|--------|
| Lane scope against `602544c` (dossier, evidence, P8a phase files, DECISIONS only) | PASS |
| Non-documentation files identical to `602544c` (`git diff --quiet 602544c HEAD -- . ':(exclude)docs'`) | PASS |
| Clean tree | PASS |
| `check_dossier` on `docs/android/dossier` (rust-ident, repo-path, brand, em-dash, placeholder) | PASS |
| Every backticked `android/` path exists | PASS |
| Every EV id is in `P8a.md` §6 and in `DOSSIER_EVIDENCE.md` | PASS |
| No em-dash, no client name, no `linux-v1.1` in the dossier | PASS |
| Linux dossier untouched and `scripts/check_dossier.py` green | PASS |
| §7.2 dangling `_FR` reference scan | PASS (the T5 annexes are now present; the `NOM_FR.md` placeholder was reworded by T12) |
| Presence of every A and P document of §4, `MANIFEST.txt`, `DOSSIER_EVIDENCE.md` | PASS (31 of 31) |
| `bash scripts/android/check_linux_frozen.sh --ref 602544c` | PASS, 17 frozen paths clean |
| `bash scripts/android/check_linux_frozen.sh --tag linux-v1.0.0-rc1` | DEFERRED: the tag does not exist (P8aD23) |
| `python3 scripts/android/kotlin_test_inventory.py --base 602544c` | PASS, base 1957, head 1957, removed 0 |
| `scripts/android/test_inventory.sh --diff` | PASS at waves 2 to 4 (run in the `int` worktree: 404 added, 0 removed, ws 1640 >= 1355); not rerun at exit because nothing outside `docs/` changed since |
| `python3 scripts/android/lane_paths.py --check` | PASS, 3923 files, 80 rules, 74 probes |
| Commit subjects Conventional (`docs` or `chore` merges), no em-dash in messages | PASS |
| No keystore, key or certificate file added | PASS |
| T5 card gate after the planner fix (§7.1, all steps, headers, sizes 113/220, 123/150, 62/120) | PASS |
| `Cited in` column of `DOSSIER_EVIDENCE.md` against the EV grep, including the T5 annexes | consistent, no change needed |

**Build.** The lane changes only files under `docs/`; every build input is byte-identical to `android/int` at `602544c`, so the lane builds exactly as `602544c` does. No cargo, Gradle, Docker or emulator run was started for this exit (none is a P8a gate, `P8a.md` §7.1).

**Not runnable on the lane, for the `android/int` merge:** the ancestry check and the dossier checks in the `int` worktree (still at `602544c`, no dossier files), and the freeze check in tag form.

## 4. Hashes

None of the dossier's hashes exist yet, and none is invented: the APK SHA-1 and SHA-256 (`EV-01`), the `libpos_ffi.so` SHA-1 (`EV-02`), the signing certificate digest (`EV-03`), the freeze tag commit and Linux binary SHA-1 (`EV-04`) and the image digest with the two unsigned APK SHA-1 (`EV-05`) are filled in P8 from the user's release key, the re-freeze and `declare_android.sh`. Git hashes of this phase are in §2. Lane head before this report: `ae5cc77e`.

## 5. Decisions taken while the user was away

All in `docs/android/DECISIONS.md`, section "Decided while the user was away (review)":

- P8aD1 to P8aD18 (first run): location of the dossier, framing as two v1.0 initial applications under Art. 99, Linux source `86088e4`, common documents not copied, placeholders, facts only from the repo, annex numbering, card sizes.
- P8aD19 to P8aD24 (rerun): lane `android/lane-p8b`, freeze tag `linux-v1.0.0-rc1` named but no value written, the 7 kept post-Z backups as the product rule (user decision), GMS as built versus target (`EV-13`), gates, the timing message.
- P8aD25 to P8aD29 (this exit): T5 closed by the planner fix instead of a third worker round; review follow-ups applied on the lane; worker commit trailers (Sonnet 5.5) kept as written; non-blocking review notes carried to P8b; no freeze step at the P8a exit.
- Integrator records of waves 1 to 4 (end of `DECISIONS.md`).

## 6. Freeze step

None. The plan sets no freeze at the end of P8a. The re-freeze `linux-v1.0.0-rc1` is prepared on `android/lane-freeze` and needs the user (plan, "Always needs the user's explicit go-ahead"). No tag was created, and `docs/android/LINUX_FREEZE.md` is outside this lane's paths and was not edited (P8aD29).

## 7. Risks and open points for the user

- **Legal framing is the producer's reading.** Two separate v1.0 initial applications under Art. 99, the nightly export declared as a copy of the legal record (`CADRE_DEMANDE_FR.md`). Nothing was sent to the SPF and the dossier asks it nothing. Needs your review before filing.
- **T5 planner fix not independently reviewed** (P8aD25): two sentences and one folder name in `FLUX_DONNEES_FR.md` §1, §11 and §12.
- **Backup retention** (7 newest post-Z backups kept) is written as the product's rule. If the engine change does not land inside `linux-v1.0.0-rc1` (`EV-19`), the text of `SECURITE_BD_FR.md` §5 and §8, `PLATEFORME_ANDROID_FR.md` §5 and the manuals must be revised before filing.
- **GMS measures** (Play Store and Google apps hidden, windowed system update policy, Play Protect) are not built: described as target, `EV-13`. Built and evidenced before filing, or the text is removed.
- **No on-site Wi-Fi change in 1.0.0.** The app has no network screen, so the site Wi-Fi is prepared on the bench before the bundle import; a change after delivery needs a newer declared version. A Gérant Wi-Fi screen is a target with no build card yet.
- **Independent hash on the till.** On a release till the APK hash is shown by the app itself; the independent checks are the public declared artifact and the reproducible build (P8aD12). An on-device check outside the app would need a decision.
- **Open questions from the T11 worker:** a corrected bundle can only be re-imported when the till is not provisioned or the deviceId does not match; a sole Gérant who forgot the PIN has no recovery on site; without HubRise the catalogue is frozen after the first import. All are stated honestly in the dossier.
- **Linux source.** The derived files follow the X1-T10 tip `86088e4`, which is not merged yet. If its text changes, rerun the derived-diff command of `P8a.md` §7.1 on each file (procedure in `DOSSIER_EVIDENCE.md`).
- **Hardware residue (`EV-15`)** stays open until a real D3 Pro is available; the dossier says the tests ran on the emulated profile.

## 8. What the next phase must know

- **Merge into `android/int`.** `android/lane-p8b` is ready: it touches only `docs/android/dossier/**`, `docs/android/DOSSIER_EVIDENCE.md`, `docs/android/phases/P8a*.md` and `docs/android/DECISIONS.md`. The other lane edits `DECISIONS.md` too: expect a textual conflict there, resolve by keeping both lanes' entries. After the merge, rerun on `android/int` the checks of `P8a.md` §7.3 and, once the tag exists, `check_linux_frozen.sh --tag linux-v1.0.0-rc1`.
- **`android/lane-p8` is not merged** (plan and the empty exit report of the first run only, P8aD19).
- **P8b** builds the PDFs and the ZIP from `docs/android/dossier/MANIFEST.txt` (common documents as `commun:` Linux PDFs, `internal/` never in the ZIP), takes the captures A01 to A98 of `internal/CAPTURES_ECRAN_RUNBOOK_FR.md` on `sunmi_d3_pro_gms_api33`, fills the placeholders and runs the final dossier check. Its work list starts with the notes of P8aD28, in particular `plat_about_engine_ref` in `android/app/src/main/res/values/strings_plat_about.xml` (still the old 1.1.0 tag), which must be corrected before the About capture.
- **P7 and the user** produce the evidence `EV-06` to `EV-14`, `EV-16`, `EV-20`; `EV-01` to `EV-05` and `EV-18` need the user (release key, re-freeze, `declare_android.sh`, filing date).
- Task worktrees: only `moyun-pos-wt/P8a-P8a-T5` is still on disk (merged; it can be removed). All card branches are kept. Nothing was pushed.

## 9. The relayed message

The message relayed with this run ("you told me it would be ready this morning, now you say at 11pm?") is about timing. This report cannot check what was promised earlier and gives no new time. Stated plainly: the P8a text dossier is done on `android/lane-p8b` (12 of 12 cards merged, lane gate green). Still to come: the merge into `android/int`, the evidence and PDFs of P7 and P8b, and the steps that need you (re-freeze, release key, declaration, filing).
