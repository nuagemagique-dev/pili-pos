# CLAUDE.md - POS SCE 2.0 MVP (Tauri + Rust)

## Project Identity

**Name:** pili-pos  
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
│  │  Rust (src-tauri shell + 5 core crates)    │  │
│  │  pos-engine   → fiscal engine, no Tauri    │  │
│  │  pos-core     → business logic, state      │  │
│  │  pos-fiscal   → SCE 2.0 protocol, FDM comm │  │
│  │  pos-print    → ESC/POS thermal printing   │  │
│  │  pos-db       → SQLite persistence         │  │
│  └──────┬──────────┬──────────┬──────────────┘  │
│         │          │          │                   │
│    ┌────▼───┐ ┌────▼───┐ ┌───▼────┐            │
│    │ SQLite │ │  FDM   │ │Printer │            │
│    │ local  │ │ (LAN)  │ │(USB/   │            │
│    │  DB    │ │GraphQL │ │ LAN)   │            │
│    └────────┘ └────────┘ └────────┘            │
└─────────────────────────────────────────────────┘
```

### Engine and shell

The fiscal core is Tauri-free and lives in `crates/pos-engine`; `src-tauri` is a thin shell around it.

- **`crates/pos-engine`** - the engine: `commands/` (every `do_*` function: checkout, signing, refund, reports, HubRise, settings), `state.rs` (`AppState`), `engine.rs` (`Engine::open(EngineConfig)`: opens the database under an exclusive `engine.lock`, migrates, recovers, loads the configuration), `background.rs` (`start_background`: automatic Z, mail outbox, presence worker, idle lock, FDM status monitor, HubRise poller and outbox; `shutdown`), `events.rs` (`EngineEvent`, `EventSink`), `install.rs` (`init_db`, `check_config`). It has no platform conditional compilation: platform differences are `EngineConfig` inputs (paths, configuration source, `SoftwareIdentity`, `HttpTls`, optional device id and support program).
- **`src-tauri`** - the shell: `main.rs` (CLI, logging, window, health check on `127.0.0.1:9999`, builds the `EngineConfig`), `ipc/` (one `#[tauri::command]` wrapper per command, a single call into `pos_engine::commands`), `events.rs` (`TauriEventSink`), `cli.rs`, `paths.rs`. The configuration is resolved from `PILI_POS_CONFIG`, else `/etc/pili-pos/pos.json`, else the development defaults.
- **`posSwVersion`** is a value the shell passes at start (`EngineConfig.identity`); on Linux it is `pili-pos/` plus the workspace version (`pili-pos/1.0.0`, the version of the first Linux filing), returned by `get_settings` and shown on the login screen (AM Art. 47).
- **UI rules owned by the engine** (P1b): the event P when an order is left (`commands/session.rs`: `do_leave_order`, `do_set_active_order`, `do_lock_session`), the clock-in at login (presence worker) and the clock-out at end of shift (`do_end_shift`, `commands/social.rs`), the 5 minute idle lock (`EngineTimers.idle_lock`, `do_user_activity`) and the FDM status polling and classification (`commands/fdm_monitor.rs`, event `fdm-status`). The Vue stores (`auth.ts`, `order.ts`, `fdm.ts`), `App.vue` and the router guard only call these commands and listen to the events; `docs/android/ENGINE.md` section 7 lists them.
- **`crates/pos-engine-testkit`** - the test harness (`publish = false`): in-memory state, FDM and HubRise mocks, UC fixture loader. It is a dev-dependency of `pos-engine` only and is not part of the certified binary.
- Integration tests live in `crates/pos-engine/tests` (the `spf_e2e_*`, `hubrise_*`, `access_control` targets, ...). `scripts/android/check_ipc_wrappers.py` checks that every wrapper stays a single engine call; `docs/android/ENGINE.md` documents the engine for the Android port.

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
- Debian 13 minimal (no desktop environment)
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
- `docs/FPSFIN_API_PROTOCOL_FDM_FINCLOUD_1.10_20260414.md` - **FPS Finance ↔ FDM cloud API protocol v1.10** (current; v1.09, `docs/FPSFIN_API_PROTOCOL_FDM_FINCLOUD_1.09_20251017_FINAL.md`, is superseded). Describes the mTLS communication between the FDM and the SPF Finances cloud. The POS does NOT speak this protocol directly, but understand the lifecycle (provisioning, daily sync, status reporting) before designing FDM error handling.

