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
  eftScript,
  settingsWith,
} from './fixtures/mock-ipc'

// UX audit P3: the bill split by dishes ("Par plats"), "Autre" for any number of equal parts,
// several friends paying in cash, and the method kept between parts. Whatever the split, the
// sale stays ONE sale: the screen sends the same payment list as for equal parts (card and
// meal voucher parts with their amount, ONE cash part without amount), no unit, no dish.

// A bill of 34,59: three Sushi Mix (30,00 less a 0,01 discount, so 10,00 + 10,00 + 9,99), one
// Miso soup at 4,60 and two offered teas (net 0,00).
const LINES = [
  {
    ...DEFAULT_ORDER_LINE,
    id: 1,
    quantity: 3,
    unit_price_cents: 1000,
    discount_cents: -1,
    name: 'Sushi Mix',
  },
  { ...DEFAULT_ORDER_LINE, id: 2, quantity: 1, unit_price_cents: 460, name: 'Soupe Miso' },
  {
    ...DEFAULT_ORDER_LINE,
    id: 3,
    quantity: 2,
    unit_price_cents: 250,
    discount_cents: -500,
    name: 'Thé offert',
  },
]
const TOTAL = 3459

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
    subtotal: 34.59,
    total: 34.59,
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

async function toPayment(
  page: Page,
  total = TOTAL,
  lines: object[] = LINES,
  extra: Record<string, () => unknown> = {},
) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: total }),
    list_cart_lines: () => lines,
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    checkout: () => SALE,
    ...extra,
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/u)
  await page.locator('.checkout-btn').click()
  await page.waitForURL(/checkout/u)
}

async function typeAmount(page: Page, digits: string) {
  for (const d of digits) {
    await page
      .locator(d === ',' ? '.nk-comma' : '.nk', { hasText: d })
      .first()
      .click()
  }
}

/** Digits on the pad of the "Autre" dialog. */
async function typeOnDialog(page: Page, digits: string) {
  for (const d of digits) await page.locator('.dialog .key', { hasText: d }).first().click()
}

