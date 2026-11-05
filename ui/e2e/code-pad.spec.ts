import { test, expect } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
  tapUserTile,
} from './fixtures/mock-ipc'

// Order entry by menu code (the "KeyB" pad of Xudox): code pad of the order screen.
const MAKI = { ...DEFAULT_MENU_ITEM, id: 1, name: 'Maki saumon x8', code: 'A12', price_cents: 1200 }
const COLA = { ...DEFAULT_MENU_ITEM, id: 2, name: 'Cola', code: 'B3', price_cents: 300 }
const MAKI_THON = { ...DEFAULT_MENU_ITEM, id: 3, name: 'Maki thon', code: 'A1', price_cents: 1100 }

test.describe('Code pad', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.removeItem('pili-pos-code-pad')
    })
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 0 }),
      list_cart_lines: () => [],
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [MAKI, COLA, MAKI_THON],
    })
    await page.goto('/')
    await tapUserTile(page)
    await page.waitForURL(/tables/u)
    await page.locator('.table-card').first().click()
    await page.waitForURL(/order/u)
    await page.getByTestId('code-pad-toggle').click()
    await expect(page.getByTestId('code-pad')).toBeVisible()
  })

  test('shows the letters the menu codes use', async ({ page }) => {
    await expect(page.getByTestId('code-key-A')).toBeVisible()
    await expect(page.getByTestId('code-key-B')).toBeVisible()
    await expect(page.getByTestId('code-key-C')).toHaveCount(0)
  })

  test('letter keys are big enough for a finger', async ({ page }) => {
    const box = await page.getByTestId('code-key-A').boundingBox()
    expect(box?.width).toBeGreaterThanOrEqual(60)
    expect(box?.height).toBeGreaterThanOrEqual(64)
  })

  test('a quantity and a code add that many', async ({ page }) => {
    for (const k of ['3', 'times', 'A', '1', '2']) await page.getByTestId(`code-key-${k}`).click()
    await expect(page.getByTestId('code-pad-qty')).toHaveText('3 ×')
    await expect(page.getByTestId('code-pad-display')).toContainText('A12')
    await expect(page.getByTestId('code-pad-message')).toContainText('3 × Maki saumon x8')
    await page.getByTestId('code-key-ok').click()

    await expect.poll(async () => (await ipcCalls(page, 'add_order_line')).length).toBe(1)
    const [call] = await ipcCalls(page, 'add_order_line')
    expect(call.input.menu_item_id).toBe(1)
    expect(call.input.quantity).toBe(3)
    await expect(page.getByTestId('code-pad-message')).toContainText('✓')
  })

  test('an unknown code is refused and nothing is added', async ({ page }) => {
    for (const k of ['B', '9']) await page.getByTestId(`code-key-${k}`).click()
    await page.getByTestId('code-key-ok').click()
    await expect(page.getByTestId('code-pad-message')).toHaveText('Code inconnu : B9')
    expect(await ipcCalls(page, 'add_order_line')).toHaveLength(0)
  })

  test('a physical keyboard types into the pad', async ({ page }) => {
    await page.keyboard.type('b3')
    await expect(page.getByTestId('code-pad-display')).toContainText('B3')
    await page.keyboard.press('Enter')
    await expect.poll(async () => (await ipcCalls(page, 'add_order_line')).length).toBe(1)
    const [call] = await ipcCalls(page, 'add_order_line')
    expect(call.input.menu_item_id).toBe(2)
  })

  test('a waiting multiplier applies to the product tapped in the grid', async ({ page }) => {
    for (const k of ['2', 'times']) await page.getByTestId(`code-key-${k}`).click()
    await expect(page.getByTestId('code-pad-message')).toContainText('Quantité 2')
    await page.locator('.item-btn', { hasText: 'Cola' }).click()
    await expect.poll(async () => (await ipcCalls(page, 'add_order_line')).length).toBe(1)
    const [call] = await ipcCalls(page, 'add_order_line')
    expect(call.input.menu_item_id).toBe(2)
    expect(call.input.quantity).toBe(2)
  })

  test('a typed code filters the grid, exact code ringed, and a tap adds the result', async ({
    page,
  }) => {
    await page.getByTestId('code-key-A').click()
    await expect(page.locator('.item-btn')).toHaveCount(2)
    await page.getByTestId('code-key-1').click()
    await expect(page.locator('.item-btn')).toHaveCount(2)
    await expect(page.locator('.item-btn.highlighted')).toContainText('Maki thon')
    await page.getByTestId('code-key-2').click()
    await expect(page.locator('.item-btn')).toHaveCount(1)
    await page.getByTestId('code-key-clear').click()
    await expect(page.locator('.item-btn')).toHaveCount(3)

    for (const k of ['A', '1']) await page.getByTestId(`code-key-${k}`).click()
    await page.locator('.item-btn', { hasText: 'Maki saumon' }).click()
    await expect.poll(async () => (await ipcCalls(page, 'add_order_line')).length).toBe(1)
    const [call] = await ipcCalls(page, 'add_order_line')
    expect(call.input.menu_item_id).toBe(1)
  })

  test('the pad stays open on this terminal', async ({ page }) => {
    expect(await page.evaluate(() => localStorage.getItem('pili-pos-code-pad'))).toBe('1')
  })
})
