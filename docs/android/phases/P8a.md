# P8a: Android dossier variant (documentation lane, parallel to P7)

Written by the P8a planner on 2026-10-10, with the user away. Lane branch `android/lane-p8`, worktree `moyun-pos-wt/lane-p8`, forked from `android/int` at `602544c` (`docs(android): record the merge of lane-export, lane-ui and lane-plat into int`). The other lane of this run is P7 (`android/lane-p7`, end-to-end parity). Both lanes are merged into `android/int` afterwards. `main` is not touched.

Planner decisions of this phase: P8aD1 to P8aD18 in `docs/android/DECISIONS.md`, section "Decided while the user was away (review)", block "P8a planner". This file repeats what the workers need, so a card can be done from this file alone.

## 1. Goal and scope

P8a is the documentation half of plan phase P8 ("Dossier + release", task "Dossier variant"). It writes the French, brand-neutral dossier of the Android product `pili-pos-android`, derived from the Linux dossier `docs/SUBMISSION_ARTIFACTS`, covering:

- the platform (Sunmi D3 Pro, Android 13, the shared Rust engine inside the APK);
- the lock-down (device owner, Lock Task, restrictions, no remote support);
- the hash procedure (reproducible APK, SHA-1 of record, About screen, public declared artifact);
- the update path (operator-started, declared-only updater, GHCR or USB stick);
- the SPF access to the data (Art. 16 and 102: SAF export to removable media);
- the deviceId (Sunmi hardware serial, FAQ Q505 b);
- the framing of the filing: Linux and Android are **two separate v1.0 initial applications** under AM Art. 99 (nothing has been filed yet), backed by the same engine source;
- the nightly export feature (X1), declared in the Android dossier as in the Linux one;
- placeholders for every piece of evidence that P7, P8b or the user produce later.

**Out of scope (do not touch):** any code, test, script, build file or Gradle file; `docs/SUBMISSION_ARTIFACTS/**` (frozen for lanes, `LANES.md`); `docs/internal/**`; every file P7 works on (androidTest, `crates/**`, `scripts/**`, `docs/android/*.md` other than `DECISIONS.md`, evidence directories); PDFs (P8b builds them); the ZIP builder (P8b). Nothing is sent to the SPF and the dossier contains no question to the SPF.

**Allowed paths of the whole lane:**

```
^(docs/android/dossier/.*|docs/android/DOSSIER_EVIDENCE[.]md|docs/android/phases/P8a[^/]*[.]md|docs/android/DECISIONS[.]md)$
```

`docs/android/DECISIONS.md` is edited only by the planner and the integrator (append-only), never by a worker card. `docs/android/phases/P8a*.md` is the planner's.

## 2. Inputs every worker reads

- This file, in full (sections 3 to 7 are binding).
- The Linux source of every derived document: **always the X1-T10 tip `86088e4`** (branch `android/X1-X1-T10`, not merged yet; it is the lane base dossier plus the nightly export annex and its updates). Read a file with `git show 86088e4:docs/SUBMISSION_ARTIFACTS/<NAME>.md`. Nothing in `docs/SUBMISSION_ARTIFACTS` changed on `android/int` since that branch forked (`2819253`), so `86088e4` is the single Linux source. Never derive from the working-tree copy, which lacks the export annex.
- Android reference notes (English, in the repo): `docs/android/PLATFORM.md` (policies, audit, watchdog, About, SPF export, updater, release tooling, reproducible APK, storage and SD card), `docs/android/APP.md` (module, engine runtime, provisioning, kiosk, known limits), `docs/android/REPRO.md`, `docs/android/BUILD.md`, `docs/android/ENGINE.md`, `docs/android/FFI.md`, `docs/android/PARITY.md` and `docs/android/parity/*.md`, `docs/android/DEVICE_TESTS.md`, `docs/android/DECISIONS.md` (sections 1 to 12 and the blocks PD201 to PD230, X1D1 to X1D29, PD248), `docs/android/spikes/S4.md`, `docs/android/phases/X1.md`, `X1-exit.md`, `P6-exit.md`.
- The law and the SPF texts: `docs/legislation/20240429_AM_aspects_techniques_certification_FR.md` (Art. 16, 47, 99 to 113), `docs/20260717_FAQ_PRODUCTEURS_FR.md` (Q505), `docs/20260903_Intro_certification_procédure.md`.
- The code, read only, to check a fact (paths below are real): `android/app/src/main/AndroidManifest.xml`, `android/app/build.gradle.kts`, `android/app/src/main/kotlin/dev/nuagemagique/pilipos/{provisioning,kiosk,engine,platform,nightly,feature}/`, `android/app/src/main/res/values/strings_*.xml` (French labels, the default locale), `android/app/src/main/res/xml/data_extraction_rules.xml`, `android/rust/pos-ffi/Cargo.toml`, `crates/pos-export/STATS_SCHEMA.md`, `scripts/release/declare_android.sh`, `scripts/android/check_repro.sh`, `android/docker/`.

## 3. Writing rules (binding)

1. **French, brand-neutral.** The dossier is in French. No client, restaurant or customer brand anywhere (the check below refuses the known names). The producer line is the Linux one: `Bruno Moyse / nuagemagique.dev (Bruxelles, BE)`. Hardware makers (Sunmi, Epson) and the FDM (module2) are named, as in the Linux dossier.
2. **No em-dash character** (U+2014) anywhere. Use a hyphen, a colon or a new sentence.
3. **Facts only from the repo.** Every technical statement must be backed by the code or by a repo note listed in section 2. A hardware fact that the repo does not prove (real D3 Pro specs, real serial format, Verified Boot, storage encryption of the real image, largest SD card, real Epson behaviour) is never invented: write what was measured on the emulated profile and add the placeholder `EV-15`. Never write a figure as measured when it is an estimate.
4. **Placeholders.** Missing evidence is written `À effectuer (EV-NN : <ce qui manque>)`, with an id of the registry in section 6. This passes the default dossier check and is caught by the final check (`pending` rule) once the dossier is listed in a manifest. Never use the markers `TODO`, `TBD`, `XXX`, `FIXME`, `à produire`, `à reformater` (the check refuses them), so write examples like a VAT number as "BE0 suivi de 9 chiffres", not with X's.
5. **Header of a derived document.** Line 1 is exactly `<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/<NAME>.md @ 86088e4 -->`. Then the Linux title with `pili-pos` replaced by `pili-pos-android`, then the identity block:
   ```
   **Distributeur :** Bruno Moyse / nuagemagique.dev (Bruxelles, BE)
   **Produit :** pili-pos-android 1.0.0 (posSwVersion `pili-pos-android/1.0.0`)
   **Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
   **Date :** À effectuer (EV-18 : date de dépôt)
   ```
   (keep only the lines the Linux document has an equivalent for, plus `Plateforme`). A new document (no Linux source) starts with `<!-- Variante Android, sans équivalent Linux -->` and the same identity block.
