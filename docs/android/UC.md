# UC: scenario format, vocabulary and comparison rules (P7)

Owner: the end-to-end lane (`e2e-parity`). Written by card P7-T2 from section 4 of `docs/android/phases/P7.md` and the decisions P7D6 to P7D8, completed from `ui/src/api/ipc.ts`, `ui/src/types/index.ts`, `src-tauri/src/ipc/*.rs`, `crates/pos-engine/src/commands/*.rs`, `android/rust/pos-ffi/src/api/*.rs`, `crates/pos-fdm-mock/src/lib.rs` and `crates/pos-printer-mock/src/lib.rs`.

This file is the contract of the two scenario interpreters: `pos-uc-check` (Rust, on the host, `android/uccheck`) and the Kotlin runner of the `uc` package of the androidTest source set. A change goes through this file and is made on both interpreters and on the checker `scripts/android/uc_scenarios_check.py` in the same card (section 10).

The checker (`python3 scripts/android/uc_scenarios_check.py`) validates every scenario file against sections 2 to 4 and refuses to run when the two command tables below differ from its own vocabulary list.

## 1. Files and names (P7.md 4.1)

- Scenarios: `android/app/src/androidTest/assets/uc/<NN>_<group>/<NNN>_<ID>.json`. `<NN>_<group>` is a group directory (`00_smoke`, `10_basic`, `20_training`, `30_reports`, `40_advanced`, `50_rounding`, `60_fdm_errors`, `70_faq`); `<NNN>` orders the scenarios of a group; `<ID>` is the primary UC id (`M110`, `T1`, `ROUND3`, `V_FDM_LOCKED`, `FAQ_Q205`, `SMOKE`). The default androidTest assets directory, so no build file changes.
- Journals: `<out>/<side>/<group>/<scenario>.json` with `<side>` = `linux` or `android` and `<scenario>` the file stem (`010_M110`). Committed by T10 under `docs/android/uc/journals/`.
- Results: `docs/android/uc/results.json`, written by `pos-uc-check compare --report`.
- Groups (P7D7): a group is a directory of scenarios that share one fresh database, run in lexical order, identically on both sides. Each group starts from an empty database (a fresh install on Android, a fresh data directory on the host). A group directory is named `<NN>_<group>` (two digits, then lowercase letters, digits and underscores); a file is named `<NNN>_<ID>.json` (three digits, `<ID>` made of letters, digits and underscores); a number is used once per group.

## 2. Scenario format (format 1, P7.md 4.2)

```json
{
  "format": 1,
  "id": "M111",
  "ids": ["M111"],
  "title": "short English description",
  "fixture": "docs/20260821_UC_BasicSet_POS_FDM_1.8/M111_signSale_Refund",
  "fixtureDeviations": [{ "path": "transaction.transactionLines.0.mainProduct.productId", "reason": "same reason as the Linux test" }],
  "compare": { "status": "ignore" },
  "steps": [
    { "cmd": "login", "args": { "insz": "94060242576", "pin": "1111" } },
    { "cmd": "create_order", "args": { "input": { "order_type": "takeaway", "table_number": null, "user_insz": "94060242576" } }, "save": "order" },
    { "cmd": "checkout", "args": { "orderId": "$order.id", "payments": [{ "method": "cash" }] }, "save": "sale", "expect": { "fields": { "success": true } } },
    { "cmd": "fdm_control", "args": { "errors": ["FDM_LOCKED"] } },
    { "cmd": "checkout", "args": { "orderId": "$order2.id", "payments": [{ "method": "cash" }] }, "expect": { "error": "FDM_LOCKED" } },
    { "cmd": "fdm_control_reset", "args": {} }
  ]
}
```

- Required: `format` (1), `id`, `ids` (the UC ids this scenario covers; empty only for SMOKE), `title`, `steps` (a non-empty array). Optional: `fixture` (a directory of the official UC folders, relative to the repository root), `fixtureDeviations`, `compare.status` (`ignore` default, `shape` for Q100 and Q110).
- `cmd`: one of the vocabulary of section 3. `args`: an object, always present (`{}` when the command takes none); for an engine command exactly the `invoke` argument object of `ui/src/api/ipc.ts`.
- **References:** any JSON string value of the form `$name.path` (whole string) is replaced by the value at `path` of the result saved under `name`; `path` is dot separated, numbers index arrays (`$order.lines.0.id`) and has at least one segment (a scalar result such as the bool of `set_training_mode` is not referenced). A reference to a name not saved by an earlier step is an error. References are allowed in `args` and in the values of `expect`. A string that starts with `$` and is not a reference is an error.
- `save`: stores the result JSON under that name (engine commands only).
- `expect` (default `{}`): `{}` means the call returns no error and, when the result is an object with a `success` field, `success` is true; `{"fields": {...}}` additionally requires each listed field (dot paths allowed) to equal the value; `{"error": "CODE"}` requires an error whose code is `CODE` (section 9) or a result with `success` false whose `error` matches `CODE` (section 9). Mock commands take neither `save` nor `expect`.
- `"repeat": N` on a step runs it N times (default 1; a positive integer). `save` inside a repeated step keeps the last result.
- The checker also rejects: a key not listed above (at the top level, in a step, in `expect`, in `compare`), a duplicate JSON key, a file that is not UTF-8, a `fixtureDeviations` entry without `path` or `reason`, `fixtureDeviations` without `fixture`, a `fixture` directory that does not exist or holds no `*GQL-Variables.json` file, an `id` that differs from the `<ID>` of the file name, an `id` that is not in `ids`, an id unknown to `scripts/uc_matrix.py` (the official ids of the three sets, `T1` to `T8`, `ROUND*`, `V_*`, `VI_*`, `FAQ_*`; `SMOKE` is the one exception), a `menu_item_id` or `menuItemId` value that is not a menu item of `src-tauri/seed_spf_test.sql`, an `insz`, `user_insz` or `employeeInsz` value that is not a user of that seed, an argument key or value the command table of section 3 does not allow, and an em-dash or en-dash anywhere.
- The users and PINs of the seed (`src-tauri/seed_spf_test.sql`): Gerant 94060242576 PIN 1111 (admin), Technicien 00000000097 PIN 0097 (admin), Elisa 84022899837 PIN 2222, John 75061189702 PIN 4444, Sammy 80113078968 PIN 5555 (cashiers), Apprenti 99012345667 PIN 3333 (waiter).

