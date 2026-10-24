import { test, expect, type Page } from '@playwright/test';
import {
  mockTauriIPC, ipcCalls, live, DEFAULT_USER, DEFAULT_ORDER, DEFAULT_CATEGORY, DEFAULT_MENU_ITEM, tapUserTile,
} from './fixtures/mock-ipc';

// The 14 EU allergens on the order screen: numbered badges on the tiles
// and in the option picker, the "Allergènes" mode that opens a product's
// sheet instead of adding it, the legend, and the print buttons.
// Maki has option groups (V11-D18: only such a product opens the option picker).
const MAKI = {
  ...DEFAULT_MENU_ITEM,
  has_modifiers: true,
  allergens: ['sesame', 'fish', 'gluten', 'milk', 'eggs', 'soybeans', 'nuts', 'celery'],
  traces: ['molluscs'],
  allergens_declared: true,
};
const COCA = { ...DEFAULT_MENU_ITEM, id: 2, name: 'Coca-Cola', price_cents: 300, allergens: [], traces: [], allergens_declared: true };
const DESSERT = { ...DEFAULT_MENU_ITEM, id: 3, name: 'Dessert du jour', price_cents: 500 };

const SHEET = (id: number, lang: string) => {
  const fr = lang === 'fr';
  return {
    lang,
    products: [id === 1
      ? {
        id: 1, name: 'Maki saumon x8', name_fr: 'Maki saumon x8', declared: true,
        contains: [
          { code: 'gluten', number: 1, name: fr ? 'Céréales contenant du gluten' : 'Cereals containing gluten' },
          { code: 'fish', number: 4, name: fr ? 'Poissons' : 'Fish' },
        ],
        traces: [{ code: 'molluscs', number: 14, name: fr ? 'Mollusques' : 'Molluscs' }],
        options: [
          { group: 'Sauce', name: 'Soja', declared: true, contains: [{ code: 'soybeans', number: 6, name: fr ? 'Soja' : 'Soybeans' }], traces: [] },
          { group: 'Sauce', name: 'Nature', declared: false, contains: [], traces: [] },
        ],
      }
      : id === 2
        ? { id: 2, name: 'Coca-Cola', name_fr: 'Coca-Cola', declared: true, contains: [], traces: [], options: [] }
        : { id: 3, name: 'Dessert du jour', name_fr: 'Dessert du jour', declared: false, contains: [], traces: [], options: [] }],
  };
};

async function orderScreen(page: Page, extra: Record<string, unknown> = {}) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 0 }),
    list_cart_lines: () => [],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [MAKI, COCA, DESSERT],
    get_allergen_sheet: live((args: any) => (globalThis as any).SHEET_OF(args.menuItemId, args.lang)),
    ...extra,
  } as any);
  await page.addInitScript(
    ([maker]) => { (globalThis as any).SHEET_OF = new Function(`return (${maker})`)(); },
    [SHEET.toString()],
  );
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
}

