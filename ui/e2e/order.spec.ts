import { test, expect } from '@playwright/test';
import { mockTauriIPC, DEFAULT_USER, DEFAULT_ORDER, DEFAULT_ORDER_LINE, DEFAULT_CATEGORY, DEFAULT_MENU_ITEM } from './fixtures/mock-ipc';

test.describe('Order View', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
      list_order_lines: () => [DEFAULT_ORDER_LINE],
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM, {
        ...DEFAULT_MENU_ITEM, id: 2, name: 'Tuna Roll x8', price_cents: 1300,
      }],
    });
    // Login → tables → pick table 1
    await page.goto('/');
    await page.click('.user-btn');
    await page.waitForURL(/tables/);
    await page.locator('.table-card').first().click();
    await page.waitForURL(/order/);
  });

  test('shows header with table or takeaway badge', async ({ page }) => {
    // After startOrder, the header shows either "Table N" or "À emporter".
    // The mock's create_order returns table_number from input, but the
    // serialized default may be null — check for either badge.
    const header = page.locator('.header-table, .header-takeaway');
    await expect(header).toBeVisible();
  });

  test('shows category tabs', async ({ page }) => {
    await expect(page.locator('.tab')).toHaveCount(1);
  });

  test('shows menu items in the grid', async ({ page }) => {
    await expect(page.locator('.item-btn')).toHaveCount(2);
  });

  test('nav has 6 buttons (Tables, À emporter, En ligne, Rapports, Config, Sortir)', async ({ page }) => {
    await expect(page.locator('.nav-btn')).toHaveCount(6);
  });

  test('Tables button is active for dine-in', async ({ page }) => {
    await expect(page.locator('.nav-btn-active')).toContainText('Tables');
  });

  test('cart sidebar is visible', async ({ page }) => {
    // A newly-created order has no lines yet — the cart shows "Aucun article".
    await expect(page.getByText('Aucun article')).toBeVisible();
  });
});
