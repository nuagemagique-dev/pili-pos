import { test, expect } from '@playwright/test';
import { mockTauriIPC, tapUserTile, DEFAULT_USER } from './fixtures/mock-ipc';

// In training mode the purple banner used to lie over the top of every
// screen: it hid the header and swallowed the taps meant for its
// buttons. It now takes its own row under the header (UX guide §9).

test.beforeEach(async ({ page }) => {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_settings: () => ({
      posId: 'CPOS0031234567',
      terminalId: 'POS-1-BAR',
      restaurantName: 'The FPS Finances Experience',
      fdmUrl: 'http://localhost:8081/graphql',
      printerAddress: null,
      trainingMode: true,
      language: 'fr',
      configProblems: [],
    }),
  });
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
});

test('the banner sits under the header, not over it', async ({ page }) => {
  const banner = await page.locator('.training-banner').boundingBox();
  const header = await page.locator('.tables-view .app-header').boundingBox();
  expect(banner).not.toBeNull();
  expect(header).not.toBeNull();
  expect(banner!.y).toBeGreaterThanOrEqual(header!.y + header!.height - 0.5);
  // The screen still fits: nothing pushed below the bottom edge.
  const view = await page.locator('.tables-view').boundingBox();
  const viewport = page.viewportSize()!;
  expect(view!.y + view!.height).toBeLessThanOrEqual(viewport.height + 0.5);
});

test('the header buttons still answer a real tap', async ({ page }) => {
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
  // The operator's name, in the header above the banner.
  await page.locator('.header-user').click();
  await expect(page.locator('.pin-display')).toBeVisible();
});

test('the watermark lets taps through', async ({ page }) => {
  await expect(page.locator('.training-watermark')).toHaveCSS('pointer-events', 'none');
  await page.locator('.table-card').nth(9).click();
  await page.waitForURL(/order/);
});
