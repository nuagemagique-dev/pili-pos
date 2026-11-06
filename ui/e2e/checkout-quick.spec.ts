import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  tapUserTile,
  eftScript,
  settingsWith,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc'

// The quick checkout (UX audit P1, P2, P6): the exact cash amount is
// already entered and the button says what it collects (PAYER ->
// Encaisser), the done screen returns to the tables by itself after
// 2 s unless there is something to read, and after a part the methods
// that can still pay the rest sit in one row.

const TICKET = {
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
}

const DELIVERY = {
  fiscalEventId: 1,
  deliveryId: null,
  medium: 'PAPER',
  email: 'not_requested',
  recipientHint: null,
  paperPrinted: true,
  needsPaper: false,
}

const SALE = {
  success: true,
  ticket: TICKET,
  error: null,
  fdm_connected: true,
  fiscal_event_id: 1,
  delivery: DELIVERY,
  kitchen_slip: 'printed',
}

// 11,52 + 3,34 = 14,86: cash pays 14,85.
const LINES = [
  { ...DEFAULT_ORDER_LINE, id: 1, unit_price_cents: 1152, vat_code: 'B', name: 'Dame Blanche' },
  { ...DEFAULT_ORDER_LINE, id: 2, unit_price_cents: 334, vat_code: 'B', name: 'Tapas Variation' },
]

type Handlers = Parameters<typeof mockTauriIPC>[1]

/** On the order screen of the first table, with the clock under the test's control. */
async function toOrder(page: Page, handlers: Handlers = {}, paymentTerminal?: string) {
  await page.clock.install()
  await eftScript(page, { pay: [{ status: 'approved', brand: 'Bancontact' }] })
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1486 }),
    list_cart_lines: () => LINES,
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    checkout: () => SALE,
    get_settings: settingsWith(paymentTerminal ? { paymentTerminal } : {}),
    ...handlers,
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/u)
}

/** PAYER. */
async function pay(page: Page) {
  await page.locator('.checkout-btn').click()
  await page.waitForURL(/checkout/u)
  await expect(page.locator('.total-value')).toHaveText('14,85')
}

const collect = (page: Page) => page.locator('.validate-btn')

test.describe('P1: the exact amount is already entered', () => {
  test('cash starts at the rounded rest, the button says what it collects', async ({ page }) => {
    await toOrder(page)
    await pay(page)
    await expect(page.locator('.cash-btn')).toHaveClass(/active/u)
    await expect(page.locator('.entry-value')).toHaveText('14,85')
    await expect(page.locator('.shortcut.exact')).toContainText('exact 14,86 · arrondi 5 cts')
    await expect(collect(page)).toHaveText('Encaisser 14,85 € en espèces')
    await expect(collect(page)).toBeEnabled()
    // Exactly what is due: no "received" line, no change.
    await expect(collect(page).locator('.validate-amount')).toHaveCount(0)
  })

  test('a typed digit replaces the amount, the change follows', async ({ page }) => {
    await toOrder(page)
    await pay(page)
    for (const d of '20') await page.locator('.nk', { hasText: d }).first().click()
    await expect(page.locator('.entry-value')).toHaveText('20')
    await expect(page.locator('.change-box')).toContainText('5,15')
    await expect(collect(page)).toContainText('Encaisser 14,85 € en espèces')
    await expect(collect(page)).toContainText('reçu 20,00 €')
  })

  test('a quick note replaces the amount', async ({ page }) => {
    await toOrder(page)
    await pay(page)
    await page.locator('.shortcut.quick').first().click()
    await expect(page.locator('.entry-value')).toHaveText('15,00')
    await expect(page.locator('.change-box')).toContainText('0,15')
    await expect(collect(page)).toContainText('reçu 15,00 €')
  })

  test('card and meal vouchers name the method and start at the part', async ({ page }) => {
    await toOrder(page)
    await pay(page)
    await page.locator('.card-btn').click()
    await expect(collect(page)).toHaveText('Encaisser 14,86 € par carte')
    await page.locator('.meal-btn').click()
    await expect(collect(page)).toHaveText('Encaisser 14,86 € en chèques-repas')
    // The first key replaces the amount, the label follows what is typed.
    for (const d of '5') await page.locator('.nk', { hasText: d }).first().click()
    await expect(collect(page)).toHaveText('Encaisser 5,00 € en chèques-repas')
  })

  test('a part says which part it collects', async ({ page }) => {
    await toOrder(page)
    await pay(page)
    await page.locator('.split-btn', { hasText: '2' }).click()
    await expect(collect(page)).toHaveText('Encaisser la part 1 · 7,45 €')
    await page.locator('.card-btn').click()
    await expect(collect(page)).toHaveText('Encaisser la part 1 · 7,43 €')
  })

  test('the cash sale is PAYER then Encaisser: two taps, the tables come back by themselves', async ({
    page,
  }) => {
    await toOrder(page)
    await pay(page) // tap 1
    await collect(page).click() // tap 2
    await expect(page.locator('.success-icon')).toBeVisible()
    expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([{ method: 'cash' }])
    await page.clock.runFor(2100)
    await page.waitForURL(/tables/u)
  })
})

