import { test, expect, type Page } from '@playwright/test';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  mockTauriIPC,
  overrideIPC,
  ipcCalls,
  tapUserTile,
  loginAndGoto,
  live,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
  fdmHealth,
  fdmMocks,
  shiftsInit,
  badgeInit,
  scanBadge,
  eftScript,
  settingsWith,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
} from './fixtures/mock-ipc';

// Certification dossier screenshot tour (docs/SUBMISSION_ARTIFACTS/
// CAPTURES_ECRAN_RUNBOOK_FR.md). Run with `npm run captures` - never
// part of the normal `npx playwright test` gate (see playwright.config.ts).
//
// Every test mocks the Tauri IPC layer exactly like the rest of the
// e2e suite (mock-ipc.ts) - no FDM mock, no SQLite, no `cargo tauri
// dev`. Each capture asserts the expected screen (the runbook's
// "Écran attendu") before taking the screenshot, so a broken screen
// fails the test instead of producing a wrong picture.
//
// All data is fictional and brand-neutral: the SPF test setup (the
// default in mock-ipc.ts's get_settings), and three test employees
// distinct from the suite's default fixture user (who is named after
// a real person - never used here).

const SCREENSHOTS_DIR = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../../docs/SUBMISSION_ARTIFACTS/screenshots',
);

/** Screenshot the current viewport under its runbook file name. The
 * kiosk is always full-screen, so every capture is a plain viewport
 * shot (no `fullPage`). */
async function capture(page: Page, name: string) {
  // PILI_CAPTURES_DRY=1 runs the scenarios (and their assertions) without
  // writing a PNG, to check a scenario without touching the dossier.
  if (process.env.PILI_CAPTURES_DRY) return;
  await page.screenshot({
    path: path.join(SCREENSHOTS_DIR, `${name}.png`),
    animations: 'disabled',
  });
}

// ── Test employees (PROFILS_UTILISATEURS_FR.md §1) ─────────────────
const GERANT = { insz: '94060242576', name: 'Gérant Test', role: 'admin' as const, active: true, has_pin: true };
const SERVEUR = { insz: '84022899837', name: 'Elisa', role: 'cashier' as const, active: true, has_pin: false };
const APPRENTI = { insz: '99012345667', name: 'Apprenti Test', role: 'waiter' as const, active: true, has_pin: false };
const STAFF = [GERANT, SERVEUR, APPRENTI];
// Kitchen staff who never use the till: no tile, no PIN, a manager clocks them in and out.
const CUISINIER = { insz: '88010112345', name: 'Cuisinier Test', role: 'waiter' as const, active: true, has_pin: false, presence_only: true };

// ── Menu seed for the main sale flow - 3 categories, VAT A/B/C ──────
const CATEGORIES = [
  { id: 1, name: 'Sushis', sort_order: 0, color: '#e94560', active: true },
  { id: 2, name: 'Boissons', sort_order: 1, color: '#3498db', active: true },
  { id: 3, name: 'Desserts', sort_order: 2, color: '#f39c12', active: true },
];

const TABLE4_ORDER_ID = 501;

// The cart built during captures 04-09: Maki x2 (VAT B), California
// x1 (VAT B), Soda x1 (VAT A), Mochi x1 (VAT C) - three VAT rates in
// one sale, and one line at quantity 2, as the runbook asks.
const FILLED_LINES = [
  { id: 1, order_id: TABLE4_ORDER_ID, menu_item_id: 1, quantity: 2, unit_price_cents: 1200, vat_code: 'B', name: 'Maki saumon x8', modifiers_json: null, created_at: new Date().toISOString(), department_id: '1', department_name: 'Sushis' },
  { id: 2, order_id: TABLE4_ORDER_ID, menu_item_id: 2, quantity: 1, unit_price_cents: 900, vat_code: 'B', name: 'California roll x6', modifiers_json: null, created_at: new Date().toISOString(), department_id: '1', department_name: 'Sushis' },
  { id: 3, order_id: TABLE4_ORDER_ID, menu_item_id: 3, quantity: 1, unit_price_cents: 300, vat_code: 'A', name: 'Soda 33cl', modifiers_json: null, created_at: new Date().toISOString(), department_id: '2', department_name: 'Boissons' },
  { id: 4, order_id: TABLE4_ORDER_ID, menu_item_id: 5, quantity: 1, unit_price_cents: 400, vat_code: 'C', name: 'Mochi glacé', modifiers_json: null, created_at: new Date().toISOString(), department_id: '3', department_name: 'Desserts' },
];
const FILLED_TOTAL_CENTS = FILLED_LINES.reduce((sum, l) => sum + l.unit_price_cents * l.quantity, 0); // 4000 = 40,00 €

/** 20 tables, a few already occupied, the rest free. */
function tablesWith(occupied: Record<number, { orderId: number; totalCents: number; lineCount: number }>) {
  const tables = [];
  for (let n = 1; n <= 20; n++) {
    const o = occupied[n];
    tables.push(
      o
        ? { tableNumber: n, orderId: o.orderId, totalCents: o.totalCents, lineCount: o.lineCount, status: 'occupied' }
        : { tableNumber: n, orderId: null, totalCents: 0, lineCount: 0, status: 'free' },
    );
  }
  return tables;
}

const OTHER_OCCUPIED = { 2: { orderId: 77, totalCents: 1850, lineCount: 2 }, 7: { orderId: 78, totalCents: 3200, lineCount: 3 }, 15: { orderId: 79, totalCents: 900, lineCount: 1 } };
const TABLES_BEFORE = tablesWith(OTHER_OCCUPIED);
const TABLES_AFTER = tablesWith({ ...OTHER_OCCUPIED, 4: { orderId: TABLE4_ORDER_ID, totalCents: FILLED_TOTAL_CENTS, lineCount: FILLED_LINES.length } });

