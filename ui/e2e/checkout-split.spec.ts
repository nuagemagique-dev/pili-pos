import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  tapUserTile,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc'

// A sale paid in several parts: card and meal vouchers for an exact
// amount each, cash for the rest (the only part rounded to 5 cents).
// The screen sends the parts to `checkout`; nothing is signed until
// the rest is paid. The manual's rounding products: Dame Blanche
// 11,52 and Tapas Variation 3,34, a sale of 14,86.

const LINES = [
  {
    ...DEFAULT_ORDER_LINE,
    id: 1,
    menu_item_id: 25003,
    unit_price_cents: 1152,
    vat_code: 'B',
    name: 'Dame Blanche',
  },
  {
    ...DEFAULT_ORDER_LINE,
    id: 2,
    menu_item_id: 28007,
    unit_price_cents: 334,
    vat_code: 'B',
    name: 'Tapas Variation',
  },
]

// The signed sale the done screen shows (its change line needs a ticket).
const SALE = {
  success: true,
  ticket: {
    restaurant_name: 'The FPS Finances Experience',
    restaurant_address: 'Boulevard Albert II 33, 1030 Brussels',
    restaurant_vat: 'BE0000000097',
    pos_id: 'CPOS0031234567',
    terminal_id: 'POS-1-BAR',
    pos_datetime: new Date().toISOString(),
    ticket_number: 1,
    event_label: 'N',
    is_training: false,
    lines: [],
    subtotal: 14.86,
    total: 14.86,
    vat_breakdown: [],
    payments: [],
    rounding: null,
    fdm_ref: null,
    short_signature: null,
    verification_url: null,
    footer: [],
    ticket_type_label: 'TICKET DE CAISSE T.V.A.',
    is_refund: false,
    lang: 'fr',
  },
  error: null,
  fdm_connected: true,
  fiscal_event_id: 1,
  delivery: null,
  kitchen_slip: 'printed',
}

async function toPayment(page: Page) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1486 }),
    list_cart_lines: () => LINES,
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    checkout: () => SALE,
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/)
  await page.locator('.checkout-btn').click()
  await page.waitForURL(/checkout/)
  // Cash alone: 14,86 is paid 14,85.
  await expect(page.locator('.total-value')).toHaveText('14,85')
}

/** Card or meal vouchers: the amount starts at the rest; typing
 * replaces it. */
async function exactPart(page: Page, button: string, digits: string | null) {
  await page.locator(button).click()
  if (digits !== null) {
    for (const d of digits) await page.locator('.nk', { hasText: d }).first().click()
  }
  await page.locator('.card-confirm').click()
}

async function payCash(page: Page, digits: string) {
  await page.locator('.cash-btn').click()
  for (const d of digits) await page.locator('.nk', { hasText: d }).first().click()
  await page.locator('.validate-btn').click()
}

test('[ROUND4] meal vouchers 11,52 then the rest in cash, rounded to 3,35', async ({ page }) => {
  await toPayment(page)
  await exactPart(page, '.meal-btn', '11,52')
  await expect(page.locator('.total-label')).toHaveText('Reste à payer')
  await expect(page.locator('.total-value')).toHaveText('3,35')
  await expect(page.locator('.paid-part').nth(0)).toContainText('11,52')

  await payCash(page, '5')
  await expect(page.locator('.success-icon')).toBeVisible()
  await expect(page.locator('.receipt-change')).toContainText('1,65')
  expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([
    { method: 'meal_voucher', amount_cents: 1152 },
    { method: 'cash' },
  ])
})

test('[ROUND5] meal vouchers, card, then 10 € cash for the 7,75 left', async ({ page }) => {
  await toPayment(page)
  await exactPart(page, '.meal-btn', '3,13')
  await exactPart(page, '.card-btn', '3,97')
  await expect(page.locator('.total-value')).toHaveText('7,75')
  await payCash(page, '10')
  await expect(page.locator('.receipt-change')).toContainText('2,25')
  expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([
    { method: 'meal_voucher', amount_cents: 313 },
    { method: 'card', amount_cents: 397 },
    { method: 'cash' },
  ])
})

test('[UC4119] card for part, meal vouchers for the rest: no cash, no change', async ({ page }) => {
  await toPayment(page)
  await exactPart(page, '.card-btn', '10')
  // The meal voucher amount starts at the rest, 4,86.
  await page.locator('.meal-btn').click()
  await expect(page.locator('.exact-value')).toHaveText('4,86')
  await page.locator('.card-confirm').click()
  await expect(page.locator('.success-icon')).toBeVisible()
  await expect(page.locator('.receipt-change')).toHaveCount(0)
  expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([
    { method: 'card', amount_cents: 1000 },
    { method: 'meal_voucher', amount_cents: 486 },
  ])
})