## 3. Vocabulary (P7.md 4.3)

Engine commands (both interpreters support all of them; the interpreters only map JSON to the engine call and forward it, they compute no amount and hold no fiscal rule, P7D6): `login`, `logout`, `set_training_mode`, `work_in`, `work_out`, `end_shift`, `create_order`, `add_order_line`, `add_order_line_with_modifiers`, `update_order_line_quantity`, `remove_order_line`, `set_active_order`, `leave_order`, `sign_table_order`, `sign_pre_bill`, `checkout`, `close_unpaid`, `refund_checkout`, `copy_ticket`, `money_in_out`, `open_drawer_fiscal`, `generate_x_report`, `generate_z_report`, `generate_user_x_report`, `check_fdm_status`, `list_fiscal_events`, `get_order`.

Mock commands (never reach the engine): `fdm_control`, `fdm_control_reset`, `printer_control`, `sleep_ms`.

### 3.1 Engine commands: arguments and call chain

The `args` column is the object `ui/src/api/ipc.ts` passes to `invoke` (top level keys are the camelCase of the Rust parameter names, which Tauri applies; the nested objects keep the field names of their Rust type). `?` marks an optional key (the engine reads an absent key as `null`). The wrapper is `src-tauri/src/ipc/<file>.rs`; each wrapper makes one call to the `pos_engine::commands::<module>::do_*` function of the next column (`scripts/android/check_ipc_wrappers.py --map`); the `PosEngine` method is the facade in `android/rust/pos-ffi/src/api/<file>.rs` (Kotlin uses the camelCase name, for example `createOrder`).

