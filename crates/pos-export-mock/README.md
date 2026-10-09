# pos-export-mock

Test receiver of the nightly export (phase X1, decision X1D8). It is the HTTP
endpoint that the POS uploads the encrypted backup, the statistics file and the
manifest to, for the unit tests of `pos-export`, the engine tests and manual
runs (including the Android emulator).

**Not part of the certified build.** It is a test tool: a dev-dependency of
`pos-export` (and later of the engine tests) and a binary of its own. It is
never a normal dependency of a shipped crate, so it is not in
`cargo tree -p pili-pos --no-default-features --features cert -e normal,build`.
It speaks plain HTTP on a local port; the engine accepts plain HTTP only
towards `127.0.0.1`, `localhost`, `[::1]` and `10.0.2.2`.

## HTTP contract

`PUT <base url>/<export_id>/<file name>`, for example
`PUT http://127.0.0.1:9330/20261008-z000042/manifest-20261008-z000042.json`.

| Request | Answer |
|---------|--------|
| No `Authorization: Bearer <token>`, or a wrong token | 401 |
| Any method other than PUT | 405 (401 when `set_unauthorized(true)`) |
| A pending `fail_next(n, status)` | `status` (the upload is counted, not stored) |
| Path with `..`, `.`, an empty segment, `\`, `%` or a control character, or empty | 400 |
| `X-Content-SHA256` missing, or not the lowercase hex SHA-256 of the body | 400 |
| Anything else | 201, the upload is stored |

Order of the checks: token, then `fail_next`, then path, then hash. A refused
token never consumes a pending `fail_next`. The headers the client sends are
`Authorization`, `Content-Type` (`application/octet-stream`, or
`application/json` for the manifest), `X-Content-SHA256` and
`User-Agent: pili-export/1`. The body limit is 1 GiB. Putting the same path
twice keeps both uploads in `received()`; `file(path)` returns the latest.

The mock is strict on one point the contract only implies: a missing
`X-Content-SHA256` is a 400, so a client that forgets the header fails its test.

## Library

```rust
let receiver = pos_export_mock::Receiver::start("test-token"); // 127.0.0.1:0
let base = receiver.url();                                       // http://127.0.0.1:<port>
```

`Receiver::start_on(addr, token, dir)` binds a chosen address; with
`dir: Some(path)` every accepted body is also written to `<dir>/<path>`. The
receiver serves on its own thread and runtime, so it works from synchronous and
asynchronous tests, and stops on drop or on `stop()` (idempotent, never waits
for a delayed answer).

Test controls, thread-safe and callable while serving:

| Call | Effect |
|------|--------|
| `fail_next(n, status)` | answer `status` to the next `n` PUTs that pass the token check |
| `set_delay(Duration)` | sleep before answering every PUT (proves the till never waits); zero clears it |
| `set_unauthorized(bool)` | answer 401 to every request |
| `received()` | `Vec<Received { path, bytes, sha256, content_type }>` in arrival order |
| `file(path)` | the body of the latest upload to `path` (`<export_id>/<file name>`) |
| `put_count()` | every PUT seen, whatever the answer |
| `set_observer(f)` | `f(path, size, status)` after each PUT (the binary prints from it) |

## Binary

```bash
printf '%s' 'a-test-token' > /tmp/export-token
cargo run -p pos-export-mock -- --listen 127.0.0.1:9330 --token-file /tmp/export-token [--dir /tmp/export-received]
```

`--listen` defaults to `127.0.0.1:9330`; `0.0.0.0:9330` lets the emulator reach
it at `http://10.0.2.2:9330`. The token file holds the bearer token (trimmed;
the token is never printed). It prints one line per PUT on stdout:

```
pos-export-mock listening on http://127.0.0.1:9330
PUT /20261008-z000042/pos-20261008-z000042.db.age 183412 bytes -> 201
PUT /20261008-z000042/stats-20261008-z000042.sqlite 40960 bytes -> 201
PUT /20261008-z000042/manifest-20261008-z000042.json 1204 bytes -> 201
```

Exit codes: 0 after `--help`, 1 when the token file or the address is unusable,
2 on a usage error.

## Tests

```bash
cargo test -p pos-export-mock -p pos-export
```