test('meal vouchers for the whole sale sign at once', async ({ page }) => {
  await toPayment(page)
  await exactPart(page, '.meal-btn', null)
  await expect(page.locator('.success-icon')).toBeVisible()
  expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([
    { method: 'meal_voucher', amount_cents: 1486 },
  ])
})

test('an amount over the rest cannot be confirmed', async ({ page }) => {
  await toPayment(page)
  await page.locator('.card-btn').click()
  for (const d of '20') await page.locator('.nk', { hasText: d }).first().click()
  await expect(page.locator('.exact-too-high')).toBeVisible()
  await expect(page.locator('.card-confirm')).toBeDisabled()
  expect(await ipcCalls(page, 'checkout')).toHaveLength(0)
})

test('the parts taken can be cancelled before anything is signed', async ({ page }) => {
  await toPayment(page)
  await exactPart(page, '.card-btn', '5')
  await expect(page.locator('.paid-part')).toHaveCount(1)
  await page.locator('.parts-clear').click()
  await expect(page.locator('.paid-part')).toHaveCount(0)
  await expect(page.locator('.total-value')).toHaveText('14,85')
  expect(await ipcCalls(page, 'checkout')).toHaveLength(0)
})

// §7.4: equal parts. Staff only pick the number of parts; each part is
// computed (remainder cents on the first parts) and taken as one more
// entry of the same payment list.

test('two equal parts: card 7,43, then cash for the other 7,43 rounded to 7,45', async ({
  page,
}) => {
  await toPayment(page)
  await page.locator('.split-btn', { hasText: '2' }).click()
  await expect(page.locator('.part-chip')).toHaveCount(2)
  await expect(page.locator('.validate-btn')).toContainText('Valider part 1')

  await page.locator('.card-btn').click()
  await expect(page.locator('.exact-value')).toHaveText('7,43')
  await page.locator('.card-confirm').click()
  await expect(page.locator('.part-chip.paid')).toContainText('Carte · 7,43 €')
  await expect(page.locator('.paid-part')).toContainText('− 7,43')
  // Cash is back as the method for the last part, rounded.
  await expect(page.locator('.cash-btn')).toHaveClass(/active/)
  await expect(page.locator('.total-value')).toHaveText('7,45')
  await expect(page.locator('.validate-btn')).toContainText('Valider part 2')

  await page.locator('.shortcut.exact').click()
  await page.locator('.validate-btn').click()
  await expect(page.locator('.success-icon')).toBeVisible()
  await expect(page.locator('.receipt-change')).toHaveCount(0)
  expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([
    { method: 'card', amount_cents: 743 },
    { method: 'cash' },
  ])
})

test('three parts, the first in cash: the cash part is sent last, as the rest', async ({
  page,
}) => {
  await toPayment(page)
  await page.locator('.split-btn', { hasText: '3' }).click()
  // 14,86 / 3 = 4,96 + 4,95 + 4,95; the cash part 4,96 is paid 4,95.
  await expect(page.locator('.part-chip.current')).toContainText('4,96')
  await expect(page.locator('.shortcut.exact')).toContainText('4,95')
  await expect(page.locator('.shortcut.exact')).toContainText('exact 4,96')
  for (const d of '5') await page.locator('.nk', { hasText: d }).first().click()
  await expect(page.locator('.change-box')).toContainText('0,05')
  await page.locator('.validate-btn').click()

  // One cash part at most: the others go on card / meal vouchers.
  await expect(page.locator('.cash-btn')).toBeDisabled()
  await expect(page.locator('.card-btn')).toHaveClass(/active/)
  await expect(page.locator('.exact-value')).toHaveText('4,95')
  await page.locator('.card-confirm').click()
  await expect(page.locator('.exact-value')).toHaveText('4,95')
  await page.locator('.card-confirm').click()

  await expect(page.locator('.success-icon')).toBeVisible()
  await expect(page.locator('.receipt-change')).toContainText('0,05')
  expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([
    { method: 'card', amount_cents: 495 },
    { method: 'card', amount_cents: 495 },
    { method: 'cash' },
  ])
})

test('the number of parts is fixed once a part is taken', async ({ page }) => {
  await toPayment(page)
  await page.locator('.split-btn', { hasText: '2' }).click()
  await page.locator('.card-btn').click()
  await page.locator('.card-confirm').click()
  await expect(page.locator('.split-btn', { hasText: '3' })).toBeDisabled()
  expect(await ipcCalls(page, 'checkout')).toHaveLength(0)
})
