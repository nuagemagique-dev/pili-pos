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

The developer manual is manual.module2.be (reached from the
dashboard's "Developer docs" link; log in again if it bounces to the
dashboard). What it says about the simulator:

- Endpoint: `http://fdm.module2.be/graphql/` (port 80), or
  `https://fdm.module2.be/graphql/` (port 443, mTLS with module2's
  client certificate, the same for the simulator and every physical
  FDM).
- Headers: `Authorization: Bearer <token>` and, optionally,
  `Simulator-Version: 2.2.0.0` (without it, the latest version).
- posId: `CPOS0031234567`.
- The Status query is spelled `status` (lowercase).
- Every module2 FDM, physical ones on the LAN included, needs its
  bearer token (read on its settings page or in the portal).
- `https://cb.module2.be` is not the simulator: it is the Cloud Bridge
  (BII) to reach a physical FDM over the internet, only for FDMs
  module2 explicitly allowed (otherwise `FDM_BII_NOT_ALLOWED`).

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
MOYUN_LIVE_FDM_URL=http://fdm.module2.be/graphql/
MOYUN_LIVE_FDM_TOKEN=<token from settings.module2.be>
# optional: pin the simulator version the evidence was signed with
MOYUN_LIVE_SIMULATOR_VERSION=2.2.0.5
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

What the first live runs (2026-09-27) taught us:

- `posDateTime` must carry at most milliseconds; a finer fraction is
  refused with a misleading "+01:00 is only allowed during winter
  time" message. The POS now sends `2026-09-27T18:31:26.931+02:00`.
- The simulator signs with a placeholder, `dummy_` + base64, and a
  short signature not derived from it. The POS accepts exactly that
  form (and only it) without the two signature checks; every other
  check still applies. `pos-fdm-mock` signs the same way with
  `MOCK_FDM_PLACEHOLDER_SIGNATURES=1`.
- A reused `posFiscalTicketNo` is signed again as a new event.
- Answers carry an extra top-level `info` object (`https`,
  `visitor_ip`, `version`) next to `data`; the POS ignores it.
- The settings page, Developer tab, "Input/Output" shows each sign
  request and its response (status queries are not listed).

If a response is refused by the POS, stop and look at it in that
Input/Output log before changing the POS.

## Error checklist (Developer tab)

For each simulated condition: switch it on in the Developer tab and
Save, run one status query and one sale (2 calls), switch it off:

```bash
MOYUN_LIVE_CONDITION=buffer_full MOYUN_LIVE_EXPECT=BUFFER_FULL \
  scripts/live_fdm.sh live_fdm_simulated_condition
```

The test writes `LIVE_condition_<condition>.json` (status answer, sale
outcome) next to the other live evidence and fails if the expected
code is missing.

Run on 2026-09-27 against FDM02000425 (sw 2.2.0.5):

| Condition | What the simulator sends | POS outcome |
|---|---|---|
| Buffer full | Status error `BUFFER_FULL` (SPF_FOD, MANDATORY) plus warning `BUFFER_NEAR_FULL`; sale answered with a GraphQL error `BUFFER_FULL` | Sale refused with `BUFFER_FULL: FDM buffer is full.`, no N number used |
| FDM locked | Status error `FDM_LOCKED` (SPF_FOD, MANDATORY, "An error has occurred, contact government."); sale answered with the same error | Sale refused with `FDM_LOCKED`, no N number used |
| Buffer near full | Warning `BUFFER_NEAR_FULL` (MANDATORY) in the status and in the sign response; `bufferCapacityUsed` stays at its real value (0.09) | Sale signed; the warning comes with the sign response, shown as a toast once per bookingDate |
| Retry count | Warning `TRANSACTION_UPLOAD_FAILED` (SPF_FOD, OPTIONAL, "FDM has more than 10 failed attempts to post business transaction.") in the status and in the sign response | Sale signed; toast once per bookingDate |

The simulated errors sit next to a "POS id's" field, which held
`CPOS0031234567` during these runs.

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

## Recording (done once, 2026-09-27)

The simulator's answers are kept in
`crates/pos-fdm-mock/fixtures/module2/` (36 calls, README there);
`module2_shape.rs` keeps the mock in the same shape and
`module2_recorded.rs` runs the POS's parsing and response check over
them. To record again (about 36 calls), with an empty directory:

```bash
export MOYUN_LIVE_RECORD_DIR=$PWD/target/live-fdm/recording UC_EVIDENCE_DIR=/tmp/live-ev
scripts/live_fdm.sh live_fdm_operator_day
scripts/live_fdm.sh live_fdm_protocol_edges
scripts/live_fdm.sh live_fdm_basic_set_rest
# Developer tab: Buffer capacity full, then Buffer capacity near full
MOYUN_LIVE_CONDITION=buffer_full scripts/live_fdm.sh live_fdm_simulated_condition
MOYUN_LIVE_CONDITION=buffer_near_full scripts/live_fdm.sh live_fdm_simulated_condition
```

The proxy removes `visitor_ip` before writing; check for the token
before copying the files into the fixtures.

What the recording taught, beyond the list above:

- The status always says `initialized: false`, and the simulator signs
  anyway. The POS keeps its indicator amber and says "La boîte noire
  n'est pas encore initialisée."
- A missing or wrong token gets `"Not a valid Token"` without a code;
  the POS shows its UNAUTHORIZED message.
- The Q110 multi-language status (several `status` fields in one
  query, the official request included) gets HTTP 500 with an empty
  body. The POS never sends it at run time; to be reported to module2.

## HTTPS and mTLS (port 443)

module2's certificates (portal, "HTTPS en mTLS": `ca.crt`,
`client.crt`, `client.key`) go in `target/live-fdm/mtls/`
(git-ignored; `MOYUN_LIVE_TLS_DIR` overrides it). The client key is
never committed. Then (about 5 calls):

```bash
scripts/live_fdm.sh live_fdm_https_mtls
```

It checks that a wrong CA is refused with `FDM_CERTIFICATE_REFUSED`
(not reported as a cable problem), signs a status and a sale with the
full setup, and asks module2 whether the call arrived encrypted
(`info.https`). Evidence: `LIVE_https_*`.

What it taught (2026-09-27):

- `ca.crt` is an X.509 v3 root (CN=RootCA). The server and client
  certificates are X.509 **v1**, without extensions or host name. The
  usual checks (webpki) refuse v1 certificates, both the server's and,
  when rustls pairs key and certificate, the client's. `pos-fiscal`
  therefore checks the server certificate itself (`fdm_tls.rs`: signed
  by the configured CA, within its dates, handshake signed with its
  key) and presents the client certificate without webpki.
- TLS 1.3, RSA-PSS SHA-256 handshake signature.
- The simulator's port 443 also answers without a client certificate.
  A physical FDM may not; the POS always presents it when configured.
- The status cache is keyed on every connection setting (URL, token,
  CA, client certificate), so a changed setting is queried again at
  once.

Offline twin: `pos-fdm-mock` with `MOCK_FDM_TLS_DIR` and
`MOCK_FDM_TLS_REQUIRE_CLIENT=1` (v1 test certificates in
`crates/pos-fdm-mock/fixtures/tls/`), exercised by
`crates/pos-fiscal/tests/fdm_https_mock.rs`.
