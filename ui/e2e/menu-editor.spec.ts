import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  loginAndGoto,
  ipcCalls,
  overrideIPC,
  live,
  DEFAULT_USER,
} from './fixtures/mock-ipc'

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
      id: 10,
      category_id: 1,
      names: { fr: 'Maki saumon', zh: '三文鱼卷' },
      price_cents: 650,
      vat_code: 'B',
      vat_code_takeaway: 'C',
      kitchen_printer: true,
      code: 'A1',
      open_price: false,
      sort_order: 1,
      active: true,
    },
    {
      id: 11,
      category_id: 1,
      names: { fr: 'Edamame' },
      price_cents: 450,
      vat_code: 'B',
      vat_code_takeaway: null,
      kitchen_printer: false,
      code: null,
      open_price: false,
      sort_order: 2,
      active: false,
    },
  ],
  groups: [
    {
      id: 20,
      menu_item_id: 10,
      names: { fr: 'Sauce' },
      min_select: 0,
      max_select: 2,
      sort_order: 1,
      active: true,
    },
  ],
  options: [
    { id: 30, group_id: 20, names: { fr: 'Soja' }, price_cents: 50, sort_order: 1, active: true },
  ],
  // Maki saumon is declared (fish, soy; traces of sesame); Edamame and the option have no information.
  allergens: [
    {
      entity_type: 'menu_item',
      entity_id: 10,
      contains: ['fish', 'soybeans'],
      may_contain: ['sesame'],
    },
  ],
}

const CHANGES = [
  {
    id: 2,
    entity_type: 'product',
    entity_id: 10,
    action: 'update',
    label: 'Maki saumon',
    changed_fields: ['price_cents', 'vat_code'],
    before_json: '{}',
    after_json: '{}',
    changed_by: '94060242576',
    changed_by_name: 'Bruno Moyse',
    changed_at: '2026-10-17 09:30:00',
  },
  {
    id: 1,
    entity_type: 'category',
    entity_id: 1,
    action: 'create',
    label: 'Sushi',
    changed_fields: ['names', 'sort_order'],
    before_json: null,
    after_json: '{}',
    changed_by: '94060242576',
    changed_by_name: 'Bruno Moyse',
    changed_at: '2026-10-17 09:00:00',
  },
]

async function editor(page: Page, over: Record<string, any> = {}) {
  await mockTauriIPC(page, {
    menu_admin_get: () => MENU,
    list_menu_changes: () => CHANGES,
    menu_save_product: () => ({ id: 10, menu: MENU }),
    menu_save_category: () => ({ id: 1, menu: MENU }),
    menu_save_modifier_group: () => ({ id: 20, menu: MENU }),
    menu_save_modifier: () => ({ id: 30, menu: MENU }),
    ...over,
  })
  await loginAndGoto(page, '/menu-editor')
  await expect(page.getByTestId('menu-editor-view')).toBeVisible()
}

const key = (page: Page, label: string) =>
  page.getByTestId('text-keyboard').getByRole('button', { name: label, exact: true })

async function type(page: Page, text: string) {
  for (const ch of text) await key(page, ch).click()
}

async function digits(page: Page, text: string) {
  for (const ch of text)
    await page.locator('.numpad .key', { hasText: new RegExp(`^${ch}$`, 'u') }).click()
}

