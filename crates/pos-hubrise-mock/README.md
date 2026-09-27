# pos-hubrise-mock

An in-memory HubRise API for development and tests. It follows the
rules of the real API that the POS depends on (`docs/hubrise/` at the
repository root):

- events reach a client only after it registered a callback
  (`POST /v1/callback`, `url: null` for passive polling) asking for
  that resource and event, and never for its own changes;
- `GET /v1/callback/events` is paginated (`count`, `cursor`,
  `X-Cursor-Next`) and lists events without their state fields;
  `DELETE /v1/callback/events/:id` acknowledges one;
- an order in `completed`, `rejected`, `cancelled` or
  `delivery_failed` cannot change status, nothing goes back to `new`
  (422);
- amounts must be `<amount> EUR` (422); `private_ref` is unique per
  location (422);
- `GET /v1/catalogs/:id` allows 10 reads a minute per connection (429);
- errors use HubRise's body (`{"error_type": ..., "message": ...}`);
- the token exchange takes HubRise's form body with HTTP Basic auth
  (a JSON body works too), and a code works once.

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
| `PUT /mock/catalog` `{"data": {...}}` | Replaces the default catalog (`{}`: the seed); announces `catalog.update`. |
| `POST /mock/faults` `{"method", "path", "status", "times", "delay_ms"}` | Fails (or delays) the next matching API requests; `DELETE` clears. |
| `GET /mock/requests?path=/v1/...` | Every API request received: method, path, query, client, body, status. `DELETE` clears. |
| `GET /mock/events?token=...` | Events waiting for a client, with state fields. |
| `POST /mock/limits` `{"catalog_reads_per_minute": n}` | Changes the catalog read limit. |
| `POST /mock/reset` | Back to the seeded state. |
