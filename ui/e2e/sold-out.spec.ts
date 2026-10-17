import { test, expect, type Page } from '@playwright/test';
import {
  mockTauriIPC, ipcCalls, live, DEFAULT_USER, DEFAULT_ORDER, DEFAULT_CATEGORY, DEFAULT_MENU_ITEM, tapUserTile,
} from './fixtures/mock-ipc';

// "Épuisé" (V11-D16): a mode of the order screen for cashiers and managers, a dialog with the
// three states, a greyed tile with a badge and a refusal toast, and the list of sold-out
// products. The engine rules are tested in crates/pos-engine/tests/sold_out.rs.

const END_OF_DAY = '2026-10-18T04:00:00+02:00';

const ITEMS = [
  { ...DEFAULT_MENU_ITEM, id: 1, name: 'Maki saumon x8' },
  { ...DEFAULT_MENU_ITEM, id: 2, name: 'Edamame', price_cents: 500 },
];

/** The order screen of a user with `role`, with a stateful sold-out engine behind the mock. */
async function orderScreen(page: Page, role = 'admin', initial: Record<number, string | null> = {}) {
  const user = { ...DEFAULT_USER, role };
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 0 }),
    list_cart_lines: () => [],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: live(() => {
      const state = (globalThis as any).__soldOut as Record<number, string | null>;
      return ((globalThis as any).__items as any[]).map((i) => ({
        ...i,
        sold_out: i.id in state,
        sold_out_until: i.id in state ? state[i.id] : null,
      }));
    }),
    list_sold_out: live(() => {
      const state = (globalThis as any).__soldOut as Record<number, string | null>;
      return ((globalThis as any).__items as any[])
        .filter((i) => i.id in state)
        .map((i) => ({ menu_item_id: i.id, name: i.name, until: state[i.id], set_by: '94060242576', set_at: '2026-10-17 10:00:00' }));
    }),
    set_sold_out: live(({ menuItemId, untilEndOfDay }: any) => {
      const state = (globalThis as any).__soldOut as Record<number, string | null>;
      state[menuItemId] = untilEndOfDay ? '2026-10-18T04:00:00+02:00' : null;
      return ((globalThis as any).__items as any[])
        .filter((i) => i.id in state)
        .map((i) => ({ menu_item_id: i.id, name: i.name, until: state[i.id], set_by: '94060242576', set_at: '2026-10-17 10:00:00' }));
    }),
    clear_sold_out: live(({ menuItemId }: any) => {
      const state = (globalThis as any).__soldOut as Record<number, string | null>;
      delete state[menuItemId];
      return ((globalThis as any).__items as any[])
        .filter((i) => i.id in state)
        .map((i) => ({ menu_item_id: i.id, name: i.name, until: state[i.id], set_by: '94060242576', set_at: '2026-10-17 10:00:00' }));
    }),
  });
  await page.addInitScript(([items, state]) => {
    (globalThis as any).__items = items;
    (globalThis as any).__soldOut = state;
  }, [ITEMS, initial] as const);
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
}

