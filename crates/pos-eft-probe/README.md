# pos-eft-probe

One manual test of the till's card terminal link, run by a person against a real
terminal. It uses the same drivers as the till (`pos-eft`), so what works here
works in the till.

Commands:

- `check` (the default): is the terminal reachable. Nothing is sent that could
  charge a card.
- `pay --amount 0.01`: prints the reference, the terminal and the amount, then
  waits for you to type `YES`. Anything else ends the run with nothing sent.
  After YES it charges the card and prints the outcome.
- `status <reference>`: the outcome of an earlier payment (use it when an answer
  was lost; `pay` prints the reference first).
- `abort <reference>`: stop a payment in progress.

**It never sends a payment without a person typing YES.** Not part of the
certified build: no shipped crate depends on it.

## Before you run it

- The Mac (or the till) must be on the shop's LAN, on the same network as the
  terminal, for the OPI driver (`ccv_opi`).
- OPI uses two connections. The probe connects to the terminal (channel 0, port
  4100). The terminal then connects back to the probe on the device port
  (channel 1, 4102 by default) to hand over the receipt. macOS may show a
  firewall prompt for that listening port: allow it, or the receipt is lost.
- The trace masks card numbers (every run of 13 to 19 digits keeps only its last
  4), never records headers or API keys, but review it before sharing it.

## Run

The terminal comes from a `pos.json` (its `payment_terminal` section; relative
paths resolve against the file's directory, as in the till) or from flags:

```bash
cargo run -p pos-eft-probe -- --config /path/to/pos.json check
cargo run -p pos-eft-probe -- --kind ccv_opi --address 192.168.1.50:4100 --device-port 4102 check
cargo run -p pos-eft-probe -- --kind ccv_opi --address 192.168.1.50:4100 pay --amount 0.01
cargo run -p pos-eft-probe -- --kind ccv_opi --address 192.168.1.50:4100 status <reference>
```

Cloud kinds take `--base-url`, `--api-key-file`, `--terminal-id`,
`--merchant-id`, `--integrator-id`, `--management-system-id`
(see `--help`). `--timeout-seconds` (30 to 600) is the longest wait for a
payment.

`--trace FILE` (default `eft-trace-<UTC timestamp>.log` in the current
directory) receives the masked wire trace (every OPI frame, every HTTP body)
and the info logs. Exit code: 0 success, 1 failure (declined, unknown,
unreachable), 2 not confirmed.

## Example session against the mock

```bash
# terminal 1: the mock terminal; it prints receipts to port 19412
PILI_EFT_MOCK_OPI_DEVICE_PORT=19412 cargo run -p pos-eft-mock

# terminal 2: the probe, same device port
cargo run -p pos-eft-probe -- --kind ccv_opi --address 127.0.0.1:19410 \
  --device-port 19412 check
cargo run -p pos-eft-probe -- --kind ccv_opi --address 127.0.0.1:19410 \
  --device-port 19412 pay --amount 0.01
```

`pay` prints the reference and asks for YES; type `YES` to get the mock's
approval.
