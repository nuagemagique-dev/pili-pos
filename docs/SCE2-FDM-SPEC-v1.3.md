# SCE 2.0 — FDM Technical Specifications v1.3 (20/05/2025)

> Source: SPF Finances — 20250520_DETAILED_DESCRIPTION_FDM_v1.3_ENG.pdf
> Contact: secr.gksce@minfin.fed.be

This document describes the functioning of the Fiscal Data Module (FDM), its communication with the POS, and its communication with the FPS Finance cloud service.

---

## Chapter 1 — Technical Requirements for the FDM

### 1.1 General Requirements

FDM must not contain functionalities other than those described in the Ministerial Decree. Additional functionality must contribute to proper operation and be documented in certification application.

Receipt/transmission of messages may **never be interrupted** for additional functionality.

#### Counters
Counters start again at 1 when they reach max value 999999999.

#### VAT Calculation
FDM calculates taxable amounts and VAT based on product lines received from POS. Rates are received from FODFIN cloud and updated automatically (including future rate changes with validity periods).

**Calculation formula** (Article 66):
```
For each VAT label:
  totalAmount = Sum of all (vat.price + Sum(priceChange.amount)) for that label
  taxableAmount = totalAmount / (1 + rate/100)   [rounded to 0.01]
  vatAmount = totalAmount - taxableAmount
```

Example:
| Qty | Description | Amount | VAT Label | Total | VAT Amount | Tax Base |
|-----|-------------|--------|-----------|-------|------------|----------|
| 2 | Food | 41.50 | B (12%) | 10.00 | 1.74 | 8.26 |
| 2 | Drinks | 10.00 | A (21%) | 54.25 | 5.81 | 48.44 |
| 1 | Food | 12.75 | B (12%) | 13.00 | 0.74 | 12.26 |
| 1 | Take away | 13.00 | C (6%) | 15.00 | 0.00 | 15.00 |
| 5 | Caution | 2.00 | X | 2.00 | 0.00 | 2.00 |

**Note:** If bookingPeriod spans midnight and VAT rate changes, FDM applies rate valid on `bookingDate`.

### 1.2 Hardware Requirements

#### Storage
- Minimum **2 GB** for transaction data buffer (enriched JSON)
- Separate from internal memory (Article 76)

### 1.3 User Interface

Minimum health indicators on device (display or LEDs). Back office access must be protected. Network configuration (POS + FPSFIN cloud) must be possible through UI.

Options: display on device, connected device (laptop/tablet), connected POS screen, or combination.

### 1.4 Firmware

Firmware must:
- Check correctness of received POS messages (JSON + GraphQL schema validation)
- Reject incorrect messages with error codes
- For correct messages: calculate, increment counters, convert, sign
- Store signed messages for transmission to FPSFIN cloud
- Transmit to FPSFIN cloud per protocol
- Receive and act on FPSFIN cloud messages
- Support secure firmware updates (approved by FPS Finance)

---

## Chapter 2 — Communication with Connected POS

### 2.1 Physical Connection

LAN cable or WiFi. TCP/IP. FDM exposes GraphQL at `/graphql` via HTTP(S) POST with `Content-Type: application/json`.

Security options (manufacturer choice): HTTP with token, HTTPS with TLS (self-signed cert), HTTPS with mTLS.

**FDM may NOT connect to third-party apps.** Only POS, FPSFIN, and FDM manufacturer apps.

### 2.2 GraphQL Service

Follows GraphQL standard of October 27, 2021. All JSON follows RFC 8259. No leading/trailing whitespace in strings.

#### Mutations
Same as POS spec — see `SCE2-POS-SPEC-v2.1.md` section 2.3.1.

#### Communication Rules (Deduplication)
Key fields: `posId`, `posDateTime`, `terminalId`, `eventLabel`, `posFiscalTicketNo`

1. Different keys → process normally
2. Identical request → duplicate response (no counter increment)
3. Same keys, different content → refuse with error

### 2.4 Digital Signature

