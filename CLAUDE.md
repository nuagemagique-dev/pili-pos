# CLAUDE.md - POS SCE 2.0 MVP (Tauri + Rust)

## Project Identity

**Name:** moyun-pos  
**Purpose:** Point-of-Sale terminal application for Belgian HoReCa restaurants, certified SCE 2.0 compliant. First deployment target: Tokyo Sushi Bar (Liège - French-language region, tickets in FR).  
**License:** Proprietary (nuagemagique.dev)  
**Author:** Bruno - backend developer, Go/TypeScript/Rust stack, Brussels

---

## Architecture Overview

This is a **Tauri v2 desktop application** running on Linux (Debian minimal + Cage Wayland compositor in kiosk mode) on a POS touchscreen terminal (Penetek A3216 M82 or Advantech USC-360J, Intel J6412, x86_64).

```
┌─────────────────────────────────────────────────┐
│                 TAURI v2 APP                     │
│  ┌───────────────────────────────────────────┐  │
│  │         Frontend (TypeScript/HTML)         │  │
│  │  - Touch-optimized POS UI                 │  │
│  │  - Menu grid, order builder, checkout     │  │
│  │  - Receipt preview                        │  │
│  └──────────────┬────────────────────────────┘  │
│                 │ Tauri IPC (invoke)             │
│  ┌──────────────▼────────────────────────────┐  │
│  │          Rust Backend (4 crates)           │  │
│  │  pos-core    → business logic, state      │  │
│  │  pos-fiscal  → SCE 2.0 protocol, FDM comm │  │
│  │  pos-print   → ESC/POS thermal printing   │  │
│  │  pos-db      → SQLite persistence         │  │
│  └──────┬──────────┬──────────┬──────────────┘  │
│         │          │          │                   │
│    ┌────▼───┐ ┌────▼───┐ ┌───▼────┐            │
│    │ SQLite │ │  FDM   │ │Printer │            │
│    │ local  │ │ (LAN)  │ │(USB/   │            │
│    │  DB    │ │GraphQL │ │ LAN)   │            │
│    └────────┘ └────────┘ └────────┘            │
└─────────────────────────────────────────────────┘
```

---

## Technology Stack

### Backend (Rust)
- **Tauri v2** - app framework, IPC, window management
- **SQLite** via `rusqlite` - local database (NOT PostgreSQL - too heavy for embedded)
- **GraphQL client** - `graphql_client` + `reqwest` for FDM communication
- **ESC/POS** - raw binary thermal printer commands via serial/USB/TCP
- **QR code** - `qrcode` crate for fiscal ticket QR codes
- **JSON serialization** - `serde` + `serde_json` for fiscal event archival
- **FDM response check** - `pos-fiscal::response_check` refuses any sign response that does not echo its request (identity, operation, label, N shortSignature = SHA-1 of digitalSignature). The ECDSA signature is stored, not verified (the POS has no FDM public key).
- **Chrono** - `chrono` for timestamp handling

### Frontend (Vue 3 + TypeScript)
- **Vue 3** - Composition API, `<script setup lang="ts">` exclusively
- **Vue Router** - hash mode (no server needed, single-page kiosk)
- **Pinia** - state management (orders, current user, FDM status)
- **Tauri IPC** via `@tauri-apps/api` - all business logic stays in Rust, Vue only handles UI
- **CSS** - scoped styles in SFCs + CSS custom properties in `tokens.css` for theming. No Tailwind, no UI framework.
- **Vite** - bundler (Tauri v2 default with Vue template)
- Touch-first: large buttons (min 48x48px), no hover states, high contrast
- **CSS Grid/Flexbox** - responsive touch layout for 15.6" 1920x1080

### Target OS
- Debian 12 minimal (no desktop environment)
- Cage compositor (Wayland kiosk mode) - auto-starts Tauri app fullscreen
- x86_64 architecture (Intel J6412)

---

## SCE 2.0 Fiscal Compliance - CRITICAL REQUIREMENTS

Reference documents (in the `docs/` directory) - these are the **official SPF Finances specifications and certification test scenarios**. Read the relevant ones before implementing `pos-fiscal`.

Each `.md` file is a 1:1 text extraction (via `pdftotext -layout`) of the corresponding PDF. **Open the `.md` for fast text search and grep; open the matching PDF under `docs/original_pdfs/` for diagrams, screenshots, and exact visual layout.** Every `.md` links to its source PDF at the top.