test.describe('Settings: Carte panel', () => {
  test('a manager opens the editor from Settings', async ({ page }) => {
    await mockTauriIPC(page, { menu_admin_get: () => MENU })
    await loginAndGoto(page, '/settings')
    await expect(page.getByTestId('menu-editor-panel').locator('h2')).toHaveText('Carte')
    await page.getByTestId('menu-editor-open').click()
    await page.waitForURL(/menu-editor/u)
    await expect(page.getByTestId('menu-editor-view')).toBeVisible()
  })

  test('the panel says who manages the menu and offers no editor', async ({ page }) => {
    await mockTauriIPC(page, { menu_admin_get: () => ({ ...MENU, managed_by: 'hubrise' }) })
    await loginAndGoto(page, '/settings')
    await expect(page.getByTestId('menu-editor-locked')).toContainText(
      'La carte est gérée par HubRise',
    )
    await expect(page.getByTestId('menu-editor-open')).toHaveCount(0)
  })

  test('a cashier does not see the panel and the engine is not asked', async ({ page }) => {
    const cashier = { ...DEFAULT_USER, role: 'cashier' }
    await mockTauriIPC(page, {
      login: () => cashier,
      list_users: () => [cashier],
      menu_admin_get: () => MENU,
    })
    await loginAndGoto(page, '/settings')
    await expect(page.locator('.toggle').first()).toBeVisible()
    await expect(page.getByTestId('menu-editor-panel')).toHaveCount(0)
    expect(await ipcCalls(page, 'menu_admin_get')).toHaveLength(0)
  })

  test('a cashier typing the route is sent back to the tables', async ({ page }) => {
    const cashier = { ...DEFAULT_USER, role: 'cashier' }
    await mockTauriIPC(page, { login: () => cashier, list_users: () => [cashier] })
    await loginAndGoto(page, '/menu-editor')
    await page.waitForURL(/tables/u)
    await expect(page.getByTestId('menu-editor-view')).toHaveCount(0)
  })
})

