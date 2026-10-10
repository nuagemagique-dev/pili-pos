# Live run against the module2 simulator (Android)

Card P7-T12, decision P7D13. English. **Prepared, never executed by the chain.** The user was away: no call to
module2 was made, the script below was only tested with a stub and a fake simulator on the loopback
(`scripts/android/test_uc_live_module2.sh`).

## What the live run proves

The Android till (the real app on the Sunmi D3 Pro profile, the Rust engine through UniFFI) signs the use-case
scenarios against an FDM implementation that we did not write: the module2 online simulator `FDM02000425`, the same
one the Linux till was tried against on 2026-09-27 (`docs/internal/LIVE_FDM_MODULE2.md`). The offline parity proof
(`UC_RESULTS.md`) shows that the Android till and the engine on the host send the same requests to our own mock;
this run shows that a third-party FDM accepts what the Android till sends and that its answers, once the till has
stored them, give the same business content as the reference.

What is compared: per scenario, the steps (every step meets its expectation), the signing operations and their
order, the business content of every request (`business_view`), the full request once the volatile paths of
`UC.md` section 7 are removed, the printed tickets after the normalisation of `UC.md` section 8, and, for the
scenarios that name an official fixture, the request against the fixture (`--fixtures`). What is **not** compared:
the counters (`--no-counters`). The simulator is a shared service with its own counters; they continue from
whatever happened before and from the Linux till's runs, so the counters of an Android run cannot equal those of
the reference. The comparison checks `fdm-counters` only in the offline run, where the mock starts at zero.

Two kinds of difference are expected and are reported, not hidden:
- `10_basic/140_Q110` (the multi-language Status query) fails against the simulator: module2 answers the query
  with several `status` fields in one request with HTTP 500 and an empty body (`LIVE_FDM_MODULE2.md`, recording
  notes), and the warning it sets up (`fdm_control`) can only be switched in the simulator's Developer tab. The
  script lists it as an expected live FAIL and does not count it in the total.
- Anything the simulator answers differently from the mock but the till accepts (the placeholder signature
  `dummy_` plus base64, the extra top-level `info` object, `initialized: false` in the status): the till is built
  to accept exactly that form, so these do not appear as differences in the comparison.

The error group `60_fdm_errors` cannot run live: its scenarios switch the simulated errors (buffer full, FDM locked,
unknown POS, unauthorized) on and off, and the simulator does that only from its Developer tab. The script refuses
that group. The error checklist of `LIVE_FDM_MODULE2.md` remains the way to try those conditions by hand against
the Android till (one status query and one sale each, two requests).

## What the user decides first

The script cannot start without all of these:

1. **The go-ahead.** `--go-ahead` on the command line and `PILI_LIVE_GO_AHEAD=yes` in the environment. Both, always.
   It is the user's word that module2 may be called now. Without either the script refuses (exit 3) and does
   nothing. The chain never passes them.
2. **A free posId slot on the simulator for the Android till.** The Android bundle made by `make_dev_bundle.sh`
   carries `pos_id` `CPOS0031234567`, `terminal_id` `POS-2-ANDROID`, the VAT and establishment numbers of the
   official test setup (`UC_TestSetup_20240806.md`), the same posId the Linux run used. The simulator's settings
   page (`settings.module2.be`, Allowed POS IDs) lists the posIds it accepts: this one must be on the list, and
   nobody else (the Linux till, a colleague, the intake test) may sign with it during the run, because the
   simulator's FDM counters and the ticket numbers belong to the posId. If a different posId must be used for the
   Android till, the bundle needs another `pos_id` and `make_dev_bundle.sh` has no option for it: ask for a card,
   do not edit the bundle by hand inside the script.
3. **The request budget.** The simulator is a free shared service: calls are made by hand, one at a time, and never
   from CI. The script caps the requests it forwards (`PILI_LIVE_MAX_REQUESTS` or `--max-requests`, default 60 as
   for `scripts/live_fdm.sh`), with a pause of 1 s between two of them (`PILI_LIVE_PAUSE_MS`). Status calls count.
   `--dry-run` prints the estimate from the committed journals: for the default groups (00_smoke and 10_basic,
   15 scenarios) 24 signing calls and 30 status calls, 54 requests, which is more than 80 percent of the default
   cap. The status monitor of the app adds a few more while a scenario waits (Q110 waits 20 s). Raise the cap
   (for example `--max-requests 100`) when the user accepts the extra calls; the script refuses to start when the
   estimate is above the cap, and fails the run if the cap is reached during it, because the journals would then
   be incomplete.

