import { test, expect, type Page } from '@playwright/test';
import { mockTauriIPC, tapUserTile, ipcCalls } from './fixtures/mock-ipc';

// The FDM indicator polls the status through one poller in the store:
// every 30 s while the FDM answers, backing off (60 s, 2 min, ... up to
// 5 min) while it does not, so a down or remote FDM is not hammered.

const statusCalls = async (page: Page) => (await ipcCalls(page, 'check_fdm_status')).length;

async function openTable(page: Page) {
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
  await expect(page.locator('.fdm')).toBeVisible();
}

/** On a table's order screen (or, with `openOrder` false, on the
 * Tables screen: an FDM that does not answer veils the sales screens),
 * once its first status check is done: the number of checks so far. */
async function onOrderScreen(page: Page, openOrder = true): Promise<number> {
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  if (openOrder) await openTable(page);
  else await expect(page.locator('.fdm')).toBeVisible();
  await expect.poll(() => statusCalls(page)).toBeGreaterThan(0);
  return statusCalls(page);
}

test('the FDM status is polled every 30 s while the FDM answers', async ({ page }) => {
  await page.clock.install();
  await mockTauriIPC(page, {
    check_fdm_status: () => ({ initialized: true, errors: [], warnings: [] }),
  });
  const n = await onOrderScreen(page);
  await page.clock.runFor(30_000);
  await expect.poll(() => statusCalls(page)).toBe(n + 1);
  await page.clock.runFor(30_000);
  await expect.poll(() => statusCalls(page)).toBe(n + 2);
});

test('an FDM that does not answer is polled less and less often', async ({ page }) => {
  await page.clock.install();
  await mockTauriIPC(page, {
    check_fdm_status: () => ({ connected: false, error: 'FDM not connected' }),
  });
  const n = await onOrderScreen(page, false);
  // First failure: next check in 60 s, not 30 s.
  await page.clock.runFor(30_000);
  expect(await statusCalls(page)).toBe(n);
  await page.clock.runFor(30_000);
  await expect.poll(() => statusCalls(page)).toBe(n + 1);
  // Second failure: next check in 2 min.
  await page.clock.runFor(60_000);
  expect(await statusCalls(page)).toBe(n + 1);
  await page.clock.runFor(60_000);
  await expect.poll(() => statusCalls(page)).toBe(n + 2);
  await expect(page.locator('.fdm.err')).toBeVisible();
});

test('changing screens keeps the one poller going, without a second one', async ({ page }) => {
  await page.clock.install();
  await mockTauriIPC(page, {
    check_fdm_status: () => ({ initialized: true, errors: [], warnings: [] }),
  });
  const n = await onOrderScreen(page);
  await page.evaluate(() => {
    window.location.hash = '/tables';
  });
  await page.waitForURL(/tables/);
  await openTable(page);
  // Every screen shows the indicator (AppHeader): the same poller goes
  // on through the screen changes, with no extra check and no second
  // poller - one check per 30 s.
  expect(await statusCalls(page)).toBe(n);
  await page.clock.runFor(30_000);
  await expect.poll(() => statusCalls(page)).toBe(n + 1);
  await page.clock.runFor(30_000);
  await expect.poll(() => statusCalls(page)).toBe(n + 2);
});
