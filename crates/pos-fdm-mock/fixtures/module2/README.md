# module2 simulator recording

What module2's online simulator answered, recorded once on 2026-09-27:
FDM `FDM02000425`, FDM software 2.2.0.5 (`Simulator-Version: 2.2.0.5`),
endpoint `http://fdm.module2.be/graphql/`, posId `CPOS0031234567`.

27 calls, one at a time, 1 s apart, through the pacing proxy of
`src-tauri/tests/common/live.rs` with `MOYUN_LIVE_RECORD_DIR` set:

| Calls | Test | What |
|---|---|---|
| 15 | `live_fdm_operator_day` | status, work in, table order, pre-bill, sale, refund, copy, money in, drawer open, cash count, training sale, X, user X, combined Z, work out |
| 7 | `live_fdm_protocol_edges` | `Status` capitalised, no token, wrong token, malformed query, Q110 as the POS sends it, unknown posId, posDateTime with microseconds |
| 1 | by hand (curl) | the official Q110 request of the Basic Set v1.7 |
| 4 | `live_fdm_simulated_condition` | status and sale with "Buffer capacity full", then "Buffer capacity near full", switched on in the Developer tab |

Files: `<label>_request.json` is the GraphQL body sent;
`<label>_response.json` is the answer; `calls.jsonl` lists each call
with its HTTP status and whether a token was sent. No token is stored
(it travels in a header, never in a body), and the `visitor_ip` the
simulator echoes in its `info` object was removed before writing.

## What the simulator does

- **Status field:** spelled `status`. `Status` (as in the spec's SDL)
  gets a GraphQL validation error: `Cannot query field "Status" on type
  "Query". Did you mean "status"?`.
- **`initialized`:** always `false`; the simulator signs anyway.
- **Empty message lists:** `[]`, never `null`.
- **`info`:** every answer it computes, and every validation error,
  carries `info {https, version}` next to `data`. Refusals (token,
  FDM codes) do not.
- **Refusals:** no `data` key; `errors[].extensions {category, code,
  showPos}`. `UNKNOWN_POS` and `INVALID_REQUEST` have category `FDM`,
  `BUFFER_FULL` has `SPF_FOD`, as in the spec's tb_messages.
- **Token:** a missing or wrong bearer token gets `{"errors": [{"message":
  "Not a valid Token"}]}`, without a code.
- **posDateTime:** more than milliseconds is refused with
  `INVALID_REQUEST` and a message about the summer/winter offset.
- **Signatures:** `digitalSignature` is `dummy_` and 38 base64
  characters; the N `shortSignature` is 40 hex characters, not derived
  from it. `fdmDateTime` is UTC with a `Z`, whole seconds.
- **Footer:** `[]` on every event.
- **Training:** a training sale has `vatCalc: null` (as the official
  M171B fixture).
- **Buffer full:** the status lists `BUFFER_FULL` in `errors` and also
  `BUFFER_NEAR_FULL` in `warnings`; a sale is refused with
  `BUFFER_FULL`.
- **Warnings in sign responses:** `BUFFER_NEAR_FULL` comes back in the
  sale's `warnings` as well as in the status.
- **Q110:** a query with several `status` fields (aliases), the official
  request included, gets HTTP 500 with an empty body. The mock keeps the
  spec's answer there (`KNOWN_DIFFERENCES` in `tests/module2_shape.rs`).

## Checks

- `crates/pos-fdm-mock/tests/module2_shape.rs`: each recorded request,
  sent to the mock set up the same way, gets an answer of the same
  shape (keys, JSON types, HTTP status).
- `crates/pos-fiscal/tests/module2_recorded.rs`: the POS parses every
  recorded answer; each signed event passes the POS's response check.