test.describe('Section 1 - écrans de flux fonctionnels', () => {
  test('01_login - une tuile par employé', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => STAFF });
    await page.goto('/');
    await expect(page.locator('.title')).toHaveText('Qui êtes-vous ?');
    await expect(page.locator('.user-btn')).toHaveCount(3);
    await expect(page.locator('.user-name')).toHaveText(['Gérant Test', 'Elisa', 'Apprenti Test']);
    await expect(page.locator('.user-role')).toHaveText(['Gérant', 'Serveur', 'Apprenti']);
    await expect(page.locator('.other-btn')).toBeVisible();
    await capture(page, '01_login');
  });

  test('02_login_pin - saisie du PIN, avant le 4e chiffre', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => STAFF, login: () => GERANT });
    await page.goto('/');
    await page.locator('.user-btn', { hasText: 'Gérant Test' }).click();
    await expect(page.locator('.pin-title')).toHaveText('Code PIN');
    await expect(page.locator('.pin-dot')).toHaveCount(4);
    for (const d of '111') await page.locator('.nk', { hasText: d }).click();
    await expect(page.locator('.pin-dot.filled')).toHaveCount(3);
    await capture(page, '02_login_pin');
  });

  test('03-09 - tables, commande, enregistrement, paiement cash', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => GERANT,
      list_users: () => STAFF,
      list_tables: () => TABLES_BEFORE,
      get_categories: () => CATEGORIES,
      // Live: the real backend resolves name/price/VAT/category from
      // the menu_item_id, so this must react to the actual argument
      // instead of always echoing the same product (see live()'s doc
      // in mock-ipc.ts). Self-contained: no closures over the test's
      // own CATEGORIES/FILLED_LINES constants.
      get_menu_items: live(({ categoryId }: any = {}) => {
        const ITEMS = [
          { id: 1, category_id: 1, name: 'Maki saumon x8', price_cents: 1200, vat_code: 'B', vat_code_takeaway: 'C', open_price: false, kitchen_printer: true, sort_order: 0, active: true },
          { id: 2, category_id: 1, name: 'California roll x6', price_cents: 900, vat_code: 'B', vat_code_takeaway: 'C', open_price: false, kitchen_printer: true, sort_order: 1, active: true },
          { id: 3, category_id: 2, name: 'Soda 33cl', price_cents: 300, vat_code: 'A', vat_code_takeaway: 'A', open_price: false, kitchen_printer: false, sort_order: 0, active: true },
          { id: 4, category_id: 2, name: 'Thé vert', price_cents: 250, vat_code: 'A', vat_code_takeaway: 'A', open_price: false, kitchen_printer: false, sort_order: 1, active: true },
          { id: 5, category_id: 3, name: 'Mochi glacé', price_cents: 400, vat_code: 'C', vat_code_takeaway: 'C', open_price: false, kitchen_printer: false, sort_order: 0, active: true },
          { id: 6, category_id: 3, name: 'Thé au litchi', price_cents: 350, vat_code: 'C', vat_code_takeaway: 'C', open_price: false, kitchen_printer: false, sort_order: 1, active: true },
        ];
        return categoryId ? ITEMS.filter((i) => i.category_id === categoryId) : ITEMS;
      }),
      // Live: must reflect which table was actually tapped (table 4),
      // not a hardcoded one - the default handler always answers
      // table_number 1.
      create_order: live(({ input }: any = {}) => ({
        id: 501,
        table_number: input?.table_number ?? null,
        order_type: input?.order_type ?? 'dine_in',
        status: 'open',
        user_insz: input?.user_insz ?? '',
        created_at: new Date().toISOString(),
        closed_at: null,
        subtotal_cents: 0,
        total_cents: 0,
      })),
      // The order starts genuinely empty (04_order_empty); the cart
      // fills up client-side as items are tapped (see order.ts).
      list_cart_lines: () => [],
      get_order: () => ({
        id: 501, table_number: 4, order_type: 'dine_in', status: 'open',
        user_insz: GERANT.insz, created_at: new Date().toISOString(), closed_at: null,
        subtotal_cents: 0, total_cents: 0,
      }),
      // Live: same reason as get_menu_items - each tapped product
      // must come back as itself, not always the first one.
      add_order_line: live(({ input }: any = {}) => {
        const CATALOG: Record<number, { name: string; price_cents: number; vat_code: string; department_id: string; department_name: string }> = {
          1: { name: 'Maki saumon x8', price_cents: 1200, vat_code: 'B', department_id: '1', department_name: 'Sushis' },
          2: { name: 'California roll x6', price_cents: 900, vat_code: 'B', department_id: '1', department_name: 'Sushis' },
          3: { name: 'Soda 33cl', price_cents: 300, vat_code: 'A', department_id: '2', department_name: 'Boissons' },
          4: { name: 'Thé vert', price_cents: 250, vat_code: 'A', department_id: '2', department_name: 'Boissons' },
          5: { name: 'Mochi glacé', price_cents: 400, vat_code: 'C', department_id: '3', department_name: 'Desserts' },
          6: { name: 'Thé au litchi', price_cents: 350, vat_code: 'C', department_id: '3', department_name: 'Desserts' },
        };
        const item = CATALOG[input?.menu_item_id as number] ?? CATALOG[1];
        return {
          id: Math.floor(Math.random() * 1_000_000) + 1000,
          order_id: input?.order_id ?? 501,
          menu_item_id: input?.menu_item_id ?? 1,
          quantity: input?.quantity ?? 1,
          unit_price_cents: input?.unit_price_cents ?? item.price_cents,
          vat_code: item.vat_code,
          name: item.name,
          modifiers_json: null,
          created_at: new Date().toISOString(),
          department_id: item.department_id,
          department_name: item.department_name,
        };
      }),
      checkout: () => ({
        success: true,
        ticket: {
          restaurant_name: 'The FPS Finances Experience',
          restaurant_address: 'Boulevard Albert II 33, 1030 Brussels',
          restaurant_vat: 'BE0000000097',
          pos_id: 'CPOS0031234567',
          terminal_id: 'POS-1-BAR',
          pos_datetime: new Date().toISOString(),
          ticket_number: 9,
          event_label: 'N',
          is_training: false,
          lines: [],
          subtotal: FILLED_TOTAL_CENTS / 100,
          total: FILLED_TOTAL_CENTS / 100,
          vat_breakdown: [],
          payments: [],
          rounding: null,
          fdm_ref: { fdmId: 'FDM-TEST-0001', fdmDateTime: new Date().toISOString(), eventLabel: 'N', eventCounter: 9, totalCounter: 9 },
          short_signature: 'TESTSIGNATURE',
          verification_url: null,
          footer: [],
          ticket_type_label: 'TICKET',
          is_refund: false,
          lang: 'fr' as const,
        },
        error: null,
        fdm_connected: true,
        fiscal_event_id: 9,
        delivery: { fiscalEventId: 9, deliveryId: null, medium: 'PAPER' as const, email: 'not_requested' as const, recipientHint: null, paperPrinted: true, needsPaper: false },
      }),
    });

    await page.goto('/');
    await tapUserTile(page, 'Gérant Test');
    await page.waitForURL(/tables/);
    await expect(page.locator('.table-card.occupied')).toHaveCount(3);
    await capture(page, '03_tables');

    // Table 4, free - starts a fresh dine-in order.
    await page.locator('.table-card').nth(3).click();
    await page.waitForURL(/order/);
    await expect(page.locator('.header-table')).toHaveText('Table 4');
    await expect(page.locator('.cart-empty')).toBeVisible();
    await capture(page, '04_order_empty');

    // A few products across 3 categories, one at quantity 2.
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click();
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click();
    await page.locator('.item-btn', { hasText: 'California roll x6' }).click();
    await page.locator('.tab', { hasText: 'Boissons' }).click();
    await page.locator('.item-btn', { hasText: 'Soda 33cl' }).click();
    await page.locator('.tab', { hasText: 'Desserts' }).click();
    await page.locator('.item-btn', { hasText: 'Mochi glacé' }).click();
    await expect(page.locator('.qty', { hasText: '2' })).toBeVisible();
    await expect(page.locator('.total-amount')).toContainText('40,00');
    await capture(page, '05_order_filled');

    // "Tables": registers the P event, sans impression. Table 4 now
    // shows as occupied - swap the mocks a real backend would have
    // updated by itself.
    await overrideIPC(page, 'list_tables', TABLES_AFTER);
    await overrideIPC(page, 'list_cart_lines', FILLED_LINES);
    await page.locator('.nav-btn', { hasText: 'Tables' }).click();
    await page.waitForURL(/tables/);
    await expect.poll(() => ipcCalls(page, 'leave_order')).toHaveLength(1);
    const table4 = page.locator('.table-card').nth(3);
    await expect(table4).toHaveClass(/occupied/);
    await expect(table4.locator('.table-total')).toContainText('40,00');
    await capture(page, '06_order_registered');

    // Back on table 4, "PAYER".
    await table4.click();
    await page.waitForURL(/order/);
    await expect(page.locator('.cart-lines')).toContainText('Mochi glacé');
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await expect(page.locator('.total-value')).toHaveText('40,00');
    await capture(page, '07_checkout_pay_method');

    // CASH, 50 €.
    await page.locator('.cash-btn').click();
    for (const d of ['5', '0']) await page.locator('.nk', { hasText: d }).click();
    await expect(page.locator('.change-box')).toContainText('10,00');
    await capture(page, '08_checkout_cash');

    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    await expect(page.locator('.receipt-fdm')).toContainText('FDM-TEST-0001');
    // 50 € for 40 €: 10 € back (checkout-change.spec.ts).
    await expect(page.locator('.receipt-change')).toContainText('10,00');
    await expect(page.locator('.delivery-msg')).toContainText('Ticket imprimé');
    await capture(page, '09_checkout_done');
  });
});

