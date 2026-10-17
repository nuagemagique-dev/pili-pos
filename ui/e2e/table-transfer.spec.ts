import { test, expect } from '@playwright/test';
import { mockTauriIPC, ipcCalls, live, overrideIPC, DEFAULT_USER, DEFAULT_ORDER, tapUserTile } from './fixtures/mock-ipc';

// Moving an open order to another table (command transfer_table), from
// the cart and from the Tables screen's "Déplacer une table" mode.

const tables = (occupied: Record<number, number> = {}) => () => {
  const all = [];
  for (let n = 1; n <= 20; n++) {
    const orderId = occupied[n] ?? null;
    all.push({
      tableNumber: n, orderId, totalCents: orderId ? 2400 : 0, lineCount: orderId ? 2 : 0,
      status: orderId ? 'occupied' : 'free', openedAt: null, preBillPrinted: false,
    });
  }
  return all;
};

async function openTable1(page: import('@playwright/test').Page, extra: Record<string, any> = {}) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
    list_tables: tables({ 4: 44 }),
    ...extra,
  });
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
}

test.describe('From the cart', () => {
  test('lists the free tables, current one and busy ones left out, and moves the order', async ({ page }) => {
    await openTable1(page);
    await page.getByTestId('transfer-table').click();

    const picker = page.getByRole('dialog', { name: 'Changer de table' });
    await expect(picker).toBeVisible();
    await expect(picker.getByTestId('pick-table-1')).toHaveCount(0);
    await expect(picker.getByTestId('pick-table-4')).toHaveCount(0);
    await expect(picker.locator('.pick')).toHaveCount(18);
    // Nothing is picked yet.
    await expect(picker.getByTestId('transfer-confirm')).toBeDisabled();

    await picker.getByTestId('pick-table-5').click();
    await picker.getByTestId('transfer-confirm').click();

    await expect(picker).toBeHidden();
    await expect(page.getByText('Commande déplacée vers la table 5')).toBeVisible();
    const calls = await ipcCalls(page, 'transfer_table');
    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({ toTable: 5 });
  });

  test('shows the refusal in French', async ({ page }) => {
    await openTable1(page);
    await overrideIPC(page, 'transfer_table', { __reject: 'table_transfer_target_busy' });
    await page.getByTestId('transfer-table').click();
    await page.getByTestId('pick-table-6').click();
    await page.getByTestId('transfer-confirm').click();
    await expect(page.getByText('Cette table a déjà une commande en cours')).toBeVisible();
  });

  test('warns when the kitchen notice did not print', async ({ page }) => {
    await openTable1(page, {
      transfer_table: live(({ orderId, toTable }: any) => ({
        orderId, fromTable: 1, toTable, kitchenSlip: 'nothing_to_print', transferSlip: 'failed',
      })),
    });
    await page.getByTestId('transfer-table').click();
    await page.getByTestId('pick-table-7').click();
    await page.getByTestId('transfer-confirm').click();
    await expect(page.getByText('Bon de commande non imprimé')).toBeVisible();
  });

  test('is absent for a takeaway', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_cart_lines: () => [],
    });
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables/);
    await page.getByRole('button', { name: 'Nouvelle vente à emporter' }).click();
    await page.waitForURL(/order/);
    await expect(page.getByTestId('transfer-table')).toHaveCount(0);
  });
});

test.describe('From the Tables screen', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_tables: tables({ 3: 42, 4: 44 }),
    });
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables/);
  });

  test('source, then target, then confirmation', async ({ page }) => {
    await page.getByTestId('move-toggle').click();
    await expect(page.getByTestId('move-hint')).toHaveText('Touchez la table à déplacer');

    // A free table is no source: tapping it does nothing.
    await page.locator('.table-card', { hasText: 'Libre' }).first().click({ force: true });
    await expect(page.getByTestId('move-hint')).toHaveText('Touchez la table à déplacer');
    await expect(page).toHaveURL(/tables/);

    await page.locator('.table-card.occupied').first().click();
    await expect(page.getByTestId('move-hint')).toHaveText('Table 3 : touchez la table libre');
    // A busy table is no target.
    await page.locator('.table-card.occupied').nth(1).click({ force: true });
    await expect(page.getByTestId('move-confirm')).toHaveCount(0);

    await page.locator('.table-card .table-num', { hasText: /^7$/ }).click();
    const confirm = page.getByTestId('move-confirm');
    await expect(confirm).toContainText('Déplacer la table 3 vers la table 7 ?');
    await page.getByTestId('move-validate').click();

    await expect(confirm).toBeHidden();
    await expect(page.getByText('Commande déplacée vers la table 7')).toBeVisible();
    const calls = await ipcCalls(page, 'transfer_table');
    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({ orderId: 42, toTable: 7 });
    // The mode is left: tapping a table opens it again.
    await expect(page.getByTestId('move-hint')).toHaveCount(0);
  });

  test('cancelling the confirmation or the mode moves nothing', async ({ page }) => {
    await page.getByTestId('move-toggle').click();
    await page.locator('.table-card.occupied').first().click();
    await page.locator('.table-card .table-num', { hasText: /^8$/ }).click();
    await page.getByTestId('move-confirm').getByRole('button', { name: 'Retour' }).click();
    await expect(page.getByTestId('move-confirm')).toHaveCount(0);

    await page.getByTestId('move-toggle').click();
    await expect(page.getByTestId('move-hint')).toHaveCount(0);
    expect(await ipcCalls(page, 'transfer_table')).toHaveLength(0);
  });
});
