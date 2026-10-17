# QR table ordering (design note)

Status: design only, no code. Target: a later certified update after v1.1.
Audience: the planner of the next lane. Language: English (internal).

## 1. Goal

A guest scans a QR code on the table, sees the menu on their phone, orders and, optionally, pays. The waiter
sees the order appear on the table and the kitchen receives it. Two modes:

- **A. Order only**: the guest sends the order, the table is billed as usual by the waiter at the till.
- **B. Order and pay**: the guest pays on the phone (Mollie or the restaurant's PSP) and the till signs the
  sale with an `ONLINE` payment.

## 2. What the spec requires

| Source | Requirement |
| --- | --- |
| `docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md` `costCenter` (around lines 1215 to 1275) | Interrupted sales (tables, customers, ...) must carry a `costCenter`; types include `TABLE`, `KIOSK`, `PLATFORM`, `WEBSHOP`, `OTHER`. `WEBSHOP` is for "orders received from the restaurant's own webshop linked to the POS and for which the signSale is not created immediately"; `KIOSK` exists for self-service terminals; `OTHER` for any other parking of orders. |
| Same file, `signOrder` (costCenter mandatory) | An order registered without being paid is an event P (`signOrder`) with its cost center, then a pre-bill (P) and the sale (N) later. |
| `docs/20260717_FAQ_PRODUCTEURS_FR.md` Q203 and Q211 | Every online order ends in an N; a prepaid order is signed N at acceptance with payment `APP` or `ONLINE`; an unpaid one is registered P and signed N at handover with the same cost center; a `costCenter.reference` is unique (never reuse "T1" across sessions). |
| `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/UC4123_CostCenterChair` and `UC4101` to `UC4104` | Cost center changes (table to chair, table to room, split bill) are legal and documented; a QR order that is moved to another table follows them. |
| `docs/legislation/20240429_AM_aspects_techniques_certification_FR.md` art. 14 | Nothing may be charged unless the FDM is operational at that moment. A phone payment taken while the FDM is down is a charge with no event. |
| Certification manual section M (third party integration) | Any online channel (platform, own site) must be declared and described in the dossier. |

## 3. Today in the code

- Webshop and platform orders (HubRise) are first-class: `commands/webshop.rs` signs prepaid orders N with
  payment `ONLINE` or `APP` at acceptance and unpaid ones P then N, with cost center `WEBSHOP` or `PLATFORM`;
  `pos-hubrise` polls HubRise and `commands/hubrise_orders_*` ingest. The tickets and the kitchen ticket print
  at acceptance. The human accepts or rejects (the "Commandes en ligne" screen).
- A table order is created by staff only (`commands/order.rs`, `commands/tables.rs`); orders have `source`
  (`pos_local`, HubRise) and `table_number`. No public endpoint exists: the engine listens on no port except
  the local health check of the Linux shell (`127.0.0.1:9999`).
- Payments: cash, card (manual or EFT), meal vouchers, `APP`, `ONLINE` (never offered at the till).
- Menu and sold-out state are local (V11: menu editor, "Epuise"), so the data a guest sees exists.
- The tsb-core webshop in the same repository family shows how a guest-facing site is built, but it is a
  separate product with its own backend.

## 4. Two architectures

### 4.1 Through HubRise (reuse what is certified)

A small guest web app (outside the till) renders the menu from the HubRise catalog and posts an order to
HubRise as a webshop order with the table number in the order notes or a custom field. The till then sees it
like any webshop order. Pros: no new inbound port on the till, the certified online-order path (Q203) is
reused as it is. Cons: depends on HubRise (a paid service, latency of the poll), the table is not a first-class
cost center (`WEBSHOP` with the table in a note), the HubRise catalog must be maintained (the menu editor is
locked under HubRise), and mode A (order only, pay at the till) fits the "unpaid, P then N" path with a
handover step per order that was designed for delivery, not for tables.

### 4.2 Own channel (a local or cloud relay)

A relay service (small cloud service or a process on the till's LAN behind a tunnel) serves the guest page and
queues orders for the till. The till **pulls** the queue (outbound only, like the HubRise poller) so that no
inbound port is opened. Orders arrive as `qr_orders` rows; staff confirm (or auto-confirm for trusted tables),
and the engine creates a normal table order, with `costCenter` of type `TABLE` (the guest chose a table) and
reference of the table session, then signs the P ORDER, prints the kitchen slip, and the waiter bills at the
till (mode A) or the order is paid online (mode B, below).

Recommendation: start with 4.2 mode A. It reuses the table order, P signing and bar slip paths unchanged and
adds one inbound queue. Mode B and the HubRise variant come after, if the owner wants them.

## 5. Proposal (4.2, mode A then B)

### 5.1 Table session and QR

- Each table has a stable QR URL `https://<relay>/t/<restaurant>/<table>?s=<session-token>`. The session token is
  rotated when the table is closed (N signed) or after N hours, so an old photo of a QR cannot order for the
  next guests. The token is the proof the guest is at the table; a manager can also refuse orders per table.
- The relay returns the menu (names in the guest's language, sold-out flags, options), prices and VAT are not
  shown or sent beyond the display price; the till recomputes everything (the guest page sends product ids,
  quantities and option ids only).

### 5.2 Order intake

- `qr_orders(id, remote_id UNIQUE, table_number, session, created_at, status, payload_json, accepted_by,
  accepted_at, order_id NULL, reject_reason)`, status `pending`, `accepted`, `rejected`, `expired`.
  Never deleted; append-only log `qr_order_log`.
- The till polls the relay; each new order becomes `pending` and an alert on the tables screen (sound and
  badge). Accepting creates or extends the table's order with the lines (prices from the till's own menu,
  sold-out products refused with `product_sold_out`), signs the P ORDER, prints the slip, and acknowledges to
  the relay so the guest sees "received".
- Several guests order at the same table: lines carry an optional `guest_label`; the bill is still one table
  (split bills use the existing flows).

### 5.3 Paying on the phone (mode B)

A guest-paid order is a **prepaid** order: the sale N is signed when the payment is confirmed, payment type
`ONLINE`, cost center the table (or `WEBSHOP` with a unique reference when no table), exactly the Q203 path of
webshop orders (`commands/webshop.rs` `sell`). Rules to carry over:

- The charge happens only after the till confirms the FDM is operational (AM art. 14): the relay asks the till
  "may I charge?" before creating the payment intent, and the intent has a short expiry.
- The payment provider webhook goes to the relay, which hands the confirmed payment to the till; the till
  signs N and the ticket is delivered digitally (the guest has no paper): the digital ticket path exists
  (`delivery.rs`, email or the verification URL).
- A failed signing after a captured payment follows the refund path with payment `ONLINE` (`RefundSpec` for
  online orders).
- Tips and split payment by seat are separate topics (see `TIPS_ON_CARD.md`).

### 5.4 Abuse and safety

Rate limits and an order size cap at the relay; a "call the waiter" button instead of free text; no personal
data is collected (GDPR: no name, no phone; the session token is the only identifier, kept for the session);
the relay never sees card data (hosted payment page of the provider).

## 6. Data model

Next free migration after 42: `qr_orders`, `qr_order_log`, `qr_table_sessions(table_number, token_hash,
created_at, closed_at)`, `order_lines.guest_label`, and `orders.source` gets the value `qr`. Settings:
relay URL, restaurant key, polling interval, auto-accept flag (off by default).

## 7. Fiscal events and payloads

- Mode A: P ORDER (`signOrder`) with `costCenter {type TABLE, id "T04", reference <session reference>}`,
  later P PRE_BILL and N by the waiter. Unchanged payloads.
- Mode B: N (`signSale`) with `ONLINE` payment (provider name in `provider`, the intent id in `reference`),
  cost center as above. If the table also has staff-entered lines, the N covers the whole bill and the guest
  pays what is due (partial payments are the split-payment path).
- Corrections after acceptance (the guest removes an item): the existing negative-line correction in P.
- No new event type; the dossier declares a new third party (relay and payment provider) in section M.

## 8. UI

- Guest page: not part of the till UIs. A small static page plus relay (outside this repository's apps), fr,
  nl, en, zh, with the same product names as the till.
- Vue and Android: an "Orders from tables" panel (badge, list, accept or reject, "table X ordered 3 items"),
  settings for the relay, a per-table "QR on/off" and a "print QR" action; strings fr, en, zh; parity rows.

## 9. Tests

Engine with a mock relay (like `pos-hubrise-mock`): intake, duplicate delivery (idempotent on `remote_id`),
sold-out refusal, FDM down refusal for mode B, crash between payment confirmation and signing (recovery),
rotated session token, reject path, accepted order equals the P payload of a staff order. Security tests of
the relay are a separate project.

## 10. Effort (worker-days)

| Layer | Mode A (order only) | Mode B (pay on phone) |
| --- | --- | --- |
| Relay service and guest page (new component) | 12 | 8 |
| Database and engine intake | 6 | 5 |
| Signing and recovery | 1 | 6 |
| Vue | 4 | 2 |
| Android and FFI | 4 | 2 |
| Tests | 6 | 6 |
| Dossier, runbook, GDPR note | 3 | 2 |
| Total | about 36 | about 31 more |

## 11. Risks

- Fiscal: money captured without an N (mode B) is the worst case; the "ask the till first" step and the
  recovery rules are mandatory, not optional.
- Security: a public page that creates orders on a till; the till only pulls and validates, never trusts prices.
- Operations: a relay is a new service to run, back up and pay for; the HubRise path avoids that.
- Table confusion: guests at the wrong table, or an old QR photographed; the rotating session token reduces it,
  staff confirmation removes it.
- Menu drift: the guest menu must equal the till menu, including sold-out, or orders are refused at the table.

## 12. Open questions

- Q1. HubRise path (4.1) or own relay (4.2)? Depends on whether the owner keeps HubRise.
- Q2. Is mode B wanted, or does the owner want to keep the bill at the till?
- Q3. Auto-accept orders or always confirm by staff? (Conservative: confirm.)
- Q4. Does the SPF require the QR ordering page to be part of the certified perimeter? Ask with the third party
  declaration.