**Core specifications:**
- `docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md` - **POS detailed description v2.2** (FR). Complete POS technical spec: GraphQL schema, mutations, types, enums, input objects, event types, VAT codes, ticket requirements, communication rules, digital signature.
- `docs/20260417_DESCRIPTION DETAILLEE FDM_am_29_04_2024_FR_1.4.md` - **FDM detailed description v1.4** (FR). FDM spec: hardware requirements, QR code encoding algorithm, enriched JSON structure, behaviour on POS↔FDM link.
- `docs/20260209_ADDENDUM_1_DESCRIPTIONS DETAILLEES POS_FDM FR_2.1 ES.md` - **Addendum 1** to the POS/FDM detailed descriptions (ES = events/signatures). Always cross-check this when reading the two specs above.
- `docs/FPSFIN_API_PROTOCOL_FDM_FINCLOUD_1.09_20251017_FINAL.md` - **FPS Finance ↔ FDM cloud API protocol v1.09**. Describes the mTLS communication between the FDM and the SPF Finances cloud. The POS does NOT speak this protocol directly, but understand the lifecycle (provisioning, daily sync, status reporting) before designing FDM error handling.

**Certification manual & application form** (in `docs/20250307_POS_CERTIFICATION_MANUEAL_FRV1.1/`):
- `20250307_FR - Manuel de certification_POS_V11.md` - main manual (110 p).
- `20250115_Manual_Intro_ToRead_FR.md` - read this first (intro / how to use the manual).
- `20250115_FORMULAIRE DEMANDE_CERTIF_POS-FDM.md` - certification request form.
- `20250115_AnnexePourVosExemples.docx` - annex template for submitting examples (Word doc, no `.md` extraction).
- Embedded copies of earlier UC sets: `20241223_UC_BasicSet_POS_FDM_1.4 (1)/`, `20041223_UC_AdvancedSet_POS_FDM_1.2/` - superseded by the newer UC folders below; kept for reference.

**Test scenarios (use cases the implementation must pass to obtain certification):**
- `docs/20250916_UC_BasicSet_POS_FDM_1.7/` - **Basic Set v1.7** - minimum mandatory flows.
  - Sales: `M110_signSale`, `M111_signSale_Refund`, `M112A/B_signSale_RefundPartial`
  - Orders / pro forma: `M121_signOrder`, `M122_signCostCenterChange`, `M123_signPrebill`
  - Cash drawer: `M130_signMoneyInOut`, `M131_signDrawerOpen`, `M132_signPaymentCorrection`, `M133_signMoneyInOut_DrawerDeclaration`
  - Staff (social): `M140_signWorkIn`, `M141_signWorkOut`
  - Invoicing & copies: `M150_signInvoice`, `M160_signCopy_Sale`, `M161_signCopy_Order`
  - Training mode: `M170_signWorkIn_Training`, `M171/171B_signSale_Training`, `M172_signCopy_Training`
  - FDM queries: `Q100_Status`, `Q110_Status_multi_lang_messages`
  - Test setup: `UC_TestSetup_20240806.md`, changelog: `CHANGELOG-UC_BasicSet1.7.md`
- `docs/20250916_UC_Reports_1.3/` - **Reports v1.3** - X/Z report scenarios.
  - `M180_signReportTurnoverX`, `M181_signReportTurnoverZ`, `M182_signReportUserX`, `M183_signReportUserZ`, `M184_signReportTurnoverZUserZ`
  - `InputReport/` - input fixtures, `Scenario Reports.md` - full scenario doc (⚠️ source PDF is diagram-only - open the PDF for the flow charts), changelog: `CHANGELOG-UC_ReportsBasicSet1.3.md`
- `docs/20250923_UC_AdvancedSet_POS_FDM_1.5/` - **Advanced Set v1.5** - optional/advanced flows (out of MVP scope, but the data model must not preclude them). Covers: composite products, price changes, rounding, table split/merge, MPV/SPV vouchers, foreign currency & tips, customer accounts, cost-center changes, advance payments / deposits, weighted products, GTIN, supplements, credit notes, etc. `UC200…UC4123`. Changelog: `CHANGELOG-UC_AdvancedSet_1.5.md`. Bundle: `20250916_AdvancedSet_1.5.zip`.