test.describe('P2: the done screen leaves by itself', () => {
  test('it shows OK and the change very large, then goes to the tables after 2 s', async ({
    page,
  }) => {
    await toOrder(page)
    await pay(page)
    for (const d of '20') await page.locator('.nk', { hasText: d }).first().click()
    await collect(page).click()
    await expect(page.locator('.success-icon')).toHaveText('OK')
    await expect(page.getByTestId('done-change')).toContainText('Rendu')
    await expect(page.getByTestId('done-change')).toContainText('5,15 €')
    const size = await page
      .locator('.receipt-change-value')
      .evaluate((el) => parseFloat(getComputedStyle(el).fontSize))
    expect(size).toBeGreaterThanOrEqual(64)
    await expect(page.getByTestId('done-auto-return')).toBeVisible()

    await page.clock.runFor(1500)
    await expect(page).toHaveURL(/checkout/u)
    await page.clock.runFor(600)
    await page.waitForURL(/tables/u)
  })

  test('a tap anywhere returns at once', async ({ page }) => {
    await toOrder(page)
    await pay(page)
    await collect(page).click()
    await expect(page.locator('.success-icon')).toBeVisible()
    await page.getByTestId('done-screen').click({ position: { x: 40, y: 200 } })
    await page.waitForURL(/tables/u)
  })

  test('Terminer returns at once, without a detour through the order screen', async ({ page }) => {
    await toOrder(page)
    await pay(page)
    await collect(page).click()
    const visited: string[] = []
    page.on('framenavigated', (frame) => {
      if (frame === page.mainFrame()) visited.push(frame.url())
    })
    await page.getByTestId('done-finish').click()
    await page.waitForURL(/tables/u)
    expect(visited.some((u) => /#\/order/u.test(u))).toBe(false)
  })

  test('a takeaway sale returns to the tables too', async ({ page }) => {
    const takeaway = { ...DEFAULT_ORDER, order_type: 'takeaway', table_number: null }
    await toOrder(page, {
      create_order: () => ({ ...takeaway, id: 2 }),
      get_order: () => ({ ...takeaway, id: 2, total_cents: 1486 }),
    })
    // The order has no table: it is a takeaway sale.
    await expect(page.locator('.header-takeaway')).toBeVisible()
    await pay(page)
    await collect(page).click()
    await expect(page.locator('.success-icon')).toBeVisible()
    await page.clock.runFor(2100)
    await page.waitForURL(/tables/u)
  })

  test('a card on the linked terminal: PAYER, Carte, Encaisser, then it returns by itself', async ({
    page,
  }) => {
    await toOrder(page, {}, 'worldline_terminal_api')
    await pay(page) // tap 1
    await page.locator('.card-btn').click() // tap 2
    await collect(page).click() // tap 3, the terminal approves
    await expect(page.locator('.success-icon')).toBeVisible()
    const [call] = await ipcCalls(page, 'checkout')
    expect(call.payments).toEqual([{ method: 'card', amount_cents: 1486, eft_payment_id: 100 }])
    await page.clock.runFor(2100)
    await page.waitForURL(/tables/u)
  })

  test.describe('it stays when there is something to read or decide', () => {
    const cases: [string, object, string][] = [
      [
        'the ticket could not be printed',
        { delivery: { ...DELIVERY, paperPrinted: false, needsPaper: true } },
        'Imprimer le ticket',
      ],
      [
        'the digital ticket waits in the outbox',
        { delivery: { ...DELIVERY, medium: 'DIGITAL', email: 'queued', paperPrinted: false } },
        'attente',
      ],
      ['the FDM was not connected', { fdm_connected: false }, 'ticket sans signature'],
      [
        'a gift voucher slip has a note',
        { gift_voucher_prints: [{ voucher_id: 1, kind: 'voucher', outcome: 'failed' }] },
        'Bon non imprimé',
      ],
    ]
    for (const [name, over, text] of cases) {
      test(name, async ({ page }) => {
        await toOrder(page, { checkout: () => ({ ...SALE, ...over }) })
        await pay(page)
        await collect(page).click()
        await expect(page.locator('.success-icon')).toBeVisible()
        await expect(page.locator('.done-screen')).toContainText(text)
        await expect(page.getByTestId('done-auto-return')).toHaveCount(0)
        await page.clock.runFor(10_000)
        await expect(page).toHaveURL(/checkout/u)
        // A tap on the screen does not leave either: only Terminer does.
        await page.getByTestId('done-screen').click({ position: { x: 40, y: 200 } })
        await expect(page).toHaveURL(/checkout/u)
        await page.getByTestId('done-finish').click()
        await page.waitForURL(/tables/u)
      })
    }
  })
})

test.describe('P6: the rest after a part', () => {
  async function afterMealVouchers(page: Page) {
    await toOrder(page)
    await pay(page)
    await page.locator('.meal-btn').click()
    for (const d of '11,52') {
      await page
        .locator(d === ',' ? '.nk-comma' : '.nk', { hasText: d })
        .first()
        .click()
    }
    await collect(page).click()
    await expect(page.locator('.paid-part')).toHaveCount(1)
  }

  test('no row before a part is taken', async ({ page }) => {
    await toOrder(page)
    await pay(page)
    await expect(page.getByTestId('rest-methods-label')).toHaveCount(0)
  })

  test('one row names the rest and the methods that can still pay it', async ({ page }) => {
    await afterMealVouchers(page)
    await expect(page.getByTestId('rest-methods-label')).toHaveText('Reste 3,34 € :')
    await expect(page.locator('.methods .method-btn')).toHaveText([
      'Espèces',
      'Carte',
      'Chèques-repas',
    ])
    // The method of the part taken stays selected, with the rest entered (UX audit P3).
    await expect(page.locator('.meal-btn')).toHaveClass(/active/u)
    await expect(collect(page)).toHaveText('Encaisser 3,34 € en chèques-repas')
    // Cash is one tap away, with the rest rounded to 5 cents.
    await page.locator('.cash-btn').click()
    await expect(page.getByTestId('rest-methods-label')).toHaveText('Reste 3,35 € :')
    await expect(collect(page)).toHaveText('Encaisser 3,35 € en espèces')
  })

  test('tapping a method pre-fills the rest: that tap and Encaisser take the next part', async ({
    page,
  }) => {
    await afterMealVouchers(page)
    await page.locator('.card-btn').click() // tap 1
    await expect(page.locator('.entry-value')).toHaveText('3,34')
    await expect(collect(page)).toHaveText('Encaisser 3,34 € par carte')
    await collect(page).click() // tap 2
    await expect(page.locator('.success-icon')).toBeVisible()
    expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([
      { method: 'meal_voucher', amount_cents: 1152 },
      { method: 'card', amount_cents: 334 },
    ])
  })

  test('cash stays offered and selected after a cash part: several friends can pay in cash', async ({
    page,
  }) => {
    await toOrder(page)
    await pay(page)
    await page.locator('.split-btn', { hasText: '3' }).click()
    await collect(page).click()
    await expect(page.getByTestId('rest-methods-label')).toContainText('Reste')
    await expect(page.locator('.cash-btn')).toHaveClass(/active/u)
    await expect(page.locator('.card-btn')).toBeVisible()
  })
})