test.describe('P3: split by dishes', () => {
  test('the dishes are listed with checkboxes, a part is the sum of what is checked', async ({
    page,
  }) => {
    await toPayment(page)
    await page.getByTestId('split-dishes').click()
    await expect(page.getByTestId('split-dishes')).toHaveClass(/active/u)
    await expect(page.getByTestId('dish-hint')).toHaveText('Cochez les plats de cette part')
    // Nothing is checked: nothing to collect yet.
    await expect(page.locator('.validate-btn')).toBeDisabled()
    await expect(page.locator('.validate-btn')).toContainText('Encaisser la part 1 · 0,00 €')

    await page.getByTestId('dish-check-2').click()
    await expect(page.locator('.validate-btn')).toContainText('Encaisser la part 1 · 4,60 €')
    await expect(page.locator('.validate-btn')).toBeEnabled()
    await expect(page.getByTestId('dish-hint')).toHaveText('Plats de cette part : 1')
    // The whole row is a target too, and a second tap clears it.
    await page.getByTestId('dish-line-2').click()
    await expect(page.locator('.validate-btn')).toContainText('0,00 €')
  })

  test('a line of 3 gives its units one by one, the cents of the line shared in order', async ({
    page,
  }) => {
    await toPayment(page)
    await page.getByTestId('split-dishes').click()
    // By card the amounts are exact (cash would be rounded to 5 cents).
    await page.locator('.card-btn').click()
    await expect(page.getByTestId('dish-count-1')).toHaveText('0 / 3')
    await page.getByTestId('dish-plus-1').click()
    await expect(page.getByTestId('dish-count-1')).toHaveText('1 / 3')
    await expect(page.locator('.validate-btn')).toContainText('10,00 €')
    await page.getByTestId('dish-plus-1').click()
    await page.getByTestId('dish-plus-1').click()
    // The third unit carries the discount cent: 10,00 + 10,00 + 9,99.
    await expect(page.locator('.validate-btn')).toContainText('29,99 €')
    await expect(page.getByTestId('dish-plus-1')).toBeDisabled()
    await page.getByTestId('dish-minus-1').click()
    await expect(page.locator('.validate-btn')).toContainText('20,00 €')
    // The checkbox checks every unit left, then clears them.
    await page.getByTestId('dish-check-1').click()
    await expect(page.getByTestId('dish-count-1')).toHaveText('3 / 3')
    await page.getByTestId('dish-check-1').click()
    await expect(page.getByTestId('dish-count-1')).toHaveText('0 / 3')
  })

  test('parts by dishes: paid dishes are greyed, the last part pays the rest, ONE sale is signed', async ({
    page,
  }) => {
    await toPayment(page)
    await page.getByTestId('split-dishes').click()

    // Part 1: the soup and one Sushi Mix, by card.
    await page.getByTestId('dish-check-2').click()
    await page.getByTestId('dish-plus-1').click()
    await page.locator('.card-btn').click()
    await expect(page.locator('.exact-value')).toHaveText('14,60')
    await expect(page.locator('.validate-btn')).toHaveText('Encaisser la part 1 · 14,60 €')
    await page.locator('.card-confirm').click()

    // The soup is paid and greyed, the Sushi Mix has 2 units left.
    await expect(page.getByTestId('dish-line-2')).toHaveClass(/dish-done/u)
    await expect(page.getByTestId('dish-check-2')).toBeDisabled()
    await expect(page.getByTestId('dish-paid-2')).toHaveText('payé')
    await expect(page.getByTestId('dish-paid-1')).toHaveText('1 sur 3 payés')
    await expect(page.getByTestId('dish-count-1')).toHaveText('0 / 2')
    await expect(page.locator('.paid-part')).toContainText('− 14,60')
    await expect(page.locator('.total-value')).toHaveText('19,99')
    // The method stays selected for the next part.
    await expect(page.locator('.card-btn')).toHaveClass(/active/u)

    // Part 2: one more Sushi Mix, in cash (the rest row still offers it).
    await page.getByTestId('dish-plus-1').click()
    await page.locator('.cash-btn').click()
    await expect(page.locator('.validate-btn')).toHaveText('Encaisser la part 2 · 10,00 €')
    await page.locator('.validate-btn').click()
    await expect(page.locator('.paid-part')).toHaveCount(2)
    await expect(page.locator('.cash-btn')).toHaveClass(/active/u)

    // Part 3: the last Sushi Mix and the offered teas pay the rest, rounded for cash.
    await page.getByTestId('dish-check-1').click()
    await page.getByTestId('dish-check-3').click()
    await expect(page.locator('.validate-btn')).toHaveText('Encaisser la part 3 · 10,00 €')
    await page.locator('.validate-btn').click()

    await expect(page.locator('.success-icon')).toBeVisible()
    const calls = await ipcCalls(page, 'checkout')
    // One sale, the same payment list as for equal parts: no unit, no dish in the request.
    expect(calls).toHaveLength(1)
    expect(calls[0].payments).toEqual([{ method: 'card', amount_cents: 1460 }, { method: 'cash' }])
    expect(Object.keys(calls[0]).toSorted()).toEqual(['delivery', 'orderId', 'payments'].toSorted())
  })

  test('an offered line costs nothing: the last part has nothing to pay for it', async ({
    page,
  }) => {
    await toPayment(page)
    await page.getByTestId('split-dishes').click()
    await page.getByTestId('dish-check-3').click()
    // Only free units checked: no part of 0,00 can be taken.
    await expect(page.locator('.validate-btn')).toBeDisabled()
  })

  test('the number buttons and Autre are fixed once a part is taken, by dishes too', async ({
    page,
  }) => {
    await toPayment(page)
    await page.getByTestId('split-dishes').click()
    await page.getByTestId('dish-check-2').click()
    await page.locator('.card-btn').click()
    await page.locator('.card-confirm').click()
    await expect(page.locator('.split-btn', { hasText: '2' })).toBeDisabled()
    await expect(page.getByTestId('split-other')).toBeDisabled()
    await expect(page.getByTestId('split-dishes')).toHaveClass(/active/u)
  })

  test('the checked dishes go back to equal parts when a number is chosen', async ({ page }) => {
    await toPayment(page)
    await page.getByTestId('split-dishes').click()
    await page.getByTestId('dish-check-2').click()
    await page.locator('.split-btn', { hasText: '2' }).click()
    await expect(page.getByTestId('split-dishes')).not.toHaveClass(/active/u)
    await expect(page.locator('.validate-btn')).toHaveText('Encaisser la part 1 · 17,30 €')
    await expect(page.getByTestId('dish-line-2')).toHaveCount(0)
  })

  test('the parts by dishes come back with the order, with their dishes greyed', async ({
    page,
  }) => {
    await toPayment(page)
    await page.getByTestId('split-dishes').click()
    await page.getByTestId('dish-check-2').click()
    await page.locator('.card-btn').click()
    await page.locator('.card-confirm').click()
    await expect(page.locator('.paid-part')).toHaveCount(1)

    await page.getByTestId('back-to-order').click()
    await expect(page).toHaveURL(/order/u)
    await page.locator('.checkout-btn').click()
    await expect(page).toHaveURL(/checkout/u)
    await expect(page.getByTestId('split-dishes')).toHaveClass(/active/u)
    await expect(page.locator('.paid-part')).toHaveCount(1)
    await expect(page.getByTestId('dish-line-2')).toHaveClass(/dish-done/u)
    expect(await ipcCalls(page, 'checkout')).toHaveLength(0)
  })

  test('the dishes a terminal card paid stay greyed after back to the order and return', async ({
    page,
  }) => {
    await eftScript(page, { pay: [{ status: 'approved' }] })
    await toPayment(page, TOTAL, LINES, {
      get_settings: settingsWith({ paymentTerminal: 'worldline_terminal_api' }),
      check_payment_terminal: () => ({
        configured: true,
        kind: 'worldline_terminal_api',
        connected: true,
        busy: false,
        detail: null,
      }),
    })
    await page.getByTestId('split-dishes').click()
    // Part 1: the soup, charged on the linked terminal.
    await page.getByTestId('dish-check-2').click()
    await page.locator('.card-btn').click()
    await page.locator('.validate-btn').click()
    await expect(page.locator('.paid-part')).toHaveCount(1)
    await expect(page.getByTestId('dish-line-2')).toHaveClass(/dish-done/u)

    await page.getByTestId('back-to-order').click()
    await expect(page).toHaveURL(/order/u)
    await page.locator('.checkout-btn').click()
    await expect(page).toHaveURL(/checkout/u)
    // The approved card is read again from the engine: its dishes are still paid, not pickable.
    await expect(page.locator('.paid-part')).toHaveCount(1)
    await expect(page.getByTestId('split-dishes')).toHaveClass(/active/u)
    await expect(page.getByTestId('dish-line-2')).toHaveClass(/dish-done/u)
    await expect(page.getByTestId('dish-check-2')).toBeDisabled()
    await expect(page.getByTestId('dish-paid-2')).toHaveText('payé')
    expect(await ipcCalls(page, 'checkout')).toHaveLength(0)
  })

  test('"Annuler les parts" frees the dishes again', async ({ page }) => {
    await toPayment(page)
    await page.getByTestId('split-dishes').click()
    await page.getByTestId('dish-check-2').click()
    await page.locator('.card-btn').click()
    await page.locator('.card-confirm').click()
    await page.getByTestId('parts-clear').click()
    await expect(page.getByTestId('dish-line-2')).not.toHaveClass(/dish-done/u)
    await expect(page.getByTestId('dish-check-2')).toBeEnabled()
  })
})