test.describe('09a-09f - ticket numérique par e-mail', () => {
  test('09a-09f - médium picker, clavier, confirmation, envoi, repli papier, panel admin', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => GERANT,
      list_users: () => STAFF,
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM],
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: true }),
    });
    await page.goto('/');
    await tapUserTile(page, 'Gérant Test');
    await page.waitForURL(/tables/);
    await page.locator('.table-card').first().click();
    await page.waitForURL(/order/);
    await page.locator('.item-btn').first().click();
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);

    // 09a - the medium picker, Papier selected by default.
    await expect(page.locator('.medium-btn.active')).toContainText('Papier');
    await expect(page.locator('.medium-btn', { hasText: 'E-mail' })).toBeVisible();
    await expect(page.locator('.medium-btn', { hasText: 'Les deux' })).toBeVisible();
    await capture(page, '09a_ticket_medium_picker');

    // 09b - typing the address on the on-screen keyboard.
    await page.locator('.medium-btn', { hasText: 'E-mail' }).click();
    await expect(page.locator('.keypad-overlay')).toBeVisible();
    for (const ch of 'client') await page.locator('.key', { hasText: ch }).click();
    await expect(page.locator('.email-display')).toContainText('client');
    await capture(page, '09b_ticket_medium_email_keyboard');

    await page.locator('.quick-key', { hasText: '@gmail.com' }).click();
    await page.locator('.act-btn.primary').click();

    // 09c - confirmation screen, large type.
    await expect(page.getByRole('heading', { name: "Confirmer l'adresse" })).toBeVisible();
    await expect(page.locator('.email-confirm-display')).toHaveText('client@gmail.com');
    await capture(page, '09c_ticket_medium_email_confirm');

    // 09d - paid: sent, masked address shown, nothing printed.
    await page.locator('.act-btn.primary').click();
    await page.locator('.cash-btn').click();
    await page.locator('.shortcut.exact').click();
    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    await expect(page.locator('.delivery-msg')).toContainText('Ticket envoyé à');
    await expect(page.locator('.delivery-msg')).toContainText('c***@g***');
    await capture(page, '09d_checkout_done_email_sent');

    // 09e - a new sale, e-mail-only again, but the mail service is
    // down: the paper ticket comes out instead, and the screen says so.
    await overrideIPC(page, 'checkout', {
      success: true, ticket: null, error: null, fdm_connected: true, fiscal_event_id: 10,
      delivery: { fiscalEventId: 10, deliveryId: 6, medium: 'PAPER', email: 'not_requested', recipientHint: 'c***@g***', paperPrinted: true, needsPaper: false },
    });
    await page.locator('.done-actions .primary').click();
    await page.waitForURL(/tables/);
    await page.locator('.table-card').first().click();
    await page.waitForURL(/order/);
    await page.locator('.item-btn').first().click();
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await page.locator('.medium-btn', { hasText: 'E-mail' }).click();
    for (const ch of 'client') await page.locator('.key', { hasText: ch }).click();
    await page.locator('.quick-key', { hasText: '@gmail.com' }).click();
    await page.locator('.act-btn.primary').click();
    await page.locator('.act-btn.primary').click();
    await page.locator('.cash-btn').click();
    await page.locator('.shortcut.exact').click();
    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    await expect(page.locator('.delivery-msg')).toContainText('Ticket imprimé');
    await capture(page, '09e_checkout_done_paper_fallback');

    // 09f - Settings, "Tickets numériques": masked addresses only.
    await overrideIPC(page, 'list_ticket_deliveries', [
      { id: 1, fiscalEventId: 9, medium: 'DIGITAL', recipientHint: 'c***@g***', status: 'sent', attempts: 1, lastError: null, fallbackPrinted: false, createdAt: new Date().toISOString() },
      { id: 2, fiscalEventId: 10, medium: 'PAPER_DIGITAL', recipientHint: 'c***@g***', status: 'fallback_printed', attempts: 3, lastError: 'timeout', fallbackPrinted: true, createdAt: new Date().toISOString() },
    ]);
    await page.goto('/#/settings');
    // The panel is below the fold - scroll it into view for the capture.
    await page.locator('.deliveries-section').scrollIntoViewIfNeeded();
    await expect(page.locator('.mail-status-pill.mail-status-ok')).toHaveCount(2);
    await expect(page.locator('.delivery-row')).toHaveCount(2);
    await capture(page, '09f_admin_ticket_deliveries');
  });
});

test('10_pre_bill - addition provisoire imprimée pour une table', async ({ page }) => {
  await mockTauriIPC(page, {
    login: () => GERANT,
    list_users: () => STAFF,
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
  });
  await page.goto('/');
  await tapUserTile(page, 'Gérant Test');
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
  await page.locator('.item-btn').first().click();
  await page.locator('.pre-bill-btn').click();
  await expect(page.locator('.toast', { hasText: 'Addition provisoire imprimée' })).toBeVisible();
  await capture(page, '10_pre_bill');
});

