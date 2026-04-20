import { test, expect } from '@playwright/test';
import { mockTauriIPC, DEFAULT_USER, DEFAULT_ORDER } from './fixtures/mock-ipc';

test.describe('Tables View', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
    });
    // Login then navigate to tables
    await page.goto('/');
    await page.click('.user-btn');
    await page.waitForURL(/tables/);
  });

  test('renders 20 tables', async ({ page }) => {
    await expect(page.locator('.table-card')).toHaveCount(20);
  });

  test('free tables show "Libre"', async ({ page }) => {
    const first = page.locator('.table-card').first();
    await expect(first.locator('.table-free')).toContainText('Libre');
  });

  test('occupied table shows item count and total', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_tables: () => {
        const tables = [];
        tables.push({
          tableNumber: 1, orderId: 42, totalCents: 3550,
          lineCount: 3, status: 'occupied',
        });
        for (let i = 2; i <= 20; i++) {
          tables.push({
            tableNumber: i, orderId: null, totalCents: 0,
            lineCount: 0, status: 'free',
          });
        }
        return tables;
      },
    });
    await page.goto('/');
    await page.click('.user-btn');
    await page.waitForURL(/tables/);

    const occupied = page.locator('.table-card.occupied');
    await expect(occupied).toHaveCount(1);
    await expect(occupied.locator('.table-total')).toContainText('35.50');
  });

  test('tapping a free table navigates to order view', async ({ page }) => {
    await page.locator('.table-card').first().click();
    await expect(page).toHaveURL(/order/);
  });
});
