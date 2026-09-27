# Live FDM: the module2 cloud simulator

module2 (PRACSYS) is an SCE 2.0 certified FDM vendor. It gave us a
cloud FDM simulator to test the POS against an implementation we did
not write. It is a shared, free service: every call is made by hand,
one at a time, and never from CI.

## Portal

| Page | What is there |
|---|---|
| remote.module2.be | Dashboard, the simulator `FDM02000425` (FDM sw 2.2.0.5), its counters |
| settings.module2.be | Allowed POS IDs (one used, one free), the bearer token |
| Developer tab | Simulated errors (buffer full, FDM locked), warnings (buffer near full, retry count), FodFIN connection |
| Logs tab | System log, "fodcall" success and error, unsent buffer, archive |
| usecases.module2.be | Their use-case console (proxied through their own `index.php`) |

Still to get from module2:

- the simulator's GraphQL endpoint URL (shown nowhere in the portal);
- a working link to the developer docs (manual.module2.be redirects
  to the dashboard).

## Identity

The simulator allows the posId of the official test setup
(`UC_TestSetup_20240806.md`), so the live suite uses its identity:

| Field | Value |
|---|---|
| posId | `CPOS0031234567` |
| terminalId | `POS-1-BAR` |
| deviceId | `F0:1D:3D:CC:13:C8` |
| vatNo | `BE0000000097` |
| estNo | `2000000042` |
| employee | `75061189702` |

## Setup

Create `moyun-pos/.env.live` (git-ignored, never committed, never
pasted anywhere):

```bash
MOYUN_LIVE_FDM_URL=https://<endpoint from module2>/graphql
MOYUN_LIVE_FDM_TOKEN=<token from settings.module2.be>
```

The token reaches the POS as in production: the suite writes it to
`target/live-fdm/fdm.token` and sets `fdm_token_file` (see
`MANUEL_INSTALLATION_FR.md`).

## Running

```bash
scripts/live_fdm.sh                                # the operator day, about 17 calls
scripts/live_fdm.sh live_fdm_status_and_one_sale   # 2 calls
```

- Calls go through a local pacing proxy (`src-tauri/tests/common/live.rs`):
  one at a time, 1 s apart (`MOYUN_LIVE_PAUSE_MS`), at most 60 per run
  (`MOYUN_LIVE_MAX_REQUESTS`). Past the cap nothing is forwarded.
- The POS ticket numbers go on from run to run
  (`target/live-fdm/state.json`, saved after every signed event).
  Deleting that file restarts them at 1, which a live FDM may refuse.
- Each signed request and response lands in
  `docs/SUBMISSION_ARTIFACTS/Report_Testing/module2/`
  (`UC_EVIDENCE_DIR` overrides it). The full output is in
  `target/live-fdm.log`.
- The client-level rehearsal also works against the simulator:
  `MOYUN_STAGING_FDM_URL=... MOYUN_STAGING_FDM_TOKEN=... cargo test -p pos-fiscal --test e2e_staging -- --ignored --test-threads=1`
  (about 13 calls, 1 s apart).

If a response is refused by the POS (for example a short signature
that is not the SHA-1 of the digital signature), stop and look at it
before changing the POS: the simulator may use a placeholder
signature.

## Error checklist (Developer tab)

For each simulated condition: switch it on in the Developer tab, run
`scripts/live_fdm.sh live_fdm_status_and_one_sale` (2 calls), check
what the POS reports, switch it off.

| Condition | Expected on the POS |
|---|---|
| Buffer full | Sale refused, blocking `BUFFER_FULL` message, status pill red |
| FDM locked | Sale refused, blocking `FDM_LOCKED` message, status pill red |
| Buffer near full | Sale signed, `BUFFER_NEAR_FULL` warning shown, pill amber |
| Retry count | To confirm: the code module2 sends is not documented yet. Status must still parse (an unknown code no longer breaks it) |

Also try once in the app (`cargo tauri dev` with `fdm_url` and
`fdm_token_file` set): the status pill polls every 30 s and backs off
to 5 min while the FDM is unreachable.

## Offline twin

`pos-fdm-mock` reproduces the simulator's switches without the
network: `POST /mock/control` for errors, warnings and the buffer,
`MOCK_FDM_TOKEN` and `MOCK_FDM_ALLOWED_POS` for auth and the POS
allowlist, `GET /mock/requests` for the log. A dry run of the live
suite against it:

```bash
PORT=8093 MOCK_FDM_TOKEN=dry MOCK_FDM_ALLOWED_POS=CPOS0031234567 target/release/pos-fdm-mock &
MOYUN_LIVE_FDM_URL=http://127.0.0.1:8093/graphql MOYUN_LIVE_FDM_TOKEN=dry \
UC_EVIDENCE_DIR=/tmp/live-ev scripts/live_fdm.sh
```

Once the endpoint is known, a one-time recording (about 30 calls, 1 s
apart) will store the simulator's answers as fixtures in
`crates/pos-fdm-mock/fixtures/module2/`, and a conformance test will
keep the mock's answers in the same shape.
