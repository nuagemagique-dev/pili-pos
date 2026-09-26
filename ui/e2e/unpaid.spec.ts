import { test, expect, type Page } from '@playwright/test';
import {
  mockTauriIPC,
  ipcCalls,
  overrideIPC,
  tapUserTile,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc';

// The customer left without paying (Advanced Set UC4109): a manager
// closes the order from the cart, confirmed by their PIN. The backend
// signs the sale with no payment line (commands/unpaid.rs); the screen
// goes back to the tables.

const CASHIER = { ...DEFAULT_USER, insz: '84022899837', name: 'Elisa Serveuse', role: 'cashier' };

async function toOrder(page: Page, user = DEFAULT_USER) {
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
    list_order_lines: () => [DEFAULT_ORDER_LINE],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
  });
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
}

async function typePin(page: Page, pin: string) {
  const dialog = page.locator('.pin-dialog');
  for (const d of pin) await dialog.getByRole('button', { name: d, exact: true }).click();
}

test('[UC4109] a manager closes the table unpaid with their PIN', async ({ page }) => {
  await toOrder(page);
  await page.locator('.unpaid-btn').click();
  await expect(page.locator('.pin-dialog .hint')).toContainText('sans paiement');
  await typePin(page, '4321');

  await page.waitForURL(/tables/);
  await expect(page.locator('.toast', { hasText: 'Vente clôturée sans paiement' })).toBeVisible();
  expect(await ipcCalls(page, 'close_unpaid')).toEqual([
    { orderId: expect.any(Number), insz: DEFAULT_USER.insz, pin: '4321' },
  ]);
  expect(await ipcCalls(page, 'checkout')).toHaveLength(0);
});

test('a wrong PIN closes nothing and says so', async ({ page }) => {
  await toOrder(page);
  await overrideIPC(page, 'close_unpaid', {
    success: false, ticket: null, error: 'pin_wrong', fdm_connected: false,
    fiscal_event_id: null, delivery: null, kitchen_slip: null,
  });
  await page.locator('.unpaid-btn').click();
  await typePin(page, '0000');
  await expect(page.locator('.pin-dialog .error-msg')).toHaveText('PIN incorrect');
  await expect(page).toHaveURL(/order/);
});

test('back leaves the order open', async ({ page }) => {
  await toOrder(page);
  await page.locator('.unpaid-btn').click();
  await page.locator('.pin-dialog').getByRole('button', { name: 'Retour' }).click();
  await expect(page.locator('.pin-dialog')).toHaveCount(0);
  expect(await ipcCalls(page, 'close_unpaid')).toHaveLength(0);
  await expect(page).toHaveURL(/order/);
});

test('a cashier does not see the button', async ({ page }) => {
  await toOrder(page, CASHIER);
  await expect(page.locator('.checkout-btn')).toBeVisible();
  await expect(page.locator('.unpaid-btn')).toHaveCount(0);
});
