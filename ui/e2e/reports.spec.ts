import { test, expect } from '@playwright/test';
import { mockTauriIPC, loginAndGoto } from './fixtures/mock-ipc';

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
});