<!-- UC-COMMANDS engine args -->
| Command | args (as `invoke` gets them) | Wrapper | Engine function | `PosEngine` method (facade file) |
|---------|------------------------------|---------|-----------------|----------------------------------|
| `login` | `{insz, pin?}` | `auth.rs` | `auth::do_login` | `login(insz, pin)` (`auth.rs`) |
| `logout` | `{}` | `auth.rs` | `auth::do_logout` | `logout()` (`auth.rs`) |
| `set_training_mode` | `{enabled}` boolean | `settings.rs` | `settings::do_set_training_mode` | `setTrainingMode(enabled)` (`reports.rs`) |
| `work_in` | `{employeeInsz}` | `social.rs` | `social::do_work_in` | `workIn(employeeInsz)` (`reports.rs`) |
| `work_out` | `{employeeInsz}` | `social.rs` | `social::do_work_out` | `workOut(employeeInsz)` (`reports.rs`) |
| `end_shift` | `{insz, pin?}` | `social.rs` | `social::do_end_shift` | `endShift(insz, pin)` (`reports.rs`) |
| `create_order` | `{input: {order_type, table_number, user_insz}}`; `order_type` is `dine_in` or `takeaway`, `table_number` an integer or `null` | `order.rs` | `order::do_create_order` | `createOrder(input)` (`orders.rs`) |
| `add_order_line` | `{input: {order_id, menu_item_id, quantity, modifiers_json?, unit_price_cents?}}`; `modifiers_json` a string or `null`, `unit_price_cents` only for a free-price item | `order.rs` | `order::do_add_order_line` | `addOrderLine(input)` (`orders.rs`) |
| `add_order_line_with_modifiers` | `{input: {order_id, menu_item_id, quantity, modifier_ids}}`; `modifier_ids` an array of integers | `modifiers.rs` | `modifiers::do_add_order_line_with_modifiers` | `addOrderLineWithModifiers(input)` (`orders.rs`) |
| `update_order_line_quantity` | `{lineId, quantity}` | `order.rs` | `order::do_update_order_line_quantity` | `updateOrderLineQuantity(lineId, quantity)` (`orders.rs`) |
| `remove_order_line` | `{lineId}` | `order.rs` | `order::do_remove_order_line` | `removeOrderLine(lineId)` (`orders.rs`) |
| `set_active_order` | `{orderId?}` an integer or `null` | `session.rs` | `session::do_set_active_order` | `setActiveOrder(orderId)` (`orders.rs`) |
| `leave_order` | `{orderId, reason}`; `reason` is `navigate`, `switch_order`, `send_to_kitchen`, `lock` or `idle` | `session.rs` | `session::do_leave_order` | `leaveOrder(orderId, reason)` (`orders.rs`) |
| `sign_table_order` | `{orderId}` | `tables.rs` | `tables::do_sign_table_order` | `signTableOrder(orderId)` (`orders.rs`) |
| `sign_pre_bill` | `{orderId}` | `tables.rs` | `tables::do_sign_pre_bill` | `signPreBill(orderId)` (`orders.rs`) |
| `checkout` | `{orderId, payments, delivery?}`; `payments` an array of `{method, amount_cents?}` (`method` is `cash`, `card`, `meal_voucher`, `app` or `online`; a cash part has no amount and pays the rest), `delivery` `{medium, email?}` (`medium` is `paper`, `digital` or `paper_digital`) | `checkout.rs` | `checkout::do_checkout_split` | `checkout(orderId, payments, delivery)` (`checkout.rs`) |
| `close_unpaid` | `{orderId, insz, pin}` | `unpaid.rs` | `unpaid::do_close_unpaid` | `closeUnpaid(orderId, insz, pin)` (`checkout.rs`) |
| `refund_checkout` | `{originalFiscalEventId, items, employeeInsz, delivery?}`; `items` an array of `{menuItemId, quantity, lineId?}` (quantity positive, the engine negates it) | `refund.rs` | `refund::do_refund_checkout_with` | `refundCheckout(originalFiscalEventId, items, employeeInsz, delivery)` (`checkout.rs`) |
| `copy_ticket` | `{fiscalEventId, employeeInsz, delivery?}` | `copy.rs` | `copy::do_copy_ticket_with` | `copyTicket(fiscalEventId, employeeInsz, delivery)` (`checkout.rs`) |
| `money_in_out` | `{employeeInsz, amountCents, direction, reason}`; `direction` is `in`, `out` or `declaration` | `financial.rs` | `financial::do_money_in_out` | `moneyInOut(employeeInsz, amountCents, direction, reason)` (`reports.rs`) |
| `open_drawer_fiscal` | `{employeeInsz}` | `financial.rs` | `financial::do_open_drawer_fiscal` | `openDrawerFiscal(employeeInsz)` (`reports.rs`) |
| `generate_x_report` | `{employeeInsz}` | `report.rs` | `report::do_generate_x_report` | `generateXReport(employeeInsz)` (`reports.rs`) |
| `generate_z_report` | `{employeeInsz}` | `report.rs` | `report::do_generate_z_report` | `generateZReport(employeeInsz)` (`reports.rs`) |
| `generate_user_x_report` | `{employeeInsz}` | `report.rs` | `report::do_generate_user_x_report` | `generateUserXReport(employeeInsz)` (`reports.rs`) |
| `check_fdm_status` | `{}` | `checkout.rs` | `checkout::do_check_fdm_status` | `checkFdmStatus()` (`checkout.rs`) |
| `list_fiscal_events` | `{limit?}` an integer (50 when absent) | `copy.rs` | `copy::do_list_fiscal_events` | `listFiscalEvents(limit)` (`checkout.rs`) |
| `get_order` | `{orderId}` | `order.rs` | `order::do_get_order` | `getOrder(orderId)` (`orders.rs`) |
<!-- /UC-COMMANDS -->

Notes on the arguments:

- Money in `args` is in cents (integers): `amount_cents`, `amountCents`, `unit_price_cents`. `payments[].amount_cents` is snake_case (the Rust `PaymentPart` has no rename) while `refund_checkout` `items[].menuItemId` and `delivery` are camelCase (`RefundItem` and `TicketDeliveryRequest` rename). `create_order` and `add_order_line` inputs are snake_case.
- `login`, `end_shift` and `close_unpaid` take a PIN as a string (`"1111"`); a seed user without a PIN logs in with `pin` absent or `null`.
- Every `insz`, `user_insz` and `employeeInsz` value must be a user of the seed, and every `menu_item_id` / `menuItemId` a menu item of the seed (the checker enforces both). A refusal is provoked with a wrong PIN or a wrong state, never with an unknown user.
- `checkout` of an order with no line, a payment that exceeds the total and so on come back as a result with `success: false` and a code in `error`, not as an exception (section 9).

### 3.2 Engine commands: results

A result is the serde JSON of the engine type, as the Vue UI sees it (`invoke` returns it). Field names are exactly what Rust serialises: a type with no `rename_all` is snake_case (`CheckoutResult`, `Order`, `OrderLine`, `User`, `FiscalTicket`), a type with `rename_all = "camelCase"` is camelCase (`ReportResult`, `SocialResult`, `FinancialResult`, `CopyResult`, `DeliveryOutcome`, `FdmReference`). On Android the Kotlin interpreter maps the uniffi record back to the same JSON names, so a `$name.path` reference means the same on both sides. A result of `null` has nothing to reference.

