# The soak of the Android till (emulator, about 3 hours)

Card P7-T12 (phase P7, end-to-end parity). English. This page records the soak run `p7-24h` on the emulator
profile of the Sunmi D3 Pro, its figures, the pass rules of P7D12 and what the figures mean for the internal
storage and the SD card.

**Read this first.**

- **This is a soak of about 3 hours (2.98 h), not 24 hours.** The run was started for 24 hours and stopped
  cleanly after its third compressed service day. The cut was decided on 2026-10-10 at 15:05 by the orchestrator,
  under the authority the user gave on 2026-10-10 ("finish everything else", "Take all decisions that need to be
  taken until the end"): a 24 hour run would have held the only emulator until Sunday (DECISIONS.md, P7D22).
- **The 24 hour soak is an item of the user's release checklist** (section 7). It can run overnight on the
  emulator or on the real D3 Pro (`RUNBOOK_D3PRO.md` section 9).
- **The run is short of the 4 hour floor of P7D12.** `p7_soak.sh report --run p7-24h` passes all 11 checks;
  `--min-hours 4` would fail its `min-hours` check (2.98 h). P7D22 accepts the run at about 3 h.
- **The soak ran on the engine before the 7-backups retention** (being merged separately). The engine of this
  run keeps every post-Z backup, so the backup growth figure below is the pre-retention one; section 5 gives
  both the measured pre-retention growth and the projection with the retention.

## 1. The run

| Item | Value |
|------|-------|
| Run id | `p7-24h` (`target/android-soak/p7-24h/`, evidence copied to `docs/android/uc/soak/`) |
| Start, end (local time, Europe/Brussels) | 2026-10-10 12:07:59, 15:06:49 |
| Covered | 10,723 s of sampling, 2.98 h; the driver ran 2.98 h (178.6 min) |
| How it ended | `p7_soak.sh stop --run p7-24h`: the driver had closed its third day with a Z and ended with `reason stop`; teardown `ok` |
| Emulator | `emulator-5554`, AVD `sunmi_d3_pro_api33` (`google_apis`, API 33, arm64, 4 GB), started by the chain, kept running after the soak |
| App | debug APK of the branch head `6d2837b` (engine at the lane base, before the backup retention), device owner, development bundle |
| FDM and printer | `pos-fdm-mock` and `pos-printer-mock` on the host, `adb reverse` on 19380, 19390, 19391 |
| Profile | 60 minutes per compressed day, 100 sales a day, idle gap 6 minutes, seed `972847343`, warm-up 3,600 s |
| Command | `bash scripts/android/p7_soak.sh start --hours 24 --run-id p7-24h` |

Teardown after the stop, checked on the emulator: no device owner, no `pilipos` package, no `adb reverse` rule,
no host mock and no sampler left, `stay_on_while_plugged_in` back to its value before the run (7), the P7
emulator lock released.

## 2. What the driver did

Three full compressed days, each: clock-in (work in), 100 sales, an X report at minute 26, an idle gap in which
the engine locked the session (after 297 to 302 s), login again, work out and Z at minute 53.

| | Day 1 | Day 2 | Day 3 | Total |
|---|------:|------:|------:|------:|
| Takeaway, cash (1 to 3 lines) | 70 | 70 | 70 | 210 |
| Table: order, pre-bill, then card | 20 | 20 | 20 | 60 |
| Split payment | 5 | 5 | 5 | 15 |
| Refund of an earlier sale | 5 | 5 | 5 | 15 |
| Sales signed (driver `ok`) | 100 | 100 | 100 | **300, 0 failed** |
| X report | 1 | 1 | 1 | 3 |
| Idle lock | 1 | 1 | 1 | 3 |
| Z report (report number) | 1 | 2 | 3 | **3, 0 failed** |

FDM operations seen by the mock (`uc/soak/fdm_operations.json`): `signSale` 300, `signOrder` 60, `signPreBill`
60, `signWorkIn` 3, `signReportTurnoverX` 3, `signWorkOut` 3, `signReportTurnoverZ` 3: 432 signing calls, plus
318 status calls. The FDM event counters run from 1 to 435 with no gap or double (the 432 signing calls and
3 more events counted by the engine). The journal reader made 531 polls, 0 overflows.

## 3. Results against the pass rules (P7D12)

Final report (`uc/soak/report_final.txt`, identical to the report taken just before the stop,
`uc/soak/report_before_stop.txt`, apart from one sample and one beat more):

| Check | Rule | Result | Verdict |
|-------|------|--------|---------|
| pid-stable | one pid all along | pid 11268 in all 177 samples of the run | PASS |
| no-crash | no crash or ANR | 0 markers in 178 samples, 0 driver errors | PASS |
| sales-signed | every sale signed | driver 300 ok, 0 failed; FDM `signSale` 300 | PASS |
| counters-continuous | counters continuous | 435 events, total counter 1..435 | PASS |
| z-done | one Z per elapsed day | 3 Z ok, 3 days started, FDM Z signed 3 | PASS |
| memory | PSS and native heap slope after the first hour at most 5 MB/h | PSS +1.95 MB/h, native heap +1.16 MB/h over 2.0 h | PASS |
| threads | at most baseline plus 20 | baseline 35, max 43, growth 8 | PASS |
| fds | at most baseline plus 50 | baseline 135, max 135, growth 0 | PASS |
| beats | largest heartbeat gap at most 120 s | 1,058 beats, largest gap 10 s | PASS |
| storage | free internal storage at least 1 GiB | free `/data` min 6.11 GiB | PASS |
| min-hours | none asked (the run is about 3 h) | 2.98 h covered | PASS |
| **total** | | **0 failed (11 checks)**, teardown ok | **PASS** |

All 12 checkpoints (one every 15 minutes, `uc/soak/checkpoints.jsonl`) were green.

The one sample taken after the driver ended (15:06:47) shows a new pid (20670): the end of the instrumentation
stops the app process. The rule counts the samples up to the driver's end, as written in `soak_sampler.py`.

## 4. Memory, threads and descriptors

From `uc/soak/samples.jsonl` (one sample a minute, `dumpsys meminfo` in kB, shown here in MB of 1,024 kB):

| Metric | First sample | Lowest | Highest | Last sample of the run |
|--------|-------------:|-------:|--------:|-----------------------:|
| PSS | 126.2 MB | 121.5 MB | 134.5 MB | 134.4 MB |
| Native heap | 20.5 MB | 20.0 MB | 29.0 MB | 29.0 MB |
| RSS | 296.2 MB | 291.8 MB | 304.7 MB | 304.5 MB |
| Threads | 43 (start-up) | 34 | 43 | 34 |
| Open fds | 135 | 135 | 135 | 135 |

- **Each Z raises the native heap by about 4 MB**, which stays until the next day starts (the post-Z backup copy):
  22.3 to 26.4 MB at the first Z, 23.8 to 28.4 MB at the second, 24.0 to 29.6 MB at the third. The level at the
  same point of each day rises by about 1.5 MB a day (22.2, 23.8, 24.1 MB at minute 52), and so does PSS (127.3,
  130.5, 132.2 MB at minute 46). The slopes are inside the limit, but **three hours cannot show whether they flatten**:
  at 1.95 MB/h a 24 hour run would add about 47 MB of PSS. This is the first figure to read in the 24 hour soak.
- Threads go back to 34 or 35 after start-up and stay there; the descriptors never move (135).

## 5. Storage: bytes per sale, bytes per Z, and the projected card size

### 5.1 Measured

| Item | Value |
|------|-------|
| `pos.db` at the start | 294,912 B (WAL 325,512 B) |
| `pos.db` file at the end | 3,383,296 B (WAL 4,132,392 B, not yet checkpointed) |
| Newest backup (a full copy of the database after the third Z) | 3,706,880 B |
| **`pos.db` growth per sale** | **11,373 B/sale** ((3,706,880 - 294,912) / 300, the sampler's rule) |
| Growth per compressed day of 100 sales | 1,130,496 B, 1,138,688 B and 1,142,784 B (from one backup to the next) |
| Backups after each Z (pre-retention) | 1 file 1,404 KiB; 2 files 3,912 KiB; 3 files 7,536 KiB |
| **Backup bytes per Z (pre-retention)** | **2,572,288 B/Z** on average over 3 Z (7,536 KiB / 3). Each Z adds one full copy of the database as it is then (1.43, 2.56 and 3.71 MB), so this figure is not a constant: it grows with the database |
| Free `/data` | 6.12 GiB at the start, 6.11 GiB at the lowest |

The 11,373 B/sale is the full cost of this mix: one to three lines a sale, the table orders and pre-bills, the
refunds, the split payments, the X, work in, work out and the Z of the day, all spread over the 100 sales of a
day. It sits inside the P6 range of 6,471 B (one-line sale) to 14,336 B (six-line sale) and replaces it as the
"realistic mix" figure.

### 5.2 Projected (100 or 300 sales a day, one Z a day, this mix)

`pos.db` grows by about 1.14 MB a day at 100 sales (3.41 MB at 300), so:

| After | `pos.db`, 100 a day | `pos.db`, 300 a day |
|-------|--------------------:|--------------------:|
| 1 year | 0.42 GB | 1.25 GB |
| 2 years | 0.83 GB | 2.49 GB |
| 3 years | 1.25 GB | 3.74 GB |
| 5 years | 2.08 GB | 6.23 GB |

**Internal storage, engine of this run (no retention).** Every Z adds a full copy of the database, so the backups
grow with the square of the days: about 0.5 GB after 30 days, 4.7 GB after 90 days, 18.5 GB after 180 days and
76 GB (70.8 GiB) after a year at 100 sales a day; 1.6 GB, 14 GB, 56 GB and 228 GB at 300 a day. The 6.1 GiB free on
the emulator would be full after about 107 days at 100 a day (61 at 300); 32 GiB after about 245 days (141 at 300).
This agrees with the P6 estimate (40 to 268 GiB a year, 32 GiB between day 126 and day 325) and confirms that the
retention, not the card, was the storage risk.

**Internal storage with the 7-backups retention** (being merged separately, not measured by this run): the
backups stay at about 7 database sizes, so the engine data is about 8 times `pos.db`: about 3.3 GB after a year at
100 sales a day and 10 GB at 300.

**The SD card (export set).** One SPF export is `pos.db` plus the 7 newest backups plus logs and the manifest,
about 8 times `pos.db`, with or without the retention:

| Export set after | 100 a day | 300 a day | Exports that fit on 64 GB (100 / 300 a day) | on 128 GB |
|------------------|----------:|----------:|------------------------------------------:|----------:|
| 1 year | 3.3 GB | 10.0 GB | 19 / 6 | 38 / 12 |
| 2 years | 6.6 GB | 19.9 GB | 9 / 3 | 19 / 6 |
| 3 years | 10.0 GB | 29.9 GB | 6 / 2 | 12 / 4 |

The app never deletes an old export from the card, so the manager removes old sets. The recommendation of P6 and
P7D17 stands: a **64 GB** microSDXC card, high endurance (or A1/U3), from a known brand, **formatted exFAT**, for
the first year at about 100 sales a day; **128 GB** at about 300 sales a day or from the second year. exFAT is
required: at 300 sales a day `pos.db` itself passes 4 GiB, the FAT32 file limit, after about 3.4 years (about
10 years at 100 a day). The database and its backups stay in app-private internal storage, never on the card
(PD220).

## 6. What this run does not prove

- **24 hours.** Whether the PSS and native heap slopes flatten, and whether anything accumulates over 24 Z, is not
  shown by 3 Z. The 24 hour soak is a release checklist item (section 7).
- **The 7-backups retention.** The engine of this run predates it; the projection with the retention is computed,
  not measured.
- **The real unit.** Deep sleep, thermal behaviour, the real FDM latency and the real Epson are hardware residue
  (`RUNBOOK_D3PRO.md` sections 9 and 11). The emulator's CPU is never suspended, so the 10 s heartbeat gap says
  nothing about the unit.
- **GMS.** The run used the `google_apis` AVD only; nothing here depends on the GMS profile (P7-T11 not delivered).

## 7. For the user: the 24 hour soak (release checklist)

Run once before the release, with the 7-backups retention merged, on either target:

- **Emulator, overnight.** Nothing else may use the emulator meanwhile (other emulator scripts wait on the P7
  lock):
  ```
  bash scripts/android/p7_soak.sh start --hours 24 --run-id p7-24h-full
  bash scripts/android/p7_soak.sh status                 # any time
  bash scripts/android/p7_soak.sh report --run p7-24h-full --min-hours 24
  ```
  The run stops by itself after 24 hours and tears down; `p7_soak.sh stop --run p7-24h-full` ends it early and
  keeps the partial report.
- **Real D3 Pro.** `RUNBOOK_D3PRO.md` section 9 (the same driver by hand, host sampling every minute).

Pass: the same 11 checks with `--min-hours 24` and 24 Z. Read first the memory slope over 23 hours and the native
heap after each Z (section 4), then the backup total, which with the retention must stop growing after the
seventh Z.

## 8. Evidence (`docs/android/uc/soak/`)

| File | What |
|------|------|
| `report_final.txt` | the report printed by `p7_soak.sh stop`, with the teardown line |
| `report_before_stop.txt` | the report taken just before the stop |
| `checkpoints.jsonl` | the 12 checkpoints, one every 15 minutes |
| `samples.jsonl` | the 178 samples (pid, PSS, native heap, RSS, threads, fds, `pos.db`, WAL, backups, free `/data`) |
| `driver.jsonl` | the driver's own events (start, days, sales, X, lock, work out, Z, beats, end) |
| `fdm_operations.json` | the FDM operations of the mock journal, counted (the 1.2 MB journal stays in `target/`) |
| `journal_meta.json` | the journal reader's polls, status calls and overflows |
| `run.json` | the run state (ports, pids, seed, start and end), without the temporary bundle path |