**Certification manual & application form** (in `docs/20250307_POS_CERTIFICATION_MANUEAL_FRV1.1/`):
- `20250307_FR - Manuel de certification_POS_V11.md` - main manual (110 p).
- `20250115_Manual_Intro_ToRead_FR.md` - read this first (intro / how to use the manual).
- `20250115_FORMULAIRE DEMANDE_CERTIF_POS-FDM.md` - certification request form.
- `20250115_AnnexePourVosExemples.docx` - annex template for submitting examples (Word doc, no `.md` extraction).
- Embedded copies of earlier UC sets: `20241223_UC_BasicSet_POS_FDM_1.4 (1)/`, `20041223_UC_AdvancedSet_POS_FDM_1.2/` - superseded by the newer UC folders below; kept for reference.

**Test scenarios (use cases the implementation must pass to obtain certification):**
- `docs/20260821_UC_BasicSet_POS_FDM_1.8/` - **Basic Set v1.8** (current; the older `docs/20250916_UC_BasicSet_POS_FDM_1.7/` is superseded, kept for reference) - minimum mandatory flows.
  - Sales: `M110_signSale`, `M111_signSale_Refund`, `M112A/B_signSale_RefundPartial`
  - Orders / pro forma: `M121_signOrder`, `M122_signCostCenterChange`, `M123_signPrebill`
  - Cash drawer: `M130_signMoneyInOut`, `M131_signDrawerOpen`, `M132_signPaymentCorrection`, `M133_signMoneyInOut_DrawerDeclaration`
  - Staff (social): `M140_signWorkIn`, `M141_signWorkOut`
  - Invoicing & copies: `M150_signInvoice`, `M160_signCopy_Sale`, `M161_signCopy_Order`
  - Training mode: `M170_signWorkIn_Training`, `M171/171B_signSale_Training`, `M172_signCopy_Training`
  - FDM queries: `Q100_Status`, `Q110_Status_multi_lang_messages`
  - Test setup: `UC_TestSetup_20240806.md` is no longer in the Basic Set 1.8 folder; use the copy in `docs/20260821_UC_Reports_1.3/` (identical content). Changelog: `CHANGELOG-UC_BasicSet1.8.md`
- `docs/20260821_UC_Reports_1.3/` - **Reports v1.3** (repackaged 2026-08-21; the older `docs/20250916_UC_Reports_1.3/` is superseded) - X/Z report scenarios.
  - `M180_signReportTurnoverX`, `M181_signReportTurnoverZ`, `M182_signReportUserX`, `M183_signReportUserZ`, `M184_signReportTurnoverZUserZ`
  - `InputReport/` - input fixtures, `Scenario Reports.md` - full scenario doc (⚠️ source PDF is diagram-only - open the PDF for the flow charts), changelog: `CHANGELOG-UC_ReportsBasicSet1.3.md`
- `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/` - **Advanced Set v1.6** (current; the older `docs/20250923_UC_AdvancedSet_POS_FDM_1.5/` is superseded, kept for reference) - optional/advanced flows (out of MVP scope, but the data model must not preclude them). Covers: composite products, price changes, rounding, table split/merge, MPV/SPV vouchers, foreign currency & tips, customer accounts, cost-center changes, advance payments / deposits, weighted products, GTIN, supplements, credit notes, etc. `UC200…UC4123`. Changelog: `CHANGELOG-UC_AdvancedSet_1.6.md`.

**Legislation** (in `docs/legislation/`, French text from Justel and the Moniteur belge): the AM of 29 April 2024 on the technical aspects of certification (authoritative for ticket wording, report contents and the automatic Z), the AR of 30 December 2009 on the cash register system (consolidated), and the AR of 25 May 2026 amending it (in force 1 July 2026).

**Design guide:**
- `docs/UX-DESIGN-GUIDE.md` - **Read before implementing Vue components.** Touch POS UX principles, layout specs, color tokens, button sizes, feedback patterns, anti-patterns, payment flow design.
- `docs/BRAND.md` - the Pili brand (logo, colours and their meaning, typography, tone). Colours are CSS tokens in `ui/src/styles/tokens.css`; every staff screen starts with `AppHeader.vue`.

**Certification procedure (2026):** `docs/20260903_Intro_certification_procédure.md` (7 steps, required documents, `Report_Testing` folder). Submission runbook: `docs/internal/ASSEMBLAGE_FINAL_FR.md`.

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
- FDM: module2 (PRACSYS, SCE 2.0 certified); test machine uses the module2 online simulator (docs/internal/LIVE_FDM_MODULE2.md)
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
- QR code: Model 2, ECC M, from the 38-character URL provided by FDM. The spec names 25x25, but that can't hold the mixed-case URL; the encoder uses version 3 (29x29), decided 2026-09-27 (see `docs/internal/PLAN_SPF_PACKAGE_2026-09.md` §3)