test.describe('Menu editor', () => {
  test('lists the categories and the products of the chosen one, hidden entries marked', async ({
    page,
  }) => {
    await editor(page)
    const cats = page.getByTestId('menu-ed-category-row')
    await expect(cats).toHaveCount(2)
    await expect(cats.nth(0)).toContainText('Sushi')
    await expect(cats.nth(1)).toContainText('Boissons')
    await expect(cats.nth(1)).toContainText('Masqué')
    const rows = page.getByTestId('menu-ed-product-row')
    await expect(rows).toHaveCount(2)
    await expect(rows.nth(0)).toContainText('Maki saumon')
    await expect(rows.nth(0)).toContainText('6,50')
    await expect(rows.nth(1)).toContainText('Masqué')
    await cats.nth(1).click()
    await expect(page.getByTestId('menu-ed-product-row')).toHaveCount(0)
  })

  test('editing a product: price on the NumPad, VAT pickers, saved as one call', async ({
    page,
  }) => {
    await editor(page)
    await page.getByTestId('menu-ed-product-row').first().click()
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Maki saumon')
    await expect(page.getByTestId('menu-ed-name-fr')).toHaveValue('Maki saumon')
    await expect(page.getByTestId('menu-ed-name-zh')).toHaveValue('三文鱼卷')
    await expect(page.getByTestId('menu-ed-price')).toHaveValue('6,50')
    await page.getByTestId('menu-ed-price').click()
    await page.locator('.numpad .key-clear').click()
    await digits(page, '720')
    await expect(page.getByTestId('menu-ed-price')).toHaveValue('7,20')
    await page.getByTestId('menu-ed-vat-A').click()
    await page.getByTestId('menu-ed-vat-takeaway-same').click()
    await page.getByTestId('menu-ed-open-price').click()
    await page.getByTestId('menu-ed-save').click()
    await expect(page.locator('.toast', { hasText: 'Carte enregistrée' })).toBeVisible()
    const calls = await ipcCalls(page, 'menu_save_product')
    expect(calls).toHaveLength(1)
    expect(calls[0].input).toMatchObject({
      id: 10,
      category_id: 1,
      price_cents: 720,
      vat_code: 'A',
      vat_code_takeaway: null,
      kitchen_printer: true,
      open_price: true,
      code: 'A1',
      active: true,
    })
    expect(calls[0].input.names).toEqual({ fr: 'Maki saumon', zh: '三文鱼卷' })
  })

  test('a new product: name on the on-screen keyboard (lower case key), then saved with no id', async ({
    page,
  }) => {
    await editor(page)
    await page.getByTestId('menu-ed-add-product').click()
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Nouveau produit')
    await page.getByTestId('menu-ed-name-fr').click()
    await key(page, 'T').click()
    await page.getByTestId('keyboard-case').click()
    await type(page, 'ea')
    await key(page, 'é').click()
    await expect(page.getByTestId('menu-ed-name-fr')).toHaveValue('Teaé')
    await page.getByTestId('menu-ed-price').click()
    await digits(page, '300')
    await page.getByTestId('menu-ed-save').click()
    const calls = await ipcCalls(page, 'menu_save_product')
    expect(calls[0].input).toMatchObject({
      id: null,
      category_id: 1,
      price_cents: 300,
      vat_code: 'B',
      active: true,
      sort_order: 3,
    })
    expect(calls[0].input.names.fr).toBe('Teaé')
  })

  test('Chinese is typed in a plain field, not on the keyboard', async ({ page }) => {
    await editor(page)
    await page.getByTestId('menu-ed-add-category').click()
    await page.getByTestId('menu-ed-name-fr').click()
    await key(page, 'B').click()
    await page.getByTestId('menu-ed-name-zh').fill('饮料')
    await page.getByTestId('menu-ed-color-none').click()
    await page.getByTestId('menu-ed-save').click()
    const calls = await ipcCalls(page, 'menu_save_category')
    expect(calls[0].input).toMatchObject({
      id: null,
      names: { fr: 'B', zh: '饮料' },
      color: null,
      active: true,
    })
  })

  test('option groups and options of a saved product', async ({ page }) => {
    await editor(page)
    await page.getByTestId('menu-ed-product-row').first().click()
    await expect(page.getByTestId('menu-ed-edit-group')).toHaveCount(1)
    await page.getByTestId('menu-ed-add-group').click()
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Nouveau groupe')
    await page.getByTestId('menu-ed-name-fr').click()
    await key(page, 'S').click()
    await page.getByTestId('menu-ed-max-plus').click()
    await page.getByTestId('menu-ed-min-plus').click()
    await expect(page.getByTestId('menu-ed-min')).toHaveText('1')
    await expect(page.getByTestId('menu-ed-max')).toHaveText('2')
    // The minimum never passes the maximum.
    await page.getByTestId('menu-ed-max-minus').click()
    await page.getByTestId('menu-ed-max-minus').click()
    await expect(page.getByTestId('menu-ed-max')).toHaveText('1')
    await page.getByTestId('menu-ed-save').click()
    const groups = await ipcCalls(page, 'menu_save_modifier_group')
    expect(groups[0].input).toMatchObject({
      id: null,
      menu_item_id: 10,
      min_select: 1,
      max_select: 1,
    })
    // The group is reopened with its options.
    await expect(page.getByTestId('menu-ed-edit-option')).toHaveCount(1)
    await page.getByTestId('menu-ed-edit-option').click()
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Soja')
    await page.getByTestId('menu-ed-price').click()
    await page.locator('.numpad .key-clear').click()
    await digits(page, '75')
    await page.getByTestId('menu-ed-save').click()
    const options = await ipcCalls(page, 'menu_save_modifier')
    expect(options[0].input).toMatchObject({ id: 30, group_id: 20, price_cents: 75 })
    await page.getByTestId('menu-ed-parent').click()
    await expect(page.getByTestId('menu-ed-edit-option')).toHaveCount(1)
  })

  test('a new product asks to be saved before it takes options', async ({ page }) => {
    await editor(page)
    await page.getByTestId('menu-ed-add-product').click()
    await expect(page.getByTestId('menu-ed-add-group')).toHaveCount(0)
    // The allergen section says the same for its part.
    await expect(page.locator('.sub .hint', { hasText: 'Enregistrez le produit' })).toBeVisible()
  })

  test('a refused save shows the engine error and keeps the form', async ({ page }) => {
    await editor(page)
    await page.getByTestId('menu-ed-add-product').click()
    await overrideIPC(page, 'menu_save_product', { __reject: 'menu_invalid|field=name' })
    await page.getByTestId('menu-ed-save').click()
    await expect(page.getByTestId('menu-ed-error')).toContainText(
      'Valeur invalide dans la carte : name',
    )
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Nouveau produit')
  })

  test('Historique lists the changes with who, when and which fields', async ({ page }) => {
    await editor(page)
    await page.getByTestId('menu-ed-history-button').click()
    const rows = page.getByTestId('menu-ed-history-row')
    await expect(rows).toHaveCount(2)
    await expect(rows.nth(0)).toContainText('Modification')
    await expect(rows.nth(0)).toContainText('Maki saumon')
    await expect(rows.nth(0)).toContainText('par Bruno Moyse')
    await expect(rows.nth(0)).toContainText('prix, TVA sur place')
    await expect(rows.nth(1)).toContainText('Création')
    expect(await ipcCalls(page, 'list_menu_changes')).toEqual([{ limit: 100 }])
  })

  test('empty history says so', async ({ page }) => {
    await editor(page, { list_menu_changes: () => [] })
    await page.getByTestId('menu-ed-history-button').click()
    await expect(page.getByTestId('menu-ed-history-empty')).toBeVisible()
  })

  test('while a catalog manages the menu the editor is read-only', async ({ page }) => {
    await editor(page, { menu_admin_get: () => ({ ...MENU, managed_by: 'hubrise' }) })
    await expect(page.getByTestId('menu-ed-locked')).toContainText('HubRise')
    await expect(page.getByTestId('menu-ed-add-category')).toHaveCount(0)
    await expect(page.getByTestId('menu-ed-add-product')).toHaveCount(0)
    await page.getByTestId('menu-ed-product-row').first().click()
    await expect(page.getByTestId('menu-ed-form-title')).toHaveCount(0)
    expect(await ipcCalls(page, 'menu_save_product')).toHaveLength(0)
  })
})