**Design guide:**
- `docs/UX-DESIGN-GUIDE.md` - **Read before implementing Vue components.** Touch POS UX principles, layout specs, color tokens, button sizes, feedback patterns, anti-patterns, payment flow design.

**Original PDFs:** all SPF Finances PDFs are preserved under `docs/original_pdfs/` (mirroring the subfolder structure above). Every `.md` links back to its source PDF in its header.

**Source (official):** https://www.systemedecaisseenregistreuse.be/fr/sce-20-0

**MVP-relevant test scenarios (Tokyo Sushi Bar dine-in only):** focus first on `M110`, `M121`, `M123`, `M130`, `M131`, `M140`, `M141`, `M160`, `M170`–`M172`, `Q100`, and the Z-report `M181`. The Advanced Set can be deferred until post-certification.

### Event Types
The POS must support 8 event types sent to the FDM:
| Code | Type | Description |
|------|------|-------------|
| N | Normal Sale | Standard dine-in transaction |
| P | Pro Forma | Table/order in progress (open tab) |
| T | Training | Training mode (not counted fiscally) |
| C | Copy | Reprint of a previous ticket |
| I | Invoice | Formal invoice generation |
| F | Financial | Cash in/out, tips, discounts |
| S | Social | Staff presence registration (ONSS) |
| R | Report | Z-report, X-report |

### VAT Codes
| Code | Rate | Description |
|------|------|-------------|
| A | 21% | Beverages (alcohol, soft drinks) |
| B | 12% | Restaurant services (dine-in food) |
| C | 6% | Takeaway food |
| D | 0% | Exempt |
| X | N/A | Out of scope |

### POS ↔ FDM Communication
- Protocol: **GraphQL** (mutations sent by POS to FDM)
- Transport: HTTP over LAN (WiFi or RJ45 to FDM device)
- FDM: Checkbox CBX01 (TAIGER BV, ~€195 excl. VAT one-time via Eutronix ref. TAFIP0002.00) - single POS connection
- The POS **NEVER** communicates directly with SPF Finances - that's the FDM's job
- FDM returns: ECDSA secp256r1 signature + control data (fdmId, fdmDateTime, eventLabel, counters, shortSignature)

### POS Identification
- `posId`: 14 characters, unique identifier for this POS installation
- `terminalId`: identifies the physical terminal
- `deviceId`: identifies the device
- `userId`: INSZ number (11 digits) of the operator, or `00000000029` for robot/automated user

### Fiscal Ticket Requirements
Every dine-in ticket must include:
- Restaurant name, address, VAT number
- Date/time of transaction
- Line items with quantities, descriptions, unit prices
- VAT breakdown per rate (A/B/C/D)
- Control data from FDM response:
  - `fdmId`
  - `fdmDateTime`
  - `eventLabel`
  - Counter values
  - `shortSignature`
  - Footer text
- QR code: Model 2, ECC M, from the 38-character URL provided by FDM. The spec names 25x25, but that can't hold the mixed-case URL; the encoder uses version 3 (29x29), to confirm with the SPF

### Critical Behavioral Rules
1. **POS MUST block** if FDM is disconnected - no transactions without FDM
2. **Sequential numbering** - ticket numbers must be strictly sequential, no gaps
3. **JSON archival** - every fiscal event must be archived as raw JSON locally in SQLite
4. **No deletion** - fiscal records can NEVER be deleted or modified
5. **Training mode** - must be visually distinct (different colors, "TRAINING" watermark)

---

## Workspace Structure