### Critical Behavioral Rules
1. **POS MUST block** if FDM is disconnected - no transactions without FDM
2. **Sequential numbering** - ticket numbers must be strictly sequential, no gaps
3. **JSON archival** - every fiscal event must be archived as raw JSON locally in SQLite
4. **No deletion** - fiscal records can NEVER be deleted or modified
5. **Training mode** - must be visually distinct (different colors, "TRAINING" watermark)

---

## Workspace Structure

```
pili-pos/
├── Cargo.toml                  # Workspace root
├── CLAUDE.md                   # This file
├── src-tauri/
│   ├── Cargo.toml              # Tauri app crate
│   ├── src/
│   │   ├── main.rs             # Tauri entry point: CLI, logging, window, health check, EngineConfig
│   │   ├── lib.rs              # Library facade (cli, events, ipc, paths)
│   │   ├── ipc/                # Tauri command wrappers (one call into pos-engine each, 102 in all, 100 in the cert build)
│   │   ├── events.rs           # TauriEventSink (engine events to the webview)
│   │   ├── cli.rs              # --version, --init-db, --check-config
│   │   └── paths.rs            # Linux install paths
│   ├── tauri.conf.json
│   └── icons/
├── crates/
│   ├── pos-engine/             # Tauri-free engine
│   │   ├── src/                # commands/, engine.rs, state.rs, background.rs, events.rs, install.rs
│   │   └── tests/              # integration tests (spf_e2e_*, hubrise_*, access_control, ...)
│   ├── pos-engine-testkit/     # Test harness (publish = false, outside the certified binary)
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
    │   │   ├── auth.ts         # Pinia store: current user, INSZ, role; lock and end of shift go through the engine
    │   │   ├── fdm.ts          # Pinia store: FDM health as classified by the engine monitor
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
1. **Cargo workspace** with the core crates, the `pos-engine` engine and the Tauri shell
2. **SQLite schema** - create tables, migration runner
3. **pos-core** - `MenuItem`, `Order`, `OrderLine`, `VatCode` types
4. **pos-db** - CRUD for menu items and orders
5. **Engine commands + Tauri wrappers** - `pos-engine` commands, wired up through `src-tauri/src/ipc` for menu queries and order management
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
- Use `thiserror` for error types, `anyhow` only in the command layer (`pos-engine` commands and the `src-tauri` wrappers)
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
- **module2** (PRACSYS, SCE 2.0 certified), chosen 2026-09-27
- Connects to POS via LAN: GraphQL over HTTP (port 80), or HTTPS (443, recommended) with module2's mTLS client certificate (`fdm_ca_file`, `fdm_client_cert_file`, `fdm_client_key_file`; module2's certificates are X.509 v1, checked by `pos-fiscal::fdm_tls`, not webpki)
- Every FDM requires its bearer token (`fdm_token_file`); the POS's posId must be allowed in the FDM settings
- Connects to SPF Finances cloud itself
- Development and certification test machine: module2 online simulator `http://fdm.module2.be/graphql/` (posId `CPOS0031234567`), no physical FDM at the intake (manual §IV: "fournissez un simulateur avec la machine test")
- Other certified FDMs are protocol-compatible, untested

### Printer
- **Epson TM-m30III** - 80mm thermal, connected over LAN (raw TCP 9100; the POS does not print over USB)
- ESC/POS protocol
- RJ11 port for cash drawer

### Cash Drawer
- Standard RJ11 connected to printer
- Opened via ESC/POS command: `0x1B 0x70 0x00 0x19 0xFA` (5 bytes)

---

## What This POS Does NOT Do (Out of Scope for MVP)

- **Own webshop frontend** - separate project (tsb-customer, Nuxt 3 SSR); its orders reach the POS only through the HubRise integration (see below)
- **Payment terminal integration** - cash + manual card entry for MVP (a linked card terminal is an optional installation setting, see "Payment terminal (EFT)")
- **Multi-terminal sync** - single terminal for MVP
- **Inventory management** - not in MVP
- **Customer loyalty** - not in MVP
- **Kitchen Display System (KDS)** - kitchen printer only for MVP

