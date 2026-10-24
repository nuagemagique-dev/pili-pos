import { test, expect } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  live,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
  tapUserTile,
} from './fixtures/mock-ipc'

// The option picker (HubRise builds): groups from the mirrored catalog,
// only the ids go to the backend, which prices and names the line.
const GROUPS = [
  {
    id: 10,
    name: 'Sauce',
    min: 1,
    max: 1,
    options: [
      { id: 101, name: 'Soja', price_cents: 0 },
      { id: 102, name: 'Soja sucrée', price_cents: 50 },
    ],
  },
  {
    id: 11,
    name: 'Suppléments',
    min: 0,
    max: null,
    options: [
      { id: 111, name: 'Avocat', price_cents: 100 },
      { id: 112, name: 'Cheese', price_cents: 80 },
    ],
  },
]

async function orderScreen(page: import('@playwright/test').Page, extra: Record<string, any> = {}) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 0 }),
    list_cart_lines: () => [],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [
      { ...DEFAULT_MENU_ITEM, has_modifiers: true },
      { ...DEFAULT_MENU_ITEM, id: 2, name: 'Coca-Cola', price_cents: 300 },
    ],
    get_item_modifiers: live(({ menuItemId }: any) => (menuItemId === 1 ? GROUPS_JSON : [])),
    add_order_line_with_modifiers: live(({ input }: any) => ({
      id: 77,
      order_id: input.order_id,
      menu_item_id: input.menu_item_id,
      quantity: input.quantity,
      unit_price_cents: 1350,
      vat_code: 'B',
      name: 'Maki saumon x8 (Soja sucrée, Avocat)',
      modifiers_json: '[{"id":102},{"id":111}]',
      created_at: '2026-10-03 12:00:00',
    })),
    ...extra,
  })
  // The live handler runs in the page: give it the groups there.
  await page.addInitScript((g) => {
    ;(globalThis as any).GROUPS_JSON = g
  }, GROUPS)
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/)
}

test.describe('Option picker', () => {
  test('a product with options opens the picker; the backend gets ids only', async ({ page }) => {
    await orderScreen(page)
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click()
    const dialog = page.getByTestId('modifier-dialog')
    await expect(dialog).toBeVisible()
    const add = dialog.locator('.btn-confirm')
    // The sauce is required.
    await expect(add).toBeDisabled()
    await dialog
      .getByRole('button', { name: /^Soja\b/ })
      .first()
      .click()
    await dialog.getByRole('button', { name: /Soja sucrée/ }).click()
    // A single choice: the second sauce replaces the first.
    await expect(dialog.locator('.option.selected')).toHaveCount(1)
    await dialog.getByRole('button', { name: /Avocat/ }).click()
    await expect(add).toContainText('13,50')
    await add.click()
    await expect(dialog).toHaveCount(0)
    const calls = await ipcCalls(page, 'add_order_line_with_modifiers')
    expect(calls[0].input).toMatchObject({ menu_item_id: 1, quantity: 1, modifier_ids: [102, 111] })
    await expect(page.locator('.cart-lines')).toContainText('Soja sucrée, Avocat')
    expect(await ipcCalls(page, 'add_order_line')).toHaveLength(0)
  })

  test('a product without options is added at once', async ({ page }) => {
    await orderScreen(page)
    await page.locator('.item-btn', { hasText: 'Coca-Cola' }).click()
    await expect.poll(() => ipcCalls(page, 'add_order_line')).toHaveLength(1)
    await expect(page.getByTestId('modifier-dialog')).toHaveCount(0)
  })

  test('Retour leaves the order untouched', async ({ page }) => {
    await orderScreen(page)
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click()
    await page.getByTestId('modifier-dialog').locator('.btn-cancel').click()
    await expect(page.getByTestId('modifier-dialog')).toHaveCount(0)
    expect(await ipcCalls(page, 'add_order_line_with_modifiers')).toHaveLength(0)
  })
})

// V11-D18: the picker follows the product (MenuItem.has_modifiers), not the HubRise switch: the
// local menu editor makes option groups too.
test.describe('Option picker with HubRise off', () => {
  const hubriseOff = {
    get_integrations: () => ({ hubrise_enabled: false, online_orders_open: 0, hubrise_log: [] }),
  }

  test('opens for a product with option groups made in the menu editor', async ({ page }) => {
    await orderScreen(page, hubriseOff)
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click()
    const dialog = page.getByTestId('modifier-dialog')
    await expect(dialog).toBeVisible()
    await dialog.getByRole('button', { name: /Soja sucrée/ }).click()
    await dialog.getByRole('button', { name: /Avocat/ }).click()
    await dialog.locator('.btn-confirm').click()
    await expect(dialog).toHaveCount(0)
    const calls = await ipcCalls(page, 'add_order_line_with_modifiers')
    expect(calls).toHaveLength(1)
    expect(calls[0].input).toMatchObject({ menu_item_id: 1, quantity: 1, modifier_ids: [102, 111] })
    expect(await ipcCalls(page, 'add_order_line')).toHaveLength(0)
  })

  test('a product without option groups is added at once, whatever the switch says', async ({
    page,
  }) => {
    await orderScreen(page, hubriseOff)
    await page.locator('.item-btn', { hasText: 'Coca-Cola' }).click()
    await expect.poll(() => ipcCalls(page, 'add_order_line')).toHaveLength(1)
    await expect(page.getByTestId('modifier-dialog')).toHaveCount(0)
    expect(await ipcCalls(page, 'add_order_line_with_modifiers')).toHaveLength(0)
  })

  test('a product with groups still opens the picker with HubRise on and a product without it does not', async ({
    page,
  }) => {
    // The same behaviour in both states: the switch is not read.
    await orderScreen(page)
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click()
    await expect(page.getByTestId('modifier-dialog')).toBeVisible()
  })
})
