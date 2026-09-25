/**
 * Tauri IPC mock for Playwright E2E tests.
 *
 * Intercepts `window.__TAURI_INTERNALS__.invoke` so every
 * `invoke("command", args)` call in the Vue app resolves with
 * test-controlled data instead of hitting the Rust backend.
 *
 * Usage in a spec:
 *   import { mockTauriIPC, overrideIPC } from './fixtures/mock-ipc';
 *   test.beforeEach(async ({ page }) => { await mockTauriIPC(page); });
 *
 * Override a specific handler in a single test:
 *   await overrideIPC(page, 'login', () => { throw 'pin_wrong'; });
 */
import type { Page } from '@playwright/test';

// ── Default fixtures ────────────────────────────────────────────

const DEFAULT_USER = {
  insz: '94060242576',
  name: 'Bruno Moyse',
  role: 'admin',
  active: true,
  has_pin: false,
};

const DEFAULT_USER_WITH_PIN = {
  ...DEFAULT_USER,
  has_pin: true,
};

const DEFAULT_CATEGORY = {
  id: 1,
  name: 'Sushi',
  color: '#e94560',
  sort_order: 0,
  active: true,
};

const DEFAULT_MENU_ITEM = {
  id: 1,
  category_id: 1,
  name: 'Maki saumon x8',
  price_cents: 1200,
  vat_code: 'C',
  kitchen_printer: 1,
  sort_order: 0,
  active: true,
  vat_code_takeaway: null,
  open_price: false,
};

const DEFAULT_ORDER = {
  id: 1,
  table_number: 1,
  order_type: 'dine_in',
  status: 'open',
  user_insz: '94060242576',
  created_at: new Date().toISOString(),
  closed_at: null,
  subtotal_cents: 0,
  total_cents: 0,
};

const DEFAULT_ORDER_LINE = {
  id: 1,
  order_id: 1,
  menu_item_id: 1,
  quantity: 1,
  unit_price_cents: 1200,
  vat_code: 'C',
  name: 'Maki saumon x8',
  modifiers_json: null,
  created_at: new Date().toISOString(),
  department_id: '1',
  department_name: 'Sushi',
};

const DEFAULT_TABLE = (n: number) => ({
  tableNumber: n,
  orderId: null,
  totalCents: 0,
  lineCount: 0,
  status: 'free',
});

// ── Handler registry ────────────────────────────────────────────

type Handler = (args?: any) => any;

function defaultHandlers(): Record<string, Handler> {
  let nextOrderId = 1;
  let nextLineId = 1;

  return {
    // Auth
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    create_user: ({ input }: any) => ({ ...input, active: true, has_pin: false }),
    set_pin: () => undefined,
    get_current_user: () => null,

    // Menu
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],

    // Orders
    create_order: ({ input }: any) => ({
      ...DEFAULT_ORDER,
      id: ++nextOrderId,
      order_type: input?.order_type ?? 'dine_in',
      table_number: input?.table_number ?? 1,
    }),
    add_order_line: ({ input }: any) => ({
      ...DEFAULT_ORDER_LINE,
      id: ++nextLineId,
      order_id: input?.order_id ?? 1,
      menu_item_id: input?.menu_item_id ?? 1,
      // Like the backend: price, VAT and name come from the catalogue.
      quantity: input?.quantity ?? 1,
    }),
    remove_order_line: () => undefined,
    update_order_line_quantity: () => undefined,
    get_order: () => DEFAULT_ORDER,
    list_order_lines: () => [DEFAULT_ORDER_LINE],
    list_open_orders: () => [],

    // Tables
    list_tables: ({ tableCount }: any) => {
      const tables = [];
      for (let i = 1; i <= (tableCount ?? 20); i++) {
        tables.push(DEFAULT_TABLE(i));
      }
      return tables;
    },
    sign_table_order: () => undefined,
    sign_pre_bill: () => undefined,

    // Checkout
    checkout: () => ({
      success: true,
      ticket: null,
      error: null,
      fdm_connected: false,
    }),
    check_fdm_status: () => ({ connected: false }),

    // Print
    print_receipt: () => undefined,
    open_drawer: () => undefined,
    check_printer: () => false,
    print_webshop_kitchen: () => undefined,
    print_webshop_customer: () => undefined,

    // Reports
    generate_x_report: () => ({ success: true, reportType: 'X', reportNo: null, error: null }),
    generate_z_report: () => ({ success: true, reportType: 'Z', reportNo: 1, error: null }),
    list_z_reports: () => [],
    needs_z_report: () => false,
    backup_database: () => '/tmp/backup.db',

    // Copy
    copy_ticket: () => ({ success: true, error: null }),
    list_fiscal_events: () => [],

    // Settings
    get_settings: () => ({
      posId: 'CFOD0061234567',
      terminalId: 'POS1',
      restaurantName: 'Restaurant Exemple',
      fdmUrl: 'http://localhost:8081/graphql',
      printerAddress: null,
      trainingMode: false,
      language: 'fr',
    }),
    set_training_mode: () => undefined,
    new_booking_period: () => undefined,

    // HubRise
    hubrise_get_connection_status: () => ({ connected: false }),
    hubrise_start_oauth_flow: () => 'http://localhost:3001/oauth2/v1/authorize',
    hubrise_complete_oauth: () => ({ connected: true }),
    hubrise_disconnect: () => undefined,
    hubrise_list_webshop_orders: () => [],
    hubrise_patch_order_status: () => undefined,
    hubrise_set_confirmed_time: () => undefined,
  };
}

