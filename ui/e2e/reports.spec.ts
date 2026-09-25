import { test, expect } from '@playwright/test';
import { mockTauriIPC, loginAndGoto, ipcCalls } from './fixtures/mock-ipc';

const SAMPLE_Z_REPORTS = [
  { id: 2, reportDate: '2026-04-12', sequenceNumber: 2, createdAt: '2026-04-12T22:30:00Z', fiscalEventId: 10 },
  { id: 1, reportDate: '2026-04-11', sequenceNumber: 1, createdAt: '2026-04-11T22:15:00Z', fiscalEventId: 5 },
];

test.describe('Reports View', () => {
  test('shows X and Z generate buttons', async ({ page }) => {
    await mockTauriIPC(page);
    await loginAndGoto(page, '/reports');
    await expect(page.getByRole('button', { name: 'Générer X', exact: true })).toBeVisible();
    await expect(page.locator('.z-btn')).toBeVisible();
  });

  test('shows Z history with entries', async ({ page }) => {
    await mockTauriIPC(page, { list_z_reports: () => SAMPLE_Z_REPORTS });
    await loginAndGoto(page, '/reports');
    await expect(page.locator('.history-row')).toHaveCount(2);
    await expect(page.locator('.history-row').first()).toContainText('Z #2');
  });

  test('history rows have reprint buttons', async ({ page }) => {
    await mockTauriIPC(page, { list_z_reports: () => SAMPLE_Z_REPORTS });
    await loginAndGoto(page, '/reports');
    await expect(page.locator('.reprint-btn')).toHaveCount(2);
  });

  test('empty history shows placeholder', async ({ page }) => {
    await mockTauriIPC(page, { list_z_reports: () => [] });
    await loginAndGoto(page, '/reports');
    await expect(page.locator('.empty')).toBeVisible();
  });

  test('Z generate button is enabled', async ({ page }) => {
    await mockTauriIPC(page);
    await loginAndGoto(page, '/reports');
    await expect(page.locator('.z-btn')).toBeEnabled();
  });

  test('a Z needs a second tap: the first one only asks to confirm', async ({ page }) => {
    await mockTauriIPC(page);
    await loginAndGoto(page, '/reports');
    const z = page.locator('.z-btn');
    await z.click();
    await expect(z).toHaveText('Confirmer la clôture ?');
    await expect(z).toHaveClass(/armed/);
    expect(await ipcCalls(page, 'generate_z_report')).toHaveLength(0);

    await z.click();
    await expect.poll(() => ipcCalls(page, 'generate_z_report')).toHaveLength(1);
    await expect(z).toHaveText('Générer Z');
  });

  test('an unconfirmed Z disarms by itself', async ({ page }) => {
    await page.clock.install();
    await mockTauriIPC(page);
    await loginAndGoto(page, '/reports');
    const z = page.locator('.z-btn');
    await z.click();
    await expect(z).toHaveText('Confirmer la clôture ?');
    await page.clock.runFor(5_000);
    await expect(z).toHaveText('Générer Z');
    expect(await ipcCalls(page, 'generate_z_report')).toHaveLength(0);
  });
});