test('11_refund - articles à rembourser, quantité ajustée', async ({ page }) => {
  const now = new Date().toISOString();
  const REFUNDABLE = [{ fiscalEventId: 9, ticketNumber: 9, orderId: TABLE4_ORDER_ID, tableNumber: 4, totalCents: FILLED_TOTAL_CENTS, createdAt: now }];
  const LINES = [
    { id: 1, order_id: TABLE4_ORDER_ID, menu_item_id: 1, quantity: 2, unit_price_cents: 1200, vat_code: 'B', name: 'Maki saumon x8', modifiers_json: null, created_at: now, department_id: '1', department_name: 'Sushis' },
    { id: 2, order_id: TABLE4_ORDER_ID, menu_item_id: 3, quantity: 1, unit_price_cents: 300, vat_code: 'A', name: 'Soda 33cl', modifiers_json: null, created_at: now, department_id: '2', department_name: 'Boissons' },
  ];
  await mockTauriIPC(page, {
    login: () => GERANT,
    list_users: () => STAFF,
    list_refundable_events: () => REFUNDABLE,
    list_order_lines: () => LINES,
  });
  await loginAndGoto(page, '/reports');
  await page.getByRole('tab', { name: 'Tickets' }).click();
  await page.locator('.ticket-row .refund-btn').first().click();
  const makiLine = page.locator('.refund-line', { hasText: 'Maki saumon x8' });
  await makiLine.click();
  await expect(makiLine.locator('.rl-qty')).toContainText('2');
  await makiLine.locator('.rl-qty-btn', { hasText: '-' }).click();
  await expect(makiLine.locator('.rl-qty')).toContainText('1');
  await expect(makiLine.locator('.rl-max')).toContainText('2');
  await expect(page.locator('.refund-confirm-btn')).toContainText('12,00');
  await capture(page, '11_refund');
});

test('12_copy - liste des tickets copiables', async ({ page }) => {
  const now = new Date().toISOString();
  const TICKETS = [
    { fiscalEventId: 12, eventLabel: 'T', ticketNumber: 11, tableNumber: null, totalCents: 400, createdAt: now },
    { fiscalEventId: 11, eventLabel: 'N', ticketNumber: 10, tableNumber: null, totalCents: -1200, createdAt: now },
    { fiscalEventId: 10, eventLabel: 'P', ticketNumber: 9, tableNumber: 2, totalCents: 1850, createdAt: now },
    { fiscalEventId: 9, eventLabel: 'N', ticketNumber: 8, tableNumber: 4, totalCents: FILLED_TOTAL_CENTS, createdAt: now },
  ];
  await mockTauriIPC(page, {
    login: () => GERANT,
    list_users: () => STAFF,
    list_copyable_tickets: () => TICKETS,
  });
  await loginAndGoto(page, '/reports');
  await page.getByRole('tab', { name: 'Tickets' }).click();
  await expect(page.locator('.copy-ticket-btn')).toHaveCount(4);
  await expect(page.getByRole('heading', { name: 'Tickets récents' })).toBeVisible();
  await capture(page, '12_copy');
});

test('13_training - bandeau et filigrane TRAINING', async ({ page }) => {
  await mockTauriIPC(page, { login: () => GERANT, list_users: () => STAFF });
  await loginAndGoto(page, '/settings');
  await page.locator('.toggle').click();
  await expect(page.locator('.toggle')).toContainText('Actif');
  // The "Tables" button sits right under the banner: a real tap, which
  // the banner used to swallow (training-banner.spec.ts).
  await page.locator('.app-header .nav-btn', { hasText: 'Tables' }).click();
  await page.waitForURL(/tables/);
  await expect(page.locator('.training-banner')).toHaveText('MODE FORMATION · Les transactions ne sont PAS enregistrées');
  await expect(page.locator('.training-watermark')).toBeVisible();
  await capture(page, '13_training');
});

test('14-15 - rapport X puis Z armé', async ({ page }) => {
  await mockTauriIPC(page, { login: () => GERANT, list_users: () => STAFF });
  await loginAndGoto(page, '/reports');

  await page.getByRole('button', { name: /^Rapport X/ }).click();
  await expect(page.locator('.toast', { hasText: 'Rapport généré (X)' })).toBeVisible();
  await capture(page, '14_reports_x');

  await page.locator('.z-btn').click();
  await expect(page.locator('.z-btn')).toHaveText('Confirmer la clôture ?');
  await expect(page.locator('.z-btn')).toHaveClass(/armed/);
  await capture(page, '15_reports_z_confirm');
});

test('16-17 - mouvement de caisse puis ouverture du tiroir', async ({ page }) => {
  await mockTauriIPC(page, { login: () => GERANT, list_users: () => STAFF });
  await loginAndGoto(page, '/reports');
  await page.getByRole('tab', { name: 'Caisse' }).click();

  await page.locator('.cash-preset', { hasText: '20 €' }).click();
  await page.locator('.cash-reason', { hasText: 'Fond de caisse' }).click();
  await page.locator('.cash-submit').click();
  await expect(page.locator('.toast', { hasText: 'Mouvement enregistré' })).toBeVisible();
  await capture(page, '16_cash_in_out');

  // Let the first toast expire so the drawer capture stays uncluttered.
  await page.waitForTimeout(3200);
  await page.locator('.drawer-btn').click();
  await expect(page.locator('.toast', { hasText: 'Tiroir ouvert' })).toBeVisible();
  await capture(page, '17_drawer_open');
});

test('18_logout - retour à l\'écran de connexion', async ({ page }) => {
  await mockTauriIPC(page, {
    login: () => SERVEUR,
    list_users: () => STAFF,
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
  });
  await page.goto('/');
  await tapUserTile(page, 'Elisa');
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
  await page.locator('.item-btn').first().click();
  await page.locator('.nav-logout').click();
  await expect(page.locator('.user-btn')).toHaveCount(3);
  await capture(page, '18_logout');
});

// The badge reader of the captures: a USB wedge reader, badge login on,
// and one badge already linked (Elisa).
const BADGE_ELISA = { insz: SERVEUR.insz, name: 'Elisa', linkedAt: '2026-10-17 09:10:00', linkedBy: GERANT.insz, linkedByName: 'Gérant Test' };
const BADGE_LOG = [
  { action: 'link', insz: SERVEUR.insz, name: 'Elisa', by: GERANT.insz, byName: 'Gérant Test', at: '2026-10-17 09:10:00', detail: null },
  { action: 'settings', insz: null, name: null, by: GERANT.insz, byName: 'Gérant Test', at: '2026-10-17 09:05:00', detail: null },
];
const BADGE_SETTINGS = { reader: 'usb' as const, loginEnabled: true, wedgeMaxGapMs: 50, wedgeMinLen: 6, wedgeMaxLen: 32 };

test('19_admin_users - employés, présence, badges et formulaire d\'ajout', async ({ page }) => {
  await badgeInit(page, { settings: BADGE_SETTINGS, badges: [BADGE_ELISA], log: BADGE_LOG });
  await mockTauriIPC(page, { login: () => GERANT, list_users: () => [...STAFF, CUISINIER] });
  await shiftsInit(page, [{ userInsz: SERVEUR.insz, startedAt: '2026-10-17T12:02:00+02:00' }]);
  await loginAndGoto(page, '/settings');
  await expect(page.locator('.worker-row')).toHaveCount(4);
  await expect(page.locator('.worker-row', { hasText: 'Gérant Test' }).locator('.worker-meta')).toContainText('Gérant');
  await expect(page.locator('.worker-row', { hasText: 'Cuisinier Test' }).locator('.worker-meta')).toHaveText('Présence seule');
  await expect(page.locator('.worker-row', { hasText: 'Elisa' }).getByTestId('badge-state')).toHaveText('Badge lié le 17/10/2026');
  await expect(page.locator('.worker-row', { hasText: 'Elisa' }).getByTestId('badge-btn')).toHaveText('Retirer le badge');
  await expect(page.locator('.worker-row', { hasText: 'Cuisinier Test' }).getByTestId('badge-btn')).toHaveText('Lier un badge');
  await expect(page.locator('.role-btn')).toHaveCount(4);
  await capture(page, '19_admin_users');
});