// ── Public API ──────────────────────────────────────────────────

/**
 * Install the Tauri IPC mock before the page loads. Call this in
 * `test.beforeEach`. The mock intercepts ALL invoke() calls so the
 * app never reaches the Rust backend.
 */
export async function mockTauriIPC(page: Page, overrides?: Record<string, Handler>) {
  const handlers = { ...defaultHandlers(), ...overrides };

  await page.addInitScript((serializedHandlers) => {
    // Reconstruct handlers from the serialized map. Each value is
    // a JSON-safe return value; we wrap it in a function.
    const map: Record<string, any> = serializedHandlers;

    // Track event listeners so tests can emit events if needed.
    const eventListeners: Record<string, Array<(payload: any) => void>> = {};

    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args?: any) => {
        // Tauri's internal plugin commands for the event system.
        if (cmd === 'plugin:event|listen') {
          const event = args?.event ?? args?.target;
          if (event && typeof event === 'string') {
            if (!eventListeners[event]) eventListeners[event] = [];
          }
          // Return a numeric handler ID (used by unlisten).
          return Promise.resolve(Math.floor(Math.random() * 100000));
        }
        if (cmd === 'plugin:event|unlisten') {
          return Promise.resolve();
        }

        if (cmd in map) {
          const val = map[cmd];
          if (val === '__THROW__') return Promise.reject(val);
          return Promise.resolve(typeof val === 'function' ? val(args) : val);
        }
        console.warn(`[mock-ipc] unmocked command: ${cmd}`, args);
        return Promise.resolve(null);
      },
      transformCallback: (fn: Function) => {
        // Tauri uses this to register JS callbacks for events.
        // Return a numeric ID that maps to the callback.
        const id = Math.floor(Math.random() * 100000);
        (window as any)[`_${id}`] = fn;
        return id;
      },
      convertFileSrc: (path: string) => path,
      metadata: {
        currentWebview: { label: 'main' },
        currentWindow: { label: 'main' },
      },
    };
  }, _serializeHandlers(handlers));
}

/**
 * Override a single IPC handler for the next navigation. Useful for
 * testing error paths:
 *
 *   await overrideIPC(page, 'login', '__THROW__');
 */
export async function overrideIPC(page: Page, cmd: string, value: any) {
  await page.evaluate(
    ([c, v]) => {
      const tauri = (window as any).__TAURI_INTERNALS__;
      const prev = tauri.invoke;
      tauri.invoke = (cmd: string, args?: any) => {
        if (cmd === c) {
          if (v === '__THROW__') return Promise.reject(v);
          return Promise.resolve(v);
        }
        return prev(cmd, args);
      };
    },
    [cmd, value] as const,
  );
}

// Serialize handlers to JSON-safe values. Functions are called with
// an empty object `{}` as arg to produce a static default return
// value. If a handler throws (e.g., it expects specific args),
// we fall back to `null` — the test can override it later.
function _serializeHandlers(h: Record<string, Handler>): Record<string, any> {
  const out: Record<string, any> = {};
  for (const [k, v] of Object.entries(h)) {
    if (typeof v === 'function') {
      try {
        out[k] = v({});
      } catch {
        out[k] = null;
      }
    } else {
      out[k] = v;
    }
  }
  return out;
}

/**
 * Login via the tile flow then SPA-navigate to a target route.
 * Uses `window.location.hash` for client-side navigation so the
 * Pinia auth state (set during login) survives — a full
 * `page.goto()` would reload the page and wipe the store.
 */
export async function loginAndGoto(page: Page, hashRoute: string) {
  await page.goto('/');
  await page.click('.user-btn');
  // After login, the app redirects to /#/tables or /#/order.
  await page.waitForURL(/tables|order/);
  // Client-side navigation (no page reload → Pinia state preserved).
  if (!hashRoute.includes('tables')) {
    await page.evaluate((route) => {
      window.location.hash = route;
    }, hashRoute);
    // Wait for Vue Router to settle.
    await page.waitForTimeout(500);
  }
}

// Re-export fixtures for direct use in tests.
export { DEFAULT_USER, DEFAULT_USER_WITH_PIN, DEFAULT_CATEGORY, DEFAULT_MENU_ITEM, DEFAULT_ORDER, DEFAULT_ORDER_LINE };