const GROUP_MIN_1 = { ...MENU, groups: [{ ...MENU.groups[0], min_select: 1 }] }

async function openProduct10(page: Page) {
  await page.getByTestId('menu-ed-product-row').first().click()
  await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Maki saumon')
}

test.describe('Menu editor: visibility instead of deletion (V11-D17)', () => {
  test('"Visible à la caisse" off saves active:false on a product, and the row then says Masqué', async ({
    page,
  }) => {
    const hidden = { ...MENU, products: [{ ...MENU.products[0], active: false }, MENU.products[1]] }
    await editor(page, { menu_save_product: () => ({ id: 10, menu: hidden }) })
    await openProduct10(page)
    await expect(page.getByTestId('menu-ed-active')).toHaveClass(/on/u)
    await page.getByTestId('menu-ed-active').click()
    await expect(page.getByTestId('menu-ed-active')).not.toHaveClass(/on/u)
    await page.getByTestId('menu-ed-save').click()
    const calls = await ipcCalls(page, 'menu_save_product')
    expect(calls).toHaveLength(1)
    expect(calls[0].input).toMatchObject({ id: 10, active: false })
    await expect(page.getByTestId('menu-ed-product-row').first()).toContainText('Masqué')
    // There is no way to delete: no such button, no such command.
    await expect(page.getByRole('button', { name: /supprimer|delete/iu })).toHaveCount(0)
    const names: string[] = await page.evaluate(() =>
      (window as any).__ipcCalls.map((c: any) => c.cmd),
    )
    expect(names.filter((n) => /delete|remove/u.test(n))).toEqual([])
  })

  test('a category is hidden the same way', async ({ page }) => {
    const hidden = {
      ...MENU,
      categories: [{ ...MENU.categories[0], active: false }, MENU.categories[1]],
    }
    await editor(page, { menu_save_category: () => ({ id: 1, menu: hidden }) })
    await page.getByTestId('menu-ed-category-row').first().click()
    await page.getByTestId('menu-ed-active').click()
    await page.getByTestId('menu-ed-save').click()
    const calls = await ipcCalls(page, 'menu_save_category')
    expect(calls[0].input).toMatchObject({ id: 1, active: false })
    await expect(page.getByTestId('menu-ed-category-row').first()).toContainText('Masqué')
  })

  test('an option group and an option are hidden the same way', async ({ page }) => {
    await editor(page)
    await openProduct10(page)
    await page.getByTestId('menu-ed-edit-group').click()
    await page.getByTestId('menu-ed-active').click()
    await page.getByTestId('menu-ed-save').click()
    const groups = await ipcCalls(page, 'menu_save_modifier_group')
    expect(groups[0].input).toMatchObject({ id: 20, menu_item_id: 10, active: false })

    await page.getByTestId('menu-ed-edit-option').click()
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Soja')
    await page.getByTestId('menu-ed-active').click()
    await page.getByTestId('menu-ed-save').click()
    const options = await ipcCalls(page, 'menu_save_modifier')
    expect(options[0].input).toMatchObject({ id: 30, group_id: 20, active: false })
  })
})

