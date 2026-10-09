# Statistics file, schema v1

This is the contract of the cleaned statistics copy that a pili-pos installation
uploads after each real Z report (decision X1D9). It is written for the
developers of the receiver and of the dashboard. The code that produces the file
is `src/stats.rs`; the tests `tests/stats_build.rs` and `tests/stats_privacy.rs`
check this document against it: every table and every column below exists in the
file, and the file has no table or column that is not listed here.

## The file

- One SQLite database per Z report, named `stats-<export_id>.sqlite`, uploaded
  beside `pos-<export_id>.db.age` (the encrypted backup) and
  `manifest-<export_id>.json`. `<export_id>` is `<booking date YYYYMMDD>-z<Z number
  on 6 digits>`, for example `stats-20261008-z000042.sqlite`.
- It describes one booking period: the sales that the Z report closed, and that Z
  report. A day is one file; there is no history in it. To follow several days,
  read several files; `meta.z_report_no` orders them.
- It is a plain SQLite file: rollback journal, no WAL, `PRAGMA user_version = 1`,
  no extension, no `STRICT` table, readable by any SQLite 3.x. Every table has a
  declared primary key except `ticket_discounts`.
- It is never modified after it is written. The producer builds it as
  `<name>.partial` and renames it when complete.
- Conventions: table and column names are snake_case. Money is an `INTEGER` in
  euro cents (the currency is `EUR`, see `meta`), VAT rates are `INTEGER` basis
  points (2100 is 21 %). Amounts are signed: a refund is negative. Date-times are
  RFC 3339 strings. Every column has a comment in the `CREATE TABLE` statement
  (`SELECT sql FROM sqlite_master`). Closed sets of values are guarded by `CHECK`
  constraints.

## What is in it, and what is not