// Proposed capture (runbook): the "Lecteur de badges" panel of the manager.
test('19b_admin_badges - le panneau Lecteur de badges', async ({ page }) => {
  await badgeInit(page, { settings: BADGE_SETTINGS, badges: [BADGE_ELISA], log: BADGE_LOG });
  await mockTauriIPC(page, { login: () => GERANT, list_users: () => [...STAFF, CUISINIER] });
  await loginAndGoto(page, '/settings');
  const panel = page.getByTestId('badge-settings');
  // The panel is folded by default: unfold it for the capture.
  await panel.getByTestId('badge-toggle').click();
  await expect(panel.locator('h2')).toHaveText('Lecteur de badges');
  await expect(panel.locator('.reader-btn.active')).toHaveText('Lecteur USB');
  await expect(panel.getByTestId('badge-login-switch')).toHaveAttribute('aria-checked', 'true');
  await expect(panel.getByTestId('badge-wedgeMaxGapMs')).toHaveText('50');
  await expect(panel.getByTestId('badge-log').locator('li')).toHaveCount(2);
  await panel.scrollIntoViewIfNeeded();
  await capture(page, '19b_admin_badges');
});

// Proposed capture (runbook): the dialog waiting for the badge to link.
test('19c_badge_enrol - présentation du badge à lier', async ({ page }) => {
  await badgeInit(page, { settings: BADGE_SETTINGS, badges: [BADGE_ELISA], log: BADGE_LOG });
  await mockTauriIPC(page, { login: () => GERANT, list_users: () => [...STAFF, CUISINIER] });
  await loginAndGoto(page, '/settings');
  await page.locator('.worker-row', { hasText: 'Apprenti Test' }).getByTestId('badge-btn').click();
  await expect(page.getByTestId('badge-present')).toHaveText('Présentez le badge de Apprenti Test');
  await capture(page, '19c_badge_enrol');
});

// Proposed capture (runbook): the manager has just clocked the presence-only employee in.
test('19a_admin_presence - le Gérant pointe un employé', async ({ page }) => {
  await mockTauriIPC(page, { login: () => GERANT, list_users: () => [...STAFF, CUISINIER] });
  await loginAndGoto(page, '/settings');
  await page.locator('.worker-row', { hasText: 'Cuisinier Test' }).getByTestId('presence-btn').click();
  await expect(page.locator('.toast', { hasText: 'Arrivée enregistrée · Cuisinier Test' })).toBeVisible();
  await expect(page.locator('.worker-row', { hasText: 'Cuisinier Test' }).getByTestId('presence-btn')).toHaveText('Départ');
  await capture(page, '19a_admin_presence');
});

// Proposed capture (runbook): a cashier is at the till and a colleague
// presents their badge; the till shows the arrival and keeps the cashier.
test('03b_badge_toast - pointage par badge pendant une vente', async ({ page }) => {
  await badgeInit(page, {
    settings: BADGE_SETTINGS,
    taps: [{ kind: 'clock_in', insz: APPRENTI.insz, name: 'Apprenti Test', at: '2026-10-17T12:02:00+02:00' }],
  });
  await mockTauriIPC(page, { login: () => SERVEUR, list_users: () => STAFF, list_tables: () => TABLES_BEFORE });
  await page.goto('/');
  await page.waitForTimeout(200);
  await tapUserTile(page, 'Elisa');
  await page.waitForURL(/tables/);
  await scanBadge(page, '04A1B2C3');
  await expect(page.locator('.toast', { hasText: 'Arrivée · Apprenti Test' })).toHaveText('Arrivée · Apprenti Test · 12:02');
  await capture(page, '03b_badge_toast');
});

// Proposed capture (runbook): the login screen with the badge reader on.
// A separate file: 01_login stays as it is (reader off, the default).
test('01b_login_badge - invitation à présenter son badge', async ({ page }) => {
  await badgeInit(page, { settings: BADGE_SETTINGS });
  await mockTauriIPC(page, { list_users: () => STAFF });
  await page.goto('/');
  await expect(page.getByTestId('badge-login-hint')).toHaveText('Ou présentez votre badge');
  await expect(page.locator('.user-btn')).toHaveCount(3);
  await capture(page, '01b_login_badge');
});