test.describe('Menu editor: the other fields', () => {
  test('product: code on the keyboard, kitchen printer, takeaway VAT and position', async ({
    page,
  }) => {
    await editor(page)
    await openProduct10(page)
    await page.getByTestId('menu-ed-code').click()
    await page
      .getByTestId('text-keyboard')
      .getByRole('button', { name: 'Effacer', exact: true })
      .click()
    await type(page, 'B2')
    await expect(page.getByTestId('menu-ed-code')).toHaveValue('B2')
    await page.getByTestId('menu-ed-kitchen').click()
    await page.getByTestId('menu-ed-vat-takeaway-A').click()
    await page.getByTestId('menu-ed-pos-plus').click()
    await page.getByTestId('menu-ed-pos-plus').click()
    await expect(page.getByTestId('menu-ed-pos')).toHaveText('3')
    await page.getByTestId('menu-ed-save').click()
    const calls = await ipcCalls(page, 'menu_save_product')
    expect(calls[0].input).toMatchObject({
      id: 10,
      code: 'B2',
      kitchen_printer: false,
      vat_code_takeaway: 'A',
      sort_order: 3,
    })
  })

  test('the product code keeps 12 characters and a name 60', async ({ page }) => {
    await editor(page)
    await openProduct10(page)
    await page.getByTestId('menu-ed-code').click()
    await page
      .getByTestId('text-keyboard')
      .getByRole('button', { name: 'Effacer', exact: true })
      .click()
    await type(page, 'ABCDEFGHIJKLM')
    await expect(page.getByTestId('menu-ed-code')).toHaveValue('ABCDEFGHIJKL')
    await page.getByTestId('menu-ed-name-fr').click()
    await page
      .getByTestId('text-keyboard')
      .getByRole('button', { name: 'Effacer', exact: true })
      .click()
    for (let i = 0; i < 61; i++) await key(page, 'A').click()
    await expect(page.getByTestId('menu-ed-name-fr')).toHaveValue('A'.repeat(60))
  })

  test('the position never goes below 0', async ({ page }) => {
    await editor(page)
    await openProduct10(page)
    await expect(page.getByTestId('menu-ed-pos')).toHaveText('1')
    await page.getByTestId('menu-ed-pos-minus').click()
    await page.getByTestId('menu-ed-pos-minus').click()
    await page.getByTestId('menu-ed-pos-minus').click()
    await expect(page.getByTestId('menu-ed-pos')).toHaveText('0')
  })

  test('updating a category: palette colour and an nl name, the other languages kept', async ({
    page,
  }) => {
    await editor(page)
    await page.getByTestId('menu-ed-category-row').first().click()
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Sushi')
    await page.getByRole('button', { name: '#22C55E' }).click()
    await page.getByTestId('menu-ed-name-nl').click()
    await type(page, 'XY')
    await page.getByTestId('menu-ed-name-en').click()
    await type(page, 'Z')
    await page.getByTestId('menu-ed-save').click()
    const calls = await ipcCalls(page, 'menu_save_category')
    expect(calls).toHaveLength(1)
    expect(calls[0].input).toMatchObject({ id: 1, color: '#22C55E', active: true, sort_order: 1 })
    expect(calls[0].input.names).toEqual({ fr: 'Sushi', zh: '寿司', nl: 'XY', en: 'Z' })
  })
})