```
moyun-pos/
├── Cargo.toml                  # Workspace root
├── CLAUDE.md                   # This file
├── src-tauri/
│   ├── Cargo.toml              # Tauri app crate
│   ├── src/
│   │   ├── main.rs             # Tauri entry point
│   │   ├── commands/           # Tauri IPC command handlers
│   │   │   ├── mod.rs
│   │   │   ├── order.rs        # Order CRUD commands
│   │   │   ├── checkout.rs     # Checkout + FDM flow
│   │   │   ├── menu.rs         # Menu/catalog queries
│   │   │   ├── auth.rs         # User login (INSZ)
│   │   │   ├── report.rs       # Z/X reports
│   │   │   └── print.rs        # Manual print triggers
│   │   └── state.rs            # Tauri managed state
│   ├── tauri.conf.json
│   └── icons/
├── crates/
│   ├── pos-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── order.rs        # Order, OrderLine, modifiers
│   │       ├── menu.rs         # MenuItem, Category, translations
│   │       ├── vat.rs          # VatCode enum, calculations
│   │       ├── user.rs         # User, INSZ validation
│   │       ├── table.rs        # Table management (dine-in)
│   │       └── config.rs       # POS configuration
│   ├── pos-fiscal/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── event.rs        # FiscalEvent enum (N/P/T/C/I/F/S/R)
│   │       ├── fdm_client.rs   # GraphQL client to FDM
│   │       ├── signature.rs    # shortSignature = hex(SHA1(digitalSignature))
│   │       ├── response_check.rs # FDM response vs request consistency check
│   │       ├── ticket.rs       # Fiscal ticket data structure
│   │       ├── archive.rs      # JSON archival logic
│   │       ├── counter.rs      # Sequential numbering
│   │       └── qr.rs           # QR code generation for tickets
│   ├── pos-print/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── escpos.rs       # ESC/POS command builder
│   │       ├── receipt.rs      # Receipt layout/formatting
│   │       ├── drawer.rs       # Cash drawer kick (0x1B 0x70 0x00 0x19 0xFA)
│   │       └── transport.rs    # USB/Serial/TCP printer connection
│   └── pos-db/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── schema.rs       # SQLite schema + migrations
│           ├── queries/        # Typed query modules
│           │   ├── orders.rs
│           │   ├── menu.rs
│           │   ├── fiscal.rs
│           │   ├── users.rs
│           │   └── reports.rs
│           └── migrate.rs      # Schema migration runner
└── ui/
    ├── index.html              # Vite entry point
    ├── vite.config.ts
    ├── tsconfig.json
    ├── package.json
    ├── src/
    │   ├── main.ts             # Vue app bootstrap + router + pinia
    │   ├── App.vue             # Root component with <RouterView>
    │   ├── api/
    │   │   └── ipc.ts          # Typed Tauri invoke wrappers
    │   ├── stores/
    │   │   ├── order.ts        # Pinia store: current order, cart, lines
    │   │   ├── menu.ts         # Pinia store: categories, items (loaded from Rust)
    │   │   ├── auth.ts         # Pinia store: current user, INSZ, role
    │   │   ├── fdm.ts          # Pinia store: FDM connection status
    │   │   └── pos.ts          # Pinia store: POS config, training mode flag
    │   ├── composables/
    │   │   ├── useVat.ts       # VAT calculation helpers
    │   │   ├── usePrint.ts     # Print trigger composable
    │   │   └── useCheckout.ts  # Checkout flow orchestration
    │   ├── router/
    │   │   └── index.ts        # Vue Router (hash mode)
    │   ├── views/
    │   │   ├── LoginView.vue       # INSZ numeric keypad login
    │   │   ├── TablesView.vue      # Table map / floor plan
    │   │   ├── OrderView.vue       # Menu grid + cart sidebar (main POS screen)
    │   │   ├── CheckoutView.vue    # Payment method + FDM call + receipt
    │   │   ├── ReceiptView.vue     # Receipt preview before print
    │   │   ├── ReportsView.vue     # X/Z report screen
    │   │   └── SettingsView.vue    # Admin settings
    │   ├── components/
    │   │   ├── MenuGrid.vue        # Category tabs + item buttons grid
    │   │   ├── CartSidebar.vue     # Order lines list + totals
    │   │   ├── NumPad.vue          # Reusable numeric keypad (INSZ, quantities, cash amount)
    │   │   ├── CategoryTabs.vue    # Horizontal scrollable category tabs
    │   │   ├── MenuItemButton.vue  # Single menu item touch button
    │   │   ├── OrderLine.vue       # Single line in cart (qty, name, price, delete)
    │   │   ├── VatSummary.vue      # VAT breakdown table (A/B/C/D)
    │   │   ├── FdmStatus.vue       # FDM connection indicator (header bar)
    │   │   ├── TrainingBanner.vue  # "TRAINING MODE" overlay when active
    │   │   ├── ModifierDialog.vue  # Modal for item modifiers/options
    │   │   └── PaymentButtons.vue  # Cash / Card / Split payment selection
    │   ├── types/
    │   │   └── index.ts            # Shared TS interfaces matching Rust types
    │   └── styles/
    │       ├── reset.css
    │       ├── tokens.css          # CSS custom properties (colors, spacing, radii)
    │       └── global.css          # Base typography, scrollbar, touch overrides
    └── public/
        └── fonts/              # Noto Sans CJK SC for Chinese simplified menu items
```