6. **Keep the Linux structure.** A derived document keeps the Linux section numbers and titles where the topic exists on Android, so a reference such as `SECURITE_BD_FR.md §5` means the same section in both dossiers. A Linux-only section keeps its number with an Android title (for example Linux "3.6 Chiffrement LUKS2" becomes "3.6 Chiffrement du stockage"), or becomes `Sans objet sur Android : <raison>`. New sections go at the end. New documents follow the outlines in section 5.
7. **Citations.** Cite another annex by its file name (`PLATEFORME_ANDROID_FR.md §5`). Cite a common document (section 4, kind C) by name too; it is part of the Android ZIP. A Linux-only document is cited with its full path (`docs/SUBMISSION_ARTIFACTS/CHECKLIST_DEPLOIEMENT_KIOSK_FR.md`) and only where the comparison is the point. Backticked repo paths must exist on this branch (the check verifies `crates/`, `src-tauri/`, `ui/`, `deploy/`, `scripts/`, `docs/` and the extra gate below verifies `android/`). `docs/SUBMISSION_ARTIFACTS/EXPORT_NOCTURNE_FR.md` does not exist on this branch: never backtick it, cite `EXPORT_NOCTURNE_FR.md` (the Android annex).
8. **Versions and names.** Android product `pili-pos-android`, version `1.0.0`, posSwVersion `pili-pos-android/1.0.0`, versionCode `10000`, application id `dev.nuagemagique.pilipos`, tag `android-v1.0.0`, declared artifact `declared-android` on GHCR. Linux product `pili-pos`, version `1.0.0`, posSwVersion `pili-pos/1.0.0`. The engine freeze tag and its commit are `EV-04` (the user's re-freeze `linux-v1.0.0-rc1`, then the final tag). Development builds carry `-dev` and are never a declared version.
9. **Sizes.** A card's diff stays near 600 lines. For a derived document the size is the smaller of its diff against the Linux source, `diff <(git show 86088e4:docs/SUBMISSION_ARTIFACTS/<NAME>.md) docs/android/dossier/<NAME>.md | grep -c '^[<>]'`, and its own line count (`wc -l`, for a document rewritten from scratch under a derived header). A new document counts in full. Each card gives its budget; going over it by more than 20 percent is a REWORK unless the worker's report explains why.
10. **Commits.** Conventional Commits with type `docs`, for example `docs(android): Android dossier platform annex`. No em-dash in commit messages. End every commit message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Commit, never push, never merge.

## 4. The Android dossier: files and owners

All files below live in `docs/android/dossier/` (P8aD1). Annex numbers are fixed here (P8aD14) and used by every card. Kind: **A** = Android document in `docs/android/dossier/` (derived from the Linux one, or new); **C** = common document, taken unchanged from the Linux dossier at the same engine tag (not copied, P8aD4); **P** = evidence document, written as a skeleton with placeholders now and filled by P8b.

| # | File | Kind | Card | Manual section covered |
|---|------|------|------|------------------------|
| 0 | `INDEX_DOSSIER_FR.md` | A (new) | T1 | list of the documents (Art. 101), reading rule of the common documents |
| 1 | `REPONSES_MANUEL_CERT_FR.md` | A | T6 | linear answers §I to §VIII |
| 2 | `LISTE_DISTRIBUTEURS_FR.md` | A | T1 | §I.E |
| 3 | `PROFIL_ENTREPRISE_FR.md` | A | T1 | §I.F |
| 4 | `MANUEL_UTILISATEUR_FR.md` | A | T9 | §I.C |
| 5 | `MANUEL_INSTALLATION_FR.md` | A (new text) | T8 | §I.C |
| 6 | `MANUEL_PROGRAMMATION_FR.md` | A | T11 | §I.C |
| 7 | `DECLARATION_FONCTIONS_INTERDITES_FR.md` | A | T7 | §III.C |
| 8 | `PROFILS_UTILISATEURS_FR.md` | A | T3 | §III.C |
| 9 | `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md` | A | T7 | §II + §III.E |
| 10 | `FLUX_DONNEES_FR.md` | A | T5 | §II.C + §III |
| 11 | `SCHEMA_BD_FR.md` | C | none | §II.C |
| 12 | `SECURITE_BD_FR.md` | A | T4 | §II.C, SPF access |
| 13 | `SCHEMA_ER_FR.md` | C | none | §II.C |
| 14 | `SCHEMA_ASSEMBLAGE_FR.md` | A | T2 | §I.C + §II.G |
| 15 | `LISTE_FONCTIONNALITES_FR.md` | A | T7 | §III |
| 16 | `LIGNES_PRODUITS_FR.md` | A | T11 | §III, lignes de produit |
| 17 | `STRATEGIE_COMPTEURS_FR.md` | C | none | §III.B + FDM non opérationnel |
| 18 | `FLUX_REMBOURSEMENT_FR.md` | A | T3 | §III.F |
| 19 | `TICKETS_EXEMPLES_FR.md` | P | T10 | §VI |
| 20 | `DECISION_PEPPOL_AP_FR.md` | A | T7 | §III.F (B2B) |
| 21 | `MAPPING_CONFORMITE_FR.md` | A | T11 | §IV + §V |
| 22 | `FEUILLE_ACQUISITION_DONNEES_FR.md` | A | T4 | §VII (DAS) |
| 23 | `RAPPORT_TESTS_BASELINE_FR.md` | A (new text) | T10 | §IV + §VIII |
| 24 | `COUVERTURE_FIXTURES_FR.md` | C | none | §IV |
| 25 | `COUVERTURE_RAPPORTS_FR.md` | C | none | §IV |
| 26 | `COUVERTURE_ADVANCED_SET_FR.md` | A | T10 | §IV |
| 27 | `RESULTATS_TESTS_UC_FR.md` | P | T10 | §IV, UC results |
| 28 | `CAPTURES_ECRAN_FR.pdf` | P (PDF, P8b) | T10 writes `internal/CAPTURES_ECRAN_RUNBOOK_FR.md` | §V + §VII.D |
| 29 | `JOURNAL_TESTS_PRE_SOUMISSION_FR.md` | P | T10 | §IV, rehearsal |
| 30 | `CHECKLIST_DEPLOIEMENT_FR.md` | A (new) | T8 | test machine |
| 31 | `GARANTIE_MAJ_LANGUES_FR.md` | A | T3 | §I.D, §II.D, §II.E, §VI |
| 32 | `TICKET_NUMERIQUE_FR.md` | C | none | §III.H |
| 33 | `COMMANDES_EN_LIGNE_FR.md` | A | T5 | §III.M |
| 34 | `EXPORT_NOCTURNE_FR.md` | A | T5 | Configuration Internet + §VII.B + DAS |
| 35 | `CADRE_DEMANDE_FR.md` | A (new) | T1 | framing of the application (Art. 99) |
| 36 | `PLATEFORME_ANDROID_FR.md` | A (new) | T2 | platform, lock-down, deviceId, storage |

Plus, outside the annexes: `docs/android/dossier/MANIFEST.txt` (T1) and `docs/android/DOSSIER_EVIDENCE.md` (T1, English, internal, never in the ZIP). The Linux documents `MATRICE_TRACABILITE_FR.md` (not in the Linux manifest) and `CHECKLIST_DEPLOIEMENT_KIOSK_FR.md` (replaced by annex 30) have no Android copy.

**Reading rule of the common documents (C).** They describe the engine, which is the same source on both platforms. Their few Linux-specific passages are listed, with the Android reading, in `INDEX_DOSSIER_FR.md` §3 (T1). The known ones, for T1 to check and complete with `grep -n -i -E 'linux|debian|/var/lib|/etc/|tauri|vue|systemd|rustdesk|usb'` on each C file at `86088e4`:

| Document | Linux passage | Android reading |
|----------|---------------|-----------------|
| `SCHEMA_BD_FR.md` | line 10, live file `/var/lib/pili-pos/pos.db` | `files/engine/pos.db` in the app-private storage, see `PLATEFORME_ANDROID_FR.md` §5 |
| `TICKET_NUMERIQUE_FR.md` | on-screen keyboard, support screen | the Android on-screen keyboard of the app, same rules and codes |
| `STRATEGIE_COMPTEURS_FR.md`, `SCHEMA_ER_FR.md`, `COUVERTURE_FIXTURES_FR.md`, `COUVERTURE_RAPPORTS_FR.md` | references to `src-tauri` or Tauri commands, if any | the same engine function, called by the Android interface through the FFI method of the same name (`docs/android/FFI.md` §11) |

## 5. Outlines of the new documents

Cards cite these sections by number, so the outlines are binding.

**`CADRE_DEMANDE_FR.md` (T1).** §1 Objet du document. §2 Deux demandes de certification initiales distinctes (Art. 99: one application per version or model; `pili-pos` 1.0.0 on Linux x86_64 and `pili-pos-android` 1.0.0 on Android are two products; neither is certified yet, so neither is a modification under Art. 113). §3 Identification des versions (Art. 47 unique version mention; posSwVersion of each line; versionCode; how a till shows it: login screen and About screen). §4 Moteur fiscal commun (the engine `crates/pos-engine` and the cert crates at the same freeze tag `EV-04`; the Android APK embeds it as `libpos_ffi.so`; the proof that both run the same engine: the freeze check, the command parity 86/86 and the journal comparison `EV-07`). §5 Ce qui diffère entre les deux produits (table: hardware, OS, UI layer, deviceId, lock-down, update path, SPF access medium, remote support, build and hash; each row points to the Android annex). §6 Modifications après certification (each product follows Art. 113 on its own after its certificate; an engine change concerns both products and is filed for both; one month notice; the urgent-fix rule of the SPF procedure as in the Linux answers §II.E). §7 Fonctions déclarées dans les deux demandes (nightly export, online orders, digital ticket; Peppol absent from both 1.0 builds). §8 Machine de test (the emulated D3 Pro profile unless a real D3 Pro is available, `EV-15`; module2 online simulator). Budget about 170 lines.

**`INDEX_DOSSIER_FR.md` (T1).** §1 Contenu du dossier (the annex table of section 4 in French, columns number, file, section of the manual, nature: "variante Android", "document commun", "preuve"). §2 Documents communs aux deux demandes (why they are shared: same engine source and tag `EV-04`). §3 Lecture des documents communs sur Android (the table of section 4, completed). §4 Preuves jointes (`Report_Testing`, captures, build reports: `EV-05`, `EV-06`, `EV-08`, `EV-09`). Budget about 140 lines.

**`PLATEFORME_ANDROID_FR.md` (T2).** §1 Matériel et système (Sunmi D3 Pro GMS variant, Android 13 API 33, arm64-v8a, 1920x1080 landscape; Epson TM-m30III over LAN; the test machine is the emulated profile `sunmi_d3_pro_gms_api33` with the same characteristics unless a real device is available, `EV-15`). §2 Architecture de l'application (one APK; Kotlin and Compose interface; the shared Rust engine `libpos_ffi.so` through UniFFI; no fiscal or money rule in Kotlin; the engine owns the fiscal triggers). §3 Propriétaire de l'appareil et verrouillage (device owner, provisioning without any Google account, Lock Task, the restriction set as built in `platform/policy/PolicySet.kt`, time and time zone Europe/Brussels, keyguard and status bar, debugging off in release builds, system update policy; GMS-specific measures `EV-13`). §4 Identifiant de l'appareil (deviceId = hardware serial read by the device owner, FAQ Q505 b; read at provisioning, compared at every engine open, mismatch blocks signing; no generated, random or `ANDROID_ID` value; format; emulator value `EMULATOR37X2X12X0`; real format `EV-15`; value of the test machine `EV-16`). §5 Stockage des données et carte SD (section 8 of this file is the content). §6 Réseau et connexions (outbound only: FDM, HubRise, TEM mail, nightly export receiver, operator-started update pull; no listening port; TLS roots bundled in the APK; FDM over HTTP or mTLS as on Linux). §7 Absence d'accès à distance (no remote support program on Android, DECISIONS §7; support on site only). §8 Arrêt, panne et reprise (foreground service, watchdog restart after a fatal engine state, restart at boot, power-cut recovery `EV-12`, 24 h run `EV-11`). §9 Limites connues et vérifications sur matériel réel (hardware residue list of DECISIONS §12, each with `EV-15`). Budget about 320 lines.

## 6. Evidence registry (placeholders)

T1 copies this table into `docs/android/DOSSIER_EVIDENCE.md` (adding a "Cited in" column, completed by T12). Every placeholder in the dossier uses one of these ids; the gate refuses any other.

| Id | Evidence | Produced by | Notes |
|----|----------|-------------|-------|
| EV-01 | SHA-1 and SHA-256 of the signed release APK `pili-pos-android 1.0.0` | `scripts/release/declare_android.sh` on the release commit | user go-ahead; never run by the chain |
| EV-02 | SHA-1 of the entry `lib/arm64-v8a/libpos_ffi.so` of that APK | same run, About screen | |
| EV-03 | SHA-256 digest of the release signing certificate | user creates the RSA release key (DECISIONS §9) | |
| EV-04 | engine freeze tag, its commit and the Linux binary SHA-1 of the same engine | user re-freeze `linux-v1.0.0-rc1`, then final | candidate SHA-1 `d4fb09a2ae2cf2501fe4d6847eb72e954d0b95ed` (X1D27) |
| EV-05 | build image digest and the two identical unsigned APK SHA-1 at the release commit | `scripts/android/check_repro.sh` | |
| EV-06 | UC results (Basic Set 1.8, Advanced Set 1.6, Reports 1.3) on the Android emulator, JSON request and response per UC | P7 UC instrumented suite | |
| EV-07 | journal and FDM request comparison Android vs Linux for the same scenarios | P7 `pos-uc-check` | |
| EV-08 | ticket examples printed on `pos-printer-mock` (DLE EOT bytes stripped), real Epson if available | P7 or P8b | |
| EV-09 | screen captures from `sunmi_d3_pro_gms_api33` | P8b | list in `internal/CAPTURES_ECRAN_RUNBOOK_FR.md` |
| EV-10 | D3 Pro runbook on both AVDs: provisioning, sale, refund and Z printed, SPF export, update, kiosk lock | P7 | |
| EV-11 | 24 h run of the app with the foreground service | P7 | |
| EV-12 | power-cut recovery (`adb emu kill` in a transaction, cold boot) | P7 | |
| EV-13 | GMS image: Play Store and user-facing Google apps hidden, install restrictions, windowed system update policy, Play Protect behaviour | platform card for PD230, then P7 GMS run | text describes the target until built |
| EV-14 | live run against the module2 online simulator (`CPOS0031234567`, terminal `POS-2-ANDROID`) | user go-ahead | |
| EV-15 | facts of a real D3 Pro (serial format, storage, card slot, Verified Boot, storage encryption, Epson, timings), or the statement that the test machine is the emulated profile | real device if available | |
| EV-16 | deviceId of the test machine as printed on its tickets | P7 or P8b | |
| EV-17 | final test counts at the release commit (Rust ws and cert, `android/rust`, Kotlin inventory, UC suite) | P8b | |
| EV-18 | filing date and document dates | user at filing | |
| EV-19 | backup retention rule on the till (PD219) | user decision, engine change with the re-freeze | text describes as built until then |
| EV-20 | nightly export on the test machine: enabled or not, holders of the age keys, receiver operator | user (X1 questions 3 and 5) | |

## 7. Gates

### 7.1 Card gate (every card; run from the card worktree root, all must pass)

Set `ALLOW` to the card's own regex (given in the card) first.

```bash
BASE=$(git merge-base HEAD android/lane-p8)
ALLOW='<card regex>'
# 1 scope: only the card's files changed
! git diff --name-only "$BASE" HEAD | grep -Ev "^($ALLOW)$"
# 2 clean tree
test -z "$(git status --porcelain)"
# 3 Android dossier check: rules rust-ident, repo-path, brand, em-dash, placeholder
python3 -I -c 'import sys,pathlib; sys.path.insert(0,"scripts"); import check_dossier as c; c.DOSSIER_DIR=pathlib.Path("docs/android/dossier").resolve(); sys.exit(c.main([]))'
# 4 every backticked android/ path exists
grep -rhoE '`android/[^` ]+`' docs/android/dossier docs/android/DOSSIER_EVIDENCE.md 2>/dev/null | tr -d '`' | sed 's/[.,;:)]*$//' | sort -u | while read -r p; do case "$p" in *'<'*|*'*'*|*'$'*) continue;; esac; [ -e "$p" ] || echo "missing: $p"; done | (! grep .)
# 5 every EV id is in the registry of this file
grep -rhoE 'EV-[0-9]{2}' docs/android/dossier docs/android/DOSSIER_EVIDENCE.md 2>/dev/null | sort -u | while read -r id; do grep -q "^| $id |" docs/android/phases/P8a.md || echo "unknown $id"; done | (! grep .)
# 6 no em-dash, no client name
! grep -rn "$(printf '\342\200\224')" docs/android/dossier docs/android/DOSSIER_EVIDENCE.md 2>/dev/null
! grep -rniE 'tokyo|sushi|yangguofu|malatang|ygf' docs/android/dossier docs/android/DOSSIER_EVIDENCE.md 2>/dev/null
# 7 the Linux dossier is untouched and still green
git diff --quiet "$BASE" HEAD -- docs/SUBMISSION_ARTIFACTS docs/internal && python3 scripts/check_dossier.py
# 8 Linux freeze invariant: no frozen path changed by this card
bash scripts/android/check_linux_frozen.sh --ref "$BASE"
# 9 Conventional Commits, docs type, no em-dash in messages
! git log --format=%B "$BASE"..HEAD | grep -n "$(printf '\342\200\224')"
! git log --format=%s "$BASE"..HEAD | grep -Ev '^docs(\([a-z0-9-]+\))?: '
# 10 lane ownership file still consistent
python3 scripts/android/lane_paths.py --check
```

Plus, per derived file of the card, the header and the size (rule 3.5 and 3.9):

```bash
f=<NAME>.md
head -1 "docs/android/dossier/$f" | grep -qx "<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/$f @ 86088e4 -->"
diff <(git show "86088e4:docs/SUBMISSION_ARTIFACTS/$f") "docs/android/dossier/$f" | grep -c '^[<>]'   # <= the card budget
```

No card runs cargo, Gradle, Docker or an emulator: P8a changes no build input. The test inventory invariant (`scripts/android/test_inventory.sh --diff`) is therefore a phase-level check (7.3), satisfied by construction on every card (the scope check proves no code changed).

### 7.2 Cross-reference check (T12 and the phase gate)

```bash
grep -rhoE '(SUBMISSION_ARTIFACTS/)?[A-Z][A-Z_]+_FR[.](md|pdf)' docs/android/dossier | grep -v '^SUBMISSION_ARTIFACTS/' | sed 's/[.]pdf$/.md/' | sort -u | while read -r f; do [ -e "docs/android/dossier/$f" ] || [ -e "docs/android/dossier/internal/$f" ] || case "$f" in SCHEMA_BD_FR.md|SCHEMA_ER_FR.md|STRATEGIE_COMPTEURS_FR.md|COUVERTURE_FIXTURES_FR.md|COUVERTURE_RAPPORTS_FR.md|TICKET_NUMERIQUE_FR.md|CAPTURES_ECRAN_FR.md) ;; *) echo "dangling: $f";; esac; done | (! grep .)
```

### 7.3 Phase gate (integrator, on `android/int` after both lanes are merged)

The lane scope, the content checks of 7.1 (steps 3 to 7, with the EV ids also checked against `docs/android/DOSSIER_EVIDENCE.md`) and 7.2 on the merged tree, the presence of every A and P document of section 4, then the plan invariants:

```bash
! git diff --name-only 602544c android/lane-p8 | grep -Ev '^(docs/android/dossier/.*|docs/android/DOSSIER_EVIDENCE[.]md|docs/android/phases/P8a[^/]*[.]md|docs/android/DECISIONS[.]md)$'
for f in INDEX_DOSSIER_FR REPONSES_MANUEL_CERT_FR LISTE_DISTRIBUTEURS_FR PROFIL_ENTREPRISE_FR MANUEL_UTILISATEUR_FR MANUEL_INSTALLATION_FR MANUEL_PROGRAMMATION_FR DECLARATION_FONCTIONS_INTERDITES_FR PROFILS_UTILISATEURS_FR DOCUMENT_PROCESSUS_TECHNIQUE_FR FLUX_DONNEES_FR SECURITE_BD_FR SCHEMA_ASSEMBLAGE_FR LISTE_FONCTIONNALITES_FR LIGNES_PRODUITS_FR FLUX_REMBOURSEMENT_FR TICKETS_EXEMPLES_FR DECISION_PEPPOL_AP_FR MAPPING_CONFORMITE_FR FEUILLE_ACQUISITION_DONNEES_FR RAPPORT_TESTS_BASELINE_FR COUVERTURE_ADVANCED_SET_FR RESULTATS_TESTS_UC_FR JOURNAL_TESTS_PRE_SOUMISSION_FR CHECKLIST_DEPLOIEMENT_FR GARANTIE_MAJ_LANGUES_FR COMMANDES_EN_LIGNE_FR EXPORT_NOCTURNE_FR CADRE_DEMANDE_FR PLATEFORME_ANDROID_FR internal/CAPTURES_ECRAN_RUNBOOK_FR; do test -f "docs/android/dossier/$f.md" || echo "missing $f"; done | (! grep .)
test -f docs/android/dossier/MANIFEST.txt && test -f docs/android/DOSSIER_EVIDENCE.md
```


- `bash scripts/android/check_linux_frozen.sh --ref 602544c` passes: neither lane changed a frozen Linux path. The tag form `check_linux_frozen.sh` (against `linux-v1.1.0-rc1`) still fails by design on the 46 export lane files (X1D2) until the user's re-freeze; that failure must list the same files as at `602544c`.
- `python3 scripts/android/kotlin_test_inventory.py --base 602544c`: `removed 0`.
- `scripts/android/test_inventory.sh --diff`: `ok no test disappeared (ws ... >= 1355)`. It builds the workspace and needs `ui/dist` and about 20 GB free; with less disk it is deferred and recorded, as in UI-PD315.
- `python3 scripts/android/lane_paths.py --check` PASS.

## 8. The relayed SD card question (content of `PLATEFORME_ANDROID_FR.md` §5)

The user asked: "I know we can put a SD card in the Sunmi D3 Pro, it will be on what the SQLite will be stored? How big should be the SD card do you think?"

**Where the SQLite lives: never on the SD card.** The engine database is `files/engine/pos.db` in the app-private internal storage of the till, next to `files/engine/backups/`, the provisioning bundle and the logs (`android/app/src/main/kotlin/dev/nuagemagique/pilipos/provisioning/ProvisioningPaths.kt`). This is by design (PD220, PD248): a fiscal journal on removable media could be pulled out, swapped, corrupted or read elsewhere. The manifest has no `android:installLocation` and requests no storage permission, so Android can never move the data to a card adopted as internal storage; cloud backup and device transfer are off. The SD card is only a removable volume reached through a SAF grant: the target of the manager's SPF export and a source of a declared update from a stick. The nightly export (X1) keeps its local copies in `files/engine/exports/` and sends them over HTTPS, never to the card.

**How big: measured, not guessed (PLATFORM.md §12, P6-T11 on the emulator).** The database grows by about 6.5 KB for a one-line sale and 14.3 KB for a six-line sale (about 0.5 GB a year at 100 sales a day, 1.6 GB at 300). One SPF export copies `pos.db` and the 7 newest post-Z backups, so it weighs about 8 times the database. Recommendation: a good-quality microSDXC card (high endurance, class A1 or U3), formatted exFAT (FAT32 refuses files of 4 GiB, which the database alone reaches after a few years at high volume): **64 GB** for the first year at about 100 sales a day, **128 GB** at about 300 sales a day or from the second year. The app never deletes a folder on the card, so the manager clears old exports. This supersedes the earlier unmeasured 16 or 32 GB guess of X1-exit (X1D28).

**What matters more than the card.** The engine keeps a full copy of `pos.db` after every Z and deletes none (PD219), so the internal storage of the till, not the card, fills up first: tens of GiB within a year at the measured rates. A retention rule on the till is an engine change for the user's re-freeze (`EV-19`). The largest card the D3 Pro accepts, its slot behaviour, its free internal space and exFAT on a real card are hardware residue (`EV-15`).

## 9. Waves and cards

Dependencies only across waves. Wave 1 cards are independent of each other because names, numbers, outlines and placeholder ids are fixed in sections 3 to 6. Every card: base = the `android/lane-p8` head when its wave starts; branch `android/P8a-P8a-T<n>`; needs-human: no; lane: `p8a-docs`.

### Wave 1

**P8a-T1 Framing, index, manifest, evidence registry, company documents.** Files: `docs/android/dossier/{CADRE_DEMANDE_FR.md, INDEX_DOSSIER_FR.md, MANIFEST.txt, PROFIL_ENTREPRISE_FR.md, LISTE_DISTRIBUTEURS_FR.md}`, `docs/android/DOSSIER_EVIDENCE.md`. Budget: new about 450 lines; `PROFIL_ENTREPRISE_FR.md` 120, `LISTE_DISTRIBUTEURS_FR.md` 20.

**P8a-T2 Platform annex and assembly schema.** Files: `PLATEFORME_ANDROID_FR.md` (new, outline 5), `SCHEMA_ASSEMBLAGE_FR.md` (derived). Budget: new about 320; derived 250.

**P8a-T3 Hash, update, languages; user profiles; refund flow.** Files: `GARANTIE_MAJ_LANGUES_FR.md`, `PROFILS_UTILISATEURS_FR.md`, `FLUX_REMBOURSEMENT_FR.md` (derived). Budget: 380, 60, 40.

**P8a-T4 Database security, SPF access, data acquisition sheet.** Files: `SECURITE_BD_FR.md`, `FEUILLE_ACQUISITION_DONNEES_FR.md` (derived). Budget: 420, 180.

**P8a-T5 Nightly export, data flows, online orders.** Files: `EXPORT_NOCTURNE_FR.md`, `FLUX_DONNEES_FR.md`, `COMMANDES_EN_LIGNE_FR.md` (derived). Budget: 220, 150, 120.

### Wave 2

**P8a-T6 Answers to the certification manual.** File: `REPONSES_MANUEL_CERT_FR.md` (derived). Budget 600.

**P8a-T7 Technical process, feature list, forbidden functions, Peppol decision.** Files: `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md`, `LISTE_FONCTIONNALITES_FR.md`, `DECLARATION_FONCTIONS_INTERDITES_FR.md`, `DECISION_PEPPOL_AP_FR.md` (derived). Budget: 300, 120, 140, 40.

**P8a-T8 Installation manual and deployment checklist.** Files: `MANUEL_INSTALLATION_FR.md` (Android text, Linux structure reused where it fits), `CHECKLIST_DEPLOIEMENT_FR.md` (new). Budget: 450 (new or diff, whichever is smaller), 180.

**P8a-T9 User manual.** File: `MANUEL_UTILISATEUR_FR.md` (derived). Budget 600.

**P8a-T10 Test report, evidence skeletons, Advanced Set coverage, capture list.** Files: `RAPPORT_TESTS_BASELINE_FR.md` (new text), `RESULTATS_TESTS_UC_FR.md`, `JOURNAL_TESTS_PRE_SOUMISSION_FR.md`, `TICKETS_EXEMPLES_FR.md` (skeletons, new), `COUVERTURE_ADVANCED_SET_FR.md` (derived), `internal/CAPTURES_ECRAN_RUNBOOK_FR.md` (new). Budget: 200, 90, 90, 60, 80, 90.

### Wave 3

**P8a-T11 Programming manual, compliance mapping, product lines.** Files: `MANUEL_PROGRAMMATION_FR.md`, `MAPPING_CONFORMITE_FR.md`, `LIGNES_PRODUITS_FR.md` (derived). Budget: 300, 100, 40.

### Wave 4

**P8a-T12 Consistency pass.** Files: any of `docs/android/dossier/**`, `docs/android/DOSSIER_EVIDENCE.md`. Budget 400 changed lines.

The full self-contained briefs, acceptance criteria and gates of every card are in the orchestrator's card list of this run (same text as below, section 10).

## 10. Card briefs

### P8a-T1

Write the documents that frame the Android application, list its content and track the missing evidence.

1. `docs/android/dossier/CADRE_DEMANDE_FR.md`: new, outline of section 5. Base it on the AM text (`docs/legislation/20240429_AM_aspects_techniques_certification_FR.md`, Art. 47, 99, 100, 101, 110, 112, 113; quote at most one sentence per article), DECISIONS sections 2 and 10 (section 10 and Q6 are superseded: there is no Linux v1.1, both products are v1.0 initial applications, P8aD2), and the plan facts in section 1 of this file. State the framing as the producer's; never ask the SPF anything and never write that a question was or will be sent.
2. `docs/android/dossier/INDEX_DOSSIER_FR.md`: new, outline of section 5, with the annex table of section 4 (in French) and the reading table of the common documents, completed by grepping each C document at `86088e4`.
3. `docs/android/dossier/MANIFEST.txt`: same header style as `git show 86088e4:docs/SUBMISSION_ARTIFACTS/MANIFEST.txt`, French comments. Entries relative to `docs/android/dossier/`: `pdf_out/<NAME>.pdf` for each A and P annex in annex order, `CAPTURES_ECRAN_FR.pdf`, and each common document as `commun:pdf_out/<NAME>.pdf` (meaning: the PDF of the Linux dossier at the engine tag, P8aD4). Comment that the PDFs and the ZIP builder come with P8b, and that `internal/` is never in the ZIP. Evidence entries (`evidence/uc`, `evidence/tickets`, `evidence/build`) as comments for P8b.
4. `docs/android/dossier/PROFIL_ENTREPRISE_FR.md` and `LISTE_DISTRIBUTEURS_FR.md`: derived from `86088e4` (rule 3.5). Change only what concerns the product: the two product lines, the Android platform, the Android deployment (provisioning bundle instead of `/etc/pili-pos/pos.json`: the configuration travels in the signed USB bundle into the app-private storage, `docs/android/APP.md` §3).
5. `docs/android/DOSSIER_EVIDENCE.md` (English): the registry of section 6 with a column "Cited in" (left empty for T12), a section "How to fill a placeholder" (find `À effectuer (EV-NN`, replace with the value and its source, remove the marker), a section "Linux source" (every derived file names `86088e4`; when X1-T10 changes after its re-review, rerun the derived-diff command of 7.1 on each file and port the change), and a section "Known differences left for P8b" (PDFs, ZIP builder, final `--final` check).

Do not touch any other file. Allowed regex: `docs/android/dossier/(CADRE_DEMANDE_FR[.]md|INDEX_DOSSIER_FR[.]md|MANIFEST[.]txt|PROFIL_ENTREPRISE_FR[.]md|LISTE_DISTRIBUTEURS_FR[.]md)|docs/android/DOSSIER_EVIDENCE[.]md`.

### P8a-T2

1. `docs/android/dossier/PLATEFORME_ANDROID_FR.md`: new, outline of section 5, and §5 carries the content of section 8 of this file in French (same figures, same recommendation, same caveats). Sources: `docs/android/PLATFORM.md` §3, §5, §6, §12; `docs/android/APP.md` §3, §4, §10; DECISIONS §3, §4, §6, §7, §12, PD206 to PD220, PD230; `docs/android/spikes/S4.md`; `android/app/src/main/AndroidManifest.xml`; `android/app/src/main/kotlin/dev/nuagemagique/pilipos/platform/policy/PolicySet.kt`; `provisioning/ProvisioningPaths.kt`. List the restrictions exactly as `PolicySet.kt` builds them (name the Android constants), and say which ones are release-only. For the GMS variant (plan update 2026-10-09: provision before any Google account, QR provisioning on a reset device, Play Store and user-facing Google apps hidden, install restrictions except the updater session, a windowed system update policy outside service hours, Play Protect), describe the target and mark it `EV-13`, because PD230 is not built yet (P8aD9).
2. `docs/android/dossier/SCHEMA_ASSEMBLAGE_FR.md`: derived from `86088e4`. Replace the Linux terminal (Penetek/Advantech, Debian, Cage) by the D3 Pro on Ethernet or Wi-Fi, keep the Epson TM-m30III on LAN port 9100 and the FDM over LAN, keep the export port sentence of X1-T10, update ports (no listening port on the till), the IP plan, the physical installation, the reassembly and the test machine variant (emulated profile, `EV-15`).

Allowed regex: `docs/android/dossier/(PLATEFORME_ANDROID_FR|SCHEMA_ASSEMBLAGE_FR)[.]md`.

### P8a-T3

1. `docs/android/dossier/GARANTIE_MAJ_LANGUES_FR.md`: derived from `86088e4`, same sections. §1.1 build reproductible: the container build (`android/docker/`, `scripts/android/check_repro.sh`, `docs/android/REPRO.md`), two builds of the unsigned APK compared byte for byte, the image digest `EV-05`, signing afterwards with the pinned `apksigner`, RSA key, v2 and v3 only (DECISIONS §9). §1.2 empreinte: SHA-1 of the signed APK of record `EV-01`, SHA-1 of `libpos_ffi.so` `EV-02`, certificate digest `EV-03`; how to check on a till (About screen, PLATFORM.md §6, and the identity file of the SPF export), and independently (download the public declared artifact `declared-android` and hash it; rebuild from the tag). §1.3 environnement verrouillé: device owner and Lock Task, cite `PLATEFORME_ANDROID_FR.md` §3. §1.4 tags: `android-v1.0.0` and the engine tag `EV-04`. §2.1 mises à jour: the declared-only updater as built (PLATFORM.md §8 and §9; DECISIONS §6, Q8): manager PIN, after the Z, no open order and no sign in flight, sources GHCR anonymous pull or USB stick through SAF, minisign then SHA-1 and SHA-256 then versionCode strictly greater then same signing certificate, PackageInstaller session, backup before install, audit rows, no rollback (Android refuses a downgrade without an uninstall, and an uninstall deletes the fiscal database). Keep the urgent-fix paragraph of the Linux answers. §2.2 accès à distance: none on Android (DECISIONS §7), support on site. §3 languages: identical rule (engine), keep the Linux text, adapt only UI mentions. §4 recap.
2. `docs/android/dossier/PROFILS_UTILISATEURS_FR.md`: derived. Remove the RustDesk right (no remote support on Android), replace `SettingsView.vue` and "Config" by the Android settings entry (French label from `android/app/src/main/res/values/strings_ui_settings.xml`), add the manager-only platform entries (export SPF, mise à jour, export nocturne) with their labels from `strings_plat_*.xml` and `strings_nightly.xml`, and the About entry visible to everyone (`docs/android/LANES.md` "Platform routes").
3. `docs/android/dossier/FLUX_REMBOURSEMENT_FR.md`: derived. "La commande Tauri `refund_checkout`" becomes the engine command of the same name called by the Android interface through the FFI (`docs/android/FFI.md` §11); keep every rule.

Allowed regex: `docs/android/dossier/(GARANTIE_MAJ_LANGUES_FR|PROFILS_UTILISATEURS_FR|FLUX_REMBOURSEMENT_FR)[.]md`.

### P8a-T4

1. `docs/android/dossier/SECURITE_BD_FR.md`: derived from `86088e4`. §1 and §2 (SQLite and application mechanisms) stay, they are the engine. §3 système is rewritten for Android with the Linux numbering: 3.1 stockage privé de l'application (Android sandbox, one UID, `files/engine`), 3.2 propriétaire de l'appareil (instead of the service user), 3.3 environnement verrouillé (instead of the sealed image; cite `PLATEFORME_ANDROID_FR.md` §3), 3.4 pas d'élévation de privilèges (no root, debugging off in release builds), 3.5 démarrage (Android Verified Boot of the vendor image: `EV-15`, never asserted), 3.6 chiffrement du stockage (Android file-based encryption: `EV-15` for the real device). §4.2 hash of the APK (cite `GARANTIE_MAJ_LANGUES_FR.md` §1.2). §5 sauvegardes: the post-Z backup as built (one full copy after every Z in `files/engine/backups/`, none deleted, PD219, `EV-19`), the 7 newest copied by the SPF export, no cloud backup (`allowBackup=false`, `android/app/src/main/res/xml/data_extraction_rules.xml`), the nightly export (cite `EXPORT_NOCTURNE_FR.md`). §6 GDPR stays. §7 audit trail: add the platform audit log (`audit.jsonl`, PLATFORM.md §4). New §8 "Accès du SPF Finances aux données (Art. 16 et 102)": the canonical text other annexes cite: read-only rights of the operator, the manager's SAF export to a USB stick or SD card (PLATFORM.md §7: `pos.db` consistent copy, the 7 newest backups, logs, audit, identity file, `LISEZMOI.txt`, `MANIFEST.sha256` checked on the device after writing), what an inspector does on site, and that no other path to the data exists on a provisioned till.
2. `docs/android/dossier/FEUILLE_ACQUISITION_DONNEES_FR.md`: derived from `86088e4` (it already holds the export section B). Replace the Linux access paths (console, `/var/lib/pili-pos`) by the SAF export of `SECURITE_BD_FR.md` §8; keep section B on the internal software connection for statistics and backup, adapted to Android (cite `EXPORT_NOCTURNE_FR.md`).

Allowed regex: `docs/android/dossier/(SECURITE_BD_FR|FEUILLE_ACQUISITION_DONNEES_FR)[.]md`.

### P8a-T5

1. `docs/android/dossier/EXPORT_NOCTURNE_FR.md`: derived from `86088e4` (annex 34 there). The export is engine code, identical on Android (`crates/pos-export`, `crates/pos-export/STATS_SCHEMA.md`, DECISIONS X1D1 to X1D29). Adapt: where the local copies live (`files/engine/exports/` in the app-private storage, never on the SD card), the Android status screen (manager-only Settings entry "Export nocturne", package `nightly`, labels from `android/app/src/main/res/values/strings_nightly.xml`), TLS (bundled roots, DECISIONS §4), the configuration carried by the signed provisioning bundle. Keep the legal framing of X1-T10 unchanged (the export is a copy, the legal record stays local). The test machine status is `EV-20`.
2. `docs/android/dossier/FLUX_DONNEES_FR.md`: derived. Replace the Tauri IPC layer by the Compose interface calling the engine through UniFFI (`docs/android/FFI.md`), keep every fiscal flow, add the provisioning bundle import and the SPF export as data flows, keep the export flows of X1-T10.
3. `docs/android/dossier/COMMANDES_EN_LIGNE_FR.md`: derived. HubRise is the same engine code; adapt only the connection step on Android (QR code plus typed code, `docs/android/parity/settings.md`) and the online orders board (`docs/android/parity/webshop.md`).

Allowed regex: `docs/android/dossier/(EXPORT_NOCTURNE_FR|FLUX_DONNEES_FR|COMMANDES_EN_LIGNE_FR)[.]md`.

### P8a-T6

`docs/android/dossier/REPONSES_MANUEL_CERT_FR.md`: derived from `86088e4`, a linear pass of the manual as the Linux file is. Read the wave 1 annexes on the lane first and keep every answer consistent with them. Change: the title and identity block (rule 3.5); §I.D the hash (`EV-01`, cite `GARANTIE_MAJ_LANGUES_FR.md` §1.2); §I.E and §I.F unchanged in substance; §II.A identification (posSwVersion `pili-pos-android/1.0.0`, versionCode, the specifications followed are the same); §II.B database access declaration (operator read-only, SPF access through `SECURITE_BD_FR.md` §8); §II.C storage location; §II.D product guarantee (reproducible APK, About screen, public declared artifact, device owner; no `sha1sum /opt/...`); §II.E updates (declared-only updater, no RustDesk, on-site access for the SPF agent at the intake: the manager PIN screens and the SPF export); §II.F hardware table (D3 Pro GMS variant, Epson TM-m30III LAN, module2; test machine = emulated profile unless a real D3 Pro, `EV-15`); §II.G assembly; §III configuration (provisioning bundle instead of console and `pos.json` editing; users and roles as `PROFILS_UTILISATEURS_FR.md`); §III.I terminals; §VII local data and cloud question as X1-T10 qualified it; §VIII signature unchanged; the "ANNEXES DU DOSSIER" table becomes the Android table of section 4 (annex numbers 0 to 36, common documents marked "commun"). Keep the fiscal answers (§III.A to §III.H, §IV, §V, §VI) as they are unless they name a Linux element. Do not add any question to the SPF. Budget 600 diff lines against the source.

Allowed regex: `docs/android/dossier/REPONSES_MANUEL_CERT_FR[.]md`.

### P8a-T7

1. `docs/android/dossier/DOCUMENT_PROCESSUS_TECHNIQUE_FR.md`: derived. New overview diagram (same box style): Compose UI, Kotlin `EngineGateway`, UniFFI over JNA, `libpos_ffi.so` = `pos-ffi` + `pos-engine` + cert crates, SQLite in app-private storage, FDM, Epson. Sources: `docs/android/ENGINE.md`, `FFI.md`, `APP.md` §2, `BUILD.md`. State the rules that keep fiscal logic in the engine (no money or fiscal rule in Kotlin, the engine owns the P event on leaving an order, the idle lock, clock-in and clock-out, FDM polling: ENGINE.md §7). Keep the X1-T10 export section.
2. `docs/android/dossier/LISTE_FONCTIONNALITES_FR.md`: derived. Mark what the Android interface offers using `docs/android/PARITY.md` (75 commands called, 9 excluded, with the reasons) and the remote support absent.
3. `docs/android/dossier/DECLARATION_FONCTIONS_INTERDITES_FR.md`: derived. Keep every engine guarantee; replace the Linux system measures by the Android ones (device owner restrictions, no sideload, no debugging in release, no remote support, no cloud backup), citing `PLATEFORME_ANDROID_FR.md` §3 and §7.
4. `docs/android/dossier/DECISION_PEPPOL_AP_FR.md`: derived. Peppol is absent from the Android 1.0 build too: `android/rust/pos-ffi/Cargo.toml` builds `pos-engine` with `default-features = false, features = ["cert"]`; the I event is not available.

Allowed regex: `docs/android/dossier/(DOCUMENT_PROCESSUS_TECHNIQUE_FR|LISTE_FONCTIONNALITES_FR|DECLARATION_FONCTIONS_INTERDITES_FR|DECISION_PEPPOL_AP_FR)[.]md`.

### P8a-T8

1. `docs/android/dossier/MANUEL_INSTALLATION_FR.md`: the Android installation manual, header of a derived document (source `MANUEL_INSTALLATION_FR.md`). Steps: unbox and connect (Ethernet or Wi-Fi, Epson on the LAN); provisioning on a device with no Google account (`adb shell dpm set-device-owner` for the producer's bench, QR provisioning after a reset for the field: 6 taps on the welcome screen, payload with the APK URL and its SHA-256), never sign in to a Google account; first start and the signed USB bundle import (`docs/android/APP.md` §3, the format built by `scripts/android/make_dev_bundle.sh`: `pos.json`, FDM token, mTLS material, optional SPF test seed; signature checked with minisign; the engine refuses to start without a configuration); deviceId recorded (cite `PLATEFORME_ANDROID_FR.md` §4); the configuration fields of `pos.json` (reuse the Linux tables of the source, they are the same engine configuration, including the export section of X1-T10); printer and FDM checks; the lock-down that follows (cite `PLATEFORME_ANDROID_FR.md` §3); the SD card recommendation (cite §5, same figures); what the installer must never do (factory reset of a till in service, uninstall: it deletes the fiscal database).
2. `docs/android/dossier/CHECKLIST_DEPLOIEMENT_FR.md`: new (replaces `docs/SUBMISSION_ARTIFACTS/CHECKLIST_DEPLOIEMENT_KIOSK_FR.md`), same layers: hardware, Android and device owner, application, documentation; the verification steps on the producer's machine (repro check, `declare_android.sh` dry run, never a real declaration without the user) and on the test machine (About screen hashes against `EV-01` and `EV-02`, policies, SPF export, update path); the demo script for the intake appointment.

Allowed regex: `docs/android/dossier/(MANUEL_INSTALLATION_FR|CHECKLIST_DEPLOIEMENT_FR)[.]md`.

### P8a-T9

`docs/android/dossier/MANUEL_UTILISATEUR_FR.md`: derived from `86088e4`. Same flows (the engine and the fiscal rules are the same), described with the Android screens: login with PIN, tables, order, checkout, close unpaid, refund, reports X and Z, settings, online orders, nightly export status, About, SPF export, update. Use the exact French labels of `android/app/src/main/res/values/strings_*.xml` (generated `strings_vue_*.xml` are the Vue texts and are the same as Linux), and `docs/android/parity/*.md` for what each screen does. Remove what Android does not have (remote support, keyboard shortcuts if any). Each screen capture reference becomes `À effectuer (EV-09 : capture A<nn> <écran>)`. Captures are numbered `A01` upward in the order of the sections of the Linux user manual at `86088e4`, one id per distinct Android screen state; T10 numbers the capture list of `internal/CAPTURES_ECRAN_RUNBOOK_FR.md` by the same rule in parallel, and T12 reconciles the two. Budget 600 diff lines against the source.

Allowed regex: `docs/android/dossier/MANUEL_UTILISATEUR_FR[.]md`.

### P8a-T10

1. `docs/android/dossier/RAPPORT_TESTS_BASELINE_FR.md`: the Android test report, header of a derived document. What exists today, with the figures of the exit reports (cite the report and commit): Rust workspace and cert suites of the shared engine (X1-exit: ws 1627 passed, cert pos-engine 591, inventory ws 1640 >= 1355), `android/rust` 242 tests, FFI command parity 86/86, UI parity 75 called and 9 excluded of 84, Kotlin inventory 1957 at `602544c`, the engine test subset on the emulator (`docs/android/DEVICE_TESTS.md`), the end-to-end runs on the emulator (`app_e2e` 11 checks, `plat_e2e` 19 checks, P6-exit). Final counts at the release commit: `EV-17`.
2. `RESULTATS_TESTS_UC_FR.md`, `JOURNAL_TESTS_PRE_SOUMISSION_FR.md`, `TICKETS_EXEMPLES_FR.md`: skeletons with the derived header, keeping the Linux structure (headings, table headers, the list of UC ids of the three sets from the Linux file) with one placeholder per block (`EV-06`, `EV-07`, `EV-10`, `EV-14`, `EV-08`, `EV-16`). Do not copy Linux results or tickets as if they were Android ones.
3. `COUVERTURE_ADVANCED_SET_FR.md`: derived; the engine coverage is the same; replace the Vue component lines by the Android screens per `docs/android/PARITY.md` and `docs/android/parity/*.md`.
4. `docs/android/dossier/internal/CAPTURES_ECRAN_RUNBOOK_FR.md`: the capture list for P8b on `sunmi_d3_pro_gms_api33`, derived structure from `docs/internal/CAPTURES_ECRAN_RUNBOOK_FR.md` (header line 1 `<!-- Dérivé de docs/internal/CAPTURES_ECRAN_RUNBOOK_FR.md @ 86088e4 -->`), ids `A01` upward in the order of the sections of the Linux user manual at `86088e4` (`git show 86088e4:docs/SUBMISSION_ARTIFACTS/MANUEL_UTILISATEUR_FR.md`), one id per distinct Android screen state (T9 numbers its references by the same rule, T12 reconciles), file names `A01_<ecran>.png`, with the state to prepare for each.

Allowed regex: `docs/android/dossier/(RAPPORT_TESTS_BASELINE_FR|RESULTATS_TESTS_UC_FR|JOURNAL_TESTS_PRE_SOUMISSION_FR|TICKETS_EXEMPLES_FR|COUVERTURE_ADVANCED_SET_FR)[.]md|docs/android/dossier/internal/CAPTURES_ECRAN_RUNBOOK_FR[.]md`. The derived-header check of 7.1 for the runbook uses `docs/internal/` instead of `docs/SUBMISSION_ARTIFACTS/`.

### P8a-T11

1. `docs/android/dossier/MANUEL_PROGRAMMATION_FR.md`: derived. The engine configuration is the same `pos.json`; on Android it is written by the producer into the signed provisioning bundle and imported (cite `MANUEL_INSTALLATION_FR.md`, now on the lane); the screens that change settings at run time are the Android Settings (`docs/android/parity/settings.md`, labels from `strings_ui_settings.xml`). Remove console and file-editing steps.
2. `MAPPING_CONFORMITE_FR.md`: derived; keep the engine rows; add rows for Art. 16 and 102 (SPF access, `SECURITE_BD_FR.md` §8), Art. 47 (version mention), Art. 112 (hash, `GARANTIE_MAJ_LANGUES_FR.md` §1), FAQ Q505 (deviceId, `PLATEFORME_ANDROID_FR.md` §4).
3. `LIGNES_PRODUITS_FR.md`: derived; the SPF test seed travels in the provisioning bundle (optional SPF test seed, `docs/android/APP.md` §3) instead of the image build; the product list is unchanged.

Allowed regex: `docs/android/dossier/(MANUEL_PROGRAMMATION_FR|MAPPING_CONFORMITE_FR|LIGNES_PRODUITS_FR)[.]md`.

### P8a-T12

Consistency pass over the whole Android dossier, after every other card is merged.

1. Run the cross-reference check 7.2 and fix every dangling name.
2. Check that these facts are written the same way everywhere (grep each): `pili-pos-android/1.0.0`, versionCode `10000`, `dev.nuagemagique.pilipos`, the deviceId rule, `files/engine/pos.db`, the 7 newest backups in the SPF export, the SD card recommendation (64 GB, 128 GB, exFAT), the number of device-owner restrictions as in `PolicySet.kt`, the annex numbers of section 4, the EV ids. Fix divergences in the dossier files only.
3. Complete the "Cited in" column of `docs/android/DOSSIER_EVIDENCE.md` from `grep -rnoE 'EV-[0-9]{2}' docs/android/dossier`; list any registry id that no document cites.
4. Check every document of section 4 exists (A and P kinds), and that `MANIFEST.txt` and `INDEX_DOSSIER_FR.md` list exactly them plus the common documents.
5. Reconcile the capture ids `A<nn>` of `MANUEL_UTILISATEUR_FR.md` with `internal/CAPTURES_ECRAN_RUNBOOK_FR.md`: every id cited in the manual is in the runbook and the reverse, same screen for the same id.
6. Read every document once for French quality and brand neutrality; no rewrite of content that is consistent.

Allowed regex: `docs/android/dossier/.*|docs/android/DOSSIER_EVIDENCE[.]md`.

## 11. Risks and open points for the user

- **X1-T10 is not merged.** The derived documents follow its tip `86088e4`. If its re-review changes text, the Android files must be re-derived (procedure in `DOSSIER_EVIDENCE.md`).
- **Common documents not copied (P8aD4).** The Android ZIP would contain six Linux PDFs whose few Linux passages are read through `INDEX_DOSSIER_FR.md` §3. If the user prefers a self-contained Android dossier, P8b copies them with the same edits.
- **GMS measures (PD230) and backup retention (PD219)** are described as target or as built with placeholders; both must be settled before filing.
- **Independent hash on the till.** On a release till debugging is off, so the APK hash on the device is shown by the app itself (About screen, SPF export identity file). The independent checks are the public declared artifact and the reproducible build. If the user wants an on-device check outside the app, it needs a decision (for example a documented intake procedure with debugging re-enabled by the app under manager PIN).
- **Hardware residue (`EV-15`)** stays open until a real D3 Pro is available; the dossier then states that tests ran on an emulated profile.
- **Dossier legal framing** (two initial applications, the export as a copy) is the producer's reading, open for the user's review; nothing is sent to the SPF.