test.describe('Menu editor: option groups', () => {
  test('a new option in a group takes the next position and its price from the NumPad', async ({
    page,
  }) => {
    await editor(page)
    await openProduct10(page)
    await page.getByTestId('menu-ed-edit-group').click()
    await page.getByTestId('menu-ed-add-option').click()
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Nouvelle option')
    await page.getByTestId('menu-ed-name-fr').click()
    await key(page, 'W').click()
    await page.getByTestId('menu-ed-price').click()
    await digits(page, '120')
    await expect(page.getByTestId('menu-ed-price')).toHaveValue('1,20')
    await page.getByTestId('menu-ed-save').click()
    const calls = await ipcCalls(page, 'menu_save_modifier')
    expect(calls).toHaveLength(1)
    expect(calls[0].input).toMatchObject({
      id: null,
      group_id: 20,
      price_cents: 120,
      sort_order: 2,
      active: true,
    })
    expect(calls[0].input.names).toEqual({ fr: 'W' })
  })

  test('the minimum can be lowered down to 0 and leaves the maximum alone', async ({ page }) => {
    await editor(page, { menu_admin_get: () => GROUP_MIN_1 })
    await openProduct10(page)
    await page.getByTestId('menu-ed-edit-group').click()
    await expect(page.getByTestId('menu-ed-min')).toHaveText('1')
    await page.getByTestId('menu-ed-min-minus').click()
    await expect(page.getByTestId('menu-ed-min')).toHaveText('0')
    await page.getByTestId('menu-ed-min-minus').click()
    await expect(page.getByTestId('menu-ed-min')).toHaveText('0')
    await expect(page.getByTestId('menu-ed-max')).toHaveText('2')
  })
})

test.describe('Menu editor: other errors', () => {
  test('a product that vanished meanwhile is told', async ({ page }) => {
    await editor(page)
    await openProduct10(page)
    await overrideIPC(page, 'menu_save_product', { __reject: 'menu_not_found' })
    await page.getByTestId('menu-ed-save').click()
    await expect(page.getByTestId('menu-ed-error')).toHaveText(
      "Cet élément de la carte n'existe plus.",
    )
  })

  test('a menu that a catalog took over meanwhile is told', async ({ page }) => {
    await editor(page)
    await openProduct10(page)
    await overrideIPC(page, 'menu_save_product', { __reject: 'hubrise_menu_managed' })
    await page.getByTestId('menu-ed-save').click()
    await expect(page.getByTestId('menu-ed-error')).toHaveText(
      'Le menu est géré par HubRise : modifiez-le là.',
    )
  })

  test('a menu that cannot be read shows the error on the editor, and Settings still offers the editor', async ({
    page,
  }) => {
    const failing = {
      menu_admin_get: live(() => {
        throw 'forbidden'
      }),
    }
    await mockTauriIPC(page, failing)
    await loginAndGoto(page, '/menu-editor')
    await expect(page.getByTestId('menu-editor-view')).toBeVisible()
    await expect(page.getByRole('alert')).toHaveText('Action réservée à un autre profil.')
    await expect(page.getByTestId('menu-ed-category-row')).toHaveCount(0)

    await page.evaluate(() => {
      window.location.hash = '/settings'
    })
    await page.waitForURL(/settings/u)
    await expect(page.getByTestId('menu-editor-open')).toBeEnabled()
  })

  test('a history that cannot be read shows the error', async ({ page }) => {
    await editor(page, {
      list_menu_changes: live(() => {
        throw 'forbidden'
      }),
    })
    await page.getByTestId('menu-ed-history-button').click()
    await expect(page.getByRole('alert')).toHaveText('Action réservée à un autre profil.')
  })
})

test.describe('Menu editor: leaving', () => {
  test('Annuler closes the form without saving', async ({ page }) => {
    await editor(page)
    await openProduct10(page)
    await page.getByTestId('menu-ed-price').click()
    await page.locator('.numpad .key-clear').click()
    await digits(page, '999')
    await page.getByTestId('menu-ed-cancel').click()
    await expect(page.getByTestId('menu-ed-hint')).toBeVisible()
    await expect(page.getByTestId('menu-ed-form-title')).toHaveCount(0)
    expect(await ipcCalls(page, 'menu_save_product')).toHaveLength(0)
    // The list still shows the saved price.
    await expect(page.getByTestId('menu-ed-product-row').first()).toContainText('6,50')
  })

  test('Retour goes back to Settings', async ({ page }) => {
    await editor(page)
    await page.getByTestId('menu-ed-back').click()
    await page.waitForURL(/settings/u)
    await expect(page.getByTestId('menu-editor-panel')).toBeVisible()
  })
})

