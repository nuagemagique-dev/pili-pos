# pos-eft-mock

A card payment terminal for development, demos and the tests of `pos-eft`. It
stands in for the three terminal protocols the till can drive:

- the **Worldline Terminal API v2** (nexo Retailer 5.1 JSON): synchronous
  payment, status and abort, and `GET /merchants/{umid}` listing the connected
  terminals (`Mock::set_terminals_online(false)` empties it), on the HTTP port;
- the **CCV Payment API**, method `terminal`: `POST /api/v1/payment`,
  `GET /api/v1/transaction`, `POST /api/v1/cancel`, on the same HTTP port;
- a **CCV terminal speaking OPI** (XML, 4-byte length prefix) on a TCP port,
  which connects back to the till's channel 1 to print the customer receipt
  before it answers.

**Not part of the certified build.** A dev-dependency of `pos-eft` and a binary
of its own, never a normal dependency of a shipped crate.

## Run

```bash
cargo run -p pos-eft-mock
PILI_EFT_MOCK_PORT=19400 PILI_EFT_MOCK_OPI_PORT=19410 \
PILI_EFT_MOCK_OPI_DEVICE_PORT=4102 cargo run -p pos-eft-mock
```

| Variable | Default | Meaning |
|----------|---------|---------|
| `PILI_EFT_MOCK_PORT` | 19400 | HTTP port (both cloud APIs, control, page), bound on 127.0.0.1 |
| `PILI_EFT_MOCK_OPI_PORT` | 19410 | OPI terminal, channel 0 |
| `PILI_EFT_MOCK_OPI_DEVICE_PORT` | 4102 | the till's channel 1 port the terminal prints to (0: no receipt) |
| `PILI_EFT_MOCK_API_KEY` | none | when set, both cloud APIs demand it (Bearer / Basic user) |

`pos.json` for each driver:

```json
{"payment_terminal": {"kind": "worldline_terminal_api", "base_url": "http://127.0.0.1:19400",
  "merchant_id": "WL000D5T0000004", "terminal_id": "26197214", "integrator_id": "demo"}}
{"payment_terminal": {"kind": "ccv_payment_api", "base_url": "http://127.0.0.1:19400",
  "terminal_id": "1234567", "management_system_id": "1234567"}}
{"payment_terminal": {"kind": "ccv_opi", "address": "127.0.0.1:19410", "device_port": 4102}}
```

## Control API

On the HTTP port:

| Call | Effect |
|------|--------|
| `POST /mock/next {"result", "brand", "delay_ms", "approved_cents"}` | queue the behaviour of the next payment; every field optional |
| `GET /mock/requests` | the requests received, JSON, newest last |
| `GET /mock/payments` | the payments started and where each stands |
| `POST /mock/reset` | forget payments, requests and queued behaviours |
| `GET /` | a page of recent payments (amount, brand, outcome), refreshed every 3 s |

`result` is `approve` (default), `decline`, `timeout` (approved on the terminal
but the answer never reaches the till: HTTP 202 for Worldline, 504 with the
transaction for CCV, a cut connection for OPI; the status calls then say
approved), `busy` (nothing charged) or `unknown` (the terminal never reaches a
final answer). `brand` defaults to `Bancontact`, `delay_ms` to 1500 so a demo
shows a short wait, `approved_cents` reports another approved amount than the
requested one. An abort during the delay ends the payment as cancelled; the
same reference twice is one charge; two payments never run on one terminal.

## Library

```rust
let mock = pos_eft_mock::Mock::start();          // 127.0.0.1, free ports
mock.url();                                       // base_url for the cloud kinds
mock.opi_addr();                                  // address for ccv_opi
mock.set_device_port(4102);                       // the till's channel 1 port
mock.next(pos_eft_mock::Next { delay_ms: 0, ..Default::default() });
```

`set_default`, `set_api_key`, `requests()`, `payments()`, `reset()` and `stop()`
complete the test controls.

## What is real and what is made up

The Worldline and CCV request checks (UUID `ExchangeIdentification`,
`SalesSystemInfo`, Basic authentication, `Idempotency-Reference`) follow the
public documentation. The answers for a declined, aborted or in-progress
payment, the CCV card circuit (`BCMC` for Bancontact) and receipt layout, and
the whole OPI exchange are modelled on the public samples, not on a real
Belgian terminal.

## Tests

```bash
cargo test -p pos-eft-mock -p pos-eft --all-features
```
