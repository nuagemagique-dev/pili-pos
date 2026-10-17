import { test, expect, type Page } from '@playwright/test';
import { mockTauriIPC, loginAndGoto, ipcCalls, overrideIPC, DEFAULT_USER } from './fixtures/mock-ipc';

// The menu editor (V11-D17): Settings panel "Carte", the three-column editor, the on-screen
// keyboard and NumPad, option groups and options, the history, the lock while a catalog manages
// the menu, and the manager-only route.

const MENU = {
  managed_by: null,
  languages: ['fr', 'nl', 'en', 'zh'],
  categories: [
    { id: 1, names: { fr: 'Sushi', zh: '寿司' }, sort_order: 1, color: '#E94560', active: true },
    { id: 2, names: { fr: 'Boissons' }, sort_order: 2, color: null, active: false },
  ],
  products: [
    {
      id: 10, category_id: 1, names: { fr: 'Maki saumon', zh: '三文鱼卷' }, price_cents: 650, vat_code: 'B',
      vat_code_takeaway: 'C', kitchen_printer: true, code: 'A1', open_price: false, sort_order: 1, active: true,
    },
    {
      id: 11, category_id: 1, names: { fr: 'Edamame' }, price_cents: 450, vat_code: 'B',
      vat_code_takeaway: null, kitchen_printer: false, code: null, open_price: false, sort_order: 2, active: false,
    },
  ],
  groups: [{ id: 20, menu_item_id: 10, names: { fr: 'Sauce' }, min_select: 0, max_select: 2, sort_order: 1, active: true }],
  options: [{ id: 30, group_id: 20, names: { fr: 'Soja' }, price_cents: 50, sort_order: 1, active: true }],
};

const CHANGES = [
  {
    id: 2, entity_type: 'product', entity_id: 10, action: 'update', label: 'Maki saumon',
    changed_fields: ['price_cents', 'vat_code'], before_json: '{}', after_json: '{}',
    changed_by: '94060242576', changed_by_name: 'Bruno Moyse', changed_at: '2026-10-17 09:30:00',
  },
  {
    id: 1, entity_type: 'category', entity_id: 1, action: 'create', label: 'Sushi',
    changed_fields: ['names', 'sort_order'], before_json: null, after_json: '{}',
    changed_by: '94060242576', changed_by_name: 'Bruno Moyse', changed_at: '2026-10-17 09:00:00',
  },
];

async function editor(page: Page, over: Record<string, any> = {}) {
  await mockTauriIPC(page, {
    menu_admin_get: () => MENU,
    list_menu_changes: () => CHANGES,
    menu_save_product: () => ({ id: 10, menu: MENU }),
    menu_save_category: () => ({ id: 1, menu: MENU }),
    menu_save_modifier_group: () => ({ id: 20, menu: MENU }),
    menu_save_modifier: () => ({ id: 30, menu: MENU }),
    ...over,
  });
  await loginAndGoto(page, '/menu-editor');
  await expect(page.getByTestId('menu-editor-view')).toBeVisible();
}

const key = (page: Page, label: string) =>
  page.getByTestId('text-keyboard').getByRole('button', { name: label, exact: true });

async function type(page: Page, text: string) {
  for (const ch of text) await key(page, ch).click();
}

async function digits(page: Page, text: string) {
  for (const ch of text) await page.locator('.numpad .key', { hasText: new RegExp(`^${ch}$`) }).click();
}

test.describe('Settings: Carte panel', () => {
  test('a manager opens the editor from Settings', async ({ page }) => {
    await mockTauriIPC(page, { menu_admin_get: () => MENU });
    await loginAndGoto(page, '/settings');
    await expect(page.getByTestId('menu-editor-panel').locator('h2')).toHaveText('Carte');
    await page.getByTestId('menu-editor-open').click();
    await page.waitForURL(/menu-editor/);
    await expect(page.getByTestId('menu-editor-view')).toBeVisible();
  });

  test('the panel says who manages the menu and offers no editor', async ({ page }) => {
    await mockTauriIPC(page, { menu_admin_get: () => ({ ...MENU, managed_by: 'hubrise' }) });
    await loginAndGoto(page, '/settings');
    await expect(page.getByTestId('menu-editor-locked')).toContainText('La carte est gérée par HubRise');
    await expect(page.getByTestId('menu-editor-open')).toHaveCount(0);
  });

  test('a cashier does not see the panel and the engine is not asked', async ({ page }) => {
    const cashier = { ...DEFAULT_USER, role: 'cashier' };
    await mockTauriIPC(page, { login: () => cashier, list_users: () => [cashier], menu_admin_get: () => MENU });
    await loginAndGoto(page, '/settings');
    await expect(page.locator('.toggle').first()).toBeVisible();
    await expect(page.getByTestId('menu-editor-panel')).toHaveCount(0);
    expect(await ipcCalls(page, 'menu_admin_get')).toHaveLength(0);
  });

  test('a cashier typing the route is sent back to the tables', async ({ page }) => {
    const cashier = { ...DEFAULT_USER, role: 'cashier' };
    await mockTauriIPC(page, { login: () => cashier, list_users: () => [cashier] });
    await loginAndGoto(page, '/menu-editor');
    await page.waitForURL(/tables/);
    await expect(page.getByTestId('menu-editor-view')).toHaveCount(0);
  });
});

