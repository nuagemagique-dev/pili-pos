# SCE 2.0 — POS Technical Specifications v2.1 (27/08/2025)

> Source: SPF Finances — DETAILED_DESCRIPTION_POS_v2.1_ENG_20250827.pdf
> Contact: secr.gksce@minfin.fed.be

This document is the **complete technical extraction** from the official SPF Finances PDF.
It describes the operation and communication between the Cash Register System (POS) and the Fiscal Data Module (FDM) for SCE 2.0 certification in Belgium.

---

## Table of Contents

- [Chapter 1 — Technical Requirements for the POS](#chapter-1--technical-requirements-for-the-pos)
- [Chapter 2 — Interaction POS ↔ FDM](#chapter-2--interaction-pos--fdm)
  - [2.1 General — Mutations List](#21-general)
  - [2.2 Communication Content](#22-content-of-the-communication)
  - [2.3 GraphQL Schema](#23-graphql-schema)
  - [2.4 Digital Signature](#24-digital-signature)

---

## Chapter 1 — Technical Requirements for the POS

### 1. Registration of Events

#### 1.1 Direct Sale (Event N)
Event N is immediately registered. The booking (and GraphQL message) always contains product lines in **chronological order**. Price changes are attached to product lines. Corrections in quantities are always separate lines in chronological order.

The final printout may be consolidated and reordered (e.g., grouped by department). The printout always includes **"VAT RECEIPT"**.

#### 1.2 Sales Registration with Complete Refund (Event N)
If a previously created event N is COMPLETELY refunded, the `fdmRefs` array references the original VAT receipt. A maximum of **one** previously created event N may be refunded per event N.

If the receipt contains only a complete refund (only negative product lines), the printout includes **"REFUND"**.

#### 1.3 Interrupted Sale (Event P)
Registration is interrupted in time (table management, customer management, web orders, kiosk orders, on-hold). Uses event P. The complete sale must always result in a final event N.

A `reference` groups all bookings of a sales transaction. Printouts include **"PRO FORMA"** and **"THIS IS NOT A VALID VAT RECEIPT"**.

#### 1.4 Account Overview / Provisional Bill (Event P)
Overview of booked orders and/or amount to be paid. Product lines may be consolidated. No new product lines or changes allowed.

Printout includes **"PRO FORMA"**, **"PROVISIONAL BILL"**, and **"THIS IS NOT A VALID VAT RECEIPT"**.

#### 1.5 Training Mode (Event T)
All registrations in training mode carry label "T". Printout includes **"TRAINING"** at top and **"THIS IS NOT A VALID VAT RECEIPT"** at bottom.

#### 1.6 Financial Transactions (Event F)
Pure financial movements unrelated to sales.

#### 1.7 Invoices (Event I)
If POS can create invoices based on previously issued VAT receipts, registered as "INVOICE" event. Product lines of original receipts included in printout **but NOT in GraphQL message**. Payment lines of originals are NOT included.

Printout includes **"INVOICE"** and **"THIS IS NOT A VALID VAT RECEIPT"**.

**Important:** From 01.01.2026, all B2B/B2G invoices must be sent via Peppol.

#### 1.8 Copies (Event C)
Copy of previously created event. Reference to original always provided (format: `fdmId` + `eventLabel` + `Counter/totalCounter`).

Copy of VAT receipt: print is **identical** to original + additional fields with `fdmRef` and `posDateTime` of copy creation.

All other copies include **"COPY"** and **"THIS IS NOT A VALID VAT RECEIPT"**.

#### 1.9 Employee Presence (Event S)
Registration of presence (start/end) of users via "SOCIAL" event. **Not printed, not a sales transaction.** Every POS must include this functionality.

### 2. Numbering of Events

Continuous numbering per event type, per terminal, or general across all — as long as `posFiscalTicketNo` + event type (or terminal) combination is unique. When max 999999999 is reached, restart at 1.

### 3. Conditions for Event Registration

1. **User must be logged in** — identified by INSZ (11 digits: national registry or BIS number)
   - External technician: `00000000097`
   - Robot user (online/kiosk/auto reports): `00000000029`
2. **POS must block** if FDM is not connected and fully operational
3. A started transaction cannot be completed without FDM signature

### 4. Data Retention

- All data (database, JSON, GraphQL messages sent and received) retained in **original form**
- Applicable fiscal retention period
- At least one port must be accessible for external data carrier (for copying)
- Manufacturer must describe database schema and security mechanisms in certification application

### 5. VAT Codes

| Code | Rate | Description |
|------|------|-------------|
| A | 21% | High (beverages) |
| B | 12% | Medium (restaurant services) |
| C | 6% | Low (takeaway food) |
| D | 0% | Zero rate |
| X | N/A | Outside VAT scope (deposits, empty goods, MPV) |

The FDM calculates taxable amounts and VAT — rates are kept up-to-date via FODFIN cloud. **Items may have dual VAT codes** (e.g., all-in menu with food B + drinks A).

### 6. The VAT Receipt

#### 6.1 POS Identification
`posId`: 14 characters, uppercase. Example: `CFOD0061234567`. Reserved for certification: `CFOD0010000001`.

#### 6.2 Terminal Identification
`terminalId`: unique per cash register under same posId (e.g., "bar", "terrace", or number).

#### 6.3 Input Device Identification
`deviceId`: hardware serial number, MAC address, or IP address. For webshop: URL.

#### 6.4 User Identification
INSZ is **NEVER printed** on VAT receipt. User must be linked internally.

#### 6.5 QR Code
Generated by POS from URL provided by FDM. **38 alphanumeric characters**, fitting 25×25 module Type 2 QR with medium ECC. Printed on paper, or as link on digital receipt.

#### 6.6 Control Data on VAT Receipt
- `fdmId`
- `fdmDateTime`
- `eventLabel`
- `eventCounter`
- `totalCounter`
- `shortSignature`
- `footer` (array of strings from FPSFIN)

### 7. Rounding of Payments

Cash payments: round to nearest 5 cents (mandatory). Payments ending in 1,2,6,7 → round down; 3,4,8,9 → round up.

Non-cash rounding: optional, applies to ALL non-cash if merchant opts in.

**Max rounding per receipt: 2 eurocents (positive or negative).**

Meal/eco/gift vouchers always processed first (fixed value, never rounded).

### 8. Consolidation and Printing Rules

- **EVENT ≠ PRINT ≠ MESSAGE to FDM** — always maintain this distinction
- Every event → message to FDM (always)
- N and INVOICE → always printed (paper/digital/both)
- R (Report) → always creates file, must be easily printable
- All other events → may be printed, must include "THIS IS NOT A VALID VAT RECEIPT"
- Kitchen/bar tickets **NEVER display amounts**
- **Consolidation is NEVER applied to GraphQL messages** — only to printouts

---

## Chapter 2 — Interaction POS ↔ FDM

### Definitions

| Abbrev | Meaning |
|--------|---------|
| FDM | Fiscal Data Module |
| POS | Point Of Sale |
| RCRS | Registered Cash Register System |
| GraphQL | Graph Query Language |
| JSON | JavaScript Object Notation |
| UTC | Coordinated Universal Time |

### Physical Connection

POS ↔ FDM connected via LAN cable or WiFi. TCP/IP protocol. FDM exposes GraphQL service via HTTP(S) at path `/graphql` with `POST` and `Content-Type: application/json`.

**If internet connection fails and WiFi is used over internet, POS-FDM connection is lost → POS stops registering.**

FDM may NOT be connected to third-party applications. Only POS, FPSFIN, and FDM manufacturer apps.

### 2.1 General

#### Mutations List

| Event | Mutations |
|-------|-----------|
| S (Social) | `signWorkIn`, `signWorkOut` |
| I (Invoice) | `signInvoice` |
| N (Normal) | `signSale` |
| P (Pro Forma) | `signCostCenterChange`, `signOrder`, `signPreBill` |
| F (Financial) | `signMoneyInOut`, `signDrawerOpen`, `signPaymentCorrection` |
| R (Report) | `signReportTurnoverX`, `signReportTurnoverZ`, `signReportUserX`, `signReportUserZ` |
| C (Copy) | `signCopy` |

All mutations accept `isTraining: Boolean! = false` parameter.

Status query (no event): `Status(language! = EN): FdmStatus!`

### 2.2 Content of the Communication

#### POS → FDM Complete Overview

```json
{
  "language": "enum",
  "ticketMedium": "enum",
  "posId": "string (14 chars)",
  "posFiscalTicketNo": "numeric (1-999999999)",
  "posSwVersion": "string (1-36 chars)",
  "terminalId": "string (1-600 chars)",
  "deviceId": "string (1-600 chars)",
  "posDateTime": "RFC3339 datetime",
  "bookingPeriodId": "GUID",
  "bookingDate": "RFC3339 date",
  "vatNo": "BE + 10 digits (modulo 97)",
  "estNo": "string (10 chars, starts 2-8)",
  "employeeId": "string (11 chars INSZ, modulo 97)",
  "customerVatNo": "string (for Invoice)",
  "invoiceNo": "string (for Invoice)",
  "fdmRefs": "[FdmReferenceInput]",
  "costCenter": "CostCenterInput",
  "transfer": "TransferInput",
  "transaction": "TransactionInput",
  "drawer": "DrawerInput",
  "financials": "[PaymentLineInput | MoneyInOutLineInput]",
  "reportNo": "numeric (Z report only)",
  "reportBookingDate": "RFC3339 date (Z report only)",
  "posDevices": "[PosDeviceInput]",
  "fdmDevices": "[FdmDeviceInput]",
  "turnover": "TurnoverInput",
  "users": "[UserItemInput]"
}
```

#### Key Field Details

**posDateTime**: Local time Brussels, ISO 8601. Example: `2022-10-20T15:01:25+02:00`

**bookingPeriodId**: GUID. Operational period (typically opening day or shift). Example: `dffcd829-a0e5-41ca-a0ae-9eb887f95637`

**bookingDate**: Date the transaction is booked to. A booking period 00:30-04:30 can be included in previous day's turnover.

**vatNo**: Format `BE0499999960` (BE + 10 digits, first digit 0 or 1, modulo 97 check).

**estNo**: 10 digits, starts with 2-8, modulo 97 check.

**employeeId**: INSZ 11 digits, modulo 97 check.

#### CostCenter Object

```
costCenter {
  id: String!          // e.g. "T17", "K08"
  type: CostCenterType!  // TABLE, CHAIR, ROOM, CUSTOMER, ON_HOLD, KIOSK, PLATFORM, WEBSHOP, OTHER
  reference: String!   // session GUID grouping connected transactions
  costCenter: CostCenterInput  // optional nested, max 2 levels deep
}
```

CHAIR may only be used if TABLE is at same or higher level. PLATFORM for linked third-party orders. WEBSHOP for own webshop orders.

#### Transaction Lines

Each `transactionLine` contains:
- `lineType`: `SINGLE_PRODUCT` or `COMPOSITE_PRODUCT`
- `mainProduct`: ProductInput (mandatory)
- `subProducts`: [ProductInput] (mandatory for COMPOSITE_PRODUCT, null for SINGLE)
- `costCenter`: optional (chair-level assignment)
- `lineTotal`: total price after price changes

**Product fields:**
- `productId`, `productName`, `departmentId`, `departmentName` (all mandatory, 1-600 chars)
- `quantity`: multiple of 0.0001, can be negative
- `negQuantityReason`: REFUND, CORRECTION, PRICE_CHANGE, COST_CENTER_CHANGE, PRODUCT_SUBSTITUTION, VOUCHER, OTHER (mandatory if quantity < 0)
- `quantityType`: PIECE, KILOGRAM, METER, LITRE, HOUR
- `unitPrice`: **4 decimals**
- `vats`: array of VAT objects (label + price + priceChanges)

**Price changes:**
- `groupingId`: groups price change across products
- `scope`: LINE or EVENT
- `type`: PUBLIC (visible) or INTERNAL
- `amount`: multiple of 0.01

#### Payment Lines

Types: UNKNOWN, CASH, CARD_DEBIT, CARD_UNKNOWN, CARD_CREDIT, CARD_OTHER, CHEQUE_MEAL, CHEQUE_OTHER, APP, ONLINE, CUSTOMER_CREDIT, ROOM_CREDIT, LOYALTY_REWARDS, VOUCHER_STORE, VOUCHER_SUPPLIER, VOUCHER_OTHER, OTHER

Amount types: PAYMENT, TIP, ROUNDING

For MoneyInOut: MONEY_IN_OUT, TIP, DRAWER_DECLARATION

#### Reports

**Z report only**: `reportNo` (sequential 1-999999999), `reportBookingDate`

Turnover object contains: transactions totals per eventLabel, departments totals, VAT totals, payments totals (with corrections breakdown), drawersOpenCount, negQuantities breakdown, priceChanges totals, invoices list.

Users array: per employee — totalAmount, firstPosDateTime, lastPosDateTime, socialEvents (IN/OUT), payments.

#### Communication Rules (Deduplication)

Key fields: `posId`, `posDateTime`, `terminalId`, `eventLabel`, `posFiscalTicketNo`

1. Different key combination → process normally
2. Identical key + identical content → duplicate, return same response, don't increment counters
3. Identical key + different content → **refuse**, return error

#### FDM → POS Response

```
SignResult {
  posId, posFiscalTicketNo, posDateTime, terminalId, deviceId
  eventOperation: EventOperation!
  fdmRef: FdmReference!        // fdmId, fdmDateTime, eventLabel, eventCounter, totalCounter
  fdmSwVersion: String!
  digitalSignature: String!    // Base64 ECDSA signature
  shortSignature: String       // only for event N: Hex(SHA1(Base64Decode(digitalSignature)))
  verificationUrl: String      // only for event N: 38-char URL for QR code
  vatCalc: [VatCalcItem!]      // only for event N: taxableAmount, vatAmount per label
  bufferCapacityUsed: Float!   // percentage
  warnings: [MessageItem!]
  informations: [MessageItem!]
  footer: [String]!            // lines to print in receipt footer
}
```

**Message display rules:**
- Warnings: show at least once per bookingDate, on first mutation response
- Errors: show every time on each mutation response
- `showPos`: MANDATORY (must show), OPTIONAL (POS decides), NEVER (don't show)

### 2.3 GraphQL Schema

#### Mutations

```graphql
signWorkIn(data: WorkInOutInput!, isTraining: Boolean! = false): SignResult
signWorkOut(data: WorkInOutInput!, isTraining: Boolean! = false): SignResult
signInvoice(data: InvoiceInput!, isTraining: Boolean! = false): SignResult
signSale(data: SaleInput!, isTraining: Boolean! = false): SignResult
signCostCenterChange(data: CostCenterChangeInput!, isTraining: Boolean! = false): SignResult
signOrder(data: OrderInput!, isTraining: Boolean! = false): SignResult
signPreBill(data: PreBillInput!, isTraining: Boolean! = false): SignResult
signMoneyInOut(data: MoneyInOutInput!, isTraining: Boolean! = false): SignResult
signDrawerOpen(data: DrawerOpenInput!, isTraining: Boolean! = false): SignResult
signPaymentCorrection(data: PaymentCorrectionInput!, isTraining: Boolean! = false): SignResult
signReportTurnoverX(data: ReportTurnoverXInput!, isTraining: Boolean! = false): SignResult
signReportTurnoverZ(data: ReportTurnoverZInput!, isTraining: Boolean! = false): SignResult
signReportUserX(data: ReportUserXInput!, isTraining: Boolean! = false): SignResult
signReportUserZ(data: ReportUserZInput!, isTraining: Boolean! = false): SignResult
signCopy(data: CopyInput!, isTraining: Boolean! = false): SignResult
```

#### Types

```graphql
type SignResult {
  posId: String!
  posFiscalTicketNo: Int!
  posDateTime: String!
  terminalId: String
  deviceId: String!
  eventOperation: EventOperation!
  fdmRef: FdmReference!
  fdmSwVersion: String!
  digitalSignature: String!
  shortSignature: String
  verificationUrl: String
  vatCalc: [VatCalcItem!]
  bufferCapacityUsed: Float!
  warnings: [MessageItem!]
  informations: [MessageItem!]
  footer: [String]!
}

type FdmReference {
  fdmId: String!
  fdmDateTime: String!
  eventLabel: EventLabel!
  eventCounter: Int!
  totalCounter: Int!
}

type VatCalcItem {
  label: VatLabel!
  rate: Float!
  taxableAmount: Float!
  vatAmount: Float!
  totalAmount: Float!
  outOfScope: Boolean!
}

type MessageItem {
  message: String!
  locations: [LocationItem!]
  extensions: ExtensionItem!
}

type LocationItem {
  line: Int!
  column: Int!
}

type ExtensionItem {
  category: Category!
  code: Code!
  data: [DataItem!]
  showPos: Display!
}

type DataItem {
  name: String!
  value: String!
}
```

#### Enums

```graphql
enum Language { EN, NL, FR, DE }
enum TicketMedium { NONE, PAPER, DIGITAL, PAPER_DIGITAL }
enum InOut { IN, OUT }
enum PriceChangeType { PUBLIC, INTERNAL }
enum PriceChangeScope { LINE, EVENT }
enum VatLabel { A, B, C, D, X }
enum EventLabel { N, P, F, S, I, R, C, T }
enum QuantityType { PIECE, KILOGRAM, METER, LITRE, HOUR }
enum TransactionLineType { SINGLE_PRODUCT, COMPOSITE_PRODUCT }
enum NegQuantityReason { REFUND, CORRECTION, PRICE_CHANGE, COST_CENTER_CHANGE, PRODUCT_SUBSTITUTION, VOUCHER, OTHER }
enum PaymentType { UNKNOWN, CASH, CARD_UNKNOWN, CARD_DEBIT, CARD_CREDIT, CARD_OTHER, CHEQUE_MEAL, CHEQUE_OTHER, APP, ONLINE, CUSTOMER_CREDIT, ROOM_CREDIT, LOYALTY_REWARDS, VOUCHER_STORE, VOUCHER_SUPPLIER, VOUCHER_OTHER, OTHER }
enum InputMethod { MANUAL, AUTOMATIC }
enum PaymentLineType { PAYMENT, TIP, ROUNDING }
enum MoneyInOutLineType { MONEY_IN_OUT, TIP, DRAWER_DECLARATION }
enum CostCenterType { TABLE, CHAIR, ROOM, CUSTOMER, ON_HOLD, KIOSK, PLATFORM, WEBSHOP, OTHER }
enum EventOperation { WORK_IN, WORK_OUT, SALE, INVOICE, COST_CENTER_CHANGE, ORDER, PRE_BILL, MONEY_IN_OUT, DRAWER_OPEN, PAYMENT_CORRECTION, COPY, REPORT_TURNOVER_X, REPORT_TURNOVER_Z, REPORT_USER_X, REPORT_USER_Z }
enum Category { SPF_FOD, FDM, OTHER }
enum Display { MANDATORY, OPTIONAL, NEVER }
enum Code { CLIENT_CERT_NEAR_EXPIRATION, CLIENT_CERT_EXPIRED, RTC_SYNC_FAILED, UPDATE_URLS_FAILED, UPDATE_TRUST_CERT_FAILED, UPDATE_TASK_LIST_FAILED, TASK_FEEDBACK_FAILED, NOP_FAILED, BUFFER_NEAR_FULL, SERVER_CERT_RESOLVE_FAILED, TRANSACTION_UPLOAD_FAILED, INITIALIZATION_FAILED, RTC_NOT_INITIALIZED, CORRUPT_RECORD_ENCOUNTERED, UPDATE_PARAMS_FAILED, UPDATE_CLIENT_CERT_FAILED, UPDATE_VAT_RATES_FAILED, UPDATE_POS_ALLOWLIST_FAILED, DUPLICATE_REQUEST, BUFFER_FULL, FDM_LOCKED, UNAUTHORIZED, INVALID_REQUEST, INTERNAL_ERROR, UNDEFINED_ERROR, UNDEFINED_OTHER, FDM_NOT_OPERATIONAL, UNKNOWN_POS }
```

#### Input Objects

```graphql
input FdmReferenceInput {
  fdmId: String!
  fdmDateTime: String!
  eventLabel: EventLabel!
  eventCounter: Int!
  totalCounter: Int!
}

input DrawerInput { id: String!, name: String! }

input CostCenterInput {
  id: String!
  type: CostCenterType!
  reference: String!
  costCenter: CostCenterInput  # recursive, max 2 levels
}

input PriceChangeInput {
  groupingId: Int
  id: String!
  name: String!
  scope: PriceChangeScope!
  type: PriceChangeType!
  amount: Float!
}

input VatInput {
  label: VatLabel!
  price: Float!
  priceChanges: [PriceChangeInput!]
}

input ProductInput {
  gtin: String
  productId: String!
  productName: String!
  departmentId: String!
  departmentName: String!
  quantity: Float!
  quantityType: QuantityType!
  negQuantityReason: NegQuantityReason
  unitPrice: Float!
  vats: [VatInput!]!
}

input ForeignCurrencyInput { amount: Float!, iso: String! }

input TransactionLineInput {
  lineType: TransactionLineType!
  mainProduct: ProductInput!
  subProducts: [ProductInput!]
  costCenter: CostCenterInput
  lineTotal: Float!
}

input TransactionInput {
  transactionLines: [TransactionLineInput!]!
  transactionTotal: Float!
}

input PaymentLineInput {
  id: String!
  name: String!
  type: PaymentType!
  provider: String
  inputMethod: InputMethod!
  amount: Float!
  amountType: PaymentLineType!
  foreignCurrency: ForeignCurrencyInput
  reference: String
  drawer: DrawerInput
}

input TransferInput {
  from: [TransferItemInput!]!
  to: [TransferItemInput!]!
}

input TransferItemInput {
  costCenter: CostCenterInput!
  transaction: TransactionInput!
}

input MoneyInOutLineInput {
  id: String!
  name: String!
  type: PaymentType!
  provider: String
  inputMethod: InputMethod!
  amount: Float!
  amountType: MoneyInOutLineType!
  foreignCurrency: ForeignCurrencyInput
  reference: String
  drawer: DrawerInput
}

# === Event-specific Input Objects ===
# All share common fields: language, vatNo, estNo, posId, posFiscalTicketNo,
# posDateTime, posSwVersion, terminalId, deviceId, bookingPeriodId,
# bookingDate, ticketMedium, employeeId

input SaleInput {
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
  fdmRef: FdmReferenceInput      # for refund of previous N
  costCenter: CostCenterInput
  transaction: TransactionInput!
  financials: [PaymentLineInput!]!
}

input OrderInput {
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
  costCenter: CostCenterInput!    # mandatory for Order
  transaction: TransactionInput!
}

input PreBillInput {
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
  costCenter: CostCenterInput
  transaction: TransactionInput!
}

input CostCenterChangeInput {
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
  transfer: TransferInput!
}

input WorkInOutInput {
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
}

input InvoiceInput {
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
  invoiceNo: String!, customerVatNo: String!
  costCenter: CostCenterInput
  fdmRefs: [FdmReferenceInput!]!
}

input MoneyInOutInput {
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
  financials: [MoneyInOutLineInput!]!
}

input DrawerOpenInput {
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
  drawer: DrawerInput
}

input PaymentCorrectionInput {
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
  financials: [PaymentLineInput!]!
  fdmRef: FdmReferenceInput!
}

input CopyInput {
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
  fdmRef: FdmReferenceInput!
}

# Report inputs (X = no reportNo/reportBookingDate, Z = with)
input ReportTurnoverXInput {
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
  posDevices: [PosDeviceInput!]!, fdmDevices: [FdmDeviceInput!]!
  turnover: TurnoverInput!
}

input ReportTurnoverZInput {
  # same as X + reportNo: Int!, reportBookingDate: String!
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
  reportNo: Int!, reportBookingDate: String!
  posDevices: [PosDeviceInput!]!, fdmDevices: [FdmDeviceInput!]!
  turnover: TurnoverInput!
}

input ReportUserXInput {
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
  posDevices: [PosDeviceInput!]!, fdmDevices: [FdmDeviceInput!]!
  users: [UserItemInput!]!
}

input ReportUserZInput {
  # same as X + reportNo, reportBookingDate
  language: Language!, vatNo: String!, estNo: String!
  posId: String!, posFiscalTicketNo: Int!, posDateTime: String!
  posSwVersion: String!, terminalId: String, deviceId: String!
  bookingPeriodId: String!, bookingDate: String!
  ticketMedium: TicketMedium!, employeeId: String!
  reportNo: Int!, reportBookingDate: String!
  posDevices: [PosDeviceInput!]!, fdmDevices: [FdmDeviceInput!]!
  users: [UserItemInput!]!
}

# Report sub-inputs
input PosDeviceInput { posId: String!, terminalId: String, firstPosDateTime: String!, lastPosDateTime: String! }
input FdmDeviceInput { fdmId: String!, firstFdmDateTime: String!, firstTotalCounter: Int!, lastFdmDateTime: String!, lastTotalCounter: Int! }
input TurnoverInput { transactions: [EventTotalInput!]!, departments: [DepartmentTotalInput!]!, vats: [VatTotalInput!]!, payments: [PaymentTotalInput!]!, drawersOpenCount: Int! = 0, negQuantities: [NegQuantityTotalInput!]!, priceChanges: [PriceChangeTotalInput!]!, invoices: [InvoiceTotalInput!] }
input EventTotalInput { eventLabel: EventLabel!, ticketCount: Int!, amount: Float! }
input DepartmentTotalInput { departmentId: String!, departmentName: String!, amount: Float! }
input VatTotalInput { label: VatLabel!, rate: Float!, taxableAmount: Float!, vatAmount: Float!, totalAmount: Float! }
input PaymentTotalInput { id: String!, name: String!, type: PaymentType!, amount: [PaymentTotalAmountInput!]! }
input PaymentTotalAmountInput { type: PaymentLineType!, normalAmount: Float!, negativeCorrections: Float!, positiveCorrections: Float!, correctionsCount: Int!, totalAmount: Float!, normalForeign: [ForeignCurrencyInput!], correctionsForeign: [ForeignCurrencyInput!] }
input NegQuantityTotalInput { negQuantityReason: NegQuantityReason!, negQuantityCount: Int!, ticketCount: Int!, amount: Float! }
input PriceChangeTotalInput { id: String!, name: String!, type: PriceChangeType!, amount: [PriceChangeVatTotalInput!]! }
input PriceChangeVatTotalInput { label: VatLabel!, negative: Float!, positive: Float! }
input InvoiceTotalInput { invoiceNo: String!, amount: Float! }
input UserItemInput { employeeId: String!, totalAmount: Float!, firstPosDateTime: String!, lastPosDateTime: String!, socialEvents: [InOutItemInput!]!, payments: [PaymentTotalInput!]! }
input InOutItemInput { posDateTime: String!, inOut: InOut! }
```

#### Status Query

```graphql
type Device {
  fdmId: String!
  fdmSwVersion: String!
  fdmDateTime: String!
  bufferCapacityUsed: Float!
}

type FdmStatus {
  device: Device!
  initialized: Boolean!
  informations: [MessageItem!]
  warnings: [MessageItem!]
  errors: [MessageItem!]
}

type Query {
  Status(language: Language! = EN): FdmStatus!
}
```

### 2.4 Digital Signature

The FDM signs the "enriched JSON" using ECDSA with secp256r1 curve.

#### Canonical JSON Rules
Before signing, JSON is canonicalized:
- Numbers: only chars 0-9, minus (-), decimal point. No scientific notation. No trailing zeros.
- All whitespace removed.
- Characters U+001F < c < U+007F are never escaped, except U+0022 (double quote) and U+005C (backslash).
- Other characters escaped as `\uXXXX` in lowercase.
- Object keys sorted lexicographically by UTF-16 code units.
- Null values are included.

#### shortSignature
`shortSignature = Hex(SHA1(Base64Decode(digitalSignature)))` — only for event N.

#### verificationUrl
38 characters total, generated by FDM. Contains encoded fdmId + totalCounter + partial digitalSignature bits. See FDM spec Chapter 3 for encoding algorithm.

---

## FDM Error/Warning Codes Reference

| Code | Message | Category | Nature | Display |
|------|---------|----------|--------|---------|
| CLIENT_CERT_NEAR_EXPIRATION | Client certificate expires in <4 months | SPF_FOD | Warning | Optional |
| CLIENT_CERT_EXPIRED | Client certificate has expired | SPF_FOD | Warning | Mandatory |
| BUFFER_NEAR_FULL | FDM buffer usage exceeds 70% | SPF_FOD | Warning | Mandatory |
| BUFFER_FULL | FDM buffer is full | SPF_FOD | Error | Mandatory |
| FDM_LOCKED | FDM is locked | SPF_FOD | Error | Mandatory |
| FDM_NOT_OPERATIONAL | FDM is not functioning | FDM | Error | Mandatory |
| UNKNOWN_POS | POS not in allowlist | FDM | Error | Mandatory |
| UNAUTHORIZED | Unauthorized request | FDM | Error | Optional |
| INVALID_REQUEST | Could not treat request | FDM | Error | Optional |
| INTERNAL_ERROR | Technical issue | FDM | Error | Optional |
| DUPLICATE_REQUEST | Duplicate of earlier request | FDM | Warning | Never |