---

## Database Schema (SQLite)

```sql
-- Core tables
-- Supported languages
CREATE TABLE languages (
    code TEXT PRIMARY KEY, -- 'zh', 'fr', 'nl', 'en'
    name TEXT NOT NULL,    -- '中文', 'Français', ...
    active INTEGER NOT NULL DEFAULT 1
);

INSERT INTO languages (code, name) VALUES
    ('zh', '中文'),
    ('fr', 'Français'),
    ('nl', 'Nederlands'),
    ('en', 'English');

-- Generic translations table (polymorphic: any entity, any field)
CREATE TABLE translations (
    id INTEGER PRIMARY KEY,
    entity_type TEXT NOT NULL, -- 'category', 'menu_item', 'modifier_group', 'modifier'
    entity_id INTEGER NOT NULL,
    field TEXT NOT NULL,       -- 'name', 'description', etc.
    lang TEXT NOT NULL REFERENCES languages(code),
    value TEXT NOT NULL,
    UNIQUE(entity_type, entity_id, field, lang)
);

CREATE INDEX idx_translations_lookup ON translations(entity_type, entity_id, lang);

-- Core tables (no more name_xx columns)
CREATE TABLE categories (
    id INTEGER PRIMARY KEY,
    sort_order INTEGER NOT NULL DEFAULT 0,
    color TEXT, -- hex color for UI grid
    active INTEGER NOT NULL DEFAULT 1
);
-- translations: ('category', id, 'name', 'zh', '寿司'), ('category', id, 'name', 'fr', 'Sushi')

CREATE TABLE menu_items (
    id INTEGER PRIMARY KEY,
    category_id INTEGER NOT NULL REFERENCES categories(id),
    price_cents INTEGER NOT NULL, -- price in eurocents, no floats!
    vat_code TEXT NOT NULL CHECK(vat_code IN ('A','B','C','D','X')),
    kitchen_printer INTEGER NOT NULL DEFAULT 1, -- send to kitchen
    sort_order INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1
);
-- translations: ('menu_item', id, 'name', 'fr', 'Saumon roll x8')

CREATE TABLE modifier_groups (
    id INTEGER PRIMARY KEY,
    min_select INTEGER NOT NULL DEFAULT 0,
    max_select INTEGER NOT NULL DEFAULT 1,
    menu_item_id INTEGER NOT NULL REFERENCES menu_items(id)
);
-- translations: ('modifier_group', id, 'name', 'fr', 'Sauce')

CREATE TABLE modifiers (
    id INTEGER PRIMARY KEY,
    group_id INTEGER NOT NULL REFERENCES modifier_groups(id),
    price_cents INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0
);
-- translations: ('modifier', id, 'name', 'fr', 'Sauce soja')

-- Order management
CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    table_number INTEGER,
    order_type TEXT NOT NULL CHECK(order_type IN ('dine_in','takeaway')),
    status TEXT NOT NULL CHECK(status IN ('open','closed','voided')) DEFAULT 'open',
    user_insz TEXT NOT NULL, -- operator INSZ
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    closed_at TEXT,
    subtotal_cents INTEGER NOT NULL DEFAULT 0,
    total_cents INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE order_lines (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES orders(id),
    menu_item_id INTEGER NOT NULL REFERENCES menu_items(id),
    quantity INTEGER NOT NULL DEFAULT 1,
    unit_price_cents INTEGER NOT NULL,
    vat_code TEXT NOT NULL,
    name TEXT NOT NULL, -- denormalized for fiscal archive
    modifiers_json TEXT, -- JSON array of applied modifiers
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Fiscal archive (APPEND-ONLY, NEVER DELETE)
CREATE TABLE fiscal_events (
    id INTEGER PRIMARY KEY,
    event_type TEXT NOT NULL CHECK(event_type IN ('N','P','T','C','I','F','S','R')),
    sequence_number INTEGER NOT NULL UNIQUE, -- strictly sequential
    order_id INTEGER REFERENCES orders(id),
    pos_id TEXT NOT NULL,
    terminal_id TEXT NOT NULL,
    user_insz TEXT NOT NULL,
    event_data_json TEXT NOT NULL, -- full event payload sent to FDM
    fdm_response_json TEXT, -- full FDM response (signature, counters, etc.)
    fdm_id TEXT,
    fdm_datetime TEXT,
    event_label TEXT,
    short_signature TEXT,
    qr_data TEXT, -- 38-char URL for QR code
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    archived INTEGER NOT NULL DEFAULT 0
);

-- Payment tracking
CREATE TABLE payments (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES orders(id),
    method TEXT NOT NULL CHECK(method IN ('cash','card','other')),
    amount_cents INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- User sessions
CREATE TABLE users (
    insz TEXT PRIMARY KEY, -- 11-digit INSZ number
    name TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('admin','cashier','waiter')),
    pin_hash TEXT, -- optional PIN for quick switch
    active INTEGER NOT NULL DEFAULT 1
);

-- Z-report archive
CREATE TABLE z_reports (
    id INTEGER PRIMARY KEY,
    report_date TEXT NOT NULL,
    sequence_number INTEGER NOT NULL,
    data_json TEXT NOT NULL, -- full Z-report data
    fiscal_event_id INTEGER REFERENCES fiscal_events(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

---

## MVP Scope - What to Build First

The MVP is the minimum to run a single dine-in service at Tokyo Sushi Bar with fiscal compliance.

### Phase 1: Foundation (build this first)
1. **Cargo workspace** with 4 crates + Tauri app
2. **SQLite schema** - create tables, migration runner
3. **pos-core** - `MenuItem`, `Order`, `OrderLine`, `VatCode` types
4. **pos-db** - CRUD for menu items and orders
5. **Tauri commands** - wire up IPC for menu queries and order management
6. **UI: Login screen** - INSZ input (numeric keypad, touch-friendly)
7. **UI: Menu grid** - category tabs + item grid + order cart sidebar
8. **UI: Order builder** - add items, adjust quantities, show running total with VAT

### Phase 2: Fiscal Core
1. **pos-fiscal** - `FiscalEvent` types, sequential counter, JSON archival
2. **pos-fiscal** - GraphQL client stub (mock FDM for dev - use zwartedoos.be virtual FDM later)
3. **pos-fiscal** - ticket data structure with all required fields
4. **pos-fiscal** - QR code generation
5. **UI: Checkout flow** - payment method selection → FDM call → receipt preview
6. **Sequence number management** - persist in SQLite, crash-safe

### Phase 3: Printing
1. **pos-print** - ESC/POS command builder (text formatting, cut, QR code raster)
2. **pos-print** - Receipt layout matching fiscal requirements
3. **pos-print** - Cash drawer kick command
4. **pos-print** - TCP transport (for Epson TM-m30III network printer)
5. Wire printing into checkout flow

### Phase 4: Reports & Polish
1. **X-report** (mid-day summary, non-resetting)
2. **Z-report** (end-of-day, resets counters, generates R-type fiscal event)
3. **Training mode** (T-type events, visual distinction)
4. **Pro forma / table management** (P-type events for open tabs)
5. **Copy ticket** (C-type event)

---

## Coding Standards

### Rust
- Follow the Google Go Style Guide philosophy adapted for Rust: clarity over cleverness
- Use `thiserror` for error types, `anyhow` only in the Tauri command layer
- All monetary values in **eurocents as i64** - NEVER use floats for money
- All timestamps as `chrono::DateTime<Utc>` in Rust, stored as ISO 8601 strings in SQLite
- Use `#[cfg(test)]` modules with unit tests for every crate
- Fiscal-related code MUST have comprehensive tests - this is legally mandated behavior

