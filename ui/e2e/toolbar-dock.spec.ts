import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
  tapUserTile,
} from './fixtures/mock-ipc'

// UX audit C1 (one toolbar row between the category tabs and the grid) and C6 (the code pad docked
// under the grid, which keeps its 5 columns).

const ITEMS = Array.from({ length: 30 }, (_, i) => ({
  ...DEFAULT_MENU_ITEM,
  id: i + 1,
  name: `Plat ${i + 1}`,
  code: i < 10 ? `A${i + 1}` : `B${i - 9}`,
}))
const VOUCHERS_ON = { enabled: true, validity_months: 12, max_amount_cents: 50000 }

async function orderScreen(page: Page, role = 'admin', vouchers = false) {
  const user = { ...DEFAULT_USER, role }
  await page.addInitScript(() => {
    localStorage.removeItem('pili-pos-code-pad')
  })
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 0 }),
    list_cart_lines: () => [],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => ITEMS,
    list_sold_out: () => [],
    get_gift_voucher_settings: () => ({ ...VOUCHERS_ON, enabled: vouchers }),
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/u)
  await expect(page.getByTestId('mode-toolbar')).toBeVisible()
}

async function box(page: Page, testId: string) {
  const b = await page.getByTestId(testId).boundingBox()
  if (!b) throw new Error(`no box for ${testId}`)
  return b
}

test.describe('Toolbar row (C1)', () => {
  test('all the modes sit on one 56 px row between the tabs and the grid', async ({ page }) => {
    await orderScreen(page, 'admin', true)
    const ids = [
      'soldout-mode',
      'code-pad-toggle',
      'allergen-mode',
      'gift-voucher-sell',
      'soldout-list-button',
    ]
    const toolbar = await box(page, 'mode-toolbar')
    expect(toolbar.height).toBe(56)
    const tabs = await page.locator('.category-tabs').boundingBox()
    const grid = await page.locator('.menu-grid').boundingBox()
    expect(toolbar.y).toBeGreaterThanOrEqual(tabs!.y + tabs!.height - 1)
    expect(grid!.y).toBeGreaterThanOrEqual(toolbar.y + toolbar.height - 1)
    const boxes = []
    for (const id of ids) boxes.push(await box(page, id))
    for (const b of boxes) {
      // One line, touch height, inside the row.
      expect(b.y).toBeCloseTo(boxes[0]!.y, 0)
      expect(b.height).toBeGreaterThanOrEqual(48)
      expect(b.y + b.height).toBeLessThanOrEqual(toolbar.y + toolbar.height + 1)
    }
    // Left to right, the list of sold-out products at the end.
    for (let i = 1; i < boxes.length; i++) expect(boxes[i]!.x).toBeGreaterThan(boxes[i - 1]!.x)
  })

  test('every button has an icon and a label', async ({ page }) => {
    await orderScreen(page, 'admin', true)
    for (const id of [
      'soldout-mode',
      'code-pad-toggle',
      'allergen-mode',
      'gift-voucher-sell',
      'soldout-list-button',
    ]) {
      const button = page.getByTestId(id)
      await expect(button.locator('svg path')).toHaveCount(1)
      expect(((await button.locator('.label').textContent()) ?? '').trim().length).toBeGreaterThan(
        2,
      )
    }
    await expect(page.getByTestId('soldout-list-button')).toContainText('Produits épuisés (0)')
  })

  test('a waiter has no sold-out button, and no voucher button while vouchers are off', async ({
    page,
  }) => {
    await orderScreen(page, 'waiter')
    await expect(page.getByTestId('soldout-mode')).toHaveCount(0)
    await expect(page.getByTestId('soldout-list-button')).toHaveCount(0)
    await expect(page.getByTestId('gift-voucher-sell')).toHaveCount(0)
    await expect(page.getByTestId('code-pad-toggle')).toBeVisible()
    await expect(page.getByTestId('allergen-mode')).toBeVisible()
    expect((await box(page, 'mode-toolbar')).height).toBe(56)
  })

  test('the hint of a mode stays on the same row, the allergen legend opens under it', async ({
    page,
  }) => {
    await orderScreen(page, 'admin')
    await page.getByTestId('soldout-mode').click()
    await expect(page.getByTestId('soldout-hint')).toBeVisible()
    const mode = await box(page, 'soldout-mode')
    const hint = await box(page, 'soldout-hint')
    expect(hint.y).toBeGreaterThanOrEqual(mode.y - 1)
    expect(hint.y + hint.height).toBeLessThanOrEqual(mode.y + mode.height + 1)
    expect((await box(page, 'mode-toolbar')).height).toBe(56)

    // The two modes exclude each other.
    await page.getByTestId('allergen-mode').click()
    await expect(page.getByTestId('soldout-hint')).toHaveCount(0)
    await expect(page.getByTestId('allergen-hint')).toBeVisible()
    await expect(page.getByTestId('allergen-legend')).toBeVisible()
    await expect(page.getByTestId('allergen-mode')).toHaveAttribute('aria-pressed', 'true')
    await expect(page.getByTestId('soldout-mode')).toHaveAttribute('aria-pressed', 'false')
  })
})

