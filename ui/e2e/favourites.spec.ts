import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  overrideIPC,
  tapUserTile,
  loginAndGoto,
  live,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc'

// UX audit C12: the "Favoris" tab. The first tab of the order screen, before the categories, hidden while
// it has no product. Its products come from the engine (the 30-day best sellers, or the manager's
// selection); a product there behaves like in its category: code, sold-out mark, add to the order. A
// manager picks the mode in Settings (panel "Favoris") and stars products in the menu editor. The
// computation of the ranking is tested in crates/pos-engine/tests/favourites.rs.

const CASHIER = { ...DEFAULT_USER, insz: '84022899837', name: 'Elisa Serveuse', role: 'cashier' }
const DRINKS = { ...DEFAULT_CATEGORY, id: 2, name: 'Boissons', color: '#00d4ff', sort_order: 1 }

const MAKI = { ...DEFAULT_MENU_ITEM, id: 1, category_id: 1, name: 'Maki saumon x8', code: 'A1' }
const SAKE = { ...DEFAULT_MENU_ITEM, id: 7, category_id: 2, name: 'Saké', price_cents: 650 }
const EDAMAME = { ...DEFAULT_MENU_ITEM, id: 9, category_id: 1, name: 'Edamame', sold_out: true }

async function toOrder(page: Page, extra: Record<string, unknown> = {}, user = DEFAULT_USER) {
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 0 }),
    list_cart_lines: () => [],
    get_categories: () => [DEFAULT_CATEGORY, DRINKS],
    get_menu_items: ({ categoryId }: any) =>
      categoryId === 2 ? [{ ...SAKE }] : [{ ...MAKI }, { ...EDAMAME }],
    ...extra,
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/u)
}

const favourites = (items: unknown[]) => ({ list_favourites: () => items })
const tab = (page: Page) => page.getByTestId('tab-favourites')

test.describe('The Favoris tab of the order screen', () => {
  test('is hidden while there is no favourite, and the first category opens', async ({ page }) => {
    await toOrder(page)
    await expect(tab(page)).toHaveCount(0)
    await expect(page.locator('.tab')).toHaveCount(2)
    await expect(page.locator('.tab.active')).toContainText('Sushi')
    await expect(page.locator('.item-btn').first()).toContainText('Maki saumon x8')
  })

  test('shows first, before the categories, with a star and opens by default', async ({ page }) => {
    await toOrder(page, favourites([{ ...SAKE }, { ...MAKI }]))
    await expect(tab(page)).toBeVisible()
    await expect(tab(page)).toContainText('Favoris')
    await expect(tab(page).locator('svg')).toBeVisible()
    const labels = await page.locator('.category-tabs .tab').allInnerTexts()
    expect(labels[0]).toContain('Favoris')
    expect(labels[1]).toContain('Sushi')
    await expect(tab(page)).toHaveClass(/active/u)
    await expect(page.locator('.item-btn')).toHaveText([/Saké/u, /Maki saumon/u])
    expect((await tab(page).boundingBox())?.height).toBeGreaterThanOrEqual(48)
  })

  test('a product added from the tab goes to the order like from its category', async ({
    page,
  }) => {
    await toOrder(page, {
      ...favourites([{ ...SAKE }]),
      add_order_line: live(() => ({
        id: 5,
        order_id: 1,
        menu_item_id: 7,
        quantity: 1,
        unit_price_cents: 650,
        vat_code: 'B',
        name: 'Saké',
      })),
    })
    await page.locator('.item-btn', { hasText: 'Saké' }).click()
    const calls = await ipcCalls(page, 'add_order_line')
    expect(calls).toHaveLength(1)
    expect(calls[0].input.menu_item_id).toBe(7)
    expect(calls[0].input.quantity).toBe(1)
  })

  test('a tab of a category leaves the favourites and a tap on Favoris comes back', async ({
    page,
  }) => {
    await toOrder(page, favourites([{ ...SAKE }]))
    await page.locator('.tab', { hasText: 'Sushi' }).click()
    await expect(page.locator('.tab.active')).toContainText('Sushi')
    await expect(page.locator('.item-btn').first()).toContainText('Maki saumon x8')
    await tab(page).click()
    await expect(tab(page)).toHaveClass(/active/u)
    await expect(page.locator('.item-btn')).toHaveCount(1)
    await expect(page.locator('.item-btn')).toContainText('Saké')
  })

  test('keeps the colour of each product own category', async ({ page }) => {
    await toOrder(page, favourites([{ ...SAKE }, { ...MAKI }]))
    const border = (name: string) =>
      page
        .locator('.item-btn', { hasText: name })
        .evaluate((el) => getComputedStyle(el).borderLeftColor)
    // Boissons is cyan (#00d4ff), Sushi red (#e94560): the tab does not paint them all alike.
    expect(await border('Saké')).toBe('rgb(0, 212, 255)')
    expect(await border('Maki saumon')).toBe('rgb(233, 69, 96)')
  })

  test('marks a sold-out product like a category does, and refuses it with the same message', async ({
    page,
  }) => {
    await toOrder(page, favourites([{ ...EDAMAME }, { ...SAKE }]))
    await expect(page.getByTestId('soldout-badge')).toHaveCount(1)
    await page.locator('.item-btn', { hasText: 'Edamame' }).click()
    await expect(page.locator('.toast.error, .toast', { hasText: /épuisé/iu })).toBeVisible()
    expect(await ipcCalls(page, 'add_order_line')).toHaveLength(0)
  })

  test('the code pad finds a product by its code from the tab, as from a category', async ({
    page,
  }) => {
    await toOrder(page, {
      ...favourites([{ ...SAKE }]),
      add_order_line: live(() => ({
        id: 5,
        order_id: 1,
        menu_item_id: 1,
        quantity: 1,
        unit_price_cents: 1200,
        vat_code: 'C',
        name: 'Maki saumon x8',
      })),
    })
    await page.getByTestId('code-pad-toggle').click()
    await page.getByTestId('code-key-A').click()
    await page.getByTestId('code-key-1').click()
    await expect(page.getByTestId('code-pad-message')).toContainText('Maki saumon x8')
    await page.getByTestId('code-key-ok').click()
    await expect.poll(async () => (await ipcCalls(page, 'add_order_line')).length).toBe(1)
    expect((await ipcCalls(page, 'add_order_line'))[0].input.menu_item_id).toBe(1)
  })

  test('the allergen mode takes the tap on a favourite like on a category product', async ({
    page,
  }) => {
    await toOrder(page, {
      ...favourites([{ ...MAKI }]),
      get_allergen_sheet: () => ({ contains: [], may_contain: [], declared: false }),
    })
    await page.getByTestId('allergen-mode').click()
    await page.locator('.item-btn', { hasText: 'Maki saumon' }).click()
    await expect(page.getByTestId('allergen-sheet')).toBeVisible()
    expect((await ipcCalls(page, 'get_allergen_sheet'))[0].menuItemId).toBe(1)
    expect(await ipcCalls(page, 'add_order_line')).toHaveLength(0)
  })

  test('falls back to the first category when the favourites empty out', async ({ page }) => {
    await toOrder(page, favourites([{ ...SAKE }]))
    await expect(tab(page)).toHaveClass(/active/u)
    await overrideIPC(page, 'list_favourites', [])
    await page.locator('.tab', { hasText: 'Sushi' }).click()
    await tab(page).click()
    await expect(page.locator('.tab.active')).toContainText('Sushi')
    await expect(tab(page)).toHaveCount(0)
  })

  test('asks the engine in the language of the screen', async ({ page }) => {
    await toOrder(page, favourites([{ ...SAKE }]))
    const calls = await ipcCalls(page, 'list_favourites')
    expect(calls.length).toBeGreaterThan(0)
    expect(calls[0]).toEqual({ lang: 'fr' })
  })

  test('a cashier sees the tab too', async ({ page }) => {
    await toOrder(page, favourites([{ ...SAKE }]), CASHIER)
    await expect(tab(page)).toBeVisible()
  })
})