test.describe('P3: Autre, any number of equal parts', () => {
  test('Autre opens a pad: 10 equal parts, the remainder cents on the first parts', async ({
    page,
  }) => {
    await toPayment(page)
    await expect(page.getByTestId('split-other')).toHaveText('Autre')
    await page.getByTestId('split-other').click()
    await expect(page.getByTestId('number-entry-ok')).toBeDisabled()
    // 1 is too few: only 2 to 20 are accepted.
    await typeOnDialog(page, '1')
    await expect(page.getByTestId('number-entry-ok')).toBeDisabled()
    await typeOnDialog(page, '0')
    await expect(page.getByTestId('number-entry-value')).toHaveText('10')
    await page.getByTestId('number-entry-ok').click()

    await expect(page.getByTestId('split-other')).toHaveText('10 parts')
    await expect(page.getByTestId('split-other')).toHaveClass(/active/u)
    await expect(page.locator('.part-chip')).toHaveCount(10)
    // 3459 / 10 = 345 and 9 cents over: nine parts of 3,46, the last one 3,45.
    await expect(page.locator('.part-chip.current')).toContainText('3,46')
    await expect(page.locator('.part-chip.next').last()).toContainText('3,45')
    await expect(page.locator('.validate-btn')).toContainText('Encaisser la part 1')
  })

  test('the pad refuses more than 20 parts', async ({ page }) => {
    await toPayment(page)
    await page.getByTestId('split-other').click()
    await typeOnDialog(page, '21')
    await expect(page.getByTestId('number-entry-ok')).toBeDisabled()
    await page.getByTestId('number-entry-ok').click({ force: true })
    await expect(page.getByTestId('split-other')).toHaveText('Autre')
  })

  test('a 7 parts split gives the remainder cent to the first part', async ({ page }) => {
    await toPayment(page)
    await page.getByTestId('split-other').click()
    await typeOnDialog(page, '7')
    await page.getByTestId('number-entry-ok').click()
    // 3459 / 7 = 494 and 1 cent over: the first part is 4,95, the others 4,94.
    await expect(page.locator('.part-chip.current')).toContainText('4,95')
    await expect(page.locator('.part-chip.next').first()).toContainText('4,94')
  })
})