Also needed: `.env.live` at the repository root (git-ignored; or `--env-file PATH`) with one `NAME=value` line per
variable, as in `docs/internal/LIVE_FDM_MODULE2.md` (Setup):

```
PILI_LIVE_FDM_URL=http://fdm.module2.be/graphql/
# optional: pin the simulator version, as for the Linux evidence
PILI_LIVE_SIMULATOR_VERSION=2.2.0.5
```

plus a line that assigns the bearer token (read on `settings.module2.be`) to `PILI_LIVE_FDM_TOKEN`, written in the
same way. The token line is deliberately not shown here with a value: the P7 gate refuses any committed line that
assigns one. These are the same names as `scripts/live_fdm.sh`. The URL must be a plain `http(s)` URL. Port 443 with mutual TLS
is not supported by this script (the proxy speaks ordinary TLS only); use port 80.

Machine state: the emulator `sunmi_d3_pro_api33` (the script, through `uc_e2e.sh`, uses it), the host mocks'
binaries (built on demand), and **no soak running**: the 24 h soak holds the P7 emulator lock; the script waits for
the lock (up to 90 minutes, then exits 75). Stop the soak first (`bash scripts/android/p7_soak.sh stop --run
p7-24h`) when its 4 h result is enough, or run after it ended.

## The exact command

```sh
# 1. look at the plan: groups, request estimate, files; reads no token, touches nothing
bash scripts/android/uc_live_module2.sh --dry-run

# 2. the real run, by the user, when module2 may be called
PILI_LIVE_GO_AHEAD=yes bash scripts/android/uc_live_module2.sh --go-ahead --max-requests 100
```

Options: `--group G` (repeatable, default `00_smoke` and `10_basic`; `20_training`, `30_reports`, `40_advanced`,
`50_rounding`, `70_faq` are also possible, check the estimate first), `--max-requests N`, `--pause-ms N`,
`--env-file PATH`, `--out DIR` (evidence directory, inside `target/`).

What the run does, in order (each line of output is `ULIVE | <check> | PASS|FAIL | <detail>`, the last one is
`ULIVE | total | PASS|FAIL | <n> failed (<m> checks)`):

1. `env`: reads the environment file and prints `token: set` or `token: none`, never the value. A missing token
   refuses the run (exit 4).
2. Takes the P7 emulator lock (`p7_emulator_lock.sh`).
3. `mocks`: puts the **live proxy** in the place of `pos-fdm-mock` for the length of the run (the real binary is
   moved aside and put back at the end, also after an error, a signal or a crash of the next run).
4. `uc_e2e`: runs `scripts/android/uc_e2e.sh` on the chosen groups. It installs the debug app, makes it device
   owner, builds a development bundle in a temporary directory outside the repository, imports it, plays the
   scenarios and writes one Android journal per scenario. The app thinks the FDM is the host mock on its usual
   port; the proxy forwards each GraphQL call to the simulator.
5. `budget`: how many requests reached the simulator, how many were refused at the cap, how many calls of the mock
   control (`fdm_control`) the proxy ignored. A run in which nothing reached the simulator fails.
6. `reference` and `compare`: records the reference side with the engine on the host (`pos-uc-check record`, its
   own mock) and compares: `pos-uc-check compare --no-counters --fixtures`.
7. `restore`: the real mock binary is back, the temporary directory is deleted.

### Why a proxy, and where the token lives

`uc_e2e.sh` starts `pos-fdm-mock` itself, at a fixed place, and builds its bundle itself, with no token. Rather than
copy the token into a bundle (which would put it on the device, and force re-signing the bundle's manifest with the
development key), the script lets the **host-side proxy** add the `Authorization: Bearer` header. The proxy is a
small Python program written into the temporary directory at run time (standard library only). It reads the
environment file itself, keeps the token in its memory only, and never prints, logs or writes it. So the token is:

- not on the command line (no argument carries it),
- not in the environment of any other process (the environment file path is passed, not its content),
- not in the provisioning bundle, not on the device, not in the journals, the evidence or any log,
- not in the repository, and not in a file that outlives the run.

