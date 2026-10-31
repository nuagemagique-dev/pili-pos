# pos-printer-mock

A network thermal printer standing in for the Epson TM-m30III (raw
TCP, ESC/POS) for development, demos and tests.

```bash
cargo run --package pos-printer-mock           # printer on :9100, page on :9180
PORT=9101 HTTP_PORT=9181 cargo run --package pos-printer-mock
```

Point `pos.json` at it: `"printer_address": "127.0.0.1:9100"` (and
`kitchen_printer_address` at a second one for the bar slip). Then open
http://localhost:9180/ to see every job as the paper shows it. For a
58 mm till (`"printer_paper_mm": 58`), start the mock with
`MOCK_PRINTER_ROLL_MM=58` so the page lays the jobs out on 35 columns.

## What it does

- Every connection is one job: its ESC/POS bytes rendered to text
  (`pos_print::render`), whether it opened the cash drawer (ESC p),
  and whether it came out.
- DLE EOT n status queries are answered as an Epson printer answers
  them (n = 1 printer, 2 offline cause, 3 error cause, 4 roll paper
  sensor), from the states below.
- A stopped printer (paper out, cover open, error) keeps the job in
  its buffer: the job is recorded with `printed: false`.
- Unplugged closes the port: connections are refused, as when the
  printer is off.
- A reachability check (connect and close) is not a job.
- Jobs are listed by `id`, the number of their connection in the order
  it was accepted: the print order, even when a long job finishes after
  a shorter one sent behind it. A connection that printed nothing leaves
  a gap in the ids.

## States

At start-up from the environment, then switched over HTTP without a
restart (a `POST` replaces all of them):

| Variable | Field | Values |
|---|---|---|
| `MOCK_PRINTER_PAPER` | `paper` | `ok`, `near_end`, `end` |
| `MOCK_PRINTER_COVER_OPEN` | `coverOpen` | `1` / `true` |
| `MOCK_PRINTER_ERROR` | `error` | `1` / `true` |
| `MOCK_PRINTER_UNPLUGGED` | `unplugged` | `1` / `true` |

```bash
curl -s localhost:9180/control -H 'content-type: application/json' -d '{"paper":"end"}'
curl -s localhost:9180/control -H 'content-type: application/json' -d '{"coverOpen":true}'
curl -s localhost:9180/control -H 'content-type: application/json' -d '{"unplugged":true}'
curl -s localhost:9180/control -H 'content-type: application/json' -d '{}'   # all fine
```

## Jobs

| Route | |
|---|---|
| `GET /` | The jobs as printed, newest first (refreshes every 3 s) |
| `GET /jobs` | JSON: `id`, `at`, `bytes`, `printed`, `drawerKick`, `text` |
| `GET /jobs/{id}` | One job as text |
| `DELETE /jobs` | Forget the jobs |

`MOCK_PRINTER_OUTBOX=/tmp/prints` also writes each job there as
`<id>.txt` (rendered) and `<id>.bin` (raw ESC/POS).

## In tests

```rust
let printer = pos_printer_mock::Printer::new(Control { cover_open: true, ..Control::default() }, None);
let addr = pos_printer_mock::bind("127.0.0.1:0".parse().unwrap(), printer.clone()).await.unwrap();
app.set_printer(&addr.to_string());
```

See `src-tauri/tests/spf_e2e_bar_slip.rs`.