test.describe('P3: several friends pay in cash', () => {
  test('three friends in cash: each part keeps its given and change, ONE cash line is sent', async ({
    page,
  }) => {
    await toPayment(page, 3009, [
      { ...DEFAULT_ORDER_LINE, id: 1, quantity: 3, unit_price_cents: 1003, name: 'Sushi Mix' },
    ])
    await page.locator('.split-btn', { hasText: '3' }).click()
    await expect(page.locator('.cash-btn')).toHaveClass(/active/u)
    // Part 1: 10,03, the cash due is 10,05; 20 € given, 9,95 change.
    await expect(page.locator('.validate-btn')).toContainText('Encaisser la part 1 · 10,05 €')
    await typeAmount(page, '20')
    await expect(page.locator('.change-box')).toContainText('9,95')
    await page.locator('.validate-btn').click()
    await expect(page.locator('.paid-part').nth(0)).toContainText('reçu 20,00 €, rendu 9,95 €')

    // Cash stays selected for friend 2: the rounding of the one cash line is shared (10,00).
    await expect(page.locator('.cash-btn')).toHaveClass(/active/u)
    await expect(page.locator('.cash-btn')).toHaveCount(1)
    await expect(page.locator('.validate-btn')).toContainText('Encaisser la part 2 · 10,00 €')
    await typeAmount(page, '10')
    await expect(page.locator('.change-box')).toContainText('0,00')
    await page.locator('.validate-btn').click()
    await expect(page.locator('.paid-part')).toHaveCount(2)

    // Friend 3 pays the rest: 30,09 rounds to 30,10, so 10,05 is due.
    await expect(page.locator('.validate-btn')).toContainText('Encaisser la part 3 · 10,05 €')
    await expect(page.locator('.total-value')).toHaveText('10,05')
    await typeAmount(page, '15')
    await expect(page.locator('.change-box')).toContainText('4,95')
    await page.locator('.validate-btn').click()

    await expect(page.locator('.success-icon')).toBeVisible()
    // The change of the last cash part is the one read on the done screen.
    await expect(page.locator('.receipt-change')).toContainText('4,95')
    const [call] = await ipcCalls(page, 'checkout')
    expect(call.payments).toEqual([{ method: 'cash' }])
  })

  test('cash, card, then cash again: the two cash parts are merged in one cash line', async ({
    page,
  }) => {
    await toPayment(page)
    await page.locator('.split-btn', { hasText: '3' }).click()
    // 34,59 in 3: 11,53 each. Friend 1 pays in cash (11,55 due), 20 € given.
    await expect(page.locator('.validate-btn')).toContainText('Encaisser la part 1 · 11,55 €')
    await typeAmount(page, '20')
    await page.locator('.validate-btn').click()
    await expect(page.locator('.paid-part').nth(0)).toContainText('rendu 8,45 €')
    // Friend 2 pays by card.
    await page.locator('.card-btn').click()
    await page.locator('.card-confirm').click()
    // Friend 3 pays the rest in cash: the rounding of the one cash line (23,06 -> 23,05) is
    // shared, so this part is 11,50.
    await page.locator('.cash-btn').click()
    await expect(page.locator('.validate-btn')).toContainText('Encaisser la part 3 · 11,50 €')
    await typeAmount(page, '15')
    await expect(page.locator('.change-box')).toContainText('3,50')
    await page.locator('.validate-btn').click()

    await expect(page.locator('.success-icon')).toBeVisible()
    await expect(page.locator('.receipt-change')).toContainText('3,50')
    const [call] = await ipcCalls(page, 'checkout')
    // The card part with its amount, ONE cash part without amount for both cash parts.
    expect(call.payments).toEqual([{ method: 'card', amount_cents: 1153 }, { method: 'cash' }])
  })
})