### HubRise integration (in the certified binary, switched per installation)
- One binary for every installation (decided 2026-10-03): the `cert` feature includes `hubrise`; only invoicing (`peppol`) stays out. Whether HubRise runs is the installation's switch: `hubrise_enabled` in `pos.json` (default off), then an admin in Settings; every change in the append-only `integration_switch_log` (migration 25, `commands/integrations.rs`). Off: no poller pass, no outbox pass, no till sale echoed, accept/reject/sync/connect refused (`hubrise_disabled`); settling an already signed order (handover, refund, closing) always works. Switching off is refused while online orders are unsettled. The UI reads the switch at runtime (`stores/integrations.ts`), no build-time flag. The installation's OAuth client is the `hubrise` section of `pos.json` (`client_id`, `client_secret_file`, URLs default to the real HubRise; `commands/hubrise.rs::oauth_config`): without it a release build refuses to connect (`hubrise_not_configured`), a debug build falls back to the `HUBRISE_*` variables of `.env`, else the local mock.
- Every online order (Deliveroo, Uber Eats, own webshop) must end in an N event signed by the FDM (SPF FAQ Q203). Prepaid: N at acceptance (APP for platforms, ONLINE for the webshop). Unpaid: P at acceptance, N at handover. Rejected orders are never signed; a cancellation after N is a manager-confirmed refund N.
- The POS menu mirrors the HubRise catalog (`crates/pos-catalog`, source-neutral; HubRise mapping in `pos-hubrise/src/catalog_map.rs`). Stable ids by `external_id`, deactivate never delete, invalid catalogs refused whole. Local menu editing is locked while a source manages the menu.
- Migrations: 1 to 9 base, 10 pending_signs (signing recovery), 11 table plan (`table_plan`, `table_plan_log` append-only; `commands/table_plan.rs`), 12 to 19 reserved for certified hotfixes, 20+ HubRise (`schema_hubrise.rs`, 25 the integration switch), 21 catalog sync (`schema_catalog.rs`, feature `catalog-sync`), 27 payment terminal (`schema_eft.rs`, not feature-gated: `eft_payments`, never deleted, a final answer never changes), 30 till discounts (`schema_till.rs`, not feature-gated: `order_entries.till_op`, append-only `till_discounts`; `commands/discount.rs`: a manager's PIN-confirmed order discount, offered item or offered line, written as priceChange TILL-DISCOUNT / TILL-FREE / TILL-FREE-LINE; a line with one unsent entry gets the change directly, else a PRICE_CHANGE correction pair; on a registered dine-in table a P ORDER is signed at once), 31 kitchen notes (`schema_till.rs`, not feature-gated: `order_lines.kitchen_note` and `kitchen_note_printed`, local data only; `commands/order.rs::do_set_line_note`: a staff member's note of at most 60 CP858-printable characters per cart line, printed under its item on the bar / kitchen slip, a MODIFICATION note slip when the line already went to the kitchen; never part of any FDM message, product name, ticket or export), 23 price changes (`order_price_changes`: an online order's discounts, spread over its product lines and sent as priceChanges of scope EVENT; `pos-fiscal/src/price_change.rs`).
- Tests run against `pos-hubrise-mock` in-process (`crates/pos-engine/tests/hubrise_*.rs`).

### Table plan (in the certified binary)
- Table plan (decided 2026-10-10): which table numbers the Tables screen shows, as numbers and ranges ("1-12, 20-25", 1 to 999, at most 200). `pos.json` gives the starting value (`tables`, else 1 to `table_count`); a manager changes it in Settings, panel "Plan de salle" (`get_table_plan` anyone, `set_table_plan` managers, error `table_plan_invalid`). Same switch semantics as HubRise: first start or a NEW `pos.json` plan sets it, otherwise the last value stays; every change in the append-only `table_plan_log`. A table out of the plan with an open order stays visible. No fiscal data depends on it (`crates/pos-core/src/table_plan.rs`).

### Payment correction (in the certified binary)
- Payment correction (M132, 2026-10-17): a manager (PIN) corrects the method of one MANUAL payment line of a sale of the current booking period: event F `signPaymentCorrection` (line removed in negative, same amount under the new method, `fdmRef` of the sale), one per sale, no ticket, no drawer. Refused: refund, unpaid, online, refunded, training mismatch, closed period, terminal card, same method, rounding. Commands `list_sale_payments` and `correct_payment` (`crates/pos-engine/src/commands/payment_correction.rs`); the reports count it in `negativeCorrections` / `positiveCorrections` (`report_totals.rs`), the user report credits the signing manager.

### Payment terminal (EFT)
- With a `payment_terminal` section in `pos.json` (`worldline_terminal_api`, `ccv_payment_api` or `ccv_opi`; drivers in `crates/pos-eft`) the till asks the card terminal for the amount instead of the operator typing the card: `eft_pay`, `eft_abort`, `eft_check` (any user), `list_eft_payments` (`commands/eft.rs`; errors `eft_not_configured`, `eft_amount_invalid`, `eft_pending`, `eft_busy`, `eft_unreachable`, `eft_rejected`, `eft_payment_not_usable`, `eft_below_charged`, `eft_edit_pending`; an order edit may not bring the total under its approved unused card payments). `get_settings` tells the UI which kind is linked (`paymentTerminal`); without it nothing changes (manual card, `MANUAL`, `CARD_DEBIT`).
- `list_unsettled_card_payments` (`UnsettledCardPayment`: payment_id, order_id, order_type, table_number, amount_cents, brand, created_at) lists the approved card payments no sale has settled yet: the Z step of both UIs warns about them (never blocks) and the table plan / takeaway list mark their orders.
- `check_payment_terminal` (`PaymentTerminalStatus`: configured, kind, connected, busy, detail) feeds the header light and the start-up row, only when configured; it never charges and never probes while a payment holds `AppState::eft_line` (answers busy).
- Every payment is stored in `eft_payments` (migration 27) BEFORE the terminal is called; at start a `sent` row becomes `unknown`. Nothing is charged unless the FDM is operational right now (AM art. 14). A payment waiting for its answer (`sent`, `unknown`) blocks other payments and its order's sale until `eft_check`; an approved payment must be used by the sale (`PaymentPart.eft_payment_id`, exact amount) or the sale is refused. The signed line is `AUTOMATIC` with the terminal's card type (ids 2/6/7), the brand as `provider`, the terminal's id as `reference`; payments are marked used in the sale's commit. Card payments are never rounded; refunds stay cash; reversing an approved payment is not supported yet.
- Tests use `Terminal::scripted` through the `eft-scripted` feature of `pos-engine` (testkit only): `crates/pos-engine/tests/eft_*.rs`.

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
| FDM vendor | module2 (PRACSYS) | free online simulator with developer portal (error simulation, logs), readable docs; standard GraphQL protocol = no lock-in |
| CJK fonts | Noto Sans CJK SC | Free, comprehensive Chinese simplified coverage for menu items. Staff is Chinese-speaking. |

---

## Development Environment

- Dev machine: macOS or Linux desktop
- `cargo tauri dev` for hot-reload development
- Mock FDM: implement a local mock server that simulates GraphQL responses
- Live FDM: `scripts/live_fdm.sh` against the module2 simulator
- SQLite DB file: `~/.local/share/pili-pos/pos.db` (dev) or `/var/lib/pili-pos/pos.db` (prod)

---

## Commands Reference

```bash
# Setup workspace
cargo init --name pili-pos
cargo tauri init

# Dev mode
cargo tauri dev

# Build release
cargo tauri build --target x86_64-unknown-linux-gnu

# Run tests (the integration tests are in crates/pos-engine/tests)
cargo test --workspace

# Certification build tests (cert feature set, HubRise in, Peppol out)
cargo test -p pili-pos --no-default-features --features cert
cargo test -p pos-engine --no-default-features --features cert

# IPC wrappers: each one a single engine call, command map
python3 scripts/android/check_ipc_wrappers.py
python3 scripts/android/check_ipc_wrappers.py --map

# Live FDM (module2 simulator, by hand only, never in CI; reads .env.live)
scripts/live_fdm.sh

# Live HubRise (test location "Liège TEST" only, by hand, never in CI; free plan = 5 orders/month)
scripts/live_hubrise_login.sh            # once: authorize URL, then `... CODE` stores target/live-hubrise/token.json
scripts/live_hubrise.sh <test name>      # see crates/pos-engine/tests/live_hubrise.rs; recordings feed crates/pos-hubrise-mock/fixtures/hubrise
scripts/live_combined.sh <test name>     # live HubRise + live module2 FDM together (crates/pos-engine/tests/live_combined.rs); PILI_LIVE_HUBRISE_REUSE_ORDER=<id> retakes an order a cut-short run placed

# Online orders end to end, offline: the real cert binary vs the HubRise, FDM and printer mocks over WebDriver (Linux container)
scripts/e2e_hubrise.sh                   # ui/e2e/real-binary/hubrise.test.mjs; logs and the app's database in target/e2e-hubrise/logs/

# Mock peripherals: FDM + email API on :8081, printer on :9100 (jobs on :9180)
cargo run --package pos-fdm-mock
cargo run --package pos-printer-mock

# Format
cargo fmt --all

# Lint
cargo clippy --workspace -- -D warnings
```