test.describe('Sold out (Épuisé)', () => {
  test('the mode opens the dialog; "jusqu\'à la fin de la journée" marks the product', async ({ page }) => {
    await orderScreen(page);
    await page.getByTestId('soldout-mode').click();
    await expect(page.getByTestId('soldout-hint')).toBeVisible();
    await page.locator('.item-btn', { hasText: 'Maki saumon' }).click();

    const dialog = page.getByTestId('soldout-dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog.getByTestId('soldout-choice-available')).toHaveClass(/current/);
    await dialog.getByTestId('soldout-choice-day').click();
    await expect(dialog).toHaveCount(0);

    expect(await ipcCalls(page, 'set_sold_out')).toEqual([{ menuItemId: 1, untilEndOfDay: true, lang: 'fr' }]);
    // The tile is greyed with its badge; nothing was added to the order.
    const tile = page.locator('.item-btn', { hasText: 'Maki saumon' });
    await expect(tile).toHaveClass(/sold-out/);
    await expect(tile.getByTestId('soldout-badge')).toHaveText('Épuisé');
    expect(await ipcCalls(page, 'add_order_line')).toHaveLength(0);
  });

  test('"jusqu\'à réactivation" sends no end of day, and "Disponible" clears it', async ({ page }) => {
    await orderScreen(page);
    await page.getByTestId('soldout-mode').click();
    await page.locator('.item-btn', { hasText: 'Edamame' }).click();
    await page.getByTestId('soldout-choice-until').click();
    expect(await ipcCalls(page, 'set_sold_out')).toEqual([{ menuItemId: 2, untilEndOfDay: false, lang: 'fr' }]);

    await page.locator('.item-btn', { hasText: 'Edamame' }).click();
    const dialog = page.getByTestId('soldout-dialog');
    await expect(dialog.getByTestId('soldout-status')).toHaveText("Jusqu'à réactivation");
    await expect(dialog.getByTestId('soldout-choice-until')).toHaveClass(/current/);
    await dialog.getByTestId('soldout-choice-available').click();
    expect(await ipcCalls(page, 'clear_sold_out')).toEqual([{ menuItemId: 2, lang: 'fr' }]);
    await expect(page.locator('.item-btn', { hasText: 'Edamame' })).not.toHaveClass(/sold-out/);
  });

  test('the dialog shows the end of the day in Brussels time', async ({ page }) => {
    await orderScreen(page, 'admin', { 1: END_OF_DAY });
    await page.getByTestId('soldout-mode').click();
    await page.locator('.item-btn', { hasText: 'Maki saumon' }).click();
    await expect(page.getByTestId('soldout-status')).toHaveText("Jusqu'à la fin de la journée (18/10 04:00)");
    await expect(page.getByTestId('soldout-choice-day')).toHaveClass(/current/);
    // Retour changes nothing.
    await page.getByTestId('soldout-cancel').click();
    await expect(page.getByTestId('soldout-dialog')).toHaveCount(0);
    expect(await ipcCalls(page, 'set_sold_out')).toHaveLength(0);
  });

  test('out of the mode a sold-out product shows the refusal and is not added', async ({ page }) => {
    await orderScreen(page, 'cashier', { 1: END_OF_DAY });
    await page.locator('.item-btn', { hasText: 'Maki saumon' }).click();
    await expect(page.locator('.toast-error', { hasText: 'Ce produit est épuisé.' })).toBeVisible();
    expect(await ipcCalls(page, 'add_order_line')).toHaveLength(0);
    // The available product is added as usual.
    await page.locator('.item-btn', { hasText: 'Edamame' }).click();
    await expect.poll(() => ipcCalls(page, 'add_order_line')).toHaveLength(1);
  });

  test('"Produits épuisés (n)" lists the products and puts one back on sale', async ({ page }) => {
    await orderScreen(page, 'admin', { 1: END_OF_DAY, 2: null });
    await expect(page.getByTestId('soldout-list-button')).toHaveText('Produits épuisés (2)');
    await page.getByTestId('soldout-list-button').click();
    const list = page.getByTestId('soldout-list');
    await expect(list.getByTestId('soldout-row')).toHaveCount(2);
    await expect(list.getByTestId('soldout-row').first()).toContainText('Maki saumon');
    await expect(list.getByTestId('soldout-row').first()).toContainText('18/10 04:00');
    await expect(list.getByTestId('soldout-row').nth(1)).toContainText('Jusqu\'à réactivation');

    await list.getByTestId('soldout-reenable').first().click();
    await expect(list.getByTestId('soldout-row')).toHaveCount(1);
    expect(await ipcCalls(page, 'clear_sold_out')).toEqual([{ menuItemId: 1, lang: 'fr' }]);
    await expect(page.getByTestId('soldout-list-button')).toHaveText('Produits épuisés (1)');
    await expect(page.locator('.item-btn', { hasText: 'Maki saumon' })).not.toHaveClass(/sold-out/);

    await list.getByTestId('soldout-reenable').click();
    await expect(list.getByTestId('soldout-list-empty')).toHaveText('Aucun produit épuisé.');
    await page.getByTestId('soldout-list-close').click();
    await expect(list).toHaveCount(0);
  });

  test('a waiter is not offered the mode nor the list, and reads no sold-out list', async ({ page }) => {
    await orderScreen(page, 'waiter', { 1: END_OF_DAY });
    await expect(page.getByTestId('soldout-mode')).toHaveCount(0);
    await expect(page.getByTestId('soldout-list-button')).toHaveCount(0);
    expect(await ipcCalls(page, 'list_sold_out')).toHaveLength(0);
    // The flag still greys the tile and the tap says why.
    await expect(page.locator('.item-btn', { hasText: 'Maki saumon' })).toHaveClass(/sold-out/);
    await page.locator('.item-btn', { hasText: 'Maki saumon' }).click();
    await expect(page.locator('.toast-error', { hasText: 'Ce produit est épuisé.' })).toBeVisible();
  });

  test('a refusal of the till is shown and the dialog stays', async ({ page }) => {
    await orderScreen(page);
    await page.getByTestId('soldout-mode').click();
    await page.locator('.item-btn', { hasText: 'Edamame' }).click();
    await page.evaluate(() => {
      const tauri = (window as any).__TAURI_INTERNALS__;
      const prev = tauri.invoke;
      tauri.invoke = (cmd: string, args?: any) =>
        cmd === 'set_sold_out' ? Promise.reject('forbidden') : prev(cmd, args);
    });
    await page.getByTestId('soldout-choice-day').click();
    await expect(page.locator('.toast-error')).toBeVisible();
    await expect(page.getByTestId('soldout-dialog')).toBeVisible();
  });
});