This mirrors the Linux live run (`scripts/live_fdm.sh`), which also goes through a pacing proxy
(`crates/pos-engine-testkit/src/live.rs`: one call at a time, 1 s apart, capped). The Android till talks plain HTTP
to the host over `adb reverse`; the proxy talks to module2. Not exercised by the proxy and left as is: the TLS path
from the till to a FDM (covered by `pos-fiscal` tests and by the mTLS run of `LIVE_FDM_MODULE2.md`).

The proxy removes `visitor_ip` from the answers it keeps for the journals (the Linux proxy does the same).

## What to look at afterwards

Evidence is in `target/android-uc-live/<date>-<time>/` (or `--out`):

| File | Content |
|------|---------|
| `journals/android/<group>/<scenario>.json` | the Android journal of each scenario: steps, the non-Status calls with the simulator's answers, the Status calls, the tickets |
| `journals/linux/...` | the reference journals (engine on the host, its own mock) |
| `results.json`, `compare.txt` | the comparison report and the `UCCHECK` lines |
| `forwarded.log` | one line per request forwarded: time, running count, operation, HTTP status. No header, no body |

1. `ULIVE | total | PASS`. If it failed, the line before says which check; for a scenario, open its journal.
2. A difference between the Android journal and the reference on a business field is a real finding: do not add a
   volatile path (P7D8). Look at the simulator's own record of the call: `settings.module2.be`, Developer tab,
   "Input/Output" (every sign request and its response; status queries are not listed) and the Logs tab (fodcall
   success and error).
3. Check the simulator for refused or odd answers (`"Not a valid Token"` means the token is wrong or expired;
   `UNKNOWN_POS` means the posId is not on the allowed list).
4. If the dossier should carry the evidence (it is not committed by the script), copy the journals and
   `forwarded.log` to `docs/SUBMISSION_ARTIFACTS/Report_Testing/module2/` by hand after reading them for secrets
   (`grep -r` for the token before copying; the proxy never writes it, the check is a habit).
5. The FDM counters of the simulator moved: write down in the run notes that the simulator's counters now include
   this run, so the next Linux run is not surprised.
6. The ticket numbers: the till's numbering continues in its own database, the database is fresh for each group
   (`uc_e2e.sh` reinstalls), so the numbers restart at 1 for each group. A live FDM may refuse a reused number or
   sign it as a new event (`LIVE_FDM_MODULE2.md`: "A reused `posFiscalTicketNo` is signed again as a new
   event"). If the simulator refuses it, note the code and run groups one at a time in separate sessions.

## What never to do

- Never commit the token, `.env.live`, or any file made from it. `.env.live` is git-ignored; keep it that way.
- Never paste the token into a chat, a ticket, a terminal history (exporting it in the shell is a habit to
  avoid: the file is the place), a log, a CI variable or a screenshot.
- Never run the script from CI, from a scheduled job, or from the chain. P7D13 forbids it; it is by hand, once,
  by the user.
- Never run it without looking at `--dry-run` first, and never raise the cap without a reason: the simulator is a
  shared service of module2.
- Never run it while the Linux till is signing against the same posId, or while the soak runs (the script waits
  for the P7 lock).
- Never replace `pos-fdm-mock` by hand while the script runs. If a run was killed, the next run restores the
  binary left aside (`target/android-mocks/debug/pos-fdm-mock.live-real`); to do it by hand, `mv` that file back
  over `pos-fdm-mock`.
- Never send the evidence to the SPF without the user: it is not part of the official test (the Linux evidence of
  2026-09-27 is).

## Tests of the script

`bash scripts/android/test_uc_live_module2.sh` (no network, no device) proves, with a fake environment file that
carries a canary token, stub `uc_e2e.sh` and `pos-uc-check`, and a fake simulator on `127.0.0.1`: the refusals (no
flag, flag only, variable only, the refused group, an estimate above the cap, a missing token or file, an unsafe
URL), `--dry-run` (reads no file, creates nothing), a full run (the token reaches the simulator as a bearer
header and appears nowhere else: not in the output, the evidence, a temporary file or the work tree), the cap (the
simulator never sees more requests than the cap), the budget check, the expected live FAIL, the restoration of the
mock binary, the deletion of the temporary directory and the recovery of a binary left aside by an interrupted run.
