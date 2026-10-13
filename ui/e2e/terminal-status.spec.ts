import { test, expect, type Page } from '@playwright/test';
import { mockTauriIPC, ipcCalls, tapUserTile, overrideIPC, DEFAULT_USER } from './fixtures/mock-ipc';

// The linked card terminal in the header lights and the start-up checks
// (command check_payment_terminal, crates/pos-engine/src/commands/eft.rs).
// The check charges nothing; it never blocks the login.

const NOT_CONFIGURED = { configured: false, kind: null, connected: false, busy: false, detail: null };
const CONNECTED = { configured: true, kind: 'ccv_opi', connected: true, busy: false, detail: null };
const BUSY = { ...CONNECTED, busy: true };
const UNREACHABLE = { ...CONNECTED, connected: false, detail: 'connection refused' };

async function open(page: Page, status: unknown) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    check_payment_terminal: () => status,
  });
  await page.goto('/');
}

async function toTables(page: Page, status: unknown) {
  await open(page, status);
  await tapUserTile(page);
  await expect(page).toHaveURL(/tables/);
}

test.describe('Terminal light in the header', () => {
  test('hidden when no terminal is configured', async ({ page }) => {
    await toTables(page, NOT_CONFIGURED);
    await expect(page.locator('.lights')).toBeVisible();
    await expect(page.getByTestId('light-terminal')).toHaveCount(0);
    expect(await ipcCalls(page, 'check_payment_terminal')).not.toHaveLength(0);
  });

  test('green when the terminal answers', async ({ page }) => {
    await toTables(page, CONNECTED);
    const light = page.getByTestId('light-terminal');
    await expect(light).toHaveClass(/ok/);
    await expect(light).toContainText('Terminal');
  });

  test('amber while a payment runs on it', async ({ page }) => {
    await toTables(page, BUSY);
    const light = page.getByTestId('light-terminal');
    await expect(light).toHaveClass(/warn/);
    await expect(light).toHaveAttribute('title', 'paiement en cours');
  });

  test('red with the reason in the tooltip when it does not answer', async ({ page }) => {
    await toTables(page, UNREACHABLE);
    const light = page.getByTestId('light-terminal');
    await expect(light).toHaveClass(/err/);
    await expect(light).toHaveAttribute('title', 'injoignable: connection refused');
  });

  test('follows the terminal: red, then green at the next check', async ({ page }) => {
    await page.clock.install();
    await toTables(page, UNREACHABLE);
    await expect(page.getByTestId('light-terminal')).toHaveClass(/err/);
    await overrideIPC(page, 'check_payment_terminal', CONNECTED);
    // The lights are polled every 30 s, like the printers.
    await page.clock.runFor(30_000);
    await expect(page.getByTestId('light-terminal')).toHaveClass(/ok/);
  });
});

test.describe('Terminal row in the start-up checks', () => {
  test('no row when no terminal is configured', async ({ page }) => {
    await open(page, NOT_CONFIGURED);
    await expect(page.locator('.check', { hasText: 'Imprimante ticket' })).toBeVisible();
    await expect(page.locator('.check', { hasText: 'Terminal de paiement' })).toHaveCount(0);
  });

  test('connected', async ({ page }) => {
    await open(page, CONNECTED);
    const row = page.locator('.check', { hasText: 'Terminal de paiement' });
    await expect(row).toHaveClass(/ok/);
    await expect(row.locator('.check-text')).toHaveText('connecté');
  });

  test('payment in progress', async ({ page }) => {
    await open(page, BUSY);
    const row = page.locator('.check', { hasText: 'Terminal de paiement' });
    await expect(row).toHaveClass(/warn/);
    await expect(row.locator('.check-text')).toHaveText('paiement en cours');
  });

  test('unreachable with the reason, and the login still works', async ({ page }) => {
    await open(page, UNREACHABLE);
    const row = page.locator('.check', { hasText: 'Terminal de paiement' });
    await expect(row).toHaveClass(/err/);
    await expect(row.locator('.check-text')).toHaveText('injoignable');
    await expect(row).toHaveAttribute('title', 'connection refused');
    await tapUserTile(page);
    await expect(page).toHaveURL(/tables/);
  });
});
