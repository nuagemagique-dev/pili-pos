/**
 * Layer 3: Integration tests against the REAL Tauri binary.
 *
 * These run in CI only (see .github/workflows/test.yml). The POS
 * binary is started with WEBKIT_INSPECTOR_SERVER so Playwright can
 * connect via CDP and interact with the live webview — no IPC mock.
 *
 * Prerequisites:
 *   - The POS binary is running (cargo tauri build + xvfb-run)
 *   - A test DB is seeded (e2e/fixtures/seed.sql)
 *   - The healthcheck on :9999 returns 200
 *   - INTEGRATION_CDP_URL env var is set
 */
import { test, expect } from '@playwright/test';

test.describe('Integration — real binary', () => {
  test.skip(!process.env.INTEGRATION_CDP_URL, 'Requires real POS binary (CI only)');

  test('healthcheck returns ok', async ({ request }) => {
    const resp = await request.get('http://127.0.0.1:9999/health');
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.status).toBe('ok');
  });

  test('login screen renders with seeded user', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.user-btn')).toBeVisible();
    await expect(page.locator('.user-name')).toContainText('CI Robot');
  });

  test('tables view renders after login', async ({ page }) => {
    await page.goto('/');
    await page.click('.user-btn');
    await expect(page).toHaveURL(/tables/);
    await expect(page.locator('.table-card')).toHaveCount(20);
  });

  test('menu items are loaded from seed', async ({ page }) => {
    await page.goto('/');
    await page.click('.user-btn');
    await page.waitForURL(/tables/);
    await page.locator('.table-card').first().click();
    await page.waitForURL(/order/);
    // The seeded DB has 3 menu items across 2 categories.
    await expect(page.locator('.item-btn')).toHaveCount(3);
  });
});
