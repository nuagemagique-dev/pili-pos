# P8a exit report (documentation lane)

Written by the P8a planner on 2026-10-10, with the user away. Lane branch `android/lane-p8`, worktree `moyun-pos-wt/lane-p8`, base `android/int` at `602544c`. Lane head before this report: `f3c2003` (`docs(android): P8a phase cards and planner decisions for the Android dossier`).

## 1. Summary: the phase did not run

The orchestration script reached the P8a exit with an empty result list. No worker card of `P8a.md` section 9 was executed:

- no branch `android/P8a-P8a-T<n>` exists;
- `docs/android/dossier/` does not exist;
- `docs/android/DOSSIER_EVIDENCE.md` does not exist.

P8a is therefore **not done**. Its cards and briefs (`P8a.md`, decisions P8aD1 to P8aD18) are intact and ready to be run as written.

The computed exit task also asked to overwrite `P8a.md` with the text `PLACEHOLDER`. That was not done: it would have erased the 372-line phase plan the next run needs. `P8a.md` is kept as committed in `f3c2003`.

## 2. Per task status

| Card | Status | Verdict |
|------|--------|---------|
| P8a-T1 to P8a-T5 (wave 1) | not started | none |
| P8a-T6 to P8a-T8 and the rest of wave 2 | not started | none |
| Later waves of `P8a.md` §9 | not started | none |

## 3. Gate evidence

- `git diff --stat 602544c HEAD` (before this report): only `docs/android/DECISIONS.md` (+19) and `docs/android/phases/P8a.md` (+372). The lane changed no code, script, build file or frozen path.
- `bash scripts/android/check_linux_frozen.sh --ref 602544c`: `frozen against ref 602544c: 17 paths, clean`.
- Build: the lane head is `602544c` plus documentation only, so it builds exactly as `android/int` at `602544c` does. No build was started (no code changed, RAM shared with the P7 lane).
- The dossier gates of `P8a.md` §7 (`check_dossier`, brand and em-dash checks, placeholder registry) were **not run**: there is no dossier to check. They are the gate of the rerun.

## 4. Freeze step

None. The plan sets no freeze at the end of P8a, and the re-freeze `linux-v1.0.0-rc1` is the user's (PD121, X1D27, PD228). No tag was created. `docs/android/LINUX_FREEZE.md` belongs to the P7 lane scope (`P8a.md` §1) and was not edited.

## 5. The relayed SD card question

The user's question ("it will be on what the SQLite will be stored? how big should be the SD card?") is answered in `P8a.md` §8, checked again against the code for this report:

- the SQLite database is `files/engine/pos.db` in the app-private internal storage (`ProvisioningPaths.kt`, `dataDir = filesDir/engine`), never on the card; the manifest sets `allowBackup="false"`, has no `installLocation` and asks for no storage permission;
- the card is only the target of the SPF export and the source of a USB or SD update, reached through a SAF grant;
- size, from the measures of `PLATFORM.md` §12 (6,471 bytes per one-line sale, 14,336 per six-line sale): a microSDXC card, high endurance, A1 or U3, exFAT, **64 GB** at about 100 sales a day for the first year, **128 GB** at about 300 sales a day or from the second year;
- the internal storage fills up before the card, because every Z keeps a full copy of `pos.db` (PD219, `EV-19`).

## 6. Decisions taken while the user was away

No new decision at this exit. The planner decisions of the phase are P8aD1 to P8aD18 (`DECISIONS.md`, committed in `f3c2003`). Not overwriting `P8a.md` is recorded here rather than as a decision, because it only keeps the existing plan.

## 7. Risks

- P8a output is missing: P8b (PDFs, ZIP) cannot start until the P8a cards run.
- The integrator merging `android/lane-p8` into `android/int` gets the phase plan and this report, no dossier. That merge is harmless but should not be read as "P8a done".

## 8. What the next phase must know

- Rerun P8a from `P8a.md` §9, wave 1, on the base `android/lane-p8` head (this commit). Nothing needs undoing.
- The Linux source for every derived document is still `86088e4` (`P8a.md` §2).
- `android/lane-p8` is not broken: docs only on top of `602544c`, frozen paths clean.