test.describe('Section 2 - messages du FDM (§V)', () => {
  const refused = (error: string) => ({
    success: false, ticket: null, error, fdm_connected: false, fiscal_event_id: null, delivery: null,
  });

  // list_cart_lines is left at its default ([DEFAULT_ORDER_LINE]),
  // same as fdm-errors.spec.ts: the table opens with one demo line
  // already in the cart, which is all a payment-refusal capture needs.
  async function openCheckoutRefused(page: Page, checkoutError: string) {
    await mockTauriIPC(page, {
      login: () => GERANT,
      list_users: () => STAFF,
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM],
      checkout: () => refused(checkoutError),
    });
    await page.goto('/');
    await tapUserTile(page, 'Gérant Test');
    await page.waitForURL(/tables/);
    await page.locator('.table-card').first().click();
    await page.waitForURL(/order/);
    await page.locator('.item-btn').first().click();
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await page.locator('.card-btn').click();
    await page.locator('.card-confirm').click();
  }

  async function openOrderWithFdmWarning(page: Page, code: string, showPos: string) {
    await mockTauriIPC(page, {
      login: () => GERANT,
      list_users: () => STAFF,
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM],
      // The health the engine reports for such a warning.
      ...fdmMocks(
        showPos === 'NEVER'
          ? fdmHealth()
          : fdmHealth({ severity: 'warning', code, mandatory: showPos === 'MANDATORY' }),
      ),
    });
    await page.goto('/');
    await tapUserTile(page, 'Gérant Test');
    await page.waitForURL(/tables/);
    await page.locator('.table-card').first().click();
    await page.waitForURL(/order/);
  }

  // Group A: a mandatory or optional code that also refuses the sale
  // outright (the runbook's "en plus: tenter un encaissement : il est
  // refusé"). CheckoutView has no header, so this is the one screen
  // that actually shows the refusal - same message text, same code,
  // as the pastille "Caisse" would (fdmMessages.ts is the single
  // source, see errors.spec.ts / fdm-errors.spec.ts).
  const GROUP_A: { num: string; file: string; code: string; message: string }[] = [
    { num: '20', file: '20_buffer_full', code: 'BUFFER_FULL', message: 'La boîte noire est pleine. Contactez votre technicien.' },
    { num: '21', file: '21_fdm_locked', code: 'FDM_LOCKED', message: 'La boîte noire est verrouillée. Contactez votre technicien.' },
    { num: '22', file: '22_fdm_not_operational', code: 'FDM_NOT_OPERATIONAL', message: "La boîte noire ne fonctionne pas. Impossible d'enregistrer les ventes." },
    { num: '23', file: '23_unknown_pos', code: 'UNKNOWN_POS', message: "Cette caisse n'est pas reconnue par la boîte noire." },
    { num: '27', file: '27_unauthorized', code: 'UNAUTHORIZED', message: "La caisse n'est pas autorisée à parler à la boîte noire." },
    { num: '28', file: '28_invalid_request', code: 'INVALID_REQUEST', message: 'Erreur de communication avec la boîte noire. Contactez votre technicien.' },
  ];

  // Group A2: the codes that leave the outcome unknown (the FDM may have
  // signed the sale). The engine keeps the request to send it again
  // (commands/signing.rs) and the error carries the FDM's code: its
  // message first, then the retry note. The text mocked here is the one
  // the engine returns (spf_e2e_fdm_failures.rs, ticket 1).
  const RETRY_NOTE =
    "La vente n'est pas encore enregistrée : le prochain essai envoie à nouveau la même demande à la boîte noire.";
  const GROUP_A2: { num: string; file: string; code: string; message: string }[] = [
    { num: '29', file: '29_internal_error', code: 'INTERNAL_ERROR', message: 'Erreur technique de la boîte noire. Contactez votre technicien.' },
    { num: '30', file: '30_undefined_error', code: 'UNDEFINED_ERROR', message: 'Erreur indéfinie de la boîte noire. Contactez votre technicien.' },
    { num: '31', file: '31_undefined_other', code: 'UNDEFINED_OTHER', message: 'Erreur de la boîte noire. Contactez votre technicien.' },
  ];

  for (const { num, file, code, message } of GROUP_A2) {
    test(`${num} - ${code} : encaissement en attente de renvoi`, async ({ page }) => {
      await openCheckoutRefused(
        page,
        `FDM error: FDM_ANSWER_PENDING|label=N|ticket=1|fdm=${code}|cause=GraphQL error: ${code}: Error simulated by the mock`,
      );
      await expect(page.locator('.checkout .error-msg')).toHaveText(`${message} ${RETRY_NOTE}`);
      await expect(page.locator('.card-btn')).toBeVisible();
      await capture(page, file);
    });
  }

  for (const { num, file, code, message } of GROUP_A) {
    test(`${num} - ${code} : encaissement refusé`, async ({ page }) => {
      await openCheckoutRefused(page, `FDM error: GraphQL error: ${code}`);
      await expect(page.locator('.checkout .error-msg')).toHaveText(message);
      await expect(page.locator('.card-btn')).toBeVisible();
      await capture(page, file);
    });
  }

  // Group B: header-only - the pastille "Caisse" changes color and
  // shows the message; no checkout is attempted for these.
  const GROUP_B: { num: string; file: string; code: string; showPos: string; message: string }[] = [
    { num: '24', file: '24_buffer_near_full', code: 'BUFFER_NEAR_FULL', showPos: 'MANDATORY', message: 'La boîte noire est presque pleine. Programmez une intervention.' },
    { num: '25', file: '25_client_cert_expired', code: 'CLIENT_CERT_EXPIRED', showPos: 'MANDATORY', message: 'Le certificat de la boîte noire a expiré. Contactez votre technicien.' },
    { num: '26', file: '26_client_cert_near_expiration', code: 'CLIENT_CERT_NEAR_EXPIRATION', showPos: 'RECOMMENDED', message: 'Le certificat de la boîte noire expire bientôt.' },
    { num: '32', file: '32_rtc_sync_failed', code: 'RTC_SYNC_FAILED', showPos: 'RECOMMENDED', message: "L'horloge de la boîte noire n'a pas pu se synchroniser." },
    { num: '33', file: '33_update_urls_failed', code: 'UPDATE_URLS_FAILED', showPos: 'RECOMMENDED', message: 'Mise à jour des URLs de la boîte noire échouée.' },
    { num: '34', file: '34_update_trust_cert_failed', code: 'UPDATE_TRUST_CERT_FAILED', showPos: 'RECOMMENDED', message: 'Mise à jour des certificats de confiance échouée.' },
    { num: '35', file: '35_update_task_list_failed', code: 'UPDATE_TASK_LIST_FAILED', showPos: 'RECOMMENDED', message: 'Téléchargement des tâches FDM échoué.' },
    { num: '36', file: '36_task_feedback_failed', code: 'TASK_FEEDBACK_FAILED', showPos: 'RECOMMENDED', message: "La boîte noire n'a pas pu signaler le résultat d'une tâche." },
    { num: '37', file: '37_nop_failed', code: 'NOP_FAILED', showPos: 'RECOMMENDED', message: 'Communication intermédiaire avec la boîte noire échouée.' },
    { num: '38', file: '38_server_cert_resolve_failed', code: 'SERVER_CERT_RESOLVE_FAILED', showPos: 'RECOMMENDED', message: 'Impossible de résoudre le certificat du serveur FDM.' },
    { num: '39', file: '39_transaction_upload_failed', code: 'TRANSACTION_UPLOAD_FAILED', showPos: 'RECOMMENDED', message: 'La boîte noire échoue à transmettre les transactions au SPF Finances.' },
    { num: '40', file: '40_initialization_failed', code: 'INITIALIZATION_FAILED', showPos: 'RECOMMENDED', message: "L'initialisation de la boîte noire a échoué." },
  ];

  for (const { num, file, code, showPos, message } of GROUP_B) {
    test(`${num} - ${code} : pastille "Caisse"`, async ({ page }) => {
      await openOrderWithFdmWarning(page, code, showPos);
      await expect(page.locator('.fdm-alert .msg')).toHaveText(message);
      await expect(page.locator('.fdm.warn')).toBeVisible();
      await capture(page, file);
    });
  }

  test('41 - FDM injoignable : encaissement refusé, câble réseau', async ({ page }) => {
    await openCheckoutRefused(page, 'FDM not connected - cannot complete transaction');
    await expect(page.locator('.checkout .error-msg')).toHaveText(
      'Connexion perdue avec la boîte noire. Vérifiez le câble réseau.',
    );
    await expect(page.locator('.card-btn')).toBeVisible();
    await capture(page, '41_fdm_unreachable');
  });
});