// ── Settings: the Favoris panel ─────────────────────────────────────

async function toSettings(page: Page, extra: Record<string, unknown> = {}, user = DEFAULT_USER) {
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    get_menu_items: () => [{ ...MAKI }, { ...SAKE }],
    ...extra,
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.evaluate(() => {
    window.location.hash = '/settings'
  })
  await page.waitForURL(/settings/u)
}

test.describe('The Favoris panel of Settings', () => {
  test('is for a manager only', async ({ page }) => {
    await toSettings(page, {}, CASHIER)
    await expect(page.getByTestId('favourites-panel')).toHaveCount(0)
  })

  test('shows the 30-day best sellers by default and explains it', async ({ page }) => {
    await toSettings(page)
    const panel = page.getByTestId('favourites-panel')
    await expect(panel.locator('h2')).toHaveText('Favoris')
    await expect(page.getByTestId('favourites-mode-top_sold')).toHaveAttribute(
      'aria-checked',
      'true',
    )
    await expect(page.getByTestId('favourites-top-note')).toContainText('18 produits')
    await expect(page.getByTestId('favourites-row')).toHaveCount(0)
    for (const mode of ['top_sold', 'selection']) {
      expect(
        (await page.getByTestId(`favourites-mode-${mode}`).boundingBox())?.height,
      ).toBeGreaterThanOrEqual(48)
    }
  })

  test('the mode switches at once and keeps the selection', async ({ page }) => {
    await toSettings(page, {
      get_favourites_settings: () => ({ mode: 'top_sold', selection: [7, 1] }),
    })
    await page.getByTestId('favourites-mode-selection').click()
    const calls = await ipcCalls(page, 'set_favourites_settings')
    expect(calls).toEqual([{ input: { mode: 'selection', selection: [7, 1] } }])
    await expect(page.getByTestId('favourites-mode-selection')).toHaveAttribute(
      'aria-checked',
      'true',
    )
    await expect(page.getByTestId('favourites-row')).toHaveText([/Saké/u, /Maki saumon/u])
    await expect(page.getByTestId('favourites-count')).toHaveText('2 sur 30')
  })

  test('removes a product of the selection, keeping the order of the others', async ({ page }) => {
    await toSettings(page, {
      get_favourites_settings: () => ({ mode: 'selection', selection: [7, 1, 9] }),
    })
    await page.getByTestId('favourites-row').first().getByTestId('favourites-remove').click()
    expect(await ipcCalls(page, 'set_favourites_settings')).toEqual([
      { input: { mode: 'selection', selection: [1, 9] } },
    ])
    await expect(page.getByTestId('favourites-row')).toHaveCount(2)
    // A product that left the menu shows its number.
    await expect(page.getByTestId('favourites-row').nth(1)).toContainText('#9')
  })

  test('says so when the selection is empty and points to the star', async ({ page }) => {
    await toSettings(page, {
      get_favourites_settings: () => ({ mode: 'selection', selection: [] }),
    })
    await expect(page.getByTestId('favourites-empty')).toBeVisible()
    await expect(page.getByTestId('favourites-panel')).toContainText('étoile')
  })

  test('shows the engine refusal', async ({ page }) => {
    await toSettings(page, {
      set_favourites_settings: () => {
        throw new Error('x')
      },
    })
    await overrideIPC(page, 'set_favourites_settings', { __reject: 'favourites_too_many|max=30' })
    await page.getByTestId('favourites-mode-selection').click()
    await expect(page.getByTestId('favourites-error')).toContainText('30')
  })
})

