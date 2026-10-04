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

// A manager always has a PIN (a manager without one must choose it at
// first login, see login.spec.ts). `tapUserTile` types it.
const DEFAULT_USER = {
  insz: '94060242576',
  name: 'Bruno Moyse',
  role: 'admin',
  active: true,
  has_pin: true,
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

const DEFAULT_CART_LINE = {
  ...DEFAULT_ORDER_LINE,
  name_fr: 'Maki saumon x8',
  name_zh: '三文鱼细卷',
  unsent_quantity: 1,
};

const DEFAULT_TABLE = (n: number) => ({
  tableNumber: n,
  orderId: null,
  totalCents: 0,
  lineCount: 0,
  status: 'free',
  openedAt: null,
  preBillPrinted: false,
});

// ── Handler registry ────────────────────────────────────────────

type Handler = (args?: any) => any;

/**
 * Marks a handler as "live": instead of being invoked ONCE with `{}`
 * at setup time (like every other default handler - see
 * `_serializeHandlers`), its SOURCE is reconstructed with `new
 * Function(...)` inside the page and called for real on every
 * invoke(), with the real args. That lets it echo something derived
 * from what it was actually asked (e.g. `checkout`'s mock delivery
 * outcome reflecting the `delivery` medium/email a test's UI
 * interaction produced), still fully synchronous - no cross-process
 * round trip, so it can't reintroduce the timing-sensitive race
 * `loginAndGoto` relies on (OrderView's mount redirects to Tables
 * asynchronously; a slow default handler would let a test observe the
 * transient `/order` URL instead of the settled `/tables`).
 *
 * Because of the `new Function` reconstruction, a live handler must
 * be fully self-contained: no closures over anything outside its own
 * body (module-level constants, helpers, counters - none of that
 * exists in the page). Keep helpers nested inside the handler itself.
 *
 * Exported so a spec can build its own live handler (e.g. captures.spec.ts's
 * catalogue-aware `add_order_line`/`get_menu_items`, needed when a single
 * test adds several different products and the static default handler -
 * which always echoes `DEFAULT_ORDER_LINE` - would return the wrong name/
 * price/VAT for every product but the first).
 */
export function live(fn: Handler): Handler {
  (fn as { __live?: boolean }).__live = true;
  return fn;
}

function defaultHandlers(): Record<string, Handler> {
  let nextOrderId = 1;
  let nextLineId = 1;

  return {
    // Auth
    login: () => DEFAULT_USER,
    login_with_new_pin: () => DEFAULT_USER,
    remote_support_status: () => ({ available: true, running: false, startedBy: null, startedAt: null }),
    remote_support_running: () => ({ running: false, startedAt: null }),
    start_remote_support: () => ({
      available: true, running: true, startedBy: 'Bruno Moyse', startedAt: '2026-09-26T08:30:00Z',
    }),
    stop_remote_support: () => ({ available: true, running: false, startedBy: null, startedAt: null }),
    list_users: () => [DEFAULT_USER],
    create_user: ({ input }: any) => ({ ...input, active: true, has_pin: false }),
    set_pin: () => undefined,
    get_current_user: () => null,
    logout: () => undefined,

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
    // The cart reads its lines here (both names, quantity not yet sent).
    list_cart_lines: () => [DEFAULT_CART_LINE],
    list_open_orders: () => [],

    // Tables
    list_tables: ({ tableCount }: any) => {
      const tables = [];
      for (let i = 1; i <= (tableCount ?? 20); i++) {
        tables.push(DEFAULT_TABLE(i));
      }
      return tables;
    },
    sign_table_order: () => 'printed',
    sign_pre_bill: () => 'printed',

    // Social (S) and financial (F) events
    work_in: () => ({ success: true, error: null }),
    work_out: () => ({ success: true, error: null }),
    list_open_shifts: () => [],
    money_in_out: () => ({ success: true, error: null }),
    open_drawer_fiscal: () => ({ success: true, error: null }),

    // Checkout
    checkout: live(({ delivery }: any = {}) => {
      function maskEmailForMock(email: string): string {
        const at = email.indexOf('@');
        if (at <= 0) return email;
        return `${email[0]}***@${email[at + 1] ?? ''}***`;
      }
      function mockDeliveryOutcome(d: any, fiscalEventId: number): any {
        const medium = d?.medium ?? 'paper';
        if (medium === 'paper' || !d) {
          return { fiscalEventId, deliveryId: null, medium: 'PAPER', email: 'not_requested', recipientHint: null, paperPrinted: true, needsPaper: false };
        }
        const recipientHint = d.email ? maskEmailForMock(d.email) : null;
        if (medium === 'digital') {
          return { fiscalEventId, deliveryId: 900, medium: 'DIGITAL', email: 'sent', recipientHint, paperPrinted: false, needsPaper: false };
        }
        return { fiscalEventId, deliveryId: 900, medium: 'PAPER_DIGITAL', email: 'sent', recipientHint, paperPrinted: true, needsPaper: false };
      }
      return {
        success: true,
        ticket: null,
        error: null,
        fdm_connected: false,
        fiscal_event_id: 1,
        delivery: mockDeliveryOutcome(delivery, 1),
        kitchen_slip: 'printed',
      };
    }),
    list_open_takeaways: () => [],
    close_unpaid: () => ({
      success: true, ticket: null, error: null, fdm_connected: true,
      fiscal_event_id: 9, delivery: null, kitchen_slip: 'printed',
    }),
    // A healthy FDM: a blocking status covers the sales screens with a
    // veil (FdmAlerts), so FDM specs set their own status.
    check_fdm_status: () => ({ initialized: true, errors: [], warnings: [] }),

    // Refund (event N, negative lines)
    list_refundable_events: () => [],
    list_copyable_tickets: () => [],
    refund_checkout: live(({ delivery }: any = {}) => {
      function maskEmailForMock(email: string): string {
        const at = email.indexOf('@');
        if (at <= 0) return email;
        return `${email[0]}***@${email[at + 1] ?? ''}***`;
      }
      function mockDeliveryOutcome(d: any, fiscalEventId: number): any {
        const medium = d?.medium ?? 'paper';
        if (medium === 'paper' || !d) {
          return { fiscalEventId, deliveryId: null, medium: 'PAPER', email: 'not_requested', recipientHint: null, paperPrinted: true, needsPaper: false };
        }
        const recipientHint = d.email ? maskEmailForMock(d.email) : null;
        if (medium === 'digital') {
          return { fiscalEventId, deliveryId: 900, medium: 'DIGITAL', email: 'sent', recipientHint, paperPrinted: false, needsPaper: false };
        }
        return { fiscalEventId, deliveryId: 900, medium: 'PAPER_DIGITAL', email: 'sent', recipientHint, paperPrinted: true, needsPaper: false };
      }
      return {
        success: true,
        ticket: null,
        error: null,
        fdm_connected: false,
        fiscal_event_id: 2,
        delivery: mockDeliveryOutcome(delivery, 2),
      };
    }),

    // Print
    check_printer: () => false,
    check_kitchen_printer: () => null,
    print_webshop_kitchen: () => undefined,
    print_webshop_customer: () => undefined,

    // Reports
    generate_x_report: () => ({ success: true, reportType: 'X', reportNo: null, error: null }),
    generate_z_report: () => ({ success: true, reportType: 'Z', reportNo: 1, error: null }),
    list_z_reports: () => [],
    needs_z_report: () => false,
    backup_database: () => '/tmp/backup.db',

    // Copy (event C)
    copy_ticket: live(({ delivery }: any = {}) => {
      function maskEmailForMock(email: string): string {
        const at = email.indexOf('@');
        if (at <= 0) return email;
        return `${email[0]}***@${email[at + 1] ?? ''}***`;
      }
      function mockDeliveryOutcome(d: any, fiscalEventId: number): any {
        const medium = d?.medium ?? 'paper';
        if (medium === 'paper' || !d) {
          return { fiscalEventId, deliveryId: null, medium: 'PAPER', email: 'not_requested', recipientHint: null, paperPrinted: true, needsPaper: false };
        }
        const recipientHint = d.email ? maskEmailForMock(d.email) : null;
        if (medium === 'digital') {
          return { fiscalEventId, deliveryId: 900, medium: 'DIGITAL', email: 'sent', recipientHint, paperPrinted: false, needsPaper: false };
        }
        return { fiscalEventId, deliveryId: 900, medium: 'PAPER_DIGITAL', email: 'sent', recipientHint, paperPrinted: true, needsPaper: false };
      }
      return {
        success: true,
        error: null,
        ticket: null,
        fiscalEventId: 3,
        delivery: mockDeliveryOutcome(delivery, 3),
      };
    }),
    list_fiscal_events: () => [],

    // Ticket delivery (paper / email / both)
    check_mail_status: () => ({ configured: false, reachable: false, allowDigitalOnly: false }),
    list_ticket_deliveries: () => [],
    print_missing_original: () => undefined,

    // Settings
    get_settings: () => ({
      posId: 'CPOS0031234567',
      terminalId: 'POS-1-BAR',
      restaurantName: 'The FPS Finances Experience',
      fdmUrl: 'http://localhost:8081/graphql',
      printerAddress: null,
      trainingMode: false,
      language: 'fr',
      configProblems: [],
    }),
    set_training_mode: () => undefined,
    new_booking_period: () => undefined,

    // HubRise: switched on by default (the webshop specs use it).
    get_integrations: () => ({ hubrise_enabled: true, online_orders_open: 0, hubrise_log: [] }),
    set_hubrise_enabled: (args: any) => ({
      hubrise_enabled: args.enabled,
      online_orders_open: 0,
      hubrise_log: [{ enabled: args.enabled, source: 'admin', changed_by: '85073003328', changed_at: '2026-10-03 10:00:00' }],
    }),
    hubrise_get_connection_status: () => ({ connected: false }),
    hubrise_status: () => ({
      connected: false, last_poll_at: null, auth_error_at: null, last_error: null,
      callback_registered_at: null, last_catalog_sync_at: null,
      outbox: { pending: 0, parked: 0 }, outbox_error: null, parked_events: 0,
      stale_orders: 0, catalog: null, catalog_awaiting_adoption: false,
    }),
    hubrise_list_channels: () => [],
    get_item_modifiers: () => [],
    hubrise_start_oauth_flow: () => ({
      url: 'http://localhost:3001/oauth2/v1/authorize',
      qr_svg: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><rect width="1" height="1"/></svg>',
    }),
    hubrise_complete_oauth: () => ({ connected: true }),
    hubrise_disconnect: () => undefined,
    hubrise_list_webshop_orders: () => [],
    hubrise_patch_order_status: () => undefined,
    hubrise_set_confirmed_time: () => undefined,
  };
}

// ── Public API ──────────────────────────────────────────────────

// Serialize handlers to page-safe values. A `live()`-marked handler
// crosses over as its own SOURCE TEXT (reconstructed with `new
// Function` in the page - see `mockTauriIPC`), so it still runs for
// real, synchronously, on every call. Every other handler is called
// ONCE with `{}` here to produce a static default return value - the
// original, timing-safe behaviour every other mocked command keeps.
// If a handler throws (e.g., it expects specific args), we fall back
// to `null` - the test can override it later.
function _serializeHandlers(h: Record<string, Handler>): Record<string, any> {
  const out: Record<string, any> = {};
  for (const [k, v] of Object.entries(h)) {
    if (typeof v === 'function') {
      if ((v as { __live?: boolean }).__live) {
        out[k] = { __live: v.toString() };
        continue;
      }
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
 * Install the Tauri IPC mock before the page loads. Call this in
 * `test.beforeEach`. The mock intercepts ALL invoke() calls so the
 * app never reaches the Rust backend.
 */
export async function mockTauriIPC(page: Page, overrides?: Record<string, Handler>) {
  const handlers = { ...defaultHandlers(), ...overrides };

  await page.addInitScript((serializedHandlers) => {
    // Reconstruct handlers from the serialized map. Most values are
    // JSON-safe return values, wrapped in a function below. A
    // `{ __live }` entry is reconstructed from its source text into a
    // real function, so it runs for real (with the real args) on
    // every call instead of once at setup time.
    const raw: Record<string, any> = serializedHandlers;
    const map: Record<string, any> = {};
    for (const [k, v] of Object.entries(raw)) {
      if (v && typeof v === 'object' && '__live' in v) {
        try {
          // eslint-disable-next-line no-new-func
          map[k] = new Function(`return (${(v as { __live: string }).__live})`)();
        } catch {
          map[k] = null;
        }
      } else {
        map[k] = v;
      }
    }

    // Event listeners (callback ids), so tests can emit events.
    const eventListeners: Record<string, number[]> = {};
    (window as any).__emitTauriEvent = (event: string, payload: any) => {
      for (const id of eventListeners[event] ?? []) {
        (window as any)[`_${id}`]?.({ event, id, payload });
      }
    };

    // Every app command, in call order - read with `ipcCalls()`.
    const calls: Array<{ cmd: string; args: any }> = [];
    (window as any).__ipcCalls = calls;

    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args?: any) => {
        if (!cmd.startsWith('plugin:')) calls.push({ cmd, args: args ?? {} });
        // Tauri's internal plugin commands for the event system.
        if (cmd === 'plugin:event|listen') {
          const event = args?.event ?? args?.target;
          if (event && typeof event === 'string') {
            if (!eventListeners[event]) eventListeners[event] = [];
            if (typeof args?.handler === 'number') eventListeners[event].push(args.handler);
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
          return Promise.resolve(typeof val === 'function' ? val(args ?? {}) : val);
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
 *   await overrideIPC(page, 'login', { __reject: 'pin_wrong' });
 */
export async function overrideIPC(page: Page, cmd: string, value: any) {
  await page.evaluate(
    ([c, v]) => {
      const tauri = (window as any).__TAURI_INTERNALS__;
      const prev = tauri.invoke;
      tauri.invoke = (cmd: string, args?: any) => {
        if (cmd === c) {
          if (v === '__THROW__') return Promise.reject(v);
          // { __reject: 'message' }: the backend's error string.
          if (v && typeof v === 'object' && '__reject' in v) return Promise.reject(v.__reject);
          return Promise.resolve(v);
        }
        return prev(cmd, args);
      };
    },
    [cmd, value] as const,
  );
}

/** Emit a backend event to the listeners the app registered. */
export async function emitTauriEvent(page: Page, event: string, payload: unknown) {
  await page.evaluate(
    ([e, p]) => (window as any).__emitTauriEvent(e, p),
    [event, payload] as const,
  );
}

/** Arguments of each call of `cmd` so far, oldest first. */
export async function ipcCalls(page: Page, cmd: string): Promise<any[]> {
  return page.evaluate(
    (c) => ((window as any).__ipcCalls as Array<{ cmd: string; args: any }>)
      .filter((call) => call.cmd === c)
      .map((call) => call.args),
    cmd,
  );
}

/** PIN typed by `tapUserTile` (the mocked `login` accepts any). */
export const TEST_PIN = '1234';

/**
 * Tap a user tile on the login screen and, when that user has a PIN,
 * type it on the overlay (the 4th digit submits).
 */
export async function tapUserTile(page: Page, name?: string) {
  const tile = name
    ? page.locator('.user-btn', { hasText: name })
    : page.locator('.user-btn').first();
  await tile.click();
  const overlay = page.locator('.pin-overlay');
  await Promise.race([
    page.waitForURL(/tables|order/),
    overlay.waitFor({ state: 'visible' }),
  ]);
  if (await overlay.isVisible()) {
    for (const d of TEST_PIN) await overlay.locator('.nk', { hasText: d }).first().click();
  }
}

/**
 * Login via the tile flow then SPA-navigate to a target route.
 * Uses `window.location.hash` for client-side navigation so the
 * Pinia auth state (set during login) survives - a full
 * `page.goto()` would reload the page and wipe the store.
 */
export async function loginAndGoto(page: Page, hashRoute: string) {
  await page.goto('/');
  await tapUserTile(page);
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
export { DEFAULT_USER, DEFAULT_USER_WITH_PIN, DEFAULT_CATEGORY, DEFAULT_MENU_ITEM, DEFAULT_ORDER, DEFAULT_ORDER_LINE, DEFAULT_CART_LINE };
