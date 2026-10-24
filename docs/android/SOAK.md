# Soak of the Android port (P7-T12)

Run `rc1-soak-3`, 2026-10-14 23:10 to 2026-10-15 09:10 UTC (10 h), on the
`sunmi_d3_pro_api33` AVD (emulator-5554), started by
`scripts/android/p7_soak.sh start --hours 10 --run-id rc1-soak-3`.

| Item | Value |
|------|-------|
| build | debug APK of `android/int` at `f0d60192` (engine frozen as `linux-v1.0.0-rc1`) |
| mix | 60 min per compressed service day, 100 sales a day, seed 589897940: 70 % takeaway paid in cash, 20 % table with order, pre-bill and card, 5 % split payment, 5 % refund; an X at mid-day, a 6 min idle gap (engine lock), work-out and Z at the end of each day |
| mocks | `pos-fdm-mock`, `pos-printer-mock` on the host (adb reverse) |
| host | MacBook on mains power, kept awake with `caffeinate -s` |
| raw report | `docs/android/uc/soak/rc1-soak-3_report.txt` |

## Result

| Check | Result |
|-------|--------|
| pid-stable | PASS, one process (pid 3526) over 593 samples |
| no-crash | PASS, 0 crash markers, 0 driver errors |
| sales-signed | 999 of 1000 sales signed, see below |
| counters-continuous | PASS, 1451 fiscal events, total counter 1 to 1451 |
| z-done | PASS, 10 Z by the driver, 0 failed, plus 1 automatic Z (11 Z signed by the FDM) |
| memory | PASS, PSS +0.38 MB/h, native heap +0.20 MB/h over 9.0 h (limit 5) |
| threads | PASS, baseline 36, max 41 |
| fds | PASS, baseline 135, max 138 |
| beats | PASS, 3562 heartbeats, largest gap 11 s (limit 120) |
| storage | PASS, free `/data` never under 6.13 GiB |
| min-hours | PASS, 10.00 h |

The one refused sale is the expected behaviour, not a defect. The
compressed days of the driver run on the real clock, and the business day
ends at 05:00 (`commands/report.rs`). At 05:00:27 a sale of the fourth
compressed day was refused with `previous_day_open`: the calendar day of
the open period was over and its Z not yet made. The automatic Z closed
that day (the eleventh Z signed by the FDM) and sales went on at once
(next sale signed at 05:01:00, 33 s later). In a restaurant nothing is sold at 05:00;
the check counts it as a failure because it compares sales one to one.

## Storage figures

| Figure | Value |
|--------|-------|
| `pos.db` | 0.32 MB at the start, 11.6 MB after 999 sales and 1451 events: about 11 KB per sale, refunds, pre-bills, presence and reports included |
| WAL | 4.2 MB at the end |
| engine backups | one per Z; retention keeps the 7 newest (`BACKUPS_KEPT`), the logs show the oldest deleted from the eighth Z on; 56 MB in the backup folder at the end |

At 300 sales a day that is about 1.2 GB of `pos.db` a year, and the 7
backups follow its size. The `backup_files` count of the sampler (29 at the
end) is not the number of engine backups: the folder listing also counts
other entries, and the app was uninstalled at teardown before they could
be listed. To check on the next run (`ls -la files/engine/backups`).

## Earlier runs

- `p7-24h` (2026-10-10): started for 24 h, stopped cleanly after 2.98 h
  (three compressed days, 300 sales signed, 3 Z, all 11 checks PASS), on
  the engine before the 7-backups retention. Its record, with the
  storage projections and the card size advice, is
  [uc/soak/p7-24h/README.md](uc/soak/p7-24h/README.md), next to its
  raw files.
- `rc1-soak` (2026-10-13): stopped at provisioning, `ProvisioningE2eTest`
  still expected the `-dev` version suffix; fixed in `f0d60192`.
- `rc1-soak-2` (2026-10-13 to 14): the Mac was on battery with the lid
  closed and slept despite `caffeinate -i`; 3 sales in 1.3 h, stopped.
  Every sale made was signed and the counters were continuous.

## Not covered here

- The 24 h length of the plan: this run is 10 h, above the 4 h acceptance
  (P7D12).
- A real D3 Pro and a real plug pull (EV-15).
- The 24 h soak on the real unit: `RUNBOOK_D3PRO.md` section 9.
