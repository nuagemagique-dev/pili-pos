import { test, expect } from '@playwright/test';
import { mockTauriIPC, DEFAULT_USER, DEFAULT_ORDER, DEFAULT_ORDER_LINE, DEFAULT_CATEGORY, DEFAULT_MENU_ITEM, tapUserTile } from './fixtures/mock-ipc';

test.describe('Order View', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
      // Like the backend: a new order has no lines.
      list_order_lines: () => [],
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM, {
        ...DEFAULT_MENU_ITEM, id: 2, name: 'Tuna Roll x8', price_cents: 1300,
      }, {
        ...DEFAULT_MENU_ITEM, id: 3, name: 'Plat du jour', price_cents: 0, open_price: true,
      }],
    });
    // Login → tables → pick table 1
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables/);
    await page.locator('.table-card').first().click();
    await page.waitForURL(/order/);
  });

  test('shows header with table or takeaway badge', async ({ page }) => {
    // After startOrder, the header shows either "Table N" or "À emporter".
    // The mock's create_order returns table_number from input, but the
    // serialized default may be null - check for either badge.
    const header = page.locator('.header-table, .header-takeaway');
    await expect(header).toBeVisible();
  });

  test('shows category tabs', async ({ page }) => {
    await expect(page.locator('.tab')).toHaveCount(1);
  });

  test('shows menu items in the grid', async ({ page }) => {
    await expect(page.locator('.item-btn')).toHaveCount(3);
  });

  test('free-price product asks for its price', async ({ page }) => {
    const item = page.locator('.item-btn', { hasText: 'Plat du jour' });
    await expect(item).toContainText('Prix libre');
    await item.click();

    const dialog = page.getByRole('dialog', { name: 'Prix libre' });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole('button', { name: 'Valider' })).toBeDisabled();
    for (const d of ['2', '0', '0', '0']) {
      await dialog.locator('.key', { hasText: new RegExp(`^${d}$`) }).click();
    }
    await expect(dialog.getByTestId('open-price-amount')).toHaveText('20,00 €');
    await dialog.getByRole('button', { name: 'Valider' }).click();
    await expect(dialog).toBeHidden();
  });

  test('nav has 6 buttons (Tables, À emporter, En ligne, Rapports, Config, Sortir)', async ({ page }) => {
    await expect(page.locator('.nav-btn')).toHaveCount(6);
  });

  test('Tables button is active for dine-in', async ({ page }) => {
    await expect(page.locator('.nav-btn-active')).toContainText('Tables');
  });

  test('cart sidebar is visible', async ({ page }) => {
    // A newly-created order has no lines yet - the cart shows "Aucun article".
    await expect(page.getByText('Aucun article')).toBeVisible();
  });
});

test.describe('Order View - busy table', () => {
  test('a table that already has an order shows its lines', async ({ page }) => {
    // The backend returns the table's open order instead of a new
    // one; the cart must show what was already ordered.
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 2400 }),
      list_order_lines: () => [{ ...DEFAULT_ORDER_LINE, quantity: 2 }],
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM],
    });
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables/);
    await page.locator('.table-card').first().click();
    await page.waitForURL(/order/);
    const cart = page.locator('.cart-lines');
    await expect(cart).toContainText(DEFAULT_ORDER_LINE.name);
    await expect(page.locator('.cart-empty')).toHaveCount(0);
  });
});
