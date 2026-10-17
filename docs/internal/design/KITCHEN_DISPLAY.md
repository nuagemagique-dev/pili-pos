# Kitchen display and station routing (design note)

Status: design only, no code. Target: a later certified update after v1.1.
Audience: the planner of the next lane. Language: English (internal).

## 1. Goal

Replace (or complete) the kitchen and bar paper slips with a screen in the kitchen: each order the waiter
registers appears on the display of the station that prepares it (kitchen, bar, sushi counter), the cook
marks it started and ready, and the waiter sees it is ready. The display shows no amount at all.

The fiscal part is nearly empty: the display reads what the till already signs (the P ORDER event of a
table, or the N of a direct sale). The work is routing, state and transport.

## 2. What the spec requires

| Source | Requirement |
| --- | --- |
| `docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md` around line 731 | "Les tickets de cuisine et de bar ne mentionnent JAMAIS de montants. S'ils sont imprimes suite a l'etablissement d'un event PRO FORMA, ils doivent contenir cette mention." A display is not a ticket, but the same prudence applies: no price on a kitchen surface. |
| Same file, around line 796 ("Vente interrompue (gestion des tables)") | The event P that holds the original order is sent in chronological order with its corrections; the matching print (provisional bill or kitchen/bar ticket) may be consolidated and grouped. A display must show the corrections (negative lines) as corrections, not hide them. |
| `docs/20260717_FAQ_PRODUCTEURS_FR.md` Q210 (composite products) | An option without extra cost is not a sub-product, is not sent to the FDM, and is handled by the POS "for the display of the kitchen or bar ticket". So the options on the display come from the till's own data (`modifiers_json`), not from the signed message. |
| Same FAQ, Q211 | Each `costCenter` has a unique `reference`: a table session must not reuse "T1" across sessions. The display keys tickets by the order reference, not by the table label. |
| Advanced Set 1.6, table cases UC240 to UC252 | Split and merge of tables move lines between cost centers; the display must follow a transfer (a ticket changes table label, its lines do not reappear as new). |
| `docs/legislation/20240429_AM_aspects_techniques_certification_FR.md` art. 43 and 44 | No digital signature on prints other than the VAT ticket (so none on a display); data entered may not change between entry and finalisation in a way that alters signed records (the display is a copy, not an edit surface). |

No certification test mentions a kitchen display; it needs no new UC and creates no obligation. The risk is
only that it must never contradict the P events.

## 3. Today in the code

- `menu_items.kitchen_printer` (integer flag, default 1; `CLAUDE.md` schema) decides whether a product goes to
  the kitchen/bar slip. `pos_db::queries::menu::sends_to_kitchen` reads it. It is a boolean, so there is exactly
  one station.
- `crates/pos-engine/src/commands/bar_slip.rs` prints the slip after a signing: the lines just transmitted,
  corrections included, for the flagged products, on `kitchen_printer_address` if set, else the ticket printer.
  A printer failure never undoes the signing (`SlipOutcome::Failed`).
- `crates/pos-print/src/bar_ticket.rs`: the official layout, no price, the "not a VAT ticket" mention.
  `kitchen_ticket.rs` is the (non-certified) layout of webshop orders.
- `commands/tables.rs`: `do_sign_table_order` signs the P ORDER when the waiter leaves the table and returns the
  slip outcome to the UI; direct sales (counter) print the slip with the N.
- `modifiers_json` on `order_lines` holds the options chosen at order time.
- Webshop and platform orders (HubRise) already print a kitchen ticket at acceptance (`print_webshop_kitchen`).

## 4. Proposal

### 4.1 Stations

- New table `stations(id, code, name_fr, name_nl, name_en, name_zh, printer_address NULL, display_enabled,
  sort_order, active)` and `menu_item_stations(menu_item_id, station_id)`. The existing flag becomes "has at
  least one station"; migration copies `kitchen_printer = 1` into a default "Cuisine" station that keeps the
  current `kitchen_printer_address`.
- A product may go to several stations (a combo to kitchen and bar). The menu editor (V11) gets a station
  picker, locked under HubRise like the rest of the menu.
- Routing is by product at order time, copied onto the order line (`order_lines.station_id`) so that a later
  menu edit cannot move a line already ordered (art. 44 spirit).

### 4.2 Kitchen tickets

On every signing that carries lines (P ORDER, or N of a direct sale), the engine creates one **kitchen ticket**
per station in the same transaction that archives the event (so a ticket exists if and only if the event was
signed, and crash recovery is the existing pending-sign machinery):

