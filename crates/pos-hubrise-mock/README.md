# pos-hubrise-mock

An in-memory HubRise API for development and tests. It follows the
rules of the real API that the POS depends on (`docs/` at the
repository root):

- events reach a client only after it registered a callback
  (`POST /v1/callback`, `url: null` for passive polling) asking for
  that resource and event, and never for its own changes; a
  non-`null` `url` (active/push callbacks) is refused (422) rather
  than silently treated as passive;
- `GET /v1/callback/events` is paginated (`count`, `cursor`,
  `X-Cursor-Next`) and lists events without their state fields;
  `DELETE /v1/callback/events/:id` acknowledges one;
- by default, an order in `completed`, `rejected`, `cancelled` or
  `delivery_failed` cannot change status, nothing goes back to `new`
  (422), which is the mock's own, invented policy (the docs define no
  transition graph); see "Toggles" below to switch to the
  docs-permissive behaviour;
- amounts must be `<amount> EUR` (422), and a 422 carries per-field
  `errors: [{field, message}]` alongside `message`;
- `private_ref` uniqueness is per-client by default, matching the
  docs (a lookup convenience, not a server constraint); see
  "Toggles" for the mock's old, location-wide-unique behaviour;
- `GET /v1/catalogs/:id`, `GET .../orders` and
  `GET .../customer_lists/:id/customers` allow 10 reads a minute per
  connection (429), on top of the connection-wide 500/min, 2500/hour,
  10000/day limits (429);
- inventory is scoped by catalog and location
  (`GET/PUT/PATCH /v1/catalogs/:id/locations/:id/inventory`, plus
  unscoped/single-scoped shorthands the mock also serves); `PATCH`
  with `stock: null` removes an entry; announces `inventory.update`;
- `POST /oauth2/v1/revoke` makes the token unusable (401 afterward)
  until `/mock/reset`;
- a write with a non-JSON `Content-Type` is refused (415); an
  unmatched route answers HubRise's `routing_error` envelope (404);
- errors use HubRise's body (`{"error_type": ..., "message": ...,
  "errors": [...]}`);
- the token exchange takes HubRise's form body with HTTP Basic auth
  (a JSON body works too), and a code works once.

## Toggles

Two behaviours the mock invented (not documented by HubRise) default
to the mock's long-standing policy, because every existing POS
integration test (`src-tauri/tests/hubrise_*.rs`) is written against
it. Flip either off with `PUT /mock/settings` to exercise the more
permissive, docs-only behaviour instead:

```
PUT /mock/settings
{"strict_transitions": false, "strict_private_ref": false}
```

- `strict_transitions` (default `true`): final statuses lock, nothing
  returns to `new`. `false`: any documented status may follow any
  other, per the docs' silence on a transition graph.
- `strict_private_ref` (default `false`): duplicate `private_ref`
  lookup is scoped to the creating client, per the docs. `true`:
  refuses (422) a `private_ref` already used by *any* client at the
  location, which is the mock's old behaviour.

`rate_limit_per_minute`, `rate_limit_per_hour` and
`rate_limit_per_day` are also settable through the same endpoint (all
default to the documented connection-wide limits). The per-minute
read limits (`catalog_reads_per_minute`, `order_list_reads_per_minute`)
are set through `POST /mock/limits` instead, below.

```
cargo run -p pos-hubrise-mock            # http://localhost:3001
HUBRISE_MOCK_PORT=3101 cargo run -p pos-hubrise-mock
```

Two clients are seeded: the POS (`dev-token-pos`, OAuth client
`pili-pos`) and the ordering side (`dev-token-webshop`, client
`tsb-webshop`). The default catalog (`pili-dev-cat-1`) is a small sushi
menu: tax rates per service type, a product with two SKUs, shared
option lists, a delivery charge (`src/builders.rs`).

Tests mount the router in-process: `pos_hubrise_mock::app(state)`.

## Control API (tests only)

| Request | Effect |
|---|---|
| `POST /mock/orders` `{"template": "...", "order": {...}}` | Creates an order as the ordering side (the POS is told). Templates: `deliveroo_prepaid`, `ubereats_prepaid`, `webshop_online`, `webshop_cash_collection`, `webshop_cash_delivery`, `eat_in`. `order` fields replace the template's. |
| `PATCH /mock/orders/:id` | Changes an order as the ordering side. |
| `POST /mock/orders/:id/cancel` | The platform cancels the order. |
| `PUT /mock/catalog` `{"data": {...}}` | Replaces the default catalog (`{}`: the seed); announces `catalog.update`. Rejects a structurally invalid catalog whole (422). |
| `PUT /mock/inventory` `{"catalog_id": "...", "entries": [...]}` | Replaces the stock entries for a catalog; announces `inventory.update`. |
| `POST /mock/inventory/expire` | Announces every default-catalog entry that carries an `expires_at`, as HubRise's documented `inventory.patch` expiry notice. |
| `POST /mock/faults` `{"method", "path", "status", "times", "delay_ms"}` | Fails (or delays) the next matching API requests; `DELETE` clears. |
| `GET /mock/requests?path=/v1/...` | Every API request received: method, path, query, client, body, status. `DELETE` clears. |
| `GET /mock/events?token=...` | Events waiting for a client, with state fields. |
| `POST /mock/limits` `{"catalog_reads_per_minute": n, "order_list_reads_per_minute": n}` | Changes the per-minute read limits. |
| `PUT /mock/settings` `{...}` | Flips the behaviour toggles, see "Toggles" above. |
| `POST /mock/reset` | Back to the seeded state (also clears revoked tokens and settings). |