<!-- UC-COMMANDS engine results -->
| Command | Result type and JSON | Fields scenarios may reference or `expect` |
|---------|----------------------|--------------------------------------------|
| `login` | `User` `{insz, name, role, active, has_pin}`; `role` is `admin`, `cashier` or `waiter` | `insz`, `name`, `role` |
| `logout` | `null` | none |
| `set_training_mode` | the new value, a bare boolean | none (not referenced) |
| `work_in` | `SocialResult` `{success, error}` | `success`, `error` |
| `work_out` | `SocialResult` `{success, error}` | `success`, `error` |
| `end_shift` | `SocialResult` `{success, error}` | `success`, `error` |
| `create_order` | `Order` `{id, table_number, order_type, status, user_insz, created_at, closed_at, subtotal_cents, total_cents}`; `status` is `open`, `closed` or `voided`; dates RFC 3339 | `id`, `total_cents`, `status` |
| `add_order_line` | `OrderLine` `{id, order_id, menu_item_id, quantity, unit_price_cents, vat_code, name, modifiers_json, created_at, department_id, department_name}`; `vat_code` is `A`, `B`, `C`, `D` or `X` | `id`, `order_id`, `unit_price_cents`, `vat_code`, `name` |
| `add_order_line_with_modifiers` | `OrderLine` (as above) | `id`, `unit_price_cents`, `name` |
| `update_order_line_quantity` | `null` | none |
| `remove_order_line` | `null` | none |
| `set_active_order` | `null` | none |
| `leave_order` | `SlipOutcome`, a bare string: `printed`, `nothing_to_print`, `no_printer` or `failed` | none (not referenced; `expect` compares through the journal) |
| `sign_table_order` | `SlipOutcome` | none |
| `sign_pre_bill` | `SlipOutcome` | none |
| `checkout` | `CheckoutResult` `{success, ticket, error, fdm_connected, fiscal_event_id, delivery, kitchen_slip}` (no `rename_all`: snake_case). `ticket` is a `FiscalTicket` (snake_case; its `fdm_ref` is `{fdmId, fdmDateTime, eventLabel, eventCounter, totalCounter}` and its `vat_breakdown` items are camelCase) or `null`; `delivery` is a `DeliveryOutcome` `{fiscalEventId, deliveryId, medium, email, recipientHint, paperPrinted, needsPaper}` or `null`; `kitchen_slip` is a `SlipOutcome` or `null` | `success`, `error`, `fdm_connected`, `fiscal_event_id` (the archive id that `copy_ticket` and `refund_checkout` take), `ticket.ticket_number`, `ticket.total`, `ticket.event_label`, `ticket.fdm_ref.eventCounter`, `ticket.fdm_ref.totalCounter`, `kitchen_slip` |
| `close_unpaid` | `CheckoutResult` | as `checkout` |
| `refund_checkout` | `CheckoutResult` | as `checkout` (`ticket.is_refund` is true) |
| `copy_ticket` | `CopyResult` `{success, error, ticket, fiscalEventId, delivery}` (camelCase; `ticket` and `delivery` as above) | `success`, `error`, `fiscalEventId` |
| `money_in_out` | `FinancialResult` `{success, error}` | `success`, `error` |
| `open_drawer_fiscal` | `FinancialResult` `{success, error}` | `success`, `error` |
| `generate_x_report` | `ReportResult` `{success, reportType, reportNo, error}`; `reportType` is `X` | `success`, `reportType`, `reportNo`, `error` |
| `generate_z_report` | `ReportResult`; `reportType` is `Z`, `reportNo` the Z number | `success`, `reportType`, `reportNo`, `error` |
| `generate_user_x_report` | `ReportResult`; `reportType` is `X-USER` | `success`, `reportType`, `reportNo`, `error` |
| `check_fdm_status` | a JSON object, never an error: `{device: {fdmId, fdmSwVersion, fdmDateTime, bufferCapacityUsed}, initialized, informations, warnings, errors}` when the FDM answers (the three lists hold `{message, extensions: {category, code, showPos}}`), `{connected: false, error}` when it does not | `initialized`, `connected`, `device.fdmId`, `device.bufferCapacityUsed`, `errors.0.extensions.code`, `warnings.0.extensions.code` |
| `list_fiscal_events` | an array, newest first, of `{id, eventType, sequenceNumber, orderId, fdmId, fdmDatetime, eventLabel, shortSignature}` (camelCase; note `fdmDatetime` with a lowercase t) | `0.id` (the latest event), `0.eventLabel`, `0.sequenceNumber`, `0.orderId` |
| `get_order` | `Order` (as `create_order`) | `id`, `status`, `total_cents`, `subtotal_cents` |
<!-- /UC-COMMANDS -->

`check_fdm_status` is cached by the engine for 20 seconds per FDM url; a scenario that changes the mock with `fdm_control` and then reads the status waits with `sleep_ms` or accepts the cached answer.

### 3.3 Mock commands

A mock command never reaches the engine. The host side talks to `pos-fdm-mock` and `pos-printer-mock` directly; the device side reaches them through `adb reverse` (P7.md 4.7: 19380 FDM, 19391 printer HTTP). Each takes no `save` and no `expect`; it fails the step when the HTTP call does not answer 2xx.