- `kitchen_tickets(id, fiscal_event_id, station_id, order_id, order_reference, table_label, created_at,
  status)` with status `new`, `started`, `ready`, `served`, `cancelled`.
- `kitchen_ticket_lines(ticket_id, order_line_id NULL, name, quantity, options_json, kind)` where `kind` is
  `add` or `correction` (a negative quantity). No price column exists, so a price cannot be shown by mistake.
- `kitchen_ticket_log(id, ticket_id, status, by_user, at)`, append-only, the timeline for service-time stats.

Paper slips stay: a station with a printer still prints (the slip is the fallback when the display is down).
The slip and the display are two outputs of the same ticket rows.

### 4.3 Status flow and who may change it

`new` -> `started` -> `ready` -> `served`. The kitchen taps a ticket to start and to bump it ready; the waiter
(or the cashier) marks served; a manager can cancel. A correction line on an existing ticket flashes it. A
cancelled order (voided) marks its tickets `cancelled`. The status is never fiscal: no event, no counter.

### 4.4 Transport

The display is a second screen of the same till or a separate device on the LAN:

1. Same device (second HDMI output, or a second window): the Vue UI route `/kitchen` or an Android activity;
   no transport.
2. Separate device (a cheap tablet in the kitchen): needs the LAN access discussed in
   `WAITER_HANDHELD_SYNC.md` (an authenticated local HTTP/WebSocket API of the main till). The display is the
   simplest client of that API: it only reads tickets and writes a status. Build it **after** the sync note's
   server mode, or limit v1 to option 1.

## 5. Data model

Next free migration after 42 (a v1.2 number, not reserved here): `stations`, `menu_item_stations`,
`order_lines.station_id`, `kitchen_tickets`, `kitchen_ticket_lines`, `kitchen_ticket_log` (append-only
triggers like `sold_out_log`). The change log of the menu editor records station edits.

## 6. Fiscal events and payloads

None new. The tickets are derived from P ORDER (`signOrder`) and N (`signSale`) after they are archived. The
only constraint is that the ticket lines equal the transmitted entries: the `bar_slip` rule "only the entries
just sent, corrections included" is kept. A test compares the tickets of a service day with the P payloads of
the Reports 1.3 scenario.

## 7. UI

- Vue: a `KitchenView.vue` route (full-screen cards by age, colour by status, large touch targets, no price),
  a station picker in the menu editor, a "ready" notification on the tables screen (toast and a dot on the
  table). Strings fr, en, zh (the kitchen may be Chinese or French speaking; the option names follow the
  device language). A Playwright spec per flow.
- Android: `feature/kitchen` with the same card screen for a tablet and a small badge on the waiter handheld;
  the notification reuses the engine event stream (`PosEvent`) rather than polling.
- Settings: stations list and their printers.

## 8. Tests

Engine: routing by product, two stations, correction lines, a table transfer, status transitions and who may
make them, a voided order, restart with open tickets, tickets created atomically with the archive (crash test
with `pending_signs`); a test that no ticket row or screen text contains a price; paper fallback; the service
day of the Reports 1.3 scenario gives the expected tickets. UI: specs and goldens for the card states.

## 9. Effort (worker-days)

| Layer | Same-device display | Separate device (needs the sync API) |
| --- | --- | --- |
| Database and migration (from the old flag) | 3 | 0 |
| Engine (routing, tickets, statuses, events) | 6 | 3 |
| Vue | 5 | 1 |
| Android and FFI | 5 | 2 |
| Menu editor and settings (stations) | 3 | 0 |
| Tests | 5 | 3 |
| Dossier (French) and runbook | 1.5 | 0.5 |
| Total | about 28.5 | about 9.5 on top |

## 10. Risks

- Routing mistakes lose food: an item with no station must still appear somewhere (default station) and the
  settings screen must warn about products with none.
- A display that misses a correction is worse than paper: the correction must be a visible event on the ticket.
- Screen-off or crash of the display device: the paper fallback and a "tickets older than N minutes" alarm.
- The V1 `kitchen_printer` flag is certified behaviour: the migration must keep every current slip identical.
- Scope creep into recipes, allergens and timing courses (fire/hold): out of scope here.

## 11. Open questions

- Q1. One kitchen only (the first customers) or several stations from the start? If one, 4.1 can be skipped
  and the work shrinks by about a third.
- Q2. Who may bump a ticket (any cook with no login, or a station PIN)? Today every action is tied to a user.
- Q3. Courses (starter, main, dessert) with a "fire" action: wanted? It changes the P event cadence.
