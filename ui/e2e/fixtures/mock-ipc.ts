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

// Handlers run inside the page, so they inline the meal-voucher brands the engine accepts
// (pos_eft::brand::is_meal_voucher): Edenred, Pluxee, Monizze, Payflip.

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

// The nightly export as the engine reports it when it is not configured.
const EXPORT_STATUS_DISABLED = {
  enabled: false,
  problem: null,
  endpoint: null,
  keepDays: 30,
  lastExportId: null,
  lastPreparedAt: null,
  lastUploadedId: null,
  lastUploadedAt: null,
  pending: 0,
  nextAttemptAt: null,
  lastError: null,
  lastErrorAt: null,
  kept: 0,
  dropped: 0,
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

/** Seed the open shifts `list_open_shifts` reports (before the page loads). */
export async function shiftsInit(page: Page, shifts: Array<{ userInsz: string; startedAt: string }>) {
  await page.addInitScript((v) => {
    (window as any).__shifts = v;
  }, shifts);
}


/** The badge reader setting as the engine reports it (reader off). */
export const BADGE_SETTINGS_OFF = {
  reader: 'off',
  loginEnabled: false,
  wedgeMaxGapMs: 150,
  wedgeMinLen: 6,
  wedgeMaxLen: 32,
  relinkNeeded: false,
};

/** One scripted answer of `badge_tap`: an outcome, or a refusal code. */
export type BadgeTapStep =
  | { kind: 'ignored' | 'clock_in' | 'clock_out' | 'presence_failed' | 'logged_in' | 'pin_required';
      insz?: string | null; name?: string | null; at?: string | null; error?: string | null; user?: unknown }
  | { reject: string };

/**
 * The badge reader's state, before the page loads: the settings, the active
 * badges and log `list_badges` returns, the answers of `badge_tap` in turn
 * (once the script is out, a tap is `ignored`) and the refusal of the next
 * `link_badge`, if any. The state lives in the page (`window.__badge`) so
 * the settings, link and revoke handlers stay consistent.
 */
export async function badgeInit(
  page: Page,
  init: {
    settings?: Partial<typeof BADGE_SETTINGS_OFF>;
    badges?: Array<{ insz: string; name: string; linkedAt: string; linkedBy: string; linkedByName: string | null }>;
    log?: unknown[];
    taps?: BadgeTapStep[];
    linkReject?: string;
    /** Names `link_badge` reports, by INSZ. */
    names?: Record<string, string>;
  },
) {
  await page.addInitScript(
    ([settings, rest]) => {
      (window as any).__badge = {
        settings,
        badges: rest.badges ?? [],
        log: rest.log ?? [],
        taps: [...(rest.taps ?? [])],
        linkReject: rest.linkReject ?? null,
        names: rest.names ?? {},
      };
    },
    [{ ...BADGE_SETTINGS_OFF, ...(init.settings ?? {}) }, init] as const,
  );
}

/** Press a badge on a USB wedge reader: the number typed fast, then Enter. */
export async function scanBadge(page: Page, token: string) {
  await page.keyboard.type(token, { delay: 5 });
  await page.keyboard.press('Enter');
}

/** The engine's FdmHealth (event "fdm-status"); healthy unless overridden. */
export interface FdmHealthMock {
  connected: boolean;
  answered: boolean;
  severity: 'blocking' | 'warning' | 'info';
  code: string | null;
  mandatory: boolean;
  checkedAt: string;
  nextCheckAt: string | null;
}

export function fdmHealth(over: Partial<FdmHealthMock> = {}): FdmHealthMock {
  return {
    connected: true,
    answered: true,
    severity: 'info',
    code: null,
    mandatory: false,
    checkedAt: '2026-10-08T10:00:00+02:00',
    nextCheckAt: null,
    ...over,
  };
}

/** `fdm_check_now` and `fdm_health` answering with this health. */
export function fdmMocks(health: FdmHealthMock): Record<string, Handler> {
  return { fdm_check_now: () => health, fdm_health: () => health };
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
    leave_order: () => 'printed',
    set_active_order: () => undefined,
    lock_session: () => ({ orderError: null }),
    sign_pre_bill: () => 'printed',


    // Staff badges: the state lives in `window.__badge` (see `badgeInit`).
    get_badge_settings: live(() => {
      const b = (window as any).__badge;
      return b
        ? b.settings
        : { reader: 'off', loginEnabled: false, wedgeMaxGapMs: 150, wedgeMinLen: 6, wedgeMaxLen: 32, relinkNeeded: false };
    }),
    set_badge_settings: live((args: any = {}) => {
      const w = window as any;
      w.__badge = w.__badge ?? { settings: {}, badges: [], log: [], taps: [], linkReject: null };
      w.__badge.settings = { ...w.__badge.settings, ...args.input, relinkNeeded: false };
      w.__badge.log = [
        { action: 'settings', insz: null, name: null, by: '94060242576', byName: 'Bruno Moyse', at: '2026-10-17 12:10:00', detail: null },
        ...w.__badge.log,
      ];
      return w.__badge.settings;
    }),
    list_badges: live(() => {
      const b = (window as any).__badge;
      return { badges: b ? b.badges : [], log: b ? b.log : [] };
    }),
    link_badge: live((args: any = {}) => {
      const w = window as any;
      const b = w.__badge;
      if (b && b.linkReject) return Promise.reject(b.linkReject);
      const name = (b && b.names && b.names[args.employeeInsz]) || args.employeeInsz;
      const at = '2026-10-17 12:15:00';
      if (b) {
        b.badges = b.badges.concat([{ insz: args.employeeInsz, name, linkedAt: at, linkedBy: '94060242576', linkedByName: 'Bruno Moyse' }]);
        b.log = [{ action: 'link', insz: args.employeeInsz, name, by: '94060242576', byName: 'Bruno Moyse', at, detail: null }, ...b.log];
      }
      return { insz: args.employeeInsz, name, linkedAt: at };
    }),
    revoke_badge: live((args: any = {}) => {
      const b = (window as any).__badge;
      if (b) {
        b.badges = b.badges.filter((x: any) => x.insz !== args.employeeInsz);
        b.log = [{ action: 'revoke', insz: args.employeeInsz, name: null, by: '94060242576', byName: 'Bruno Moyse', at: '2026-10-17 12:20:00', detail: 'manager' }, ...b.log];
      }
      return null;
    }),
    badge_tap: live(() => {
      const b = (window as any).__badge;
      const step = b && b.taps.shift();
      if (step && step.reject) return Promise.reject(step.reject);
      return { kind: 'ignored', insz: null, name: null, at: null, error: null, user: null, ...(step ?? {}) };
    }),

    // Social (S) and financial (F) events
    work_in: () => ({ success: true, error: null }),
    work_out: () => ({ success: true, error: null }),
    // Presence registered by a manager: the open shifts live in the page
    // (`window.__shifts`, seeded by `shiftsInit`), so clock-in then out
    // shows in the next `list_open_shifts`.
    list_open_shifts: live(() => {
      return ((window as any).__shifts ?? []).slice();
    }),
    clock_in_employee: live((args: any = {}) => {
      const w = window as any;
      w.__shifts = (w.__shifts ?? []).concat([
        { userInsz: args.employeeInsz, startedAt: '2026-10-17T12:02:00+02:00' },
      ]);
      return { success: true, error: null };
    }),
    clock_out_employee: live((args: any = {}) => {
      const w = window as any;
      w.__shifts = (w.__shifts ?? []).filter((s: any) => s.userInsz !== args.employeeInsz);
      return { success: true, error: null };
    }),
    end_shift: () => ({ success: true, error: null }),
    user_activity: () => null,
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
    // A healthy FDM, as the engine monitor reports it: a blocking
    // status covers the sales screens with a veil (FdmAlerts), so FDM
    // specs set their own health (`fdmMocks`). check_fdm_status stays a
    // valid command but the UI no longer calls it.
    fdm_health: () => fdmHealth(),
    fdm_check_now: () => fdmHealth(),
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

    // Nightly export (manager only): not configured unless a spec says otherwise
    export_status: () => EXPORT_STATUS_DISABLED,
    export_run_now: () => EXPORT_STATUS_DISABLED,

    // Not linked unless a spec answers `check_payment_terminal` (overrides / overrideIPC).
    check_payment_terminal: () => ({ configured: false, kind: null, connected: false, busy: false, detail: null }),

    // Card payments charged and not yet settled by a sale: none unless a spec overrides it.
    list_unsettled_card_payments: () => [],

    // Linked card terminal: none unless a spec overrides get_settings.paymentTerminal.
    // The commands follow the script of `eftScript` (window.__eft), else approve.
    list_eft_payments: live((args: any = {}) => {
      const e = (window as any).__eft;
      if (!e) return [];
      return Object.values(e.payments).filter(
        (p: any) =>
          p.order_id === args.orderId && !p.used &&
          (p.status === 'approved' || p.status === 'sent' || p.status === 'unknown'),
      );
    }),
    eft_abort: live(() => {
      const e = (window as any).__eft;
      if (e) {
        e.aborted = true;
        if (e.release) e.release();
      }
      return undefined;
    }),
    eft_pay: live((args: any = {}) => {
      const e = (window as any).__eft;
      function make(step: any, id: number) {
        const ok = step.status === 'approved';
        return {
          id, order_id: args.orderId, amount_cents: args.amountCents, status: step.status,
          card_type: ok ? 'debit' : null, brand: ok ? (step.brand ?? 'Bancontact') : null,
          terminal_reference: ok ? `T-${id}` : null, authorisation_code: ok ? 'A1' : null,
          masked_pan: ok ? 'XXXX XXXX XXXX 1234' : null, meal_voucher: ok && ['Edenred', 'Pluxee', 'Monizze', 'Payflip'].includes(step.brand), detail: step.detail ?? null,
          created_at: '2026-10-12 10:00:00', used: false,
        };
      }
      if (!e) return make({ status: 'approved' }, 1);
      const step = e.pay.shift() ?? { status: 'approved' };
      e.payCalls.push({ orderId: args.orderId, amountCents: args.amountCents });
      if (step.reject) return Promise.reject(step.reject);
      const id = e.nextId++;
      if (step.hold) {
        // The terminal waits for the card until eft_abort (or a test) releases it.
        return new Promise((resolve) => {
          e.release = () => {
            e.release = null;
            const after = step.afterAbort ?? { status: 'cancelled' };
            e.payments[id] = make(after, id);
            resolve(e.payments[id]);
          };
        });
      }
      e.payments[id] = make(step, id);
      return e.payments[id];
    }),
    eft_check: live((args: any = {}) => {
      const e = (window as any).__eft;
      const known = e ? e.payments[args.id] : null;
      const step = (e && e.check.shift()) || { status: 'approved' };
      const ok = step.status === 'approved';
      const base = known ?? { id: args.id, order_id: 1, amount_cents: 0, created_at: '2026-10-12 10:00:00', used: false };
      const next = {
        ...base, status: step.status,
        card_type: ok ? 'debit' : null, brand: ok ? (step.brand ?? 'Bancontact') : null,
        terminal_reference: ok ? `T-${args.id}` : null, authorisation_code: ok ? 'A1' : null,
        masked_pan: ok ? 'XXXX XXXX XXXX 1234' : null, meal_voucher: ok && ['Edenred', 'Pluxee', 'Monizze', 'Payflip'].includes(step.brand), detail: step.detail ?? null,
      };
      if (e) e.payments[args.id] = next;
      return next;
    }),

    // Settings
    get_settings: () => ({
      posId: 'CPOS0031234567',
      terminalId: 'POS-1-BAR',
      restaurantName: 'The FPS Finances Experience',
      fdmUrl: 'http://localhost:8081/graphql',
      printerAddress: null,
      trainingMode: false,
      language: 'fr',
      posSwVersion: 'pili-pos/1.0.0',
      configProblems: [],
    }),
    set_training_mode: () => undefined,
    new_booking_period: () => undefined,

    // Table plan: 1 to 20 until a spec changes it.
    get_table_plan: () => ({ plan: '1-20', table_count: 20, log: [] }),
    set_table_plan: (args: any) => ({ plan: args.plan, table_count: 0, log: [] }),

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

/** One scripted answer of the terminal. */
export interface EftStep {
  status: 'approved' | 'declined' | 'cancelled' | 'unknown' | 'sent';
  brand?: string;
  detail?: string;
  /** `eft_pay` rejects with this error (nothing was charged). */
  reject?: string;
  /** `eft_pay` waits (the terminal waits for the card) until `eft_abort`. */
  hold?: boolean;
  /** With `hold`: what the abort ends with (default cancelled). */
  afterAbort?: EftStep;
}

/** A payment already known to the engine (crash recovery). */
export interface EftKnown {
  id: number;
  order_id: number;
  amount_cents: number;
  status: 'approved' | 'unknown' | 'sent';
  brand?: string | null;
}

/**
 * The terminal's scripted answers: `pay` for each `eft_pay` in turn,
 * `check` for each `eft_check`, `known` for payments that already exist
 * (`list_eft_payments`). Call before the page loads.
 */
export async function eftScript(
  page: Page,
  script: { pay?: EftStep[]; check?: EftStep[]; known?: EftKnown[] },
) {
  await page.addInitScript((s) => {
    const payments: Record<number, unknown> = {};
    for (const k of s.known ?? []) {
      const ok = k.status === 'approved';
      payments[k.id] = {
        ...k, card_type: ok ? 'debit' : null, brand: ok ? (k.brand ?? 'Bancontact') : null,
        terminal_reference: ok ? `T-${k.id}` : null, authorisation_code: null, masked_pan: null,
        meal_voucher: ok && ['Edenred', 'Pluxee', 'Monizze', 'Payflip'].includes(k.brand), detail: null, created_at: '2026-10-12 09:00:00', used: false,
      };
    }
    (window as any).__eft = {
      pay: [...(s.pay ?? [])], check: [...(s.check ?? [])], payments, nextId: 100,
      payCalls: [], release: null, aborted: false,
    };
  }, script);
}

/** The settings of a till with a linked card terminal (or training). */
export function settingsWith(over: Record<string, unknown>) {
  return () => ({
    posId: 'CPOS0031234567', terminalId: 'POS-1-BAR', restaurantName: 'The FPS Finances Experience',
    fdmUrl: 'http://localhost:8081/graphql', printerAddress: null, trainingMode: false,
    language: 'fr', posSwVersion: 'pili-pos/1.0.0', configProblems: [], ...over,
  });
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
export { EXPORT_STATUS_DISABLED, DEFAULT_USER, DEFAULT_USER_WITH_PIN, DEFAULT_CATEGORY, DEFAULT_MENU_ITEM, DEFAULT_ORDER, DEFAULT_ORDER_LINE, DEFAULT_CART_LINE };