In: the sale events (fiscal label `N`) archived in the booking period, one
`tickets` row each, and the turnover Z report that closed the period. Amounts are
those the FDM signed (the archived request and the FDM's VAT calculation); the
producer never recomputes them from the menu.

Out: training events (label `T`), staff presence (`S`), copies (`C`), orders and
pro forma bills (`P`), money in and out and drawer events (`F`), invoices (`I`),
the user Z report, and everything that is not a fiscal event (orders still open,
the catalogue, settings). A refund is not left out: it is a sale event with
negative amounts.

## Tables

### `meta`

Key and value pairs describing the file. The value is always text.

| column | type | description |
|--------|------|-------------|
| `key` | TEXT, primary key | name of the value, see the list below |
| `value` | TEXT | the value |

Keys, all always present:

| key | value |
|-----|-------|
| `format` | `pili-stats` |
| `schema_version` | `1` |
| `export_id` | the export id, as in the file name |
| `pos_id` | fiscal identifier of the POS (`posId`) |
| `terminal_id` | identifier of the terminal |
| `pos_sw_version` | software version of the POS, for example `pili-pos/1.1.0` |
| `z_report_no` | number of the Z report, decimal |
| `booking_date` | booking date of the closed period, `YYYY-MM-DD` |
| `period_opened_at` | when the period opened (UTC, RFC 3339) |
| `period_closed_at` | when the Z closed it (UTC, RFC 3339) |
| `currency` | `EUR` |
| `generated_at` | when this file was generated (UTC, RFC 3339); the only value that changes when the same period is exported again |

### `users`

The users that appear in this file, as pseudonyms (see "Pseudonyms"). Only users
who signed a ticket of this file or made its Z report are listed.

| column | type | description |
|--------|------|-------------|
| `user_ref` | TEXT, primary key | pseudonym: `system`, or `u-` and 16 lowercase hex characters |
| `role` | TEXT | `admin`, `cashier`, `waiter` or `system`; an employee unknown to the POS is reported as `cashier` |

### `tickets`

One row per sale event of the period (including refunds), in the order they were
signed.

| column | type | description |
|--------|------|-------------|
| `ticket_id` | INTEGER, primary key | local id of the sale event in the POS archive; the key of every `ticket_*` table. Unique within one installation, not across installations |
| `event_label` | TEXT | label under which the FDM signed the event: `N` |
| `ticket_no` | INTEGER | fiscal ticket number of the POS (`posFiscalTicketNo`) |
| `kind` | TEXT | `sale`, or `refund` (the event refers to an earlier ticket, or its total is negative) |
| `channel` | TEXT | `table`, `takeaway`, `delivery` or `online`, see "Channels" |
| `table_number` | INTEGER, nullable | table of a `table` ticket, NULL for the other channels |
| `user_ref` | TEXT | pseudonym of the user who signed the sale, a `users` row |
| `fdm_datetime` | TEXT | time of the FDM signature as the FDM gave it (RFC 3339, UTC) |
| `total_cents` | INTEGER | value of the ticket after price changes; negative for a refund |
| `unpaid` | INTEGER | 1 when the sale was closed without any payment (the customer left), else 0 |

### `ticket_lines`

The lines of a ticket, in the order of the event. A line that was corrected
before the sale appears with its negative twin, as in the signed event.

| column | type | description |
|--------|------|-------------|
| `ticket_id` | INTEGER | ticket of the line |
| `line_no` | INTEGER | position in the ticket, from 1; primary key with `ticket_id` |
| `product_code` | TEXT, nullable | menu code of the product (`menu_items.code`, for example `A2`); NULL when the menu has none or the product is no longer in the menu |
| `product_name` | TEXT | name of the product as signed |
| `department` | TEXT, nullable | department (category) name as signed; NULL when empty |
| `quantity` | REAL | quantity sold; negative on a refund or a correction |
| `unit_price_cents` | INTEGER | unit price, VAT included, before price changes |
| `amount_cents` | INTEGER | value of the line after price changes: `round(quantity x unit_price_cents) + discount_cents`, signed |
| `discount_cents` | INTEGER | sum of the price changes of the line, negative for a discount, 0 when none |
| `vat_label` | TEXT | `A` (21 %), `B` (12 %), `C` (6 %), `D` (0 %) or `X` (out of scope) |
| `vat_rate_bp` | INTEGER, nullable | VAT rate in basis points, taken from the FDM's VAT calculation of the ticket |

### `ticket_vat`

The FDM's VAT calculation of a ticket, one row per VAT label.

| column | type | description |
|--------|------|-------------|
| `ticket_id` | INTEGER | ticket |
| `vat_label` | TEXT | VAT label; primary key with `ticket_id` |
| `vat_rate_bp` | INTEGER | VAT rate in basis points |
| `base_cents` | INTEGER | taxable amount |
| `vat_cents` | INTEGER | VAT amount |
| `total_cents` | INTEGER | amount with VAT (`base_cents + vat_cents`) |

### `ticket_payments`

The payment lines of a ticket, in the order of the event. An unpaid ticket has
none.

| column | type | description |
|--------|------|-------------|
| `ticket_id` | INTEGER | ticket |
| `line_no` | INTEGER | position of the payment line, from 1; primary key with `ticket_id` |
| `method` | TEXT | `cash`, `card`, `meal_voucher`, `app` (paid in an ordering platform's app), `online` (paid on the restaurant's webshop) or `other`, see "Payment methods" |
| `kind` | TEXT | `payment`, `rounding` (the 5-cent cash rounding, on its own line after the cash payment) or `tip` |
| `amount_cents` | INTEGER | amount; negative on a refund |

The `payment` lines add up to the ticket total, an unpaid ticket excepted; the
`rounding` line is the cash adjustment on top of it.

### `ticket_discounts`

The price changes (discounts and surcharges) of a ticket. No primary key.

| column | type | description |
|--------|------|-------------|
| `ticket_id` | INTEGER | ticket |
| `line_no` | INTEGER, nullable | line the change applies to; NULL for a change on the whole ticket (its amount is the sum over the lines it was spread on) |
| `kind` | TEXT | `public` (shown to the customer) or `internal` (a recalculation) |
| `amount_cents` | INTEGER | amount, negative for a discount |

The sum of `ticket_discounts.amount_cents` equals the sum of
`ticket_lines.discount_cents`. The names of the promotions are not exported.

### `z_report`

The Z report of the period, one row.

| column | type | description |
|--------|------|-------------|
| `z_report_no` | INTEGER, primary key | number of the Z report |
| `booking_date` | TEXT | booking date of the closed period (`YYYY-MM-DD`) |
| `event_label` | TEXT | label under which the FDM signed the Z: `R` |
| `ticket_no` | INTEGER | fiscal ticket number of the Z |
| `user_ref` | TEXT | pseudonym of the user who made the Z; `system` for the automatic Z of a day left open |
| `fdm_datetime` | TEXT | time of the FDM signature of the Z (RFC 3339, UTC) |
| `sales_count` | INTEGER | number of sale tickets (label `N`) of the period, refunds included |
| `refunds_count` | INTEGER | number of those tickets that are refunds |
| `total_cents` | INTEGER | value of all sale tickets, as signed in the Z |

### `z_vat`

The VAT totals of the Z report, one row per VAT label, as signed.

| column | type | description |
|--------|------|-------------|
| `z_report_no` | INTEGER | Z report |
| `vat_label` | TEXT | VAT label; primary key with `z_report_no` |
| `vat_rate_bp` | INTEGER | VAT rate in basis points |
| `base_cents` | INTEGER | taxable amount |
| `vat_cents` | INTEGER | VAT amount |
| `total_cents` | INTEGER | amount with VAT |

### `z_payments`

The payment totals of the Z report, as signed, one row per method and kind.

| column | type | description |
|--------|------|-------------|
| `z_report_no` | INTEGER | Z report |
| `method` | TEXT | payment method, as in `ticket_payments` |
| `kind` | TEXT | `payment`, `rounding` or `tip`; primary key with `z_report_no` and `method` |
| `amount_cents` | INTEGER | total |

## Consistency rules

For one file, these hold (the tests check them):

- `SUM(tickets.total_cents) = z_report.total_cents` and `COUNT(*)` of `tickets` is
  `z_report.sales_count`.
- `SUM(ticket_vat.<x>_cents)` per VAT label equals `z_vat.<x>_cents`.
- `SUM(ticket_payments.amount_cents)` per `(method, kind)` equals the matching
  `z_payments` row.
- For each ticket, `total_cents` is the sum of its `ticket_lines.amount_cents` and
  of its `ticket_vat.total_cents`.
- Rebuilding the same period from the same data gives the same rows; only
  `meta.generated_at` changes.

## Channels

`tickets.channel` comes from the order the sale closed:

| channel | when |
|---------|------|
| `table` | a dine-in order of the POS (`order_type` `dine_in`), `table_number` set |
| `takeaway` | a takeaway sale at the till |
| `delivery` | an online order (it came from HubRise: a webshop or a platform) whose service type is `delivery` |
| `online` | any other online order (collection, eat-in) |

A refund takes the channel and the table of the ticket it refers to when that
ticket is in the POS archive; otherwise it is classified from its own order (the
till creates refunds as takeaway orders).

## Payment methods

`method` is derived from the payment type of the signed event:

| signed type | `method` |
|-------------|----------|
| `CASH` | `cash` |
| `CARD_DEBIT`, `CARD_CREDIT`, `CARD_UNKNOWN`, `CARD_OTHER` | `card` |
| `CHEQUE_MEAL` | `meal_voucher` |
| `APP` | `app` |
| `ONLINE` | `online` |
| any other | `other` |

## Pseudonyms

A user is identified by `user_ref`, never by name or INSZ/NISS.

- The system identity (automatic Z, online orders) is `system`.
- Any other user is `u-` followed by the first 16 lowercase hexadecimal
  characters of `SHA-256("pili-stats-user/1" || key || INSZ)`, where `key` is 32
  random bytes created once per installation, `INSZ` is the ASCII digits of the
  national number, and `||` is concatenation.
- The key lives in `exports/pseudonym.key` on the installation (mode 0600). It is
  never uploaded, is not stored in `pos.db`, and is therefore not in the encrypted
  backup either. Nobody who holds only the statistics files can go back to a
  person, or tell whether two installations have a person in common.
- The same person has the same `user_ref` in every file of the same installation
  (so shifts and productivity can be followed over days), and a different one on
  another installation. If the key file is lost, a new key is created and the
  pseudonyms change from then on; treat `user_ref` as an installation-local
  label, not as an identity.

## Privacy guarantees

The file never contains: INSZ or NISS, user names, PIN hashes, tokens or secrets,
e-mail addresses, phone numbers, postal addresses, customer names, any part of a
HubRise payload, ticket texts, FDM signatures, QR or verification data, VAT numbers
of customers, or names of promotions. The producer's tests enforce it on a database
that is full of such data:

- the set of tables is exactly the one in this document;
- no column name contains `insz`, `niss`, `pin`, `hash`, `token`, `secret`,
  `password`, `mail`, `phone`, `address`, `customer`, `signature` or `qr`, and no
  column is named `name` or `user_name` (`product_name` is a product, not a person);
- no text value in any table contains a run of 11 digits, an `@`, a bcrypt prefix
  (`$2a$`, `$2b$`, `$2y$`) or any of the planted secrets, and the bytes of the file
  contain none of them either.

Free-text columns hold product, department and technical identifiers only
(`product_name`, `department`, `pos_id`, `terminal_id`, `pos_sw_version`).

## Versioning

`meta.schema_version` is the version of this document. The rules:

- A breaking change bumps `schema_version` (and the `format` suffix of the manifest
  follows the exporter, not this file): removing or renaming a table or a column,
  changing a type, a unit or the meaning of a value, or narrowing a `CHECK` set.
- Within a version, the producer may add columns, tables and values to a `CHECK`
  set. It never removes or renames anything. A reader must therefore select columns
  by name, ignore columns and tables it does not know, and treat a value it does
  not know as "other".
- A receiver must refuse a file whose `meta.format` is not `pili-stats`, and should
  keep (not interpret) a file whose `schema_version` is higher than it supports.
- This document and the `v1` tests change together with the code: adding a column
  means adding its row here.
