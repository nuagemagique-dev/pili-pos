import { test, expect } from '@playwright/test';
import { mockTauriIPC, loginAndGoto, ipcCalls, overrideIPC, DEFAULT_USER } from './fixtures/mock-ipc';

// The Peppol panel of Settings: managers only, drawn only when the
// installation sends invoices over Peppol.

const DELIVERY = {
  id: 9, fiscalEventId: 41, invoiceNo: '20260002', customerEndpointId: '0208:0308357159', apVendor: 'storecove',
  apMessageId: null, status: 'failed', errorMsg: 'recipient not on Peppol', attempts: 3, nextAttemptAt: null,
  lastAttemptAt: '2026-10-17 09:00:00', parkedAt: '2026-10-17 09:00:00', acceptedAt: null,
  createdAt: '2026-10-17 08:00:00', lookupStatus: 'not_registered',
};

const CONFIGURED = {
  configured: true, provider: 'storecove', senderId: '0208:0123456789', queued: 2, accepted: 14, failed: 1,
  deliveries: [DELIVERY, { ...DELIVERY, id: 10, status: 'queued', errorMsg: null, parkedAt: null }],
};

test.describe('Settings: Peppol panel', () => {
  test('shows the setup, the queue counts and the parked deliveries with their reason', async ({ page }) => {
    await mockTauriIPC(page, { list_peppol_deliveries: () => CONFIGURED });
    await loginAndGoto(page, '/settings');
    const panel = page.getByTestId('peppol-panel');
    await expect(panel.locator('h2')).toHaveText('Peppol');
    await expect(panel.getByTestId('peppol-provider')).toHaveText('storecove');
    await expect(panel.getByTestId('peppol-sender')).toHaveText('0208:0123456789');
    await expect(panel.getByTestId('peppol-queued')).toContainText('2');
    await expect(panel.getByTestId('peppol-accepted')).toContainText('14');
    await expect(panel.getByTestId('peppol-failed')).toContainText('1');
    const parked = panel.getByTestId('peppol-parked-9');
    await expect(parked).toContainText('20260002');
    await expect(parked).toContainText('Motif : recipient not on Peppol');
    await expect(parked).toContainText('3 tentative(s)');
    // Only the parked row is listed.
    await expect(panel.getByTestId('peppol-parked-10')).toHaveCount(0);
  });

  test('Réessayer puts the delivery back in the queue and reloads the panel', async ({ page }) => {
    await mockTauriIPC(page, { list_peppol_deliveries: () => CONFIGURED, retry_peppol_delivery: () => null });
    await loginAndGoto(page, '/settings');
    await overrideIPC(page, 'list_peppol_deliveries', { ...CONFIGURED, queued: 3, failed: 0, deliveries: [{ ...DELIVERY, status: 'queued', errorMsg: null, parkedAt: null }] });
    await page.getByTestId('peppol-retry-9').click();
    expect(await ipcCalls(page, 'retry_peppol_delivery')).toEqual([{ deliveryId: 9 }]);
    await expect(page.getByTestId('peppol-none-parked')).toHaveText('Aucun envoi bloqué.');
    await expect(page.getByTestId('peppol-queued')).toContainText('3');
    await expect(page.locator('.toast', { hasText: "Envoi remis en file d'attente." })).toBeVisible();
  });

  test('a refused retry shows the engine error', async ({ page }) => {
    await mockTauriIPC(page, { list_peppol_deliveries: () => CONFIGURED });
    await loginAndGoto(page, '/settings');
    await overrideIPC(page, 'retry_peppol_delivery', { __reject: 'peppol_delivery_not_parked' });
    await page.getByTestId('peppol-retry-9').click();
    await expect(page.locator('.toast-error', { hasText: "Seul un envoi en échec" })).toBeVisible();
    await expect(page.getByTestId('peppol-parked-9')).toBeVisible();
  });

  test('nothing is drawn when Peppol is not configured', async ({ page }) => {
    await mockTauriIPC(page);
    await loginAndGoto(page, '/settings');
    await expect(page.locator('.table-plan')).toBeVisible();
    await expect(page.getByTestId('peppol-panel')).toHaveCount(0);
  });

  test('an empty queue says no delivery is parked', async ({ page }) => {
    await mockTauriIPC(page, { list_peppol_deliveries: () => ({ ...CONFIGURED, queued: 0, failed: 0, deliveries: [] }) });
    await loginAndGoto(page, '/settings');
    await expect(page.getByTestId('peppol-none-parked')).toBeVisible();
  });

  test('a cashier does not see the panel and the engine is not asked', async ({ page }) => {
    const cashier = { ...DEFAULT_USER, role: 'cashier' };
    await mockTauriIPC(page, { login: () => cashier, list_users: () => [cashier], list_peppol_deliveries: () => CONFIGURED });
    await loginAndGoto(page, '/settings');
    await expect(page.locator('.toggle')).toBeVisible();
    await expect(page.getByTestId('peppol-panel')).toHaveCount(0);
    expect(await ipcCalls(page, 'list_peppol_deliveries')).toHaveLength(0);
  });
});