test.describe('Menu editor: kitchen course and allergens (lane 4 in the editor)', () => {
  const COURSES = {
    enabled: true,
    category_courses: [
      { category_id: 1, course: 1 },
      { category_id: 7, course: 3 },
    ],
  }

  test('a category shows its course and a new one is saved with the others kept', async ({
    page,
  }) => {
    await editor(page, {
      get_course_settings: () => COURSES,
      set_course_settings: live((args: any) => args.input),
    })
    await page.getByTestId('menu-ed-category-row').first().click()
    await expect(page.getByTestId('menu-ed-course-1')).toHaveClass(/on/u)
    await expect(page.getByTestId('menu-ed-course-off')).toHaveCount(0)
    await page.getByTestId('menu-ed-course-2').click()
    await page.getByTestId('menu-ed-save').click()
    await expect(page.locator('.toast', { hasText: 'Carte enregistrée' })).toBeVisible()
    const calls = await ipcCalls(page, 'set_course_settings')
    expect(calls).toHaveLength(1)
    expect(calls[0].input.enabled).toBe(true)
    expect(calls[0].input.category_courses).toEqual([
      { category_id: 7, course: 3 },
      { category_id: 1, course: 2 },
    ])
  })

  test('Direct takes the category out of the mapping; an unchanged course writes nothing', async ({
    page,
  }) => {
    await editor(page, {
      get_course_settings: () => COURSES,
      set_course_settings: live((args: any) => args.input),
    })
    await page.getByTestId('menu-ed-category-row').first().click()
    await page.getByTestId('menu-ed-save').click()
    await expect(page.locator('.toast', { hasText: 'Carte enregistrée' })).toBeVisible()
    expect(await ipcCalls(page, 'set_course_settings')).toHaveLength(0)
    await page.getByTestId('menu-ed-course-0').click()
    await page.getByTestId('menu-ed-save').click()
    const calls = await ipcCalls(page, 'set_course_settings')
    expect(calls[0].input.category_courses).toEqual([{ category_id: 7, course: 3 }])
  })

  test('with courses off the picker says the choice waits for the switch', async ({ page }) => {
    await editor(page)
    await page.getByTestId('menu-ed-category-row').first().click()
    await expect(page.getByTestId('menu-ed-course-0')).toHaveClass(/on/u)
    await expect(page.getByTestId('menu-ed-course-off')).toBeVisible()
  })

  test('a declared product shows its allergens; a tap cycles contains, traces, nothing', async ({
    page,
  }) => {
    await editor(page)
    await page.getByTestId('menu-ed-product-row').first().click()
    await expect(page.getByTestId('menu-ed-allergens-declared')).toHaveClass(/on/u)
    await expect(page.getByTestId('menu-ed-allergen-fish')).toHaveAttribute(
      'data-level',
      'contains',
    )
    await expect(page.getByTestId('menu-ed-allergen-sesame')).toHaveAttribute(
      'data-level',
      'traces',
    )
    await expect(page.getByTestId('menu-ed-allergen-milk')).toHaveAttribute('data-level', 'none')
    await page.getByTestId('menu-ed-allergen-milk').click()
    await expect(page.getByTestId('menu-ed-allergen-milk')).toHaveAttribute(
      'data-level',
      'contains',
    )
    await page.getByTestId('menu-ed-allergen-fish').click()
    await expect(page.getByTestId('menu-ed-allergen-fish')).toHaveAttribute('data-level', 'traces')
    await page.getByTestId('menu-ed-allergen-sesame').click()
    await expect(page.getByTestId('menu-ed-allergen-sesame')).toHaveAttribute('data-level', 'none')
    await page.getByTestId('menu-ed-save').click()
    await expect(page.locator('.toast', { hasText: 'Carte enregistrée' })).toBeVisible()
    const calls = await ipcCalls(page, 'set_allergens')
    expect(calls).toHaveLength(1)
    expect(calls[0].input).toMatchObject({
      entity_type: 'menu_item',
      entity_id: 10,
      declared: true,
    })
    expect([...calls[0].input.contains].sort((a, b) => a.localeCompare(b))).toEqual([
      'milk',
      'soybeans',
    ])
    expect(calls[0].input.may_contain).toEqual(['fish'])
    // The editor reads the menu again for the new list.
    expect((await ipcCalls(page, 'menu_admin_get')).length).toBeGreaterThanOrEqual(2)
  })

  test('an entry with no information can be declared with no allergen', async ({ page }) => {
    await editor(page)
    await page.getByTestId('menu-ed-product-row').first().click()
    await page.getByTestId('menu-ed-edit-group').click()
    await page.getByTestId('menu-ed-edit-option').click()
    await expect(page.getByTestId('menu-ed-allergens-unknown')).toBeVisible()
    await page.getByTestId('menu-ed-allergens-declared').click()
    await expect(page.getByTestId('menu-ed-allergens-none')).toBeVisible()
    await page.getByTestId('menu-ed-save').click()
    const calls = await ipcCalls(page, 'set_allergens')
    expect(calls[0].input).toEqual({
      entity_type: 'modifier',
      entity_id: 30,
      declared: true,
      contains: [],
      may_contain: [],
    })
  })

  test('a declared product goes back to "no information"', async ({ page }) => {
    await editor(page)
    await page.getByTestId('menu-ed-product-row').first().click()
    await page.getByTestId('menu-ed-allergens-declared').click()
    await expect(page.getByTestId('menu-ed-allergens-unknown')).toBeVisible()
    await page.getByTestId('menu-ed-save').click()
    const calls = await ipcCalls(page, 'set_allergens')
    expect(calls[0].input).toEqual({
      entity_type: 'menu_item',
      entity_id: 10,
      declared: false,
      contains: [],
      may_contain: [],
    })
  })

  test('saving a product without touching its allergens does not write them', async ({ page }) => {
    await editor(page)
    await page.getByTestId('menu-ed-product-row').first().click()
    await page.getByTestId('menu-ed-save').click()
    await expect(page.locator('.toast', { hasText: 'Carte enregistrée' })).toBeVisible()
    expect(await ipcCalls(page, 'set_allergens')).toHaveLength(0)
  })

  test('an option takes its allergens as a modifier', async ({ page }) => {
    await editor(page)
    await page.getByTestId('menu-ed-product-row').first().click()
    await page.getByTestId('menu-ed-edit-group').click()
    await page.getByTestId('menu-ed-edit-option').click()
    await page.getByTestId('menu-ed-allergens-declared').click()
    await page.getByTestId('menu-ed-allergen-soybeans').click()
    await page.getByTestId('menu-ed-save').click()
    const calls = await ipcCalls(page, 'set_allergens')
    expect(calls[0].input).toEqual({
      entity_type: 'modifier',
      entity_id: 30,
      declared: true,
      contains: ['soybeans'],
      may_contain: [],
    })
  })

  test('a new product asks to be saved before it takes allergens', async ({ page }) => {
    await editor(page)
    await page.getByTestId('menu-ed-add-product').click()
    await expect(page.getByTestId('menu-ed-allergens')).toContainText(
      "Enregistrez l'entrée pour lui donner ses allergènes.",
    )
    await expect(page.getByTestId('menu-ed-allergens-declared')).toHaveCount(0)
  })

  test('the allergen editor is locked with the rest while a catalog manages the menu', async ({
    page,
  }) => {
    await editor(page, { menu_admin_get: () => ({ ...MENU, managed_by: 'hubrise' }) })
    await expect(page.getByTestId('menu-ed-locked')).toBeVisible()
    await page.getByTestId('menu-ed-product-row').first().click()
    await expect(page.getByTestId('menu-ed-allergens')).toHaveCount(0)
  })
})