test.describe('Allergens', () => {
  test('a tile shows the numbers of what the product contains, at most six', async ({ page }) => {
    await orderScreen(page);
    const maki = page.locator('.item-btn', { hasText: 'Maki saumon x8' });
    const badges = maki.getByTestId('allergen-badge');
    await expect(badges).toHaveCount(6);
    // Annex order: gluten 1, eggs 3, fish 4, soybeans 6, milk 7, nuts 8.
    await expect(badges).toHaveText(['1', '3', '4', '6', '7', '8']);
    await expect(maki.getByTestId('allergen-more')).toHaveText('+2');
    // No allergen, or no information: no badge.
    await expect(page.locator('.item-btn', { hasText: 'Coca-Cola' }).getByTestId('allergen-badge')).toHaveCount(0);
    await expect(page.locator('.item-btn', { hasText: 'Dessert du jour' }).getByTestId('allergen-badge')).toHaveCount(0);
  });

  test('the mode shows the legend and a tap opens the sheet instead of adding', async ({ page }) => {
    await orderScreen(page);
    await expect(page.getByTestId('allergen-legend')).toHaveCount(0);
    await page.getByTestId('allergen-mode').click();
    const legend = page.getByTestId('allergen-legend');
    await expect(legend.locator('li')).toHaveCount(14);
    await expect(legend.locator('li').first()).toContainText('Céréales contenant du gluten');
    await expect(legend.locator('li').last()).toContainText('Mollusques');

    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click();
    const sheet = page.getByTestId('allergen-sheet');
    await expect(sheet).toBeVisible();
    await expect(sheet.getByTestId('allergen-contains')).toContainText('1 Céréales contenant du gluten, 4 Poissons');
    await expect(sheet.getByTestId('allergen-traces')).toContainText('14 Mollusques');
    await expect(sheet.getByTestId('allergen-options')).toContainText('Soja');
    await expect(sheet.getByTestId('allergen-options')).toContainText('Information non disponible');
    // Nothing was added to the order.
    expect(await ipcCalls(page, 'add_order_line')).toHaveLength(0);
    expect(await ipcCalls(page, 'get_item_modifiers')).toHaveLength(0);
    await sheet.getByTestId('allergen-close').click();
    await expect(sheet).toHaveCount(0);

    // Out of the mode a tap adds again.
    await page.getByTestId('allergen-mode').click();
    await page.locator('.item-btn', { hasText: 'Coca-Cola' }).click();
    await expect.poll(async () => (await ipcCalls(page, 'add_order_line')).length).toBeGreaterThan(0);
  });

  test('"no allergen" and "no information" are told apart', async ({ page }) => {
    await orderScreen(page);
    await page.getByTestId('allergen-mode').click();
    await page.locator('.item-btn', { hasText: 'Coca-Cola' }).click();
    await expect(page.getByTestId('allergen-none')).toHaveText('Aucun allergène');
    await expect(page.getByTestId('allergen-unknown')).toHaveCount(0);
    await page.getByTestId('allergen-close').click();

    await page.locator('.item-btn', { hasText: 'Dessert du jour' }).click();
    await expect(page.getByTestId('allergen-unknown')).toContainText('Information non disponible, demandez au responsable');
    await expect(page.getByTestId('allergen-none')).toHaveCount(0);
  });

  test('the sheet prints for one product or for the whole menu', async ({ page }) => {
    await orderScreen(page);
    await page.getByTestId('allergen-mode').click();
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click();
    await page.getByTestId('allergen-print-one').click();
    await expect(page.getByTestId('allergen-print-outcome')).toHaveText('Fiche imprimée');
    await page.getByTestId('allergen-print-all').click();
    await expect.poll(async () => (await ipcCalls(page, 'print_allergen_sheet')).length).toBe(2);
    const calls = await ipcCalls(page, 'print_allergen_sheet');
    expect(calls[0]).toMatchObject({ menuItemId: 1, lang: 'fr' });
    expect(calls[1]).toMatchObject({ menuItemId: null, lang: 'fr' });
  });

  test('a printer problem is told, not hidden', async ({ page }) => {
    await orderScreen(page, { print_allergen_sheet: () => 'no_printer' });
    await page.getByTestId('allergen-mode').click();
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click();
    await page.getByTestId('allergen-print-one').click();
    await expect(page.getByTestId('allergen-print-outcome')).toHaveText('Aucune imprimante configurée');
  });

  test('the option picker shows the numbers of each option', async ({ page }) => {
    await orderScreen(page, {
      get_item_modifiers: live(() => [
        { id: 10, name: 'Sauce', min: 0, max: 1, options: [
          { id: 101, name: 'Soja', price_cents: 0, allergens: ['soybeans', 'gluten'], traces: [], allergens_declared: true },
          { id: 102, name: 'Nature', price_cents: 0, allergens: [], traces: [], allergens_declared: false },
        ] },
      ]),
    });
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click();
    const dialog = page.getByTestId('modifier-dialog');
    await expect(dialog).toBeVisible();
    const soja = dialog.getByRole('button', { name: /Soja/ });
    await expect(soja.getByTestId('allergen-badge')).toHaveText(['1', '6']);
    await expect(dialog.getByRole('button', { name: /Nature/ }).getByTestId('allergen-badge')).toHaveCount(0);
  });
  test('in English the sheet is read twice and the French names follow in parentheses', async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem('pili-pos-language', 'en'));
    await orderScreen(page);
    await page.getByTestId('allergen-mode').click();
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click();
    await expect(page.getByTestId('allergen-contains')).toContainText(
      '1 Cereals containing gluten, 4 Fish (Céréales contenant du gluten, Poissons)',
    );
    await expect(page.getByTestId('allergen-traces')).toContainText('14 Molluscs (Mollusques)');
    expect(await ipcCalls(page, 'get_allergen_sheet')).toEqual([
      { menuItemId: 1, lang: 'en' },
      { menuItemId: 1, lang: 'fr' },
    ]);
  });

  test('in French the sheet is read once, without a second language', async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem('pili-pos-language', 'fr'));
    await orderScreen(page);
    await page.getByTestId('allergen-mode').click();
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click();
    await expect(page.getByTestId('allergen-contains')).toContainText('1 Céréales contenant du gluten, 4 Poissons');
    await expect(page.getByTestId('allergen-contains')).not.toContainText('(');
    expect(await ipcCalls(page, 'get_allergen_sheet')).toEqual([{ menuItemId: 1, lang: 'fr' }]);
  });

  for (const [ui, printed] of [['zh', 'fr'], ['en', 'en'], ['fr', 'fr']] as const) {
    test(`with the UI in ${ui} the sheet prints in ${printed}`, async ({ page }) => {
      await page.addInitScript((l) => localStorage.setItem('pili-pos-language', l), ui);
      await orderScreen(page);
      await page.getByTestId('allergen-mode').click();
      await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click();
      await page.getByTestId('allergen-print-one').click();
      await expect.poll(async () => (await ipcCalls(page, 'print_allergen_sheet')).length).toBe(1);
      expect((await ipcCalls(page, 'print_allergen_sheet'))[0]).toEqual({ menuItemId: 1, lang: printed });
    });
  }

  test('a sheet the engine cannot find is told and shows no allergens', async ({ page }) => {
    await orderScreen(page, { get_allergen_sheet: live(() => { throw 'allergen_target_not_found|id=1'; }) });
    await page.getByTestId('allergen-mode').click();
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click();
    await expect(page.getByTestId('allergen-error')).toHaveText('Produit ou option introuvable (1).');
    await expect(page.getByTestId('allergen-contains')).toHaveCount(0);
    await expect(page.getByTestId('allergen-unknown')).toHaveCount(0);
  });

  test('a print the engine refuses is told in the same place', async ({ page }) => {
    await orderScreen(page, { print_allergen_sheet: live(() => { throw 'allergen_target_not_found|id=1'; }) });
    await page.getByTestId('allergen-mode').click();
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click();
    await expect(page.getByTestId('allergen-contains')).toBeVisible();
    await page.getByTestId('allergen-print-one').click();
    await expect(page.getByTestId('allergen-error')).toHaveText('Produit ou option introuvable (1).');
    await expect(page.getByTestId('allergen-print-outcome')).toHaveCount(0);
  });

  test('a legend the engine refuses is told when the mode opens', async ({ page }) => {
    await orderScreen(page, { list_allergens: live(() => { throw 'allergen_target_not_found|id=0'; }) });
    await page.getByTestId('allergen-mode').click();
    await expect(page.getByTestId('allergen-legend-error')).toHaveText('Produit ou option introuvable (0).');
    await expect(page.getByTestId('allergen-legend').locator('li')).toHaveCount(0);
  });
});