<!-- UC-COMMANDS mock args -->
| Command | args | HTTP call |
|---------|------|-----------|
| `fdm_control` | the `Control` object of `pos-fdm-mock`: `errors` (array of FDM codes, for example `["FDM_LOCKED"]`), `warnings` (array of codes), `bufferPct` (number or `null`), `initialized` (boolean or `null`), `token` (string or `null`), `allowedPos` (array of posId or `null`), `placeholderSignatures` (boolean). Every key is optional | `POST <fdm base>/mock/control` with the args object as the JSON body; answers `{"ok":true}` |
| `fdm_control_reset` | `{}` | `POST <fdm base>/mock/control` with the body `{}`; answers `{"ok":true}` |
| `printer_control` | the `Control` object of `pos-printer-mock`: `paper` (`ok`, `near_end` or `end`), `coverOpen`, `error`, `unplugged` (booleans). Every key is optional | `POST <printer http base>/control` with the args object as the JSON body; answers the new control state |
| `sleep_ms` | `{"ms": N}`, N an integer from 0 to 10000 | none: the interpreter waits N milliseconds |
<!-- /UC-COMMANDS -->

Details the interpreters and scenario authors rely on (read from `crates/pos-fdm-mock/src/lib.rs` and `crates/pos-printer-mock/src/lib.rs`):

- `<fdm base>` is the base url of the FDM the engine is configured with, without `/graphql` (the mock serves `/graphql`, `/mock/control` and `/mock/requests` on one port). `<printer http base>` is the printer mock's HTTP side (`/jobs`, `/jobs/{id}`, `/control`), not its raw print port.
- **The wire names are camelCase.** `Control` is `serde(rename_all = "camelCase", default)`. P7.md 4.3 names the fields as the Rust struct does (`buffer_pct`, `allowed_pos`); on the wire they are `bufferPct` and `allowedPos`, and the mock silently ignores an unknown key, so a snake_case key would do nothing. The checker rejects `buffer_pct`, `allowed_pos`, `placeholder_signatures` and `cover_open` with a message naming the right key.
- **A control POST replaces the whole state.** A key that is absent goes back to its default (`errors` and `warnings` empty, `bufferPct`, `initialized`, `token` and `allowedPos` absent). `fdm_control_reset` is therefore `POST {}`. A scenario that wants two errors sends both in one call. `GET <fdm base>/mock/control` reads the state (the token shows as `(set)`).
- `errors` and `warnings` take the FDM codes of the manual (`FDM_LOCKED`, `BUFFER_FULL`, `BUFFER_NEAR_FULL`, `UNAUTHORIZED`, `UNKNOWN_POS`, ...); the critical ones refuse every sign request, `initialized` is false when `errors` is set unless given.
- The FDM request log is `GET <fdm base>/mock/requests` (last 100 calls, each `{at, operation, request: {query, variables}, response}`); `DELETE <fdm base>/mock/requests` clears it and answers `{"ok":true}`. The host clears it before each scenario. These two calls belong to the runner, not to the scenario vocabulary.
- The printer jobs are `GET <printer http base>/jobs` (each `{id, at, bytes, printed, drawerKick, text}`), `GET <printer http base>/jobs/{id}` (the text as `text/plain`) and `DELETE <printer http base>/jobs` (204, clears them). The runner clears them before each scenario.

## 4. The smoke scenario (P7.md 4.6)

`android/app/src/androidTest/assets/uc/00_smoke/001_SMOKE.json` is the first scenario: a login, one takeaway sale of two Perrier (menu item 12007) paid in cash, then an X report. T2, T3 and T4 each commit this file byte-identical (SHA-256 `198014c546dcab66ac40e8386d3806d3889b9d4069ca9b64088f16ab8456dcc5`, two-space indent, final newline), so the identical adds merge cleanly. Its JSON block is in section 4.6 of `docs/android/phases/P7.md`.

## 5. Journal format (format 1, both sides, P7.md 4.4)

```json
{
  "format": 1,
  "side": "android",
  "group": "10_basic",
  "scenario": "020_M111",
  "ids": ["M111"],
  "posSwVersion": "pili-pos-android/1.0.0-dev",
  "steps": [{ "cmd": "checkout", "ok": true, "error": null, "result": { "success": true, "fiscal_event_id": 3 } }],
  "fdm": [{ "operation": "signSale", "request": {}, "response": {} }],
  "statusCalls": [{ "request": {}, "response": {} }],
  "tickets": ["text of each printer job, in print order"],
  "verdict": "PASS"
}
```

- `steps` has one entry per executed step (a step with `repeat` N gives N entries, a mock command gives an entry with `result` null). `ok` is true when the step met its `expect`; `error` is the error text when the call failed or the `error` of a `success: false` result, else `null`; `result` is the result JSON the interpreter got (may be `null`). The comparison reads `cmd` and `ok`.
- `fdm` holds the non-Status calls of `GET /mock/requests` in order (the mock keeps the last 100 calls, so the host clears the log before each scenario); `request` and `response` exactly as the mock logged them (`request` is the GraphQL body `{query, variables}`). `statusCalls` holds the Status calls (`operation` is `Status`).
- `tickets` are the printer mock's job texts, every job in `id` order (`GET /jobs`, `GET /jobs/{id}`); a job that only kicks the drawer has an empty text and is still one entry.
- `posSwVersion` is the value the side declares: `pili-pos/<[workspace.package] version of the root Cargo.toml>` for the reference (the engine on the host), `pili-pos-android/<version>` for the device.
- `verdict` is PASS when every step met its `expect`.