test.describe('Menu editor', () => {
  test('lists the categories and the products of the chosen one, hidden entries marked', async ({ page }) => {
    await editor(page);
    const cats = page.getByTestId('menu-ed-category-row');
    await expect(cats).toHaveCount(2);
    await expect(cats.nth(0)).toContainText('Sushi');
    await expect(cats.nth(1)).toContainText('Boissons');
    await expect(cats.nth(1)).toContainText('Masqué');
    const rows = page.getByTestId('menu-ed-product-row');
    await expect(rows).toHaveCount(2);
    await expect(rows.nth(0)).toContainText('Maki saumon');
    await expect(rows.nth(0)).toContainText('6,50');
    await expect(rows.nth(1)).toContainText('Masqué');
    await cats.nth(1).click();
    await expect(page.getByTestId('menu-ed-product-row')).toHaveCount(0);
  });

  test('editing a product: price on the NumPad, VAT pickers, saved as one call', async ({ page }) => {
    await editor(page);
    await page.getByTestId('menu-ed-product-row').first().click();
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Maki saumon');
    await expect(page.getByTestId('menu-ed-name-fr')).toHaveValue('Maki saumon');
    await expect(page.getByTestId('menu-ed-name-zh')).toHaveValue('三文鱼卷');
    await expect(page.getByTestId('menu-ed-price')).toHaveValue('6,50');
    await page.getByTestId('menu-ed-price').click();
    await page.locator('.numpad .key-clear').click();
    await digits(page, '720');
    await expect(page.getByTestId('menu-ed-price')).toHaveValue('7,20');
    await page.getByTestId('menu-ed-vat-A').click();
    await page.getByTestId('menu-ed-vat-takeaway-same').click();
    await page.getByTestId('menu-ed-open-price').click();
    await page.getByTestId('menu-ed-save').click();
    await expect(page.locator('.toast', { hasText: 'Carte enregistrée' })).toBeVisible();
    const calls = await ipcCalls(page, 'menu_save_product');
    expect(calls).toHaveLength(1);
    expect(calls[0].input).toMatchObject({
      id: 10, category_id: 1, price_cents: 720, vat_code: 'A', vat_code_takeaway: null,
      kitchen_printer: true, open_price: true, code: 'A1', active: true,
    });
    expect(calls[0].input.names).toEqual({ fr: 'Maki saumon', zh: '三文鱼卷' });
  });

  test('a new product: name on the on-screen keyboard (lower case key), then saved with no id', async ({ page }) => {
    await editor(page);
    await page.getByTestId('menu-ed-add-product').click();
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Nouveau produit');
    await page.getByTestId('menu-ed-name-fr').click();
    await key(page, 'T').click();
    await page.getByTestId('keyboard-case').click();
    await type(page, 'ea');
    await key(page, 'é').click();
    await expect(page.getByTestId('menu-ed-name-fr')).toHaveValue('Teaé');
    await page.getByTestId('menu-ed-price').click();
    await digits(page, '300');
    await page.getByTestId('menu-ed-save').click();
    const calls = await ipcCalls(page, 'menu_save_product');
    expect(calls[0].input).toMatchObject({
      id: null, category_id: 1, price_cents: 300, vat_code: 'B', active: true, sort_order: 3,
    });
    expect(calls[0].input.names.fr).toBe('Teaé');
  });

  test('Chinese is typed in a plain field, not on the keyboard', async ({ page }) => {
    await editor(page);
    await page.getByTestId('menu-ed-add-category').click();
    await page.getByTestId('menu-ed-name-fr').click();
    await key(page, 'B').click();
    await page.getByTestId('menu-ed-name-zh').fill('饮料');
    await page.getByTestId('menu-ed-color-none').click();
    await page.getByTestId('menu-ed-save').click();
    const calls = await ipcCalls(page, 'menu_save_category');
    expect(calls[0].input).toMatchObject({ id: null, names: { fr: 'B', zh: '饮料' }, color: null, active: true });
  });

  test('option groups and options of a saved product', async ({ page }) => {
    await editor(page);
    await page.getByTestId('menu-ed-product-row').first().click();
    await expect(page.getByTestId('menu-ed-edit-group')).toHaveCount(1);
    await page.getByTestId('menu-ed-add-group').click();
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Nouveau groupe');
    await page.getByTestId('menu-ed-name-fr').click();
    await key(page, 'S').click();
    await page.getByTestId('menu-ed-max-plus').click();
    await page.getByTestId('menu-ed-min-plus').click();
    await expect(page.getByTestId('menu-ed-min')).toHaveText('1');
    await expect(page.getByTestId('menu-ed-max')).toHaveText('2');
    // The minimum never passes the maximum.
    await page.getByTestId('menu-ed-max-minus').click();
    await page.getByTestId('menu-ed-max-minus').click();
    await expect(page.getByTestId('menu-ed-max')).toHaveText('1');
    await page.getByTestId('menu-ed-save').click();
    const groups = await ipcCalls(page, 'menu_save_modifier_group');
    expect(groups[0].input).toMatchObject({ id: null, menu_item_id: 10, min_select: 1, max_select: 1 });
    // The group is reopened with its options.
    await expect(page.getByTestId('menu-ed-edit-option')).toHaveCount(1);
    await page.getByTestId('menu-ed-edit-option').click();
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Soja');
    await page.getByTestId('menu-ed-price').click();
    await page.locator('.numpad .key-clear').click();
    await digits(page, '75');
    await page.getByTestId('menu-ed-save').click();
    const options = await ipcCalls(page, 'menu_save_modifier');
    expect(options[0].input).toMatchObject({ id: 30, group_id: 20, price_cents: 75 });
    await page.getByTestId('menu-ed-parent').click();
    await expect(page.getByTestId('menu-ed-edit-option')).toHaveCount(1);
  });

  test('a new product asks to be saved before it takes options', async ({ page }) => {
    await editor(page);
    await page.getByTestId('menu-ed-add-product').click();
    await expect(page.getByTestId('menu-ed-add-group')).toHaveCount(0);
    await expect(page.locator('.sub .hint')).toContainText('Enregistrez le produit');
  });

  test('a refused save shows the engine error and keeps the form', async ({ page }) => {
    await editor(page);
    await page.getByTestId('menu-ed-add-product').click();
    await overrideIPC(page, 'menu_save_product', { __reject: 'menu_invalid|field=name' });
    await page.getByTestId('menu-ed-save').click();
    await expect(page.getByTestId('menu-ed-error')).toContainText('Valeur invalide dans la carte : name');
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Nouveau produit');
  });

  test('Historique lists the changes with who, when and which fields', async ({ page }) => {
    await editor(page);
    await page.getByTestId('menu-ed-history-button').click();
    const rows = page.getByTestId('menu-ed-history-row');
    await expect(rows).toHaveCount(2);
    await expect(rows.nth(0)).toContainText('Modification');
    await expect(rows.nth(0)).toContainText('Maki saumon');
    await expect(rows.nth(0)).toContainText('par Bruno Moyse');
    await expect(rows.nth(0)).toContainText('prix, TVA sur place');
    await expect(rows.nth(1)).toContainText('Création');
    expect(await ipcCalls(page, 'list_menu_changes')).toEqual([{ limit: 100 }]);
  });

  test('empty history says so', async ({ page }) => {
    await editor(page, { list_menu_changes: () => [] });
    await page.getByTestId('menu-ed-history-button').click();
    await expect(page.getByTestId('menu-ed-history-empty')).toBeVisible();
  });

  test('while a catalog manages the menu the editor is read-only', async ({ page }) => {
    await editor(page, { menu_admin_get: () => ({ ...MENU, managed_by: 'hubrise' }) });
    await expect(page.getByTestId('menu-ed-locked')).toContainText('HubRise');
    await expect(page.getByTestId('menu-ed-add-category')).toHaveCount(0);
    await expect(page.getByTestId('menu-ed-add-product')).toHaveCount(0);
    await page.getByTestId('menu-ed-product-row').first().click();
    await expect(page.getByTestId('menu-ed-form-title')).toHaveCount(0);
    expect(await ipcCalls(page, 'menu_save_product')).toHaveLength(0);
  });
});
