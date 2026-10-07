import { test, expect } from '@playwright/test';
import { mockTauriIPC, DEFAULT_USER, DEFAULT_ORDER, DEFAULT_ORDER_LINE, DEFAULT_CATEGORY, DEFAULT_MENU_ITEM, tapUserTile } from './fixtures/mock-ipc';

// PROFILS_UTILISATEURS_FR.md §2: the backend refuses what a role may
// not do; the UI does not offer it.
async function openTableAs(page: import('@playwright/test').Page, role: string) {
  const user = { ...DEFAULT_USER, role };
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
    list_cart_lines: () => [DEFAULT_ORDER_LINE],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
  });
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
}

test.describe('What each role is offered', () => {
  test('an apprentice orders but neither bills, takes payment nor manages', async ({ page }) => {
    await openTableAs(page, 'waiter');
    await expect(page.locator('.cart-lines')).toContainText(DEFAULT_ORDER_LINE.name);
    await expect(page.locator('.checkout-btn')).toHaveCount(0);
    await expect(page.locator('.pre-bill-btn')).toHaveCount(0);
    await expect(page.locator('.nav-btn', { hasText: 'Rapports' })).toHaveCount(0);
    await expect(page.locator('.nav-btn', { hasText: 'Config' })).toHaveCount(0);
  });

  test('a server bills and takes payment but does not manage', async ({ page }) => {
    await openTableAs(page, 'cashier');
    await expect(page.locator('.checkout-btn')).toBeVisible();
    await expect(page.locator('.pre-bill-btn')).toBeVisible();
    await expect(page.locator('.nav-btn', { hasText: 'Rapports' })).toHaveCount(0);
    await expect(page.locator('.nav-btn', { hasText: 'Config' })).toHaveCount(0);
  });

  test('a manager is offered everything', async ({ page }) => {
    await openTableAs(page, 'admin');
    await expect(page.locator('.checkout-btn')).toBeVisible();
    await expect(page.locator('.pre-bill-btn')).toBeVisible();
    await expect(page.locator('.nav-btn', { hasText: 'Rapports' })).toBeVisible();
    await expect(page.locator('.nav-btn', { hasText: 'Config' })).toBeVisible();
  });

  test('locking the screen ends the backend session and keeps the shift', async ({ page }) => {
    await openTableAs(page, 'cashier');
    await page.locator('.nav-logout').click();
    await expect.poll(async () =>
      page.evaluate(() => ((window as any).__ipcCalls as Array<{ cmd: string }>)
        .map((c) => c.cmd)
        .filter((c) => c === 'lock_session' || c === 'work_out' || c === 'logout')),
    ).toEqual(['lock_session']);
  });
});
