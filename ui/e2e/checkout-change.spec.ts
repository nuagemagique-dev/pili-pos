import { test, expect, type Page } from '@playwright/test';
import {
  mockTauriIPC,
  overrideIPC,
  tapUserTile,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc';

// The change the done screen tells the operator to hand back. It used
// to be computed after the cart was cleared, against a zero total, so
// every cash sale showed the whole amount received as change.

const SALE = {
  success: true,
  ticket: {
    restaurant_name: 'Restaurant Exemple',
    restaurant_address: 'Liège',
    restaurant_vat: 'BE0499999960',
    pos_id: 'CFOD0061234567',
    terminal_id: 'POS1',
    pos_datetime: new Date().toISOString(),
    ticket_number: 1,
    event_label: 'N',
    is_training: false,
    lines: [],
    subtotal: 24,
    total: 24,
    vat_breakdown: [],
    payments: [],
    rounding: null,
    fdm_ref: null,
    short_signature: null,
    verification_url: null,
    footer: [],
    ticket_type_label: 'TICKET DE CAISSE T.V.A.',
    is_refund: false,
    lang: 'fr',
  },
  error: null,
  fdm_connected: true,
  fiscal_event_id: 1,
  delivery: null,
};

const REFUSED = {
  success: false, ticket: null, error: 'FDM error: GraphQL error: FDM_LOCKED',
  fdm_connected: false, fiscal_event_id: null, delivery: null,
};

// A 24,00 order (the mocked line plus the item tapped), on the payment step.
async function toPayment(page: Page, checkout: object = SALE) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
    list_order_lines: () => [DEFAULT_ORDER_LINE],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    checkout: () => checkout,
  });
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
  await page.locator('.item-btn').first().click();
  await page.locator('.checkout-btn').click();
  await page.waitForURL(/checkout/);
  await expect(page.locator('.total-value')).toHaveText('24,00');
}

async function payCash(page: Page, digits: string) {
  await page.locator('.cash-btn').click();
  for (const d of digits) await page.locator('.nk', { hasText: d }).first().click();
  await page.locator('.act-btn.primary', { hasText: 'Valider' }).click();
}

test('a 30 € payment on a 24 € sale gives 6 € back', async ({ page }) => {
  await toPayment(page);
  await payCash(page, '30');
  await expect(page.locator('.success-icon')).toBeVisible();
  await expect(page.locator('.receipt-change')).toContainText('6,00');
});

test('the exact amount gives no change line', async ({ page }) => {
  await toPayment(page);
  await page.locator('.cash-btn').click();
  await page.locator('.shortcut.exact').click();
  await expect(page.locator('.success-icon')).toBeVisible();
  await expect(page.locator('.receipt-change')).toHaveCount(0);
});

test('a card payment after a refused cash attempt gives no change', async ({ page }) => {
  await toPayment(page, REFUSED);
  await payCash(page, '50');
  await expect(page.locator('.checkout .error-msg')).toBeVisible();

  await overrideIPC(page, 'checkout', SALE);
  await page.locator('.card-btn').click();
  await page.locator('.card-confirm').click();
  await expect(page.locator('.success-icon')).toBeVisible();
  await expect(page.locator('.receipt-change')).toHaveCount(0);
});