ECDSA with secp256r1 curve. See POS spec section 2.4 for canonical JSON rules.

---

## Chapter 3 — Verification URL / QR Code

The FDM generates a URL for the QR code on VAT receipts.

### Structure
```
https://www.gs2.be/ + [19 base-62 encoded characters]
```

Total URL = 38 characters. Fits in 25×25 QR code Type 2 with medium ECC.

### Encoded Value Contains
1. **Signature bit counter** (2 bits): indicates how many signature bits are included (28/32/36/40)
2. **Digital signature bits** (28-40 bits from first 5 bytes of digitalSignature)
3. **fdmId** (41 bits: 3 letters A-Z + 8 digits, encoded as integer, XOR-masked)
4. **totalCounter** (30 bits, XOR-masked)
5. **Padding** (zero bits to reach exactly 19 base-62 characters)

### Base-62 Character Set
```
0-9 → 0-9
A-Z → 10-35
a-z → 36-61
```

Encoding: 5-bit units for values 0 (00000) and 31 (11111), 6-bit units for all others.

### XOR Mask Generation
From first two bytes of digitalSignature (B1, B2):
```
mask = B2
mask = (mask << 8) + B1
mask = (mask << 8) + ~B1
mask = (mask << 8) + ~B2
mask = (mask << 8) + B1
mask = (mask << 8) + B2
```

Least significant 41 bits → mask for fdmId.
Inverse of 48-bit mask, least significant 30 bits → mask for totalCounter.

### Encoder/Decoder Reference Implementation

See the PDF for complete C# implementations (`Encode` and `Decode` methods). Key points:

- Encoder tries 40 bits first, reduces by 4 bits (36, 32, 28) if output exceeds 19 chars
- Bit stream processes MSB first
- Values 0 and 1 in base-62 use 5 bits, all others use 6 bits

---

## Chapter 4 — Temporary Storage of Data

- Minimum 2 GB buffer for enriched JSON
- Messages **cannot be deleted or modified**
- Can only be copied/viewed via back office
- Buffer must be accessible for reading/copying via backoffice (even without internet)
- Forwarded messages deleted only after successful transmission + 10-day retention

---

## Chapter 5 — Communication FDM ↔ FODFIN Cloud

### 5.1 Connection and Security

Via internet, configured through FDM back office. Minimum config options:
- FODFIN cloud service URLs
- Time server URL

Security: authentication certificate + mTLS protocol.

### 5.2 First Time Setup

1. POS and FDM registered in GKS e-service by stakeholders
2. Enterprise/establishment level: FDM linked to POS
3. On first startup, FDM contacts FODFIN cloud:
   - Receives unique personalized authentication certificate
   - Receives initialization package (VAT rates, frequency settings, etc.)

### 5.3 API Protocol

Described in separate document: `FPSFIN_API_PROTOCOL_FDM_FINCLOUD_1.09_20251017_FINAL.pdf`

---

## Chapter 6 — Online Forwarding of Transaction Data

### 6.1 JSON Message to FPS Finance

The enriched JSON (POS data + FDM calculations/signature) is sent to `ws_reg` endpoint:

```json
{
  "unsentEventCount": 0,        // 0 for NOP
  "fdmId": "FOD01987654",
  "events": [
    {
      "enrichedEventData": { },   // POS data + FDM enrichments
      "digitalSignature": "...",  // Base64 ECDSA signature
      "shortSignature": "...",    // SHA-1 hex (event N only)
      "fdmLocalisation": {
        "geoLocation": {          // optional, if FDM has GPS
          "latitude": 50.851415,
          "longitude": 4.354365
        }
      }
    }
  ]
}
```

In case of corrupt transaction:
```json
{
  "rawCounter": 1,
  "rawBytes": "...",    // base64 of corrupt data (if partly readable)
  "rawInfo": "..."      // error message from FDM
}
```

Forwarded messages retained by FPSFIN for at least 3 years. FDM retains for 10 days after successful transmission.
