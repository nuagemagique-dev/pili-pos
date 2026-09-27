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
// All data is fictional and brand-neutral: "Restaurant Exemple" (the
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
  await page.screenshot({
    path: path.join(SCREENSHOTS_DIR, `${name}.png`),
    animations: 'disabled',
  });
}

// ── Test employees (PROFILS_UTILISATEURS_FR.md §1) ─────────────────
const GERANT = { insz: '94060242576', name: 'Gérant Test', role: 'admin' as const, active: true, has_pin: true };
const SERVEUR = { insz: '84022899837', name: 'Serveur Test', role: 'cashier' as const, active: true, has_pin: false };
const APPRENTI = { insz: '99012345667', name: 'Apprenti Test', role: 'waiter' as const, active: true, has_pin: false };
const STAFF = [GERANT, SERVEUR, APPRENTI];

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
    await expect(page.locator('.title')).toHaveText('Restaurant Exemple');
    await expect(page.locator('.user-btn')).toHaveCount(3);
    await expect(page.locator('.user-name')).toHaveText(['Gérant Test', 'Serveur Test', 'Apprenti Test']);
    await expect(page.locator('.user-role')).toHaveText(['Gérant', 'Serveur', 'Apprenti']);
    await expect(page.locator('.other-btn')).toBeVisible();
    await capture(page, '01_login');
  });

  test('02_login_pin - saisie du PIN, avant le 4e chiffre', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => STAFF, login: () => GERANT });
    await page.goto('/');
    await page.locator('.user-btn', { hasText: 'Gérant Test' }).click();
    await expect(page.locator('.pin-title')).toHaveText('Saisir le PIN');
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
      // A healthy FDM, not the suite-wide default (disconnected) - this
      // flow depicts a normal working sale, not an FDM incident.
      check_fdm_status: () => ({ connected: true }),
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
          restaurant_name: 'Restaurant Exemple',
          restaurant_address: 'Liège',
          restaurant_vat: 'BE0499999960',
          pos_id: 'CFOD0061234567',
          terminal_id: 'POS1',
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
    await expect(page.locator('.total-amount')).toHaveText('40,00');
    await capture(page, '05_order_filled');

    // "Tables": registers the P event, sans impression. Table 4 now
    // shows as occupied - swap the mocks a real backend would have
    // updated by itself.
    await overrideIPC(page, 'list_tables', TABLES_AFTER);
    await overrideIPC(page, 'list_cart_lines', FILLED_LINES);
    await page.locator('.nav-btn', { hasText: 'Tables' }).click();
    await page.waitForURL(/tables/);
    await expect.poll(() => ipcCalls(page, 'sign_table_order')).toHaveLength(1);
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
    // A healthy FDM, not the suite-wide default (disconnected) - this
    // flow depicts a normal working sale, not an FDM incident.
    check_fdm_status: () => ({ connected: true }),
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
    list_cart_lines: () => LINES,
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
  await expect(page.locator('.toggle')).toContainText('ACTIF');
  // "Retour" sits right under the banner: a real tap, which the banner
  // used to swallow (training-banner.spec.ts). No order is open, so
  // the order screen hands over to the tables.
  await page.locator('.settings .nav-btn', { hasText: 'Retour' }).click();
  await page.waitForURL(/tables/);
  await expect(page.locator('.training-banner')).toHaveText('MODE FORMATION : les transactions ne sont PAS enregistrées');
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
  await tapUserTile(page, 'Serveur Test');
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
  await page.locator('.item-btn').first().click();
  await page.locator('.nav-logout').click();
  await expect(page.locator('.user-btn')).toHaveCount(3);
  await capture(page, '18_logout');
});

test('19_admin_users - employés et formulaire d\'ajout', async ({ page }) => {
  await mockTauriIPC(page, { login: () => GERANT, list_users: () => STAFF });
  await loginAndGoto(page, '/settings');
  await expect(page.locator('.worker-row')).toHaveCount(3);
  await expect(page.locator('.worker-row', { hasText: 'Gérant Test' }).locator('.worker-meta')).toContainText('Gérant');
  await expect(page.locator('.role-btn')).toHaveCount(3);
  await capture(page, '19_admin_users');
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
      check_fdm_status: () => ({
        initialized: true,
        errors: [],
        warnings: [{ message: code, extensions: { category: 'FDM', code, showPos } }],
      }),
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
    { num: '29', file: '29_internal_error', code: 'INTERNAL_ERROR', message: 'Erreur technique de la boîte noire. Contactez votre technicien.' },
    { num: '30', file: '30_undefined_error', code: 'UNDEFINED_ERROR', message: 'Erreur indéfinie de la boîte noire. Contactez votre technicien.' },
  ];

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
    { num: '31', file: '31_undefined_other', code: 'UNDEFINED_OTHER', showPos: 'RECOMMENDED', message: 'Erreur de la boîte noire. Contactez votre technicien.' },
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
        error: 'FDM error: FDM_RESPONSE_MISMATCH: posFiscalTicketNo attendu 42, reçu 41',
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
      'Réponse de la boîte noire incohérente. Vente non enregistrée. Contactez votre technicien.',
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