### Vue 3 / TypeScript
- Strict mode, no `any`
- **Composition API only** - `<script setup lang="ts">` in every SFC, never Options API
- **Pinia** for all shared state - components never hold business state locally
- All Tauri IPC calls go through typed wrappers in `api/ipc.ts` - components call stores, stores call ipc
- Scoped styles in SFCs (`<style scoped>`) + CSS custom properties from `tokens.css`
- No UI framework (no Vuetify, no PrimeVue, no Nuxt UI) - custom components only, optimized for touch POS
- Props: use `defineProps<{}>()` with TS interfaces, not runtime validation
- Emits: use `defineEmits<{}>()` typed
- Composables (`use*.ts`) for reusable logic shared across views

### General
- All user-facing text must support Chinese simplified (ZH, primary for staff) and French (FR, for fiscal compliance and clients). Dutch/English are bonus.
- Comments in English
- Commit messages: conventional commits (feat/fix/refactor/docs)

---

## Hardware Context

### POS Terminal
- **Penetek A3216 M82** or **Advantech USC-360J**
- Intel Celeron J6412, 8GB RAM, 128GB SSD
- 15.6" FHD 1920x1080 PCAP touchscreen
- IP65 front panel
- Fanless, x86_64, Linux compatible
- USB ports, RJ45 LAN, COM ports, RJ11 cash drawer