test.describe('Écrans additionnels (42-44)', () => {
  test('42 - réponse FDM incohérente : vente non enregistrée', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => GERANT,
      list_users: () => STAFF,
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM],
      checkout: () => ({
        success: false,
        ticket: null,
        error:
          'FDM error: FDM_ANSWER_PENDING|label=N|ticket=42|fdm=FDM_RESPONSE_MISMATCH|cause=FDM_RESPONSE_MISMATCH: posFiscalTicketNo attendu 42, reçu 41',
        fdm_connected: false,
        fiscal_event_id: null,
        delivery: null,
      }),
    });
    await page.goto('/');
    await tapUserTile(page, 'Gérant Test');
    await page.waitForURL(/tables/);
    await page.locator('.table-card').first().click();
    await page.waitForURL(/order/);
    await page.locator('.item-btn').first().click();
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await page.locator('.card-btn').click();
    await page.locator('.card-confirm').click();
    await expect(page.locator('.checkout .error-msg')).toHaveText(
      "Réponse de la boîte noire incohérente. Vente non enregistrée. Contactez votre technicien. La vente n'est pas encore enregistrée : le prochain essai envoie à nouveau la même demande à la boîte noire.",
    );
    await capture(page, '42_fdm_response_mismatch');
  });

  test('43 - premier login : un gérant sans PIN en choisit un', async ({ page }) => {
    const NEW_GERANT = { ...GERANT, has_pin: false };
    await mockTauriIPC(page, { list_users: () => [NEW_GERANT], login: () => NEW_GERANT });
    await page.goto('/');
    await page.locator('.user-btn', { hasText: 'Gérant Test' }).click();
    await expect(page.locator('.pin-dialog .title')).toHaveText('Choisissez votre PIN');
    await expect(page.locator('.pin-dialog .who')).toHaveText('Gérant Test');
    await expect(page.locator('.pin-dialog .hint')).toBeVisible();
    await capture(page, '43_first_login_pin');
  });

  test('44 - assistance à distance en cours', async ({ page }) => {
    const RUNNING = { available: true, running: true, startedBy: 'Gérant Test', startedAt: '2026-09-26T08:30:00Z' };
    await mockTauriIPC(page, { login: () => GERANT, list_users: () => STAFF, remote_support_status: () => RUNNING });
    await loginAndGoto(page, '/settings');
    // The section is below the fold - scroll it into view so the
    // capture shows the running session, not just the bottom banner.
    await page.locator('.support-section').scrollIntoViewIfNeeded();
    await expect(page.locator('.support-section .support-state.running')).toBeVisible();
    await expect(page.locator('.support-banner')).toContainText('Gérant Test');
    await capture(page, '44_remote_support');
  });
});

test.describe('Écrans additionnels (45-49, 09g)', () => {
  test('45 - assistance à distance en cours, vue depuis l\'écran de connexion', async ({ page }) => {
    // Logged out, only whether a session runs is known, not who started
    // it; anyone may stop it from the banner.
    await mockTauriIPC(page, {
      list_users: () => STAFF,
      remote_support_running: () => ({ running: true, startedAt: '2026-09-26T08:30:00Z' }),
    });
    await page.goto('/');
    const banner = page.locator('.support-banner');
    await expect(banner).toContainText('Assistance à distance en cours', { timeout: 8000 });
    await expect(banner.getByRole('button', { name: 'Arrêter la session' })).toBeVisible();
    await expect(page.locator('.user-btn')).toHaveCount(3);
    await capture(page, '45_login_remote_support_banner');
  });

  test('46 - PIN verrouillé après des essais incorrects : délai d\'attente', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => STAFF });
    await page.goto('/');
    await overrideIPC(page, 'login', { __reject: 'pin_locked|seconds=30' });
    await page.locator('.user-btn', { hasText: 'Gérant Test' }).click();
    for (const d of '9999') await page.locator('.nk', { hasText: d }).click();
    await expect(page.locator('.error-msg')).toHaveText('Trop de PIN incorrects : réessayez dans 30 s.');
    await capture(page, '46_pin_locked');
  });

  test('47 - un gérant sans PIN ne peut pas le choisir seul : un autre gérant le fixe', async ({ page }) => {
    // Another manager already has a PIN, so this tile may not claim
    // one (first_pin_needs_manager).
    const NO_PIN_GERANT = { ...GERANT, has_pin: false };
    await mockTauriIPC(page, { list_users: () => [NO_PIN_GERANT, SERVEUR] });
    await page.goto('/');
    await overrideIPC(page, 'login_with_new_pin', { __reject: 'first_pin_needs_manager' });
    await page.locator('.user-btn', { hasText: 'Gérant Test' }).click();
    const dialog = page.locator('.pin-dialog');
    await expect(dialog.locator('.title')).toHaveText('Choisissez votre PIN');
    for (const round of [0, 1]) {
      void round;
      for (const d of '4321') await dialog.getByRole('button', { name: d, exact: true }).click();
    }
    await expect(dialog.locator('.error-msg')).toHaveText(
      'Un gérant doit choisir le PIN de ce compte dans Paramètres.',
    );
    await capture(page, '47_first_pin_needs_manager');
  });

  test('48 - FDM_ANSWER_PENDING : réponse de la boîte noire attendue, la vente est renvoyée', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => GERANT,
      list_users: () => STAFF,
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM],
      checkout: () => ({
        success: false,
        ticket: null,
        error: 'FDM error: FDM_ANSWER_PENDING|label=N|ticket=42|cause=timeout',
        fdm_connected: false,
        fiscal_event_id: null,
        delivery: null,
      }),
    });
    await page.goto('/');
    await tapUserTile(page, 'Gérant Test');
    await page.waitForURL(/tables/);
    await page.locator('.table-card').first().click();
    await page.waitForURL(/order/);
    await page.locator('.item-btn').first().click();
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await page.locator('.card-btn').click();
    await page.locator('.card-confirm').click();
    await expect(page.locator('.checkout .error-msg')).toHaveText(
      "La caisse attend la réponse de la boîte noire pour l'opération précédente. Vérifiez la connexion et réessayez.",
    );
    await capture(page, '48_fdm_answer_pending');
  });

  test('49 - remboursement par ligne : un produit vendu à deux prix', async ({ page }) => {
    const now = new Date().toISOString();
    const REFUNDABLE = [{ fiscalEventId: 9, ticketNumber: 9, orderId: TABLE4_ORDER_ID, tableNumber: 4, totalCents: 2100, createdAt: now }];
    // The same product on two lines: one at the menu price, one after a
    // price change. Each line is refunded at the price it was sold at.
    const LINES = [
      { id: 1, order_id: TABLE4_ORDER_ID, menu_item_id: 1, quantity: 1, unit_price_cents: 1200, vat_code: 'B', name: 'Maki saumon x8', modifiers_json: null, created_at: now, department_id: '1', department_name: 'Sushis' },
      { id: 2, order_id: TABLE4_ORDER_ID, menu_item_id: 1, quantity: 1, unit_price_cents: 900, vat_code: 'B', name: 'Maki saumon x8', modifiers_json: null, created_at: now, department_id: '1', department_name: 'Sushis' },
    ];
    await mockTauriIPC(page, {
      login: () => GERANT,
      list_users: () => STAFF,
      list_refundable_events: () => REFUNDABLE,
      list_order_lines: () => LINES,
    });
    await loginAndGoto(page, '/reports');
    await page.getByRole('tab', { name: 'Tickets' }).click();
    await page.locator('.ticket-row .refund-btn').first().click();
    const lines = page.locator('.refund-line');
    await expect(lines).toHaveCount(2);
    await expect(lines.nth(0)).toContainText('12,00');
    await expect(lines.nth(1)).toContainText('9,00');
    await lines.nth(1).click();
    await expect(lines.nth(1)).toHaveClass(/selected/);
    await expect(lines.nth(0)).not.toHaveClass(/selected/);
    await expect(page.locator('.refund-confirm-btn')).toContainText('9,00');
    await capture(page, '49_refund_per_line_two_prices');
  });

  test('09g - fin d\'encaissement : le ticket papier doit encore être imprimé', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => GERANT,
      list_users: () => STAFF,
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM],
      checkout: () => ({
        success: true, ticket: null, error: null, fdm_connected: true, fiscal_event_id: 7,
        delivery: { fiscalEventId: 7, deliveryId: null, medium: 'PAPER', email: 'not_requested', recipientHint: null, paperPrinted: false, needsPaper: true },
      }),
    });
    await page.goto('/');
    await tapUserTile(page, 'Gérant Test');
    await page.waitForURL(/tables/);
    await page.locator('.table-card').first().click();
    await page.waitForURL(/order/);
    await page.locator('.item-btn').first().click();
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await page.locator('.cash-btn').click();
    await page.locator('.shortcut.exact').click();
    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Imprimer le ticket' })).toBeVisible();
    await capture(page, '09g_checkout_done_print_ticket');
  });
});

