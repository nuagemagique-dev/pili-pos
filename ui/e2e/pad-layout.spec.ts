import { test, expect, type Locator, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  loginAndGoto,
  tapUserTile,
  DEFAULT_USER,
  DEFAULT_USER_WITH_PIN,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc'

// UX audit C10: every numeric pad uses the calculator layout like the
// code pad, 7 8 9 on top, then 4 5 6, 1 2 3 and the bottom row (clear or
// comma, 0, backspace). One helper reads the position of every key and
// checks the grid, so a pad that drifts back to 1 2 3 on top fails here.

interface Box {
  x: number
  y: number
  width: number
  height: number
}

async function boxOf(scope: Locator, label: string): Promise<Box> {
  const key = scope.locator('button').filter({ hasText: new RegExp(`^${label}$`, 'u') })
  await expect(key).toHaveCount(1)
  const box = await key.boundingBox()
  if (!box) throw new Error(`no box for key ${label}`)
  return box
}

/** The ten digit keys sit on a 3 column grid in calculator order, 0 under the middle column. */
async function expectCalculatorLayout(scope: Locator) {
  const b: Record<string, Box> = {}
  for (const d of '0123456789') b[d] = await boxOf(scope, d)
  const rows = [
    ['7', '8', '9'],
    ['4', '5', '6'],
    ['1', '2', '3'],
  ]
  for (const row of rows) {
    // Same line, left to right.
    expect(b[row[1]!]!.y).toBeCloseTo(b[row[0]!]!.y, 0)
    expect(b[row[2]!]!.y).toBeCloseTo(b[row[0]!]!.y, 0)
    expect(b[row[1]!]!.x).toBeGreaterThan(b[row[0]!]!.x)
    expect(b[row[2]!]!.x).toBeGreaterThan(b[row[1]!]!.x)
  }
  // Rows go down, columns line up.
  expect(b['4']!.y).toBeGreaterThan(b['7']!.y)
  expect(b['1']!.y).toBeGreaterThan(b['4']!.y)
  expect(b['4']!.x).toBeCloseTo(b['7']!.x, 0)
  expect(b['1']!.x).toBeCloseTo(b['7']!.x, 0)
  expect(b['5']!.x).toBeCloseTo(b['8']!.x, 0)
  expect(b['3']!.x).toBeCloseTo(b['9']!.x, 0)
  // The zero is on the bottom row, in the middle column.
  expect(b['0']!.y).toBeGreaterThan(b['1']!.y)
  expect(b['0']!.x).toBeCloseTo(b['2']!.x, 0)
}

async function toOrder(page: Page) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
    list_cart_lines: () => [{ ...DEFAULT_ORDER_LINE, unit_price_cents: 1200 }],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    get_gift_voucher_settings: () => ({
      enabled: true,
      validity_months: 12,
      max_amount_cents: 50000,
    }),
  } as any)
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/u)
}

test.describe('Numeric pads: 7 8 9 on top (C10)', () => {
  test('login: the INSZ pad', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => [DEFAULT_USER] })
    await page.goto('/')
    await page.click('.other-btn')
    await expectCalculatorLayout(page.locator('.numpad-section .numpad'))
    // The bottom row keeps C, 0, backspace.
    await expect(page.locator('.numpad-section .nk-clear')).toHaveText('C')
    await expect(page.locator('.numpad-section .nk-back')).toBeVisible()
  })

  test('login: the PIN pad', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => [DEFAULT_USER_WITH_PIN] })
    await page.goto('/')
    await page.click('.user-btn')
    await expectCalculatorLayout(page.locator('.pin-overlay .numpad'))
  })

  test('the PIN dialog on the order screen', async ({ page }) => {
    await toOrder(page)
    await page.locator('.header-user').click()
    await page.getByTestId('user-menu-pin').click()
    await expectCalculatorLayout(page.locator('.pin-dialog'))
  })

  test('the gift voucher amount dialog', async ({ page }) => {
    await toOrder(page)
    await page.getByTestId('gift-voucher-sell').click()
    await expect(page.getByRole('dialog', { name: 'Vendre un bon cadeau' })).toBeVisible()
    await expectCalculatorLayout(page.locator('.numpad'))
  })

  test('cash in and out: the "Autre" pad', async ({ page }) => {
    await mockTauriIPC(page)
    await loginAndGoto(page, '/reports')
    await page.getByRole('tab', { name: 'Caisse' }).click()
    await page.locator('.cash-other').click()
    await expectCalculatorLayout(page.locator('.numpad'))
  })

  test('the checkout amount pad keeps its comma in the bottom row', async ({ page }) => {
    await toOrder(page)
    await page.locator('.checkout-btn').click()
    await page.waitForURL(/checkout/u)
    const pad = page.locator('.entry-right .numpad')
    await expectCalculatorLayout(pad)
    await expect(pad.locator('.nk-comma')).toHaveText(',')
    await expect(pad.locator('.nk-back')).toBeVisible()
  })
})