### FDM (Fiscal Data Module)
- **Checkbox CBX01** (TAIGER BV, ~€195 excl. VAT one-time) - distributed by Eutronix (ref. TAFIP0002.00)
- LAN (RJ45) + WiFi + USB, power supply included
- Connects to POS via LAN (GraphQL over HTTP)
- Connects to SPF Finances cloud via mTLS (handled by FDM itself)
- For development: Checkbox dev kit (€380 excl. VAT) = physical dev FDM + GraphQL emulator + docs + support
- Alternative (protocol-compatible, untested on hardware): BMC FDM Light (€12.95/month, zwartedoos.be)

### Printer
- **Epson TM-m30III** - 80mm thermal, USB + LAN + WiFi + Bluetooth
- ESC/POS protocol
- RJ11 port for cash drawer

### Cash Drawer
- Standard RJ11 connected to printer
- Opened via ESC/POS command: `0x1B 0x70 0x00 0x19 0xFA` (5 bytes)

---

## What This POS Does NOT Do (Out of Scope for MVP)

- **Online ordering / webshop** - separate project (tsb-customer, Nuxt 3 SSR)
- **Delivery platform integration** - no HubRise in MVP, to be added later
- **Payment terminal integration** - cash + manual card entry for MVP
- **Multi-terminal sync** - single terminal for MVP
- **Inventory management** - not in MVP
- **Customer loyalty** - not in MVP
- **Kitchen Display System (KDS)** - kitchen printer only for MVP
- **Webshop orders passing through FDM** - delivery/takeaway orders from webshop do NOT go through FDM (only dine-in is fiscally registered)

---

## Key Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| App framework | Tauri v2 | Native performance, Rust backend, small binary, kiosk-ready |
| Local DB | SQLite | Embedded, zero-config, crash-safe, perfect for single-terminal POS |
| Frontend | Vue 3 + Composition API | Familiar stack (Nuxt 3 on other TSB projects), reactive, SFC scoped styles, Pinia for state |
| FDM protocol | GraphQL client | SCE 2.0 spec mandates GraphQL for POS↔FDM communication |
| Money type | i64 eurocents | Never use floats for financial calculations |
| Printer protocol | Raw ESC/POS | Direct binary commands, no driver dependency, works on any thermal printer |
| OS | Debian + Cage | Minimal attack surface, auto-starts in kiosk mode, no desktop clutter |
| FDM vendor | Checkbox CBX01 (TAIGER BV) | €195 one-time via Eutronix (beats BMC's €12.95/month rental after ~15 months); dev kit €380 with physical FDM + emulator; standard GraphQL protocol = no lock-in |
| CJK fonts | Noto Sans CJK SC | Free, comprehensive Chinese simplified coverage for menu items. Staff is Chinese-speaking. |

---

## Development Environment

- Dev machine: macOS or Linux desktop
- `cargo tauri dev` for hot-reload development
- Mock FDM: implement a local mock server that simulates GraphQL responses
- Later: connect to the Checkbox emulator / physical dev FDM for realistic testing
- SQLite DB file: `~/.local/share/moyun-pos/pos.db` (dev) or `/var/lib/moyun-pos/pos.db` (prod)

---

## Commands Reference

```bash
# Setup workspace
cargo init --name moyun-pos
cargo tauri init

# Dev mode
cargo tauri dev

# Build release
cargo tauri build --target x86_64-unknown-linux-gnu

# Run tests
cargo test --workspace

# Format
cargo fmt --all

# Lint
cargo clippy --workspace -- -D warnings
```