// ── Terminal de paiement lié (EFT) ─────────────────────────────────
// Captures 01a, 03a, 03b, 09h-09l, 15a. The terminal is scripted
// through eftScript / settingsWith / check_payment_terminal /
// list_unsettled_card_payments (mock-ipc.ts); nothing is charged.

const TERMINAL_OK = { configured: true, kind: 'worldline_terminal_api', connected: true, busy: false, detail: null };
const EFT_LINE = { ...DEFAULT_ORDER_LINE, id: 1, unit_price_cents: 1200 };

/** The checkout of a 12,00 EUR order with a linked terminal. */
async function eftCheckout(page: Page, script: Parameters<typeof eftScript>[1]) {
  await eftScript(page, script);
  await mockTauriIPC(page, {
    login: () => GERANT,
    list_users: () => STAFF,
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
    list_cart_lines: () => [EFT_LINE],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    get_settings: settingsWith({ paymentTerminal: 'worldline_terminal_api' }),
    check_payment_terminal: () => TERMINAL_OK,
  });
  await page.goto('/');
  await tapUserTile(page, 'Gérant Test');
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
  await page.locator('.checkout-btn').click();
  await page.waitForURL(/checkout/);
}

test.describe('Terminal de paiement lié (EFT)', () => {
  test('01a - démarrage : ligne « Terminal de paiement »', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => STAFF, check_payment_terminal: () => TERMINAL_OK, check_printer: () => true });
    await page.goto('/');
    const row = page.locator('.check', { hasText: 'Terminal de paiement' });
    await expect(row).toHaveClass(/ok/);
    await expect(row.locator('.check-text')).toHaveText('connecté');
    await capture(page, '01a_login_terminal_row');
  });

  test('03a-03b - voyant TERMINAL, tables et « Carte encaissée »', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => GERANT,
      list_users: () => STAFF,
      list_tables: () => TABLES_AFTER,
      check_payment_terminal: () => TERMINAL_OK,
      check_printer: () => true,
      list_unsettled_card_payments: () => [
        { payment_id: 1, order_id: TABLE4_ORDER_ID, order_type: 'dine_in', table_number: 4, amount_cents: FILLED_TOTAL_CENTS, brand: 'Bancontact', created_at: '2026-10-13 12:00:00' },
      ],
    });
    await page.goto('/');
    await tapUserTile(page, 'Gérant Test');
    await page.waitForURL(/tables/);
    await expect(page.getByTestId('light-terminal')).toHaveClass(/ok/);
    await expect(page.getByTestId('light-terminal')).toContainText('Terminal');
    await expect(page.locator('.table-card').nth(3).getByTestId('card-charged')).toHaveText('Carte encaissée');
    await capture(page, '03a_tables_terminal_card_charged');
  });

  test('09h - encaissement : Terminal ou Saisie manuelle', async ({ page }) => {
    await eftCheckout(page, { pay: [{ status: 'approved' }] });
    await page.locator('.card-btn').click();
    await expect(page.getByTestId('eft-mode')).toBeVisible();
    await expect(page.locator('.card-hint')).toHaveText('Le montant est envoyé au terminal à la validation');
    await capture(page, '09h_checkout_eft_mode');
  });

  test('09i - « Paiement sur le terminal »', async ({ page }) => {
    await eftCheckout(page, { pay: [{ status: 'approved', hold: true }] });
    await page.locator('.card-btn').click();
    await page.locator('.validate-btn').click();
    await expect(page.getByTestId('eft-title')).toHaveText('Paiement sur le terminal');
    await expect(page.getByTestId('eft-overlay')).toContainText('Le client présente sa carte sur le terminal');
    await capture(page, '09i_eft_overlay');
  });

  test('09j - paiement refusé', async ({ page }) => {
    await eftCheckout(page, { pay: [{ status: 'declined', detail: 'Carte refusée par la banque' }] });
    await page.locator('.card-btn').click();
    await page.locator('.validate-btn').click();
    await expect(page.getByTestId('eft-title')).toHaveText('Paiement refusé');
    await expect(page.getByTestId('eft-detail')).toHaveText('Carte refusée par la banque');
    await capture(page, '09j_eft_declined');
  });

  test('09l - carte de chèques-repas acceptée : Chèques-repas · Edenred', async ({ page }) => {
    await eftCheckout(page, { pay: [{ status: 'approved', brand: 'Edenred' }] });
    await page.locator('.card-btn').click();
    for (const d of '5') await page.locator('.nk', { hasText: d }).first().click();
    await page.locator('.validate-btn').click();
    await expect(page.locator('.paid-part')).toContainText('Chèques-repas');
    await expect(page.locator('.paid-part')).toContainText('Edenred');
    await capture(page, '09l_eft_meal_voucher');
  });

  test('09k - réponse inconnue : Vérifier', async ({ page }) => {
    await eftCheckout(page, { pay: [{ status: 'unknown', detail: 'no answer' }], check: [{ status: 'approved' }] });
    await page.locator('.card-btn').click();
    await page.locator('.validate-btn').click();
    await expect(page.getByTestId('eft-title')).toHaveText('Réponse du terminal inconnue');
    await expect(page.getByTestId('eft-check')).toBeVisible();
    await capture(page, '09k_eft_unknown');
  });

  test('15a - Z : cartes encaissées pas encore enregistrées', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => GERANT,
      list_users: () => STAFF,
      list_unsettled_card_payments: () => [
        { payment_id: 1, order_id: 42, order_type: 'dine_in', table_number: 4, amount_cents: 1700, brand: 'Bancontact', created_at: '2026-10-13 12:00:00' },
        { payment_id: 2, order_id: 43, order_type: 'takeaway', table_number: null, amount_cents: 950, brand: null, created_at: '2026-10-13 12:05:00' },
      ],
    });
    await loginAndGoto(page, '/reports');
    await expect(page.getByTestId('z-unsettled')).toContainText('Paiements par carte encaissés mais pas encore enregistrés dans une vente :');
    await capture(page, '15a_reports_z_unsettled_cards');
  });
});