## 6. Results format and comparison checks (P7.md 4.5)

Per scenario, one line `UCCHECK | <group>/<scenario> | PASS|FAIL | <checks>`, then `UCCHECK | total | PASS|FAIL | <n> failed (<m> scenarios)`, exit 1 on any FAIL.

| Check | Rule |
|-------|------|
| `steps` | both journals PASS, same commands, same `ok` per step |
| `fdm-ops` | same list of operations |
| `fdm-business` | `business_view` of every request equal, in order |
| `fdm-counters` | (operation, eventLabel, eventCounter, totalCounter) from each response's fdmRef equal (skipped with `--no-counters`, live mode) |
| `fdm-full` | the whole requests equal once the volatile paths of section 7 are removed |
| `sw-version` | Android `posSwVersion` starts with `pili-pos-android/`, the reference with `pili-pos/`; every request carries its side's value |
| `tickets` | same number of tickets, equal texts after the normalisation of section 8 |
| `fixture` | with `--fixtures` and a `fixture` field: the matching request's `business_view` equals the fixture's `GQL-Variables.json` one, except the listed `fixtureDeviations` (a listed deviation that does not occur is a FAIL) |
| `status-shape` | only when `compare.status` is `shape`: same keys in the Status responses; the keys of a side are the union over its Status answers (an answer whose list is empty shows no key below it, so a monitor call made before the scenario changed the mock must not make the sides differ) |

What "equal" means (P7D8): per scenario, every step has the expected outcome on both sides; the non-Status FDM calls are the same operations in the same order, with equal `business_view` (`crates/pos-engine-testkit/src/compare.rs`), equal counters and an equal full request once the volatile paths are removed; equal ticket texts after normalisation; and, when the scenario names an official fixture, a `business_view` equal to the fixture's (with the same intended deviations as the Linux test, each with its reason). Status calls are counted and ignored, except in the Q100 and Q110 scenarios, where their shape is compared. A real difference between Android and the reference is an ESCALATE, never a new volatile path.

`business_view` keeps the whole `fdmRef` object of a request (a refund, a copy). Its `fdmDateTime` is the FDM clock reading of the referenced event and is volatile (section 7): the comparison removes `fdmRef.fdmDateTime` from both views before it compares `fdm-business` between two runs. Against an official fixture the field is compared as the Linux test compares it.

`--report FILE` writes:

```json
{"format":1,"generated":"<ISO>","reference":"engine on the host","scenarios":[{"group":"10_basic","scenario":"020_M111","ids":["M111"],"checks":{"steps":"PASS","fdm-ops":"PASS","fdm-business":"PASS","fdm-counters":"PASS","fdm-full":"PASS","sw-version":"PASS","tickets":"PASS","fixture":"SKIP","status-shape":"SKIP"},"verdict":"PASS"}]}
```

Each check is `PASS`, `FAIL` or `SKIP`; the `verdict` is `FAIL` when any check is `FAIL`. The report names the reference side "engine on the host" (P7D4): it is the engine compiled for the host, calling the same `do_*` functions as the Tauri wrappers, not the Linux cert binary.

## 7. Volatile paths of the `fdm-full` check

A path below is removed from both requests before they are compared. Paths are written from the request body `{query, variables}`; `*` stands for any array index. A path listed once applies to every operation that carries it. The `query` text, `variables.isTraining` and every business field are compared.

| Path | Reason |
|------|--------|
| `variables.data.posDateTime` | the clock reading of the event; differs on every run and between the two sides |
| `variables.data.bookingDate` | date of the booking period: the day of the run |
| `variables.data.bookingPeriodId` | a random UUID created when the booking period opens |
| `variables.data.reportBookingDate` | booking date of a Z report: the day of the run |
| `variables.data.posDevices.*.firstPosDateTime` | report period start, a POS clock reading |
| `variables.data.posDevices.*.lastPosDateTime` | report period end, a POS clock reading |
| `variables.data.posDevices.*.posId` | installation identity (the Android bundle and the host configuration may carry different values) |
| `variables.data.posDevices.*.terminalId` | installation identity |
| `variables.data.fdmDevices.*.firstFdmDateTime` | report period start, an FDM clock reading of the run |
| `variables.data.fdmDevices.*.lastFdmDateTime` | report period end, an FDM clock reading of the run |
| `variables.data.users.*.firstPosDateTime` | first event of a user in a user report, a clock reading |
| `variables.data.users.*.lastPosDateTime` | last event of a user in a user report, a clock reading |
| `variables.data.users.*.socialEvents.*.posDateTime` | clock reading of a work in or work out registration |
| `variables.data.fdmRef.fdmDateTime` | the FDM clock reading of the referenced event, echoed back by the POS (refund, copy, correction) |
| `variables.data.fdmRefs.*.fdmDateTime` | same, for an invoice that references several events |
| `variables.data.posId` | installation identity, see `posDevices.*.posId` |
| `variables.data.terminalId` | installation identity: `POS-2-ANDROID` on the device, the host configuration value on the reference |
| `variables.data.deviceId` | device identity: read from the device on Android, from the configuration on the host |
| `variables.data.posSwVersion` | `pili-pos-android/<version>` against `pili-pos/<version>`; the `sw-version` check proves each side's value |
| `*.digitalSignature` (any depth) | the FDM signature, derived from the signed clock reading and identity; no request carries it today, the rule keeps a future field from failing on noise |
| `*.shortSignature` (any depth) | SHA-1 of the signature, same reason |
| `*.verificationUrl` (any depth) | URL built from the signature, same reason |

