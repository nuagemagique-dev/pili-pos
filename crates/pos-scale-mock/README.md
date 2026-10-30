# pos-scale-mock

A checkout scale for development and tests: it speaks the Dialog 06 records
of `pos-scale` over TCP, and a small HTTP server changes what is on it.

```bash
cargo run --package pos-scale-mock
```

| Variable | Default | |
| --- | --- | --- |
| `MOCK_SCALE_PORT` | 9190 | The Dialog 06 port (the POS `scale.tcp_address`) |
| `MOCK_SCALE_CONTROL_PORT` | 9191 | The control port |
| `MOCK_SCALE_CHECK` | `4711F336` | The record 10 values it accepts |

```bash
curl -X POST localhost:9191/load -d '{"grams":278}' -H 'content-type: application/json'
curl -X POST localhost:9191/motion -d '{"polls":5}' -H 'content-type: application/json'
curl -X POST localhost:9191/check -d '{"pending":true}' -H 'content-type: application/json'
curl localhost:9191/state
```

It accepts any record 10 equal to `MOCK_SCALE_CHECK` and ignores the random
number: the real algorithm is unknown. Passing this mock proves the dialog,
not that a real scale will accept the values.