test.describe('Docked code pad (C6)', () => {
  test('the pad is a dock of about 300 px under the grid and the grid keeps 5 columns', async ({
    page,
  }) => {
    await orderScreen(page, 'admin')
    const before = await page.locator('.menu-grid').boundingBox()
    await page.getByTestId('code-pad-toggle').click()
    await expect(page.getByTestId('code-pad')).toBeVisible()
    const dock = await box(page, 'code-pad')
    const grid = await page.locator('.menu-grid').boundingBox()
    expect(dock.height).toBeGreaterThanOrEqual(290)
    expect(dock.height).toBeLessThanOrEqual(310)
    // Under the grid, over the whole width of the left area.
    expect(dock.y).toBeGreaterThanOrEqual(grid!.y + grid!.height - 1)
    expect(dock.width).toBeCloseTo(grid!.width, 0)
    expect(grid!.width).toBeCloseTo(before!.width, 0)
    // Five product buttons on the first row, as without the pad.
    const xs = await page.locator('.menu-grid > *').evaluateAll((els) => {
      const { top } = (els[0] as HTMLElement).getBoundingClientRect()
      return els
        .map((e) => (e as HTMLElement).getBoundingClientRect())
        .filter((r) => Math.abs(r.top - top) < 2)
        .map((r) => Math.round(r.left))
    })
    expect(xs).toHaveLength(5)
    // The grid scrolls above the dock.
    const scrolls = await page
      .locator('.menu-grid')
      .evaluate((e) => e.scrollHeight > e.clientHeight)
    expect(scrolls).toBe(true)
  })

  test('field and message on the left, digits in the middle, OK on the right, all touch sized', async ({
    page,
  }) => {
    await orderScreen(page, 'admin')
    await page.getByTestId('code-pad-toggle').click()
    const field = await box(page, 'code-pad-display')
    const message = await box(page, 'code-pad-message')
    const seven = await box(page, 'code-key-7')
    const letter = await box(page, 'code-key-A')
    const ok = await box(page, 'code-key-ok')
    const dock = await box(page, 'code-pad')
    expect(message.y).toBeGreaterThan(field.y)
    expect(seven.x).toBeGreaterThan(field.x + field.width - 1)
    expect(ok.x).toBeGreaterThan(seven.x + seven.width - 1)
    expect(letter.x + letter.width).toBeLessThanOrEqual(seven.x + 1)
    for (const b of [seven, letter, ok]) expect(b.height).toBeGreaterThanOrEqual(48)
    // Every key stays inside the dock.
    for (const k of ['0', '9', 'times', 'back', 'ok', 'A', 'B']) {
      const b = await box(page, `code-key-${k}`)
      expect(b.y).toBeGreaterThanOrEqual(dock.y - 1)
      expect(b.y + b.height).toBeLessThanOrEqual(dock.y + dock.height + 1)
    }
  })

  test('a typed code filters the grid to a row and rings the match', async ({ page }) => {
    await orderScreen(page, 'admin')
    await page.getByTestId('code-pad-toggle').click()
    for (const k of ['A', '1']) await page.getByTestId(`code-key-${k}`).click()
    await expect(page.getByTestId('code-pad-message')).toContainText('Plat 1')
    const grid = await page.locator('.menu-grid').boundingBox()
    const dock = await box(page, 'code-pad')
    // A1 and A10: one row of the grid, above the dock.
    await expect(page.locator('.menu-grid > *')).toHaveCount(2)
    const last = await page.locator('.menu-grid > *').last().boundingBox()
    expect(last!.y + last!.height).toBeLessThanOrEqual(dock.y)
    expect(last!.y).toBeGreaterThanOrEqual(grid!.y)
    // The pad closes with its toggle and the grid takes the room back.
    await page.getByTestId('code-pad-toggle').click()
    await expect(page.getByTestId('code-pad')).toHaveCount(0)
  })
})