// ── Menu editor: the star ────────────────────────────────────────────

const MENU = {
  managed_by: null,
  languages: ['fr'],
  categories: [{ id: 1, names: { fr: 'Sushi' }, sort_order: 1, color: '#E94560', active: true }],
  products: [
    {
      id: 10,
      category_id: 1,
      names: { fr: 'Maki saumon' },
      price_cents: 650,
      vat_code: 'B',
      vat_code_takeaway: null,
      kitchen_printer: true,
      code: null,
      open_price: false,
      sold_by_weight: false,
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
      sold_by_weight: false,
      sort_order: 2,
      active: true,
    },
  ],
  groups: [],
  options: [],
  allergens: [],
}

test.describe('The star of the menu editor', () => {
  test('shows the starred products and toggles one at once, in the order of addition', async ({
    page,
  }) => {
    await mockTauriIPC(page, {
      menu_admin_get: () => MENU,
      list_menu_changes: () => [],
      get_favourites_settings: () => ({ mode: 'selection', selection: [11] }),
      set_favourite: live(({ menuItemId, favourite }: any) => {
        const w = globalThis as any
        w.__stars ??= [11]
        w.__stars = favourite
          ? [...w.__stars.filter((x: number) => x !== menuItemId), menuItemId]
          : w.__stars.filter((x: number) => x !== menuItemId)
        return { mode: 'selection', selection: w.__stars }
      }),
    })
    await loginAndGoto(page, '/menu-editor')
    const stars = page.getByTestId('menu-ed-star')
    await expect(stars).toHaveCount(2)
    await expect(stars.nth(0)).toHaveAttribute('aria-pressed', 'false')
    await expect(stars.nth(1)).toHaveAttribute('aria-pressed', 'true')
    expect((await stars.nth(0).boundingBox())?.width).toBeGreaterThanOrEqual(48)
    expect((await stars.nth(0).boundingBox())?.height).toBeGreaterThanOrEqual(48)

    await stars.nth(0).click()
    expect(await ipcCalls(page, 'set_favourite')).toEqual([{ menuItemId: 10, favourite: true }])
    await expect(stars.nth(0)).toHaveAttribute('aria-pressed', 'true')

    await stars.nth(1).click()
    expect((await ipcCalls(page, 'set_favourite'))[1]).toEqual({ menuItemId: 11, favourite: false })
    await expect(stars.nth(1)).toHaveAttribute('aria-pressed', 'false')
  })

  test('works while a catalog manages the menu (local data)', async ({ page }) => {
    await mockTauriIPC(page, {
      menu_admin_get: () => ({ ...MENU, managed_by: 'hubrise' }),
      list_menu_changes: () => [],
    })
    await loginAndGoto(page, '/menu-editor')
    await page.getByTestId('menu-ed-star').first().click()
    expect(await ipcCalls(page, 'set_favourite')).toHaveLength(1)
  })

  test('shows the refusal of the engine', async ({ page }) => {
    await mockTauriIPC(page, { menu_admin_get: () => MENU, list_menu_changes: () => [] })
    await loginAndGoto(page, '/menu-editor')
    await overrideIPC(page, 'set_favourite', { __reject: 'favourites_too_many|max=30' })
    await page.getByTestId('menu-ed-star').first().click()
    await expect(page.locator('.toast', { hasText: '30' })).toBeVisible()
  })
})