test.describe('P3: the change shown on the done screen', () => {
  test('cash then card: the change was handed back at the table, none is shown at the end', async ({
    page,
  }) => {
    await toPayment(page)
    await page.locator('.split-btn', { hasText: '3' }).click()
    // Friend 1 pays in cash with 20 €: the change is read on the part, once.
    await typeAmount(page, '20')
    await page.locator('.validate-btn').click()
    await expect(page.locator('.paid-part').nth(0)).toContainText('rendu 8,45 €')
    // Friends 2 and 3 pay by card: the sale ends on a card part.
    await page.locator('.card-btn').click()
    await page.locator('.card-confirm').click()
    await page.locator('.card-confirm').click()

    await expect(page.locator('.success-icon')).toBeVisible()
    await expect(page.getByTestId('done-change')).toHaveCount(0)
    const [call] = await ipcCalls(page, 'checkout')
    expect(call.payments).toEqual([
      { method: 'card', amount_cents: 1153 },
      { method: 'card', amount_cents: 1153 },
      { method: 'cash' },
    ])
  })
})

test.describe('P3: the method stays between parts', () => {
  test('a card part keeps the card selected, the rest row offers the others', async ({ page }) => {
    await toPayment(page)
    await page.locator('.split-btn', { hasText: '3' }).click()
    await page.locator('.card-btn').click()
    await page.locator('.card-confirm').click()
    await expect(page.locator('.card-btn')).toHaveClass(/active/u)
    await expect(page.getByTestId('rest-methods-label')).toContainText('Reste')
    await expect(page.locator('.methods .method-btn')).toHaveText([
      'Espèces',
      'Carte',
      'Chèques-repas',
    ])
    await expect(page.locator('.validate-btn')).toContainText('Encaisser la part 2')
    await page.locator('.card-confirm').click()
    // Part 3 is on card too: nothing but card parts, no cash line.
    await page.locator('.card-confirm').click()
    await expect(page.locator('.success-icon')).toBeVisible()
    const [call] = await ipcCalls(page, 'checkout')
    expect(call.payments).toEqual([
      { method: 'card', amount_cents: 1153 },
      { method: 'card', amount_cents: 1153 },
      { method: 'card', amount_cents: 1153 },
    ])
  })
})