Never volatile (a difference here is a real difference): the operation and its order, the `query` text, `employeeId`, `vatNo`, `estNo`, `language`, `ticketMedium`, `posFiscalTicketNo`, `isTraining`, `costCenter`, every transaction line (product id, name, department, quantity, unit price, VAT label and amount, line total), `transactionTotal`, every `financials` payment line, `drawer`, `fdmRef.fdmId`, `fdmRef.eventLabel`, `fdmRef.eventCounter`, `fdmRef.totalCounter`, `reportNo` and the report turnover totals.

## 8. Ticket normalisation

The text of each printer job is the ESC/POS stream rendered by `crates/pos-print/src/render.rs` (48 columns). The same rules are applied to both sides, in this order, with the regexes below (Python `re`, multi-line mode, so `^` and `$` match at each line):

| # | Regex | Replacement | Reason |
|---|-------|-------------|--------|
| 1 | `^(\s*fdmDateTime: ).*;\s*$` | `\1<fdm-datetime>;` | the FDM clock reading of the event, in the control data of receipts and reports |
| 2 | `^(\s*shortSignature: )[0-9a-fA-F]+;\s*$` | `\1<short-signature>;` | report form of the short signature (hash of the signature, derived from the clock) |
| 3 | `^(\s*)[0-9a-fA-F]{40};\s*$` | `\1<short-signature>;` | receipt form: the 40 hexadecimal characters on the line after `totalCounter: N; shortSignature:` |
| 4 | `\[QR: [^\]]*\]` | `[QR: <qr-payload>]` | the verification URL printed as a QR code, built from the signature |
| 5 | `^(\s*(?:Logiciel de caisse v\.:\|Kassasoftware v\.:\|Kassensoftware v\.:)) .*$` | `\1 <sw-version>` | the software version line differs by design (`pili-pos/...` against `pili-pos-android/...`), proven by `sw-version` |
| 6 | `^(\s*Input device MAC:) .*$` | `\1 <device-id>` | the device identity |
| 7 | `^(\s*(?:POS Serial:\|Terminal:)) .*$` | `\1 <identity>` | installation identity on receipts |
| 8 | `^(\s*(?:POSID\|TerminalId))\s+\S+\s*$` | `\1 <identity>` | installation identity on reports (the value is right aligned, so its padding changes with its length) |
| 9 | `^\s*[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\s*$` | `<uuid>` | the booking period id printed on the line after `BookingPeriodId:` |
| 10 | `\b\d{2}/\d{2}/(?:\d{4}|\d{2}) \d{2}:\d{2}:\d{2}\b` | `<datetime>` | date and time of a bar or kitchen slip (four-digit year) and of the X and Z report header next to `Rapport X` (`ReportDetails.created_at`, formatted `%d/%m/%y %H:%M:%S` with a two-digit year, for example `10/10/26 11:35:00`) |
| 11 | `\b\d{4}-\d{2}-\d{2} \d{2}:\d{2}\b` | `<datetime>` | date and time in a report (`2026-04-12 11:30`) |
| 12 | `\b\d{4}-\d{2}-\d{2}\b` | `<date>` | report date line |
| 13 | `\b\d{2}\.\d{2}\.\d{4}\b` | `<date>` | receipt date (`23.06.2026`) and the date of "Copie etablie le" |
| 14 | `(?<![\d:])\d{2}:\d{2}(?::\d{2})?(?![\d:])` | `<time>` | receipt time (`11:35`) and the time of "Copie etablie le" |
| 15 | `[ \t]+$` per line, `\r\n` to `\n`, trailing blank lines removed | | transport only |

Everything else is compared as printed: ticket numbers, operator names, lines, quantities, prices, VAT columns, totals, payments, rounding, the counters of the control data (`Event: N; eventCounter: 3;`, `totalCounter: 5;`), the legal mentions and the "Informations FDM d'origine" block. A rule matches nothing on a ticket without that line; a rule is never widened to hide a difference (P7D8).

## 9. The error code rule

An engine function returns `Err(String)`; the text is `code|k=v|k=v` (`pin_locked|seconds=30`, or `order_no_lines` alone), with the codes of `pos_engine::commands::errors` (`errors::ALL` and `errors::hubrise::ALL`, 85 codes at 042d911), or a text that starts with `FDM`. The FFI facade (`android/rust/pos-ffi/src/error.rs`, `docs/android/FFI.md` section 6) maps it to `PosException`:

- `code|k=v` with a known code gives `PosException.Coded(code, params)`;
- a text starting with `FDM` (after trimming) gives `PosException.Fdm(detail)`;
- anything else gives `PosException.Internal(detail)`; a call after `shutdown()` gives `PosException.EngineClosed`. An `Internal` or `EngineClosed` error never matches an `expect.error`; it fails the step.

`expect.error` is compared with the code of the error:

- host side (`pos-uc-check`): for an `Err(String)` whose trimmed text starts with `FDM` (the `sign_table_order` and `sign_pre_bill` commands return an FDM refusal as an `Err`, for example `FDM error: GraphQL error: FDM_LOCKED: FDM is locked.`), the same strip rule as Android, defined below (remove one leading `FDM error: `, then one leading `GraphQL error: `, then match `CODE` followed by `|`, `:`, a space or the end of the text); for any other `Err(String)` (the `code|k=v` form), the text before the first `|`;
- Android: `Coded.code`, or, for `PosException.Fdm`, the code of its detail with the same strip rule;
- an `Err` with an unknown code (`PosException.Internal` on Android, a text that is neither a known `code|k=v` nor an FDM text on the host) never matches an `expect.error` on either side, so both sides agree and the step fails on both.

A step whose call returns `Ok` with `success: false` (the checkout, refund, copy, report, social and financial families report their refusals inside the result, as the Tauri commands do) matches `expect.error` when the result's `error` text matches `CODE`. The `error` text of an FDM refusal is not the bare code: the engine wraps it (`pos-engine/src/commands/signing.rs`, `pos-fiscal/src/fdm_client.rs`) as `FDM error: GraphQL error: FDM_LOCKED: FDM is locked.`, and a lost answer as `FDM error: FDM_ANSWER_PENDING|label=N|ticket=3|cause=...`. So both interpreters compare in this way, which completes the "starts with CODE" of section 2: remove one leading `FDM error: ` and then one leading `GraphQL error: ` if present, then the text matches `CODE` when it equals `CODE` or starts with `CODE` followed by `|`, `:` or a space. This keeps `"error": "FDM_LOCKED"` working on the `checkout` of the specification and `"error": "order_no_lines"` on a `code|k=v` result.

## 10. How to add a command

One card, three places, never one without the others:

1. Both interpreters: the Rust one in `android/uccheck` (`pos-uc-check`, calling the `do_*` function the Tauri wrapper calls) and the Kotlin one in the `uc` package of the androidTest source set (a mapping from JSON to the uniffi records and a forward through `EngineGateway.call { }`). The interpreters compute nothing.
2. This file: a row in each of the command tables of sections 3.1 and 3.2 (or 3.3 for a mock command), in the order of the list at the start of section 3, with the `args` exactly as `ui/src/api/ipc.ts` sends them, the wrapper and `do_*` function, the `PosEngine` method, the result type with its serde field names, and the fields scenarios may reference.
3. The checker `scripts/android/uc_scenarios_check.py`: the name in `ENGINE_COMMANDS` (or `MOCK_COMMANDS`) in the same order as the table, and its argument shape in `ENGINE_ARGS` (or the mock branch); then a good and a bad case in `scripts/android/test_uc_scenarios_check.py`. The checker compares its list with the `UC-COMMANDS` blocks of this file and fails when they differ.

Then run `python3 scripts/android/test_uc_scenarios_check.py`, `python3 scripts/android/uc_scenarios_check.py` and `bash scripts/android/p7_gate.sh --base android/lane-p7`. A command that is not an existing Tauri command, or one that needs a new engine function, is a change to frozen paths: stop and ask the planner.

## 11. Decisions this file rests on (P7D6 to P7D8)

- **P7D6 Scenario vocabulary.** A scenario is data: a list of steps named exactly like the Tauri commands, whose `args` are exactly what `ui/src/api/ipc.ts` passes to `invoke` and whose saved result is exactly what `invoke` returns (the serde JSON of the engine type, as Vue sees it). One interpreter per side (Rust in `pos-uc-check`, Kotlin in the androidTest `uc` package). The Kotlin interpreter only maps JSON to the uniffi records and forwards the call through `EngineGateway.call { }`; it computes no amount and holds no fiscal rule.
- **P7D7 Groups.** A group is a directory of scenarios that share one fresh database, run in lexical order, identically on both sides. Each group starts from an empty database (a fresh install on Android, a fresh data directory on the host).
- **P7D8 What "equal" means.** Per scenario: every step has the expected outcome on both sides; the non-Status FDM calls are the same operations in the same order, with equal `business_view` (`crates/pos-engine-testkit/src/compare.rs`), equal counters (operation, eventLabel, eventCounter, totalCounter) and an equal full request once the documented volatile paths are removed (dates, installation identity, `posSwVersion`, signatures; each path listed with its reason in this file); equal ticket texts after the documented normalisation; and, when the scenario names an official fixture, a `business_view` equal to the fixture's (with the same intended deviations as the Linux test, each with its reason). Status calls are counted and ignored, except in the Q100 and Q110 scenarios, where their shape is compared. A real difference between Android and the reference is an ESCALATE, never a new volatile path.
